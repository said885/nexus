use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Post-quantum identity for the desktop client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NexusIdentity {
    pub kyber_public_key: Vec<u8>,
    pub kyber_private_key: Vec<u8>,
    pub x25519_public_key: Vec<u8>,
    pub x25519_private_key: Vec<u8>,
    pub signing_public_key: Vec<u8>,
    pub signing_private_key: Vec<u8>,
    pub identity_hash: String,
    pub fingerprint: String,
}

/// Double Ratchet state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatchetState {
    pub root_key: Vec<u8>,
    pub send_chain_key: Vec<u8>,
    pub recv_chain_key: Option<Vec<u8>>,
    pub send_counter: u64,
    pub recv_counter: u64,
    pub dh_keypair: Option<DhKeyPair>,
    pub their_dh_public: Option<Vec<u8>>,
    pub previous_chain_length: u64,
    pub skipped_keys: Vec<SkippedKey>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhKeyPair {
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkippedKey {
    pub dh_public: Vec<u8>,
    pub counter: u64,
    pub message_key: Vec<u8>,
}

/// Encrypted message structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub dh_public: Vec<u8>,
    pub msg_n: u64,
    pub prev_chain_len: u64,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

/// Pre-key bundle for X3DH
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreKeyBundle {
    pub identity_key: Vec<u8>,
    pub signed_prekey: Vec<u8>,
    pub signed_prekey_signature: Vec<u8>,
    pub one_time_prekey: Option<Vec<u8>>,
}

/// Crypto operations
pub struct NexusCrypto;

impl NexusCrypto {
    pub fn new() -> Self {
        Self
    }

    /// Generate a new identity with post-quantum keys
    pub fn generate_identity(&self) -> Result<NexusIdentity, String> {
        // Generate random keys (in production, use actual PQ crypto libraries)
        let kyber_public = generate_random_bytes(1568);
        let kyber_private = generate_random_bytes(3168);
        let x25519_public = generate_random_bytes(32);
        let x25519_private = generate_random_bytes(32);
        let signing_public = generate_random_bytes(1952);
        let signing_private = generate_random_bytes(4000);

        // Compute identity hash (BLAKE3)
        let mut hasher = blake3::Hasher::new();
        hasher.update(&kyber_public);
        hasher.update(&x25519_public);
        hasher.update(&signing_public);
        let identity_hash = hex::encode(hasher.finalize().as_bytes());

        // Compute fingerprint
        let fingerprint = format!(
            "{} {} {} {}",
            &identity_hash[0..8],
            &identity_hash[8..16],
            &identity_hash[16..24],
            &identity_hash[24..32]
        );

        Ok(NexusIdentity {
            kyber_public_key: kyber_public,
            kyber_private_key: kyber_private,
            x25519_public_key: x25519_public,
            x25519_private_key: x25519_private,
            signing_public_key: signing_public,
            signing_private_key: signing_private,
            identity_hash,
            fingerprint,
        })
    }

    /// Initialize ratchet state from shared secret (sender side)
    pub fn init_ratchet_sender(
        &self,
        shared_secret: &[u8],
        their_dh_public: &[u8],
    ) -> Result<RatchetState, String> {
        if shared_secret.len() < 32 {
            return Err("Shared secret too short".into());
        }

        // Generate DH keypair
        let dh_private = generate_random_bytes(32);
        let dh_public = derive_key(&dh_private, b"dh-pub");

        // Perform DH and derive keys
        let dh_output = self.perform_dh(&dh_private, their_dh_public)?;
        let (root_key, send_chain_key) = self.kdf_rk(shared_secret, &dh_output);

        Ok(RatchetState {
            root_key,
            send_chain_key,
            recv_chain_key: None,
            send_counter: 0,
            recv_counter: 0,
            dh_keypair: Some(DhKeyPair {
                public_key: dh_public,
                private_key: dh_private,
            }),
            their_dh_public: Some(their_dh_public.to_vec()),
            previous_chain_length: 0,
            skipped_keys: Vec::new(),
        })
    }

    /// Initialize ratchet state from shared secret (receiver side)
    pub fn init_ratchet_receiver(
        &self,
        shared_secret: &[u8],
        local_dh_private: &[u8],
    ) -> Result<RatchetState, String> {
        if shared_secret.len() < 32 {
            return Err("Shared secret too short".into());
        }

        let dh_public = derive_key(local_dh_private, b"dh-pub");

        Ok(RatchetState {
            root_key: shared_secret.to_vec(),
            send_chain_key: Vec::new(),
            recv_chain_key: None,
            send_counter: 0,
            recv_counter: 0,
            dh_keypair: Some(DhKeyPair {
                public_key: dh_public,
                private_key: local_dh_private.to_vec(),
            }),
            their_dh_public: None,
            previous_chain_length: 0,
            skipped_keys: Vec::new(),
        })
    }

    /// X3DH key agreement (sender side)
    pub fn x3dh_sender(
        &self,
        my_identity_private: &[u8],
        my_ephemeral_private: &[u8],
        their_bundle: &PreKeyBundle,
    ) -> Result<Vec<u8>, String> {
        // DH1 = DH(IKa, SPKb)
        let dh1 = self.perform_dh(my_identity_private, &their_bundle.signed_prekey)?;

        // DH2 = DH(EKa, IKb)
        let dh2 = self.perform_dh(my_ephemeral_private, &their_bundle.identity_key)?;

        // DH3 = DH(EKa, SPKb)
        let dh3 = self.perform_dh(my_ephemeral_private, &their_bundle.signed_prekey)?;

        // DH4 = DH(EKa, OPKb) if available
        let mut combined = Vec::new();
        combined.extend_from_slice(&dh1);
        combined.extend_from_slice(&dh2);
        combined.extend_from_slice(&dh3);

        if let Some(opk) = &their_bundle.one_time_prekey {
            let dh4 = self.perform_dh(my_ephemeral_private, opk)?;
            combined.extend_from_slice(&dh4);
        }

        // Derive master secret
        Ok(derive_key(&combined, b"NexusX3DH-v1"))
    }

    /// X3DH key agreement (receiver side)
    pub fn x3dh_receiver(
        &self,
        my_identity_private: &[u8],
        my_signed_prekey_private: &[u8],
        my_one_time_prekey_private: Option<&[u8]>,
        their_identity_public: &[u8],
        their_ephemeral_public: &[u8],
    ) -> Result<Vec<u8>, String> {
        // DH1 = DH(SPKb, IKa)
        let dh1 = self.perform_dh(my_signed_prekey_private, their_identity_public)?;

        // DH2 = DH(IKB, EKa)
        let dh2 = self.perform_dh(my_identity_private, their_ephemeral_public)?;

        // DH3 = DH(SPKb, EKa)
        let dh3 = self.perform_dh(my_signed_prekey_private, their_ephemeral_public)?;

        let mut combined = Vec::new();
        combined.extend_from_slice(&dh1);
        combined.extend_from_slice(&dh2);
        combined.extend_from_slice(&dh3);

        if let Some(opk_priv) = my_one_time_prekey_private {
            let dh4 = self.perform_dh(opk_priv, their_ephemeral_public)?;
            combined.extend_from_slice(&dh4);
        }

        // Derive master secret
        Ok(derive_key(&combined, b"NexusX3DH-v1"))
    }

    /// Perform simplified DH (in production use X25519)
    fn perform_dh(&self, private_key: &[u8], public_key: &[u8]) -> Result<Vec<u8>, String> {
        if private_key.len() < 32 || public_key.len() < 32 {
            return Err("Invalid key size".into());
        }
        // Simplified: XOR + hash (production would use X25519)
        let mut combined = Vec::new();
        combined.extend_from_slice(private_key);
        combined.extend_from_slice(public_key);
        Ok(derive_key(&combined, b"dh"))
    }

    /// KDF for root key derivation
    fn kdf_rk(&self, root_key: &[u8], dh_output: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut combined = Vec::new();
        combined.extend_from_slice(root_key);
        combined.extend_from_slice(dh_output);

        let derived = derive_key(&combined, b"kdf-rk");
        let root_key_out = derived[..32].to_vec();
        let chain_key = derived[32..64].to_vec();

        (root_key_out, chain_key)
    }

    /// KDF for chain key derivation
    fn kdf_ck(&self, chain_key: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let next_chain = derive_key(chain_key, b"chain");
        let message_key = derive_key(chain_key, b"message");
        (next_chain, message_key)
    }

    /// Encrypt a message using the ratchet
    pub fn encrypt(
        &self,
        state: &mut RatchetState,
        plaintext: &[u8],
    ) -> Result<EncryptedMessage, String> {
        let send_chain_key = state.send_chain_key.clone();
        if send_chain_key.is_empty() {
            return Err("No send chain key".into());
        }

        // Derive next chain key and message key
        let (next_chain, message_key) = self.kdf_ck(&send_chain_key);
        state.send_chain_key = next_chain;

        // Encrypt with ChaCha20-Poly1305
        let nonce = generate_random_bytes(12);
        let ciphertext = chacha_encrypt(&message_key, &nonce, plaintext)?;

        let msg_n = state.send_counter;
        state.send_counter += 1;

        let dh_public = state
            .dh_keypair
            .as_ref()
            .map(|kp| kp.public_key.clone())
            .unwrap_or_default();

        Ok(EncryptedMessage {
            dh_public,
            msg_n,
            prev_chain_len: state.previous_chain_length,
            ciphertext,
            nonce,
        })
    }

    /// Decrypt a message using the ratchet
    pub fn decrypt(
        &self,
        state: &mut RatchetState,
        encrypted: &EncryptedMessage,
    ) -> Result<Vec<u8>, String> {
        // Check for skipped keys first
        let skip_idx = state
            .skipped_keys
            .iter()
            .position(|k| k.dh_public == encrypted.dh_public && k.counter == encrypted.msg_n);

        if let Some(idx) = skip_idx {
            let skipped = state.skipped_keys.remove(idx);
            return chacha_decrypt(
                &skipped.message_key,
                &encrypted.nonce,
                &encrypted.ciphertext,
            );
        }

        // Check if we need to ratchet
        if let Some(their_dh) = &state.their_dh_public {
            if *their_dh != encrypted.dh_public {
                // New DH ratchet step
                self.ratchet_step(state, &encrypted.dh_public)?;
            }
        } else {
            // First message - set their DH public
            state.their_dh_public = Some(encrypted.dh_public.clone());
        }

        // Skip message keys if needed
        let recv_chain_key = state.recv_chain_key.clone().unwrap_or_default();
        if recv_chain_key.is_empty() {
            return Err("No receive chain key".into());
        }

        // Derive next chain key and message key
        let (next_chain, message_key) = self.kdf_ck(&recv_chain_key);
        state.recv_chain_key = Some(next_chain);
        state.recv_counter += 1;

        chacha_decrypt(&message_key, &encrypted.nonce, &encrypted.ciphertext)
    }

    /// Perform a DH ratchet step
    fn ratchet_step(&self, state: &mut RatchetState, their_dh_public: &[u8]) -> Result<(), String> {
        let dh_private = state
            .dh_keypair
            .as_ref()
            .ok_or("No DH keypair")?
            .private_key
            .clone();

        // DH with their new key
        let dh_output = self.perform_dh(&dh_private, their_dh_public)?;

        // Derive new root key and receive chain key
        let (new_root, recv_ck) = self.kdf_rk(&state.root_key, &dh_output);
        state.root_key = new_root;
        state.recv_chain_key = Some(recv_ck);

        // Generate new DH keypair
        let new_dh_private = generate_random_bytes(32);
        let new_dh_public = derive_key(&new_dh_private, b"dh-pub");

        // DH with their key again for send chain
        let dh_output2 = self.perform_dh(&new_dh_private, their_dh_public)?;
        let (new_root2, send_ck) = self.kdf_rk(&state.root_key, &dh_output2);
        state.root_key = new_root2;
        state.send_chain_key = send_ck;

        state.dh_keypair = Some(DhKeyPair {
            public_key: new_dh_public,
            private_key: new_dh_private,
        });
        state.their_dh_public = Some(their_dh_public.to_vec());
        state.previous_chain_length = state.send_counter;
        state.send_counter = 0;
        state.recv_counter = 0;

        Ok(())
    }

    /// Sign data with signing key
    pub fn sign(&self, signing_private_key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
        // Simplified signing (production would use Dilithium or Ed25519)
        let mut combined = Vec::new();
        combined.extend_from_slice(signing_private_key);
        combined.extend_from_slice(data);
        Ok(derive_key(&combined, b"sign"))
    }

    /// Verify signature
    pub fn verify(
        &self,
        signing_public_key: &[u8],
        data: &[u8],
        signature: &[u8],
    ) -> Result<bool, String> {
        // Simplified verification
        let mut combined = Vec::new();
        combined.extend_from_slice(signing_public_key);
        combined.extend_from_slice(data);
        let expected = derive_key(&combined, b"sign");
        Ok(expected == signature)
    }
}

/// Encrypt with ChaCha20-Poly1305
fn chacha_encrypt(key: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("Key must be 32 bytes".into());
    }
    if nonce.len() != 12 {
        return Err("Nonce must be 12 bytes".into());
    }

    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;
    let nonce = Nonce::from_slice(nonce);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok(ciphertext)
}

/// Decrypt with ChaCha20-Poly1305
fn chacha_decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("Key must be 32 bytes".into());
    }
    if nonce.len() != 12 {
        return Err("Nonce must be 12 bytes".into());
    }

    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;
    let nonce = Nonce::from_slice(nonce);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(plaintext)
}

fn generate_random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

fn derive_key(key: &[u8], info: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(key);
    hasher.update(info);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let crypto = NexusCrypto::new();
        let identity = crypto.generate_identity().unwrap();

        assert_eq!(identity.kyber_public_key.len(), 1568);
        assert_eq!(identity.kyber_private_key.len(), 3168);
        assert_eq!(identity.x25519_public_key.len(), 32);
        assert_eq!(identity.x25519_private_key.len(), 32);
        assert_eq!(identity.signing_public_key.len(), 1952);
        assert_eq!(identity.signing_private_key.len(), 4000);
        assert_eq!(identity.identity_hash.len(), 64); // hex of 32 bytes
        assert!(!identity.fingerprint.is_empty());

        // Fingerprint should have spaces
        assert!(identity.fingerprint.contains(' '));
    }

    #[test]
    fn test_identity_uniqueness() {
        let crypto = NexusCrypto::new();
        let identity1 = crypto.generate_identity().unwrap();
        let identity2 = crypto.generate_identity().unwrap();

        // Different identities should have different hashes
        assert_ne!(identity1.identity_hash, identity2.identity_hash);
        assert_ne!(identity1.fingerprint, identity2.fingerprint);
    }

    #[test]
    fn test_ratchet_sender_init() {
        let crypto = NexusCrypto::new();
        let shared_secret = generate_random_bytes(32);
        let their_dh_public = generate_random_bytes(32);

        let state = crypto
            .init_ratchet_sender(&shared_secret, &their_dh_public)
            .unwrap();

        assert_eq!(state.root_key.len(), 32);
        assert!(!state.send_chain_key.is_empty());
        assert!(state.recv_chain_key.is_none());
        assert_eq!(state.send_counter, 0);
        assert_eq!(state.recv_counter, 0);
        assert!(state.dh_keypair.is_some());
        assert!(state.their_dh_public.is_some());
    }

    #[test]
    fn test_ratchet_receiver_init() {
        let crypto = NexusCrypto::new();
        let shared_secret = generate_random_bytes(32);
        let dh_private = generate_random_bytes(32);

        let state = crypto
            .init_ratchet_receiver(&shared_secret, &dh_private)
            .unwrap();

        assert_eq!(state.root_key, shared_secret);
        assert!(state.send_chain_key.is_empty());
        assert!(state.recv_chain_key.is_none());
        assert!(state.dh_keypair.is_some());
    }

    #[test]
    fn test_ratchet_encrypt() {
        let crypto = NexusCrypto::new();
        let shared_secret = generate_random_bytes(32);
        let their_dh_public = generate_random_bytes(32);

        let mut state = crypto
            .init_ratchet_sender(&shared_secret, &their_dh_public)
            .unwrap();

        let plaintext = b"Hello, NEXUS!";
        let encrypted = crypto.encrypt(&mut state, plaintext).unwrap();

        // Ciphertext should be different from plaintext
        assert_ne!(encrypted.ciphertext, plaintext);
        // Ciphertext should be plaintext + 16 bytes (auth tag)
        assert_eq!(encrypted.ciphertext.len(), plaintext.len() + 16);
        // Nonce should be 12 bytes
        assert_eq!(encrypted.nonce.len(), 12);
        // Counter should increment
        assert_eq!(state.send_counter, 1);
    }

    #[test]
    fn test_ratchet_multiple_encryptions() {
        let crypto = NexusCrypto::new();
        let shared_secret = generate_random_bytes(32);
        let their_dh_public = generate_random_bytes(32);

        let mut state = crypto
            .init_ratchet_sender(&shared_secret, &their_dh_public)
            .unwrap();

        let plaintext1 = b"Message 1";
        let plaintext2 = b"Message 2";

        let encrypted1 = crypto.encrypt(&mut state, plaintext1).unwrap();
        let encrypted2 = crypto.encrypt(&mut state, plaintext2).unwrap();

        // Different messages should produce different ciphertexts
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        assert_eq!(state.send_counter, 2);
    }

    #[test]
    fn test_x3dh_sender() {
        let crypto = NexusCrypto::new();

        let my_identity_priv = generate_random_bytes(32);
        let my_ephemeral_priv = generate_random_bytes(32);

        let their_bundle = PreKeyBundle {
            identity_key: generate_random_bytes(32),
            signed_prekey: generate_random_bytes(32),
            signed_prekey_signature: generate_random_bytes(64),
            one_time_prekey: Some(generate_random_bytes(32)),
        };

        let sender_secret = crypto
            .x3dh_sender(&my_identity_priv, &my_ephemeral_priv, &their_bundle)
            .unwrap();

        assert_eq!(sender_secret.len(), 32);
    }

    #[test]
    fn test_x3dh_sender_without_one_time_prekey() {
        let crypto = NexusCrypto::new();

        let my_identity_priv = generate_random_bytes(32);
        let my_ephemeral_priv = generate_random_bytes(32);

        let their_bundle = PreKeyBundle {
            identity_key: generate_random_bytes(32),
            signed_prekey: generate_random_bytes(32),
            signed_prekey_signature: generate_random_bytes(64),
            one_time_prekey: None,
        };

        let sender_secret = crypto
            .x3dh_sender(&my_identity_priv, &my_ephemeral_priv, &their_bundle)
            .unwrap();

        assert_eq!(sender_secret.len(), 32);
    }

    #[test]
    fn test_x3dh_receiver() {
        let crypto = NexusCrypto::new();

        let my_identity_priv = generate_random_bytes(32);
        let my_signed_prekey_priv = generate_random_bytes(32);
        let my_one_time_prekey_priv = generate_random_bytes(32);

        let their_identity_pub = generate_random_bytes(32);
        let their_ephemeral_pub = generate_random_bytes(32);

        let receiver_secret = crypto
            .x3dh_receiver(
                &my_identity_priv,
                &my_signed_prekey_priv,
                Some(&my_one_time_prekey_priv),
                &their_identity_pub,
                &their_ephemeral_pub,
            )
            .unwrap();

        assert_eq!(receiver_secret.len(), 32);
    }

    #[test]
    fn test_sign_verify() {
        let crypto = NexusCrypto::new();
        let signing_priv = generate_random_bytes(64);
        let signing_pub = generate_random_bytes(32);
        let data = b"test data";

        let signature = crypto.sign(&signing_priv, data).unwrap();
        let valid = crypto.verify(&signing_pub, data, &signature).unwrap();

        assert!(valid);
    }

    #[test]
    fn test_sign_verify_different_data() {
        let crypto = NexusCrypto::new();
        let signing_priv = generate_random_bytes(64);
        let signing_pub = generate_random_bytes(32);

        let data1 = b"data one";
        let data2 = b"data two";

        let signature1 = crypto.sign(&signing_priv, data1).unwrap();
        let signature2 = crypto.sign(&signing_priv, data2).unwrap();

        // Different data should produce different signatures
        assert_ne!(signature1, signature2);

        // Signature for data1 should not verify against data2
        let valid = crypto.verify(&signing_pub, data2, &signature1).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_chacha_encrypt_decrypt() {
        let key = generate_random_bytes(32);
        let nonce = generate_random_bytes(12);
        let plaintext = b"Hello, ChaCha20!";

        let ciphertext = chacha_encrypt(&key, &nonce, plaintext).unwrap();
        assert_ne!(ciphertext, plaintext);

        let decrypted = chacha_decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_chacha_wrong_key() {
        let key1 = generate_random_bytes(32);
        let key2 = generate_random_bytes(32);
        let nonce = generate_random_bytes(12);
        let plaintext = b"Secret message";

        let ciphertext = chacha_encrypt(&key1, &nonce, plaintext).unwrap();

        // Decryption with wrong key should fail
        let result = chacha_decrypt(&key2, &nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_chacha_invalid_key_size() {
        let short_key = generate_random_bytes(16); // Too short
        let nonce = generate_random_bytes(12);
        let plaintext = b"test";

        let result = chacha_encrypt(&short_key, &nonce, plaintext);
        assert!(result.is_err());
    }

    #[test]
    fn test_chacha_invalid_nonce_size() {
        let key = generate_random_bytes(32);
        let short_nonce = generate_random_bytes(6); // Too short
        let plaintext = b"test";

        let result = chacha_encrypt(&key, &short_nonce, plaintext);
        assert!(result.is_err());
    }

    #[test]
    fn test_kdf_rk() {
        let crypto = NexusCrypto::new();
        let root_key = generate_random_bytes(32);
        let dh_output = generate_random_bytes(32);

        let (new_root, chain_key) = crypto.kdf_rk(&root_key, &dh_output);

        assert_eq!(new_root.len(), 32);
        assert_eq!(chain_key.len(), 32);
        assert_ne!(new_root, root_key);
    }

    #[test]
    fn test_kdf_ck() {
        let crypto = NexusCrypto::new();
        let chain_key = generate_random_bytes(32);

        let (next_chain, message_key) = crypto.kdf_ck(&chain_key);

        assert_eq!(next_chain.len(), 32);
        assert_eq!(message_key.len(), 32);
        assert_ne!(next_chain, chain_key);
        assert_ne!(message_key, chain_key);
    }

    #[test]
    fn test_kdf_ck_deterministic() {
        let crypto = NexusCrypto::new();
        let chain_key = generate_random_bytes(32);

        let (next1, msg1) = crypto.kdf_ck(&chain_key);
        let (next2, msg2) = crypto.kdf_ck(&chain_key);

        assert_eq!(next1, next2);
        assert_eq!(msg1, msg2);
    }

    #[test]
    fn test_random_bytes_uniqueness() {
        let bytes1 = generate_random_bytes(32);
        let bytes2 = generate_random_bytes(32);

        assert_ne!(bytes1, bytes2);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let key = generate_random_bytes(32);
        let info = b"test-info";

        let derived1 = derive_key(&key, info);
        let derived2 = derive_key(&key, info);

        assert_eq!(derived1, derived2);
        assert_eq!(derived1.len(), 32);
    }

    #[test]
    fn test_derive_key_different_inputs() {
        let key1 = generate_random_bytes(32);
        let key2 = generate_random_bytes(32);
        let info = b"test-info";

        let derived1 = derive_key(&key1, info);
        let derived2 = derive_key(&key2, info);

        assert_ne!(derived1, derived2);
    }
}
