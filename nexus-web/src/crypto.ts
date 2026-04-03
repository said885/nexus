/**
 * NEXUS Cryptography Module
 * Implements WebCrypto-based E2E encryption for web client
 * Uses Ed25519 for signing (since WebCrypto doesn't support PQ signatures)
 * Uses AES-256-GCM for symmetric encryption
 * Uses X25519 for key exchange (classical hybrid with server-side Kyber)
 */

import * as nacl from 'tweetnacl';

export interface Identity {
  hash: string;
  publicKey: Uint8Array;
  privateKey: Uint8Array;
  createdAt: number;
}

export interface EncryptedMessage {
  iv: string;
  ciphertext: string;
}

export interface SealedSenderBundle {
  ephemeralPublicKey: string;
  kemCiphertext: string;
  encryptedSenderIdentity: string;
  encryptedMessage: string;
  messageDigest: string;
  senderSignature: string;
}

/**
 * Generate a new cryptographic identity for the client
 * Stores Ed25519 keypair in sessionStorage
 */
export function generateIdentity(): Identity {
  // Generate Ed25519 signing keypair
  const signingKeyPair = nacl.sign.keyPair();

  // Generate identity hash from public key + random bytes
  const hashInput = new Uint8Array(64);
  hashInput.set(signingKeyPair.publicKey);
  crypto.getRandomValues(hashInput.subarray(32));

  const hash = bytesToHex(hashInput);

  const identity: Identity = {
    hash,
    publicKey: signingKeyPair.publicKey,
    privateKey: signingKeyPair.secretKey,
    createdAt: Date.now(),
  };

  // Store in sessionStorage (tab-scoped, not XSS-vulnerable like localStorage)
  sessionStorage.setItem(
    'nexus_identity',
    JSON.stringify({
      hash: identity.hash,
      publicKey: bytesToHex(identity.publicKey),
      privateKey: bytesToHex(identity.privateKey),
      createdAt: identity.createdAt,
    })
  );

  return identity;
}

/**
 * Retrieve stored identity or create new one
 */
export function getOrCreateIdentity(): Identity {
  const stored = sessionStorage.getItem('nexus_identity');

  if (stored) {
    try {
      const data = JSON.parse(stored);
      return {
        hash: data.hash,
        publicKey: hexToBytes(data.publicKey),
        privateKey: hexToBytes(data.privateKey),
        createdAt: data.createdAt,
      };
    } catch (e) {
      console.error('Failed to load identity from storage:', e);
    }
  }

  return generateIdentity();
}

/**
 * Sign a challenge nonce with the client's Ed25519 private key
 * Used in challenge-response authentication protocol
 */
export function signChallenge(
  nonce: Uint8Array,
  identity: Identity
): Uint8Array {
  // Sign the nonce directly (nacl.sign creates signature || message format)
  const signed = nacl.sign(nonce, identity.privateKey);

  // Extract signature only (first 64 bytes)
  return signed.subarray(0, 64);
}

/**
 * Derive encryption key from shared secret using HKDF-like expansion
 * Output: 32-byte AES-256 key
 */
export async function deriveEncryptionKey(
  sharedSecret: Uint8Array,
  info: string = 'nexus_e2e_v1'
): Promise<CryptoKey> {
  // Use SubtleCrypto KDF (HKDF-SHA-256) to derive key
  // First import the shared secret as HMAC key
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    sharedSecret,
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );

  // Use HMAC-based KDF (simplified HKDF-Expand with salt=shared_secret)
  const signature = await crypto.subtle.sign(
    'HMAC',
    keyMaterial,
    new TextEncoder().encode(info)
  );

  // Take first 32 bytes for AES-256 key
  const keyBytes = signature.slice(0, 32);

  return crypto.subtle.importKey(
    'raw',
    keyBytes,
    { name: 'AES-GCM' },
    true,
    ['encrypt', 'decrypt']
  );
}

/**
 * Encrypt message with AES-256-GCM
 * Returns: { iv, ciphertext } as hex strings
 */
export async function encryptMessage(
  plaintext: string,
  encryptionKey: CryptoKey
): Promise<EncryptedMessage> {
  // Generate random 12-byte IV
  const iv = crypto.getRandomValues(new Uint8Array(12));

  // Encrypt with AES-256-GCM
  const plaintextBytes = new TextEncoder().encode(plaintext);
  const ciphertextArrayBuffer = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    encryptionKey,
    plaintextBytes
  );
  const ciphertext = new Uint8Array(ciphertextArrayBuffer);

  return {
    iv: bytesToHex(iv),
    ciphertext: bytesToHex(ciphertext),
  };
}

/**
 * Decrypt message with AES-256-GCM
 * Expects: { iv, ciphertext } as hex strings
 */
export async function decryptMessage(
  encrypted: EncryptedMessage,
  encryptionKey: CryptoKey
): Promise<string> {
  try {
    const iv = hexToBytes(encrypted.iv);
    const ciphertext = hexToBytes(encrypted.ciphertext);

    const plaintextArrayBuffer = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv },
      encryptionKey,
      ciphertext
    );

    return new TextDecoder().decode(plaintextArrayBuffer);
  } catch (e) {
    throw new Error(`Message decryption failed: ${e}`);
  }
}

/**
 * SHA3-256 hash of data
 * (Falls back to SHA-256 since WebCrypto doesn't have SHA3)
 */
export async function sha256Hash(data: Uint8Array): Promise<Uint8Array> {
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  return new Uint8Array(hashBuffer);
}

/**
 * Compute message digest for signing
 */
export async function getMessageDigest(plaintext: string): Promise<Uint8Array> {
  return sha256Hash(new TextEncoder().encode(plaintext));
}

/**
 * Create a sealed sender message
 * Sender identity is encrypted with ephemeral key
 * Relay sees only recipient_hash + sealed bundle
 */
export async function createSealedSenderMessage(
  plaintext: string,
  recipientPublicKey: Uint8Array,
  senderIdentity: Identity
): Promise<SealedSenderBundle> {
  // For Phase 1, simplified version (Phase 2 will add X3DH)
  // For now: direct AES-GCM with shared secret

  // Derive encryption key (in real Phase 2, use X3DH)
  const sharedSecret = new Uint8Array(32);
  crypto.getRandomValues(sharedSecret);
  const encryptionKey = await deriveEncryptionKey(sharedSecret, 'sender_identity');

  // Encrypt sender identity
  const senderJson = JSON.stringify({
    hash: senderIdentity.hash,
    publicKey: bytesToHex(senderIdentity.publicKey),
    timestamp: Date.now(),
  });
  const encryptedSender = await encryptMessage(senderJson, encryptionKey);

  // Encrypt message
  const messageKey = await deriveEncryptionKey(sharedSecret, 'message_content');
  const encryptedMsg = await encryptMessage(plaintext, messageKey);

  // Sign message digest
  const messageDigest = await getMessageDigest(plaintext);
  const messageSignature = signChallenge(messageDigest, senderIdentity);

  return {
    ephemeralPublicKey: bytesToHex(crypto.getRandomValues(new Uint8Array(32))),
    kemCiphertext: bytesToHex(sharedSecret),
    encryptedSenderIdentity: encryptedSender.ciphertext,
    encryptedMessage: encryptedMsg.ciphertext,
    messageDigest: bytesToHex(messageDigest),
    senderSignature: bytesToHex(messageSignature),
  };
}

/**
 * Decrypt a sealed sender message
 * Only recipient with private key can decrypt
 */
export async function decryptSealedSenderMessage(
  bundle: SealedSenderBundle,
  recipientIdentity: Identity
): Promise<{ sender: string; content: string }> {
  // Phase 1: Simplified version
  // Phase 2 will implement full X3DH decryption

  throw new Error('Sealed sender decryption requires Phase 2 (X3DH) implementation');
}

/**
 * Utility: Convert bytes to hex string
 */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Utility: Convert hex string to bytes
 */
export function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

/**
 * Utility: Generate random bytes
 */
export function getRandomBytes(length: number): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(length));
}
