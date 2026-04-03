#![allow(missing_docs, dead_code)]

//! WebSocket Transport Layer Encryption
//!
//! Implements ChaCha20-Poly1305 AEAD encryption for WebSocket frames
//! with per-message nonce tracking and HMAC-SHA256 header authentication.

use std::sync::Arc;

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce as AeadNonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Size of a ChaCha20-Poly1305 nonce
const NONCE_SIZE: usize = 12;

/// Size of a ChaCha20-Poly1305 key
const KEY_SIZE: usize = 32;

/// Size of HMAC-SHA256 output
const HMAC_SIZE: usize = 32;

/// Maximum consecutive nonce increments before rekeying
const MAX_MESSAGES_PER_KEY: u64 = 1_000_000;

/// WebSocket message headers to be authenticated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MessageHeader {
    /// Sender identifier
    pub sender: String,
    /// Recipient identifier (or group name)
    pub recipient: String,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
    /// Unique message identifier
    pub message_id: String,
    /// Encryption version
    pub encryption_version: u8,
}

impl MessageHeader {
    /// Serialize header to bytes for HMAC
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
}

/// Encrypted WebSocket frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EncryptedFrame {
    /// Encryption version
    pub version: u8,
    /// Per-message nonce
    pub nonce: Vec<u8>,
    /// HMAC of header for integrity
    pub header_hmac: Vec<u8>,
    /// Encrypted message payload
    pub ciphertext: Vec<u8>,
}

/// Transport encryption context for a single WebSocket connection
pub(crate) struct TransportEncryption {
    /// ChaCha20-Poly1305 cipher instance
    cipher: ChaCha20Poly1305,
    /// Current session key
    session_key: [u8; KEY_SIZE],
    /// Current message counter (for nonce generation)
    message_counter: Arc<RwLock<u64>>,
    /// HMAC secret key
    hmac_key: [u8; 32],
}

impl TransportEncryption {
    /// Create a new transport encryption context from session key
    pub(crate) fn new(session_key: [u8; KEY_SIZE]) -> Self {
        let cipher =
            ChaCha20Poly1305::new_from_slice(&session_key).expect("Key initialization failed");

        // Derive HMAC key from session key
        let hmac_key = Self::derive_hmac_key(&session_key);

        Self {
            cipher,
            session_key,
            message_counter: Arc::new(RwLock::new(0)),
            hmac_key,
        }
    }

    /// Derive HMAC key using HKDF
    fn derive_hmac_key(base_key: &[u8; KEY_SIZE]) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(None, base_key);
        let mut hmac_key = [0u8; 32];
        hk.expand(b"hmac_key", &mut hmac_key)
            .expect("HKDF expand failed");
        hmac_key
    }

    /// Encrypt a message and authenticate its header
    pub(crate) fn encrypt_message(
        &self,
        header: &MessageHeader,
        plaintext: &[u8],
    ) -> Result<EncryptedFrame, String> {
        // Step 1: Increment counter and derive nonce
        let counter = {
            let mut cnt = self.message_counter.write();
            *cnt += 1;

            // Rekey if necessary
            if *cnt > MAX_MESSAGES_PER_KEY {
                return Err("Message limit reached, rekey required".to_string());
            }

            *cnt
        };

        let nonce = self.derive_nonce_from_counter(counter);

        // Step 2: Compute header HMAC for authentication
        let header_hmac = self.compute_header_hmac(header)?;

        // Step 3: Create additional authenticated data (AAD)
        // AAD includes header bytes + HMAC to prevent header tampering
        let aad = [&header.to_bytes()[..], &header_hmac[..]].concat();

        // Step 4: Encrypt message with ChaCha20-Poly1305
        let aead_nonce = AeadNonce::from_slice(&nonce);
        let payload = Payload {
            msg: plaintext,
            aad: &aad,
        };

        let ciphertext = self
            .cipher
            .encrypt(aead_nonce, payload)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        Ok(EncryptedFrame {
            version: 1,
            nonce: nonce.to_vec(),
            header_hmac,
            ciphertext,
        })
    }

    /// Decrypt a message and verify its integrity
    pub(crate) fn decrypt_message(
        &self,
        frame: &EncryptedFrame,
        expected_header: &MessageHeader,
    ) -> Result<Vec<u8>, String> {
        // Step 1: Verify nonce length
        if frame.nonce.len() != NONCE_SIZE {
            return Err("Invalid nonce length".to_string());
        }

        // Step 2: Verify header HMAC
        let computed_hmac = self.compute_header_hmac(expected_header)?;
        if !constant_time_eq(&frame.header_hmac, &computed_hmac) {
            return Err("Header integrity check failed".to_string());
        }

        // Step 3: Create AAD
        let aad = [&expected_header.to_bytes()[..], &frame.header_hmac[..]].concat();

        // Step 4: Decrypt message
        let aead_nonce = AeadNonce::from_slice(&frame.nonce);
        let payload = Payload {
            msg: &frame.ciphertext,
            aad: &aad,
        };

        self.cipher
            .decrypt(aead_nonce, payload)
            .map_err(|e| format!("Decryption failed: {}", e))
    }

    /// Compute HMAC-SHA256 of message header
    fn compute_header_hmac(&self, header: &MessageHeader) -> Result<Vec<u8>, String> {
        let header_bytes = header.to_bytes();

        // HMAC-SHA256
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.hmac_key)
            .map_err(|e| format!("HMAC init failed: {}", e))?;
        mac.update(&header_bytes);
        let result = mac.finalize();

        Ok(result.into_bytes().to_vec())
    }

    /// Derive a nonce from message counter using counter mode
    fn derive_nonce_from_counter(&self, counter: u64) -> [u8; NONCE_SIZE] {
        let mut hasher = Sha256::new();
        hasher.update(b"nonce");
        hasher.update(counter.to_le_bytes());
        let hash = hasher.finalize();

        let mut nonce = [0u8; NONCE_SIZE];
        nonce.copy_from_slice(&hash[..NONCE_SIZE]);
        nonce
    }

    /// Get current message counter for statistics
    pub(crate) fn get_counter(&self) -> u64 {
        *self.message_counter.read()
    }

    /// Check if rekey is needed
    pub(crate) fn needs_rekey(&self) -> bool {
        *self.message_counter.read() > MAX_MESSAGES_PER_KEY * 95 / 100
    }

    /// Derive new session key using HKDF
    pub(crate) fn derive_new_session_key(&self) -> [u8; KEY_SIZE] {
        let hk = Hkdf::<Sha256>::new(None, &self.session_key);
        let mut new_key = [0u8; KEY_SIZE];
        hk.expand(b"session_rekey", &mut new_key)
            .expect("HKDF expand failed");
        new_key
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

/// Per-message Key Encryption Key (KEK) for forward secrecy
pub(crate) struct PerMessageKEK {
    /// Base session key
    base_key: [u8; KEY_SIZE],
    /// Message index
    message_index: u64,
}

impl PerMessageKEK {
    /// Create new per-message KEK generator
    pub(crate) fn new(base_key: [u8; KEY_SIZE]) -> Self {
        Self {
            base_key,
            message_index: 0,
        }
    }

    /// Derive a unique encryption key for this message
    pub(crate) fn derive_key(&mut self) -> [u8; KEY_SIZE] {
        let hk = Hkdf::<Sha256>::new(None, &self.base_key);
        let mut key = [0u8; KEY_SIZE];

        let context = format!("message_key_{}", self.message_index);
        hk.expand(context.as_bytes(), &mut key)
            .expect("HKDF expand failed");

        self.message_index += 1;
        key
    }

    /// Reset key derivation counter
    pub(crate) fn reset(&mut self) {
        self.message_index = 0;
    }
}

/// HMAC-SHA256 wrapper for header authentication
pub(crate) struct HeaderAuthenticator {
    secret_key: [u8; 32],
}

impl HeaderAuthenticator {
    /// Create new header authenticator
    pub(crate) fn new(secret_key: [u8; 32]) -> Self {
        Self { secret_key }
    }

    /// Compute HMAC of header
    pub(crate) fn authenticate(&self, header: &MessageHeader) -> Vec<u8> {
        let header_bytes = header.to_bytes();
        let mut mac =
            <HmacSha256 as Mac>::new_from_slice(&self.secret_key).expect("HMAC init failed");
        mac.update(&header_bytes);
        mac.finalize().into_bytes().to_vec()
    }

    /// Verify header HMAC
    pub(crate) fn verify(&self, header: &MessageHeader, signature: &[u8]) -> bool {
        let computed = self.authenticate(header);
        constant_time_eq(&computed, signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_key() -> [u8; KEY_SIZE] {
        let mut key = [0u8; KEY_SIZE];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
        key
    }

    fn create_test_header() -> MessageHeader {
        MessageHeader {
            sender: "alice@example.com".to_string(),
            recipient: "bob@example.com".to_string(),
            timestamp: 1704067200000,
            message_id: "msg_123".to_string(),
            encryption_version: 1,
        }
    }

    #[test]
    fn test_transport_encryption_roundtrip() {
        let key = create_test_key();
        let transport = TransportEncryption::new(key);
        let header = create_test_header();
        let plaintext = b"Hello, World!";

        // Encrypt
        let frame = transport
            .encrypt_message(&header, plaintext)
            .expect("Encryption failed");

        assert_eq!(frame.version, 1);
        assert_eq!(frame.nonce.len(), NONCE_SIZE);
        assert_eq!(frame.header_hmac.len(), HMAC_SIZE);
        assert!(!frame.ciphertext.is_empty());

        // Decrypt
        let decrypted = transport
            .decrypt_message(&frame, &header)
            .expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_header_authentication() {
        let key = create_test_key();
        let auth = HeaderAuthenticator::new(key);
        let header = create_test_header();

        let hmac = auth.authenticate(&header);
        assert_eq!(hmac.len(), HMAC_SIZE);

        // Same header should verify
        assert!(auth.verify(&header, &hmac));

        // Modified header should fail
        let mut modified = header.clone();
        modified.sender = "charlie@example.com".to_string();
        assert!(!auth.verify(&modified, &hmac));
    }

    #[test]
    fn test_per_message_kek() {
        let key = create_test_key();
        let mut kek = PerMessageKEK::new(key);

        let key1 = kek.derive_key();
        let key2 = kek.derive_key();
        let key3 = kek.derive_key();

        // All keys should be different
        assert_ne!(key1, key2);
        assert_ne!(key2, key3);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_nonce_uniqueness() {
        let key = create_test_key();
        let transport = TransportEncryption::new(key);
        let header = create_test_header();
        let plaintext = b"Test message";

        let frame1 = transport
            .encrypt_message(&header, plaintext)
            .expect("Encryption 1 failed");

        let frame2 = transport
            .encrypt_message(&header, plaintext)
            .expect("Encryption 2 failed");

        // Nonces should be different for each message
        assert_ne!(frame1.nonce, frame2.nonce);
    }

    #[test]
    fn test_constant_time_comparison() {
        let a = b"Hello";
        let b_same = b"Hello";
        let b_diff = b"World";

        assert!(constant_time_eq(a, b_same));
        assert!(!constant_time_eq(a, b_diff));
    }

    #[test]
    fn test_counter_increment() {
        let key = create_test_key();
        let transport = TransportEncryption::new(key);
        let header = create_test_header();

        assert_eq!(transport.get_counter(), 0);

        transport
            .encrypt_message(&header, b"msg1")
            .expect("Encryption 1 failed");
        assert_eq!(transport.get_counter(), 1);

        transport
            .encrypt_message(&header, b"msg2")
            .expect("Encryption 2 failed");
        assert_eq!(transport.get_counter(), 2);
    }

    #[test]
    fn test_rekey_detection() {
        let key = create_test_key();
        let transport = TransportEncryption::new(key);

        // Manually increment counter to beyond threshold
        *transport.message_counter.write() = MAX_MESSAGES_PER_KEY * 95 / 100 + 1;

        assert!(transport.needs_rekey());
    }

    #[test]
    fn test_decrypt_with_wrong_header() {
        let key = create_test_key();
        let transport = TransportEncryption::new(key);
        let header = create_test_header();
        let plaintext = b"Secret message";

        let frame = transport
            .encrypt_message(&header, plaintext)
            .expect("Encryption failed");

        // Try to decrypt with different header
        let mut wrong_header = header.clone();
        wrong_header.sender = "attacker@example.com".to_string();

        let result = transport.decrypt_message(&frame, &wrong_header);
        assert!(result.is_err());
    }
}
