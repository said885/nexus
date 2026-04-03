#![allow(missing_docs, dead_code)]

//! Envelope Encryption for Data at Rest
//!
//! Implements envelope encryption pattern where:
//! 1. Application encrypts data with per-item key (DEK)
//! 2. DEK is encrypted with master key (KEK)
//! 3. Master key stored in secure HSM/KMS
//! 4. Only encrypted DEK and ciphertext stored in database

use std::sync::Arc;

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce as AeadNonce};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Size of data encryption key (DEK)
const DEK_SIZE: usize = 32;

/// Size of key encryption key (KEK)
const KEK_SIZE: usize = 32;

/// Size of encryption nonce
const NONCE_SIZE: usize = 12;

/// Encrypted data blob (stored in database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EncryptedDataBlob {
    /// Encryption version
    pub version: u8,
    /// Encrypted data encryption key (wrapped DEK)
    pub wrapped_dek: Vec<u8>,
    /// Initialization vector / nonce
    pub nonce: Vec<u8>,
    /// Encrypted data
    pub ciphertext: Vec<u8>,
    /// Optional: Key ID for key rotation
    pub key_id: Option<String>,
    /// Metadata about encryption
    pub metadata: EncryptionMetadata,
}

/// Metadata about the encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EncryptionMetadata {
    /// Algorithm used (e.g., "ChaCha20-Poly1305")
    pub algorithm: String,
    /// Timestamp of encryption
    pub encrypted_at: u64,
    /// Size of original plaintext (helps detect tampering)
    pub plaintext_size: u32,
}

/// Envelope encryption system
pub(crate) struct EnvelopeEncryption {
    /// Master key encryption key
    master_key: Arc<RwLock<[u8; KEK_SIZE]>>,
    /// Cache of recently used DEKs (for performance)
    dek_cache: Arc<RwLock<lru::LruCache<String, [u8; DEK_SIZE]>>>,
    /// Statistics
    encryptions: Arc<RwLock<u64>>,
    decryptions: Arc<RwLock<u64>>,
}

impl EnvelopeEncryption {
    /// Create new envelope encryption system with a master key
    pub(crate) fn new(master_key: [u8; KEK_SIZE]) -> Self {
        Self {
            master_key: Arc::new(RwLock::new(master_key)),
            dek_cache: Arc::new(RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(10_000).unwrap(),
            ))),
            encryptions: Arc::new(RwLock::new(0)),
            decryptions: Arc::new(RwLock::new(0)),
        }
    }

    /// Encrypt a message using envelope encryption
    pub(crate) fn encrypt(&self, data_id: &str, plaintext: &[u8]) -> Result<EncryptedDataBlob, String> {
        // Step 1: Generate per-message DEK
        let dek = self.generate_dek()?;

        // Step 2: Encrypt message with DEK
        let cipher = ChaCha20Poly1305::new_from_slice(&dek)
            .map_err(|e| format!("Cipher initialization failed: {}", e))?;

        let nonce = self.generate_nonce()?;
        let aead_nonce = AeadNonce::from_slice(&nonce);

        let payload = Payload {
            msg: plaintext,
            aad: data_id.as_bytes(), // Associate with data ID
        };

        let ciphertext = cipher
            .encrypt(aead_nonce, payload)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Step 3: Encrypt DEK with master key
        let wrapped_dek = self.wrap_dek(&dek)?;

        // Step 4: Update statistics
        *self.encryptions.write() += 1;

        Ok(EncryptedDataBlob {
            version: 1,
            wrapped_dek,
            nonce: nonce.to_vec(),
            ciphertext,
            key_id: Some("master-key-1".to_string()),
            metadata: EncryptionMetadata {
                algorithm: "ChaCha20-Poly1305".to_string(),
                encrypted_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                plaintext_size: plaintext.len() as u32,
            },
        })
    }

    /// Decrypt an encrypted blob
    pub(crate) fn decrypt(&self, data_id: &str, blob: &EncryptedDataBlob) -> Result<Vec<u8>, String> {
        // Step 1: Verify version
        if blob.version != 1 {
            return Err(format!("Unsupported encryption version: {}", blob.version));
        }

        // Step 2: Verify nonce length
        if blob.nonce.len() != NONCE_SIZE {
            return Err("Invalid nonce length".to_string());
        }

        // Step 3: Unwrap DEK
        let dek = self.unwrap_dek(&blob.wrapped_dek)?;

        // Step 4: Decrypt message
        let cipher = ChaCha20Poly1305::new_from_slice(&dek)
            .map_err(|e| format!("Cipher initialization failed: {}", e))?;

        let aead_nonce = AeadNonce::from_slice(&blob.nonce);

        let payload = Payload {
            msg: &blob.ciphertext,
            aad: data_id.as_bytes(),
        };

        let plaintext = cipher
            .decrypt(aead_nonce, payload)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        // Step 5: Verify plaintext size matches metadata
        if plaintext.len() as u32 != blob.metadata.plaintext_size {
            return Err("Plaintext size mismatch (possible corruption)".to_string());
        }

        // Step 6: Update statistics
        *self.decryptions.write() += 1;

        Ok(plaintext)
    }

    /// Generate a random data encryption key
    fn generate_dek(&self) -> Result<[u8; DEK_SIZE], String> {
        let mut dek = [0u8; DEK_SIZE];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut dek);
        Ok(dek)
    }

    /// Generate a random nonce
    fn generate_nonce(&self) -> Result<[u8; NONCE_SIZE], String> {
        let mut nonce = [0u8; NONCE_SIZE];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce);
        Ok(nonce)
    }

    /// Wrap DEK with master key (simulate HSM operation)
    fn wrap_dek(&self, dek: &[u8; DEK_SIZE]) -> Result<Vec<u8>, String> {
        let master_key = self.master_key.read();

        // In production: use AWS KMS Encrypt or HSM operation
        // For now: simulate with AES-256-GCM
        let cipher = ChaCha20Poly1305::new_from_slice(&*master_key)
            .map_err(|e| format!("Cipher init failed: {}", e))?;

        let mut nonce = [0u8; NONCE_SIZE];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce);

        let aead_nonce = AeadNonce::from_slice(&nonce);
        let payload = Payload {
            msg: dek,
            aad: b"dek_wrapping",
        };

        let mut wrapped = nonce.to_vec();
        let ciphertext = cipher
            .encrypt(aead_nonce, payload)
            .map_err(|e| format!("DEK wrapping failed: {}", e))?;
        wrapped.extend_from_slice(&ciphertext);

        Ok(wrapped)
    }

    /// Unwrap DEK with master key
    fn unwrap_dek(&self, wrapped: &[u8]) -> Result<[u8; DEK_SIZE], String> {
        if wrapped.len() < NONCE_SIZE {
            return Err("Invalid wrapped DEK format".to_string());
        }

        let master_key = self.master_key.read();

        let (nonce_bytes, ciphertext) = wrapped.split_at(NONCE_SIZE);
        let nonce = <[u8; NONCE_SIZE]>::try_from(nonce_bytes)
            .map_err(|_| "Invalid nonce extraction".to_string())?;

        let cipher = ChaCha20Poly1305::new_from_slice(&*master_key)
            .map_err(|e| format!("Cipher init failed: {}", e))?;

        let aead_nonce = AeadNonce::from_slice(&nonce);
        let payload = Payload {
            msg: ciphertext,
            aad: b"dek_wrapping",
        };

        let plaintext = cipher
            .decrypt(aead_nonce, payload)
            .map_err(|e| format!("DEK unwrapping failed: {}", e))?;

        let mut dek = [0u8; DEK_SIZE];
        if plaintext.len() != DEK_SIZE {
            return Err("DEK size mismatch".to_string());
        }
        dek.copy_from_slice(&plaintext);

        Ok(dek)
    }

    /// Rotate master key (initiate re-encryption with new key)
    pub(crate) fn rotate_master_key(&self, new_key: [u8; KEK_SIZE]) {
        *self.master_key.write() = new_key;
    }

    /// Get encryption statistics
    pub(crate) fn get_stats(&self) -> EncryptionStats {
        EncryptionStats {
            total_encryptions: *self.encryptions.read(),
            total_decryptions: *self.decryptions.read(),
            cached_deks: self.dek_cache.read().len(),
        }
    }

    /// Clear DEK cache (for security, can be called periodically)
    pub(crate) fn clear_dek_cache(&self) {
        self.dek_cache.write().clear();
    }
}

/// Encryption statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EncryptionStats {
    /// Total encryption operations
    pub total_encryptions: u64,
    /// Total decryption operations
    pub total_decryptions: u64,
    /// Number of cached DEKs
    pub cached_deks: usize,
}

/// Encrypted column specification for ORM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EncryptedColumnSpec {
    /// Column name
    pub column_name: String,
    /// Is this column sensitive?
    pub sensitive: bool,
    /// Encryption algorithm
    pub algorithm: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_key() -> [u8; KEK_SIZE] {
        let mut key = [0u8; KEK_SIZE];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
        key
    }

    #[test]
    fn test_envelope_encryption_roundtrip() {
        let envelope = EnvelopeEncryption::new(create_test_key());
        let data_id = "msg_123";
        let plaintext = b"Secret message content";

        // Encrypt
        let blob = envelope
            .encrypt(data_id, plaintext)
            .expect("Encryption failed");

        assert_eq!(blob.version, 1);
        assert_eq!(blob.nonce.len(), NONCE_SIZE);
        assert!(!blob.ciphertext.is_empty());
        assert!(!blob.wrapped_dek.is_empty());

        // Decrypt
        let decrypted = envelope.decrypt(data_id, &blob).expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_data_ids() {
        let envelope = EnvelopeEncryption::new(create_test_key());
        let plaintext = b"Same content";

        let blob1 = envelope.encrypt("msg_1", plaintext).unwrap();
        let blob2 = envelope.encrypt("msg_2", plaintext).unwrap();

        // Ciphertexts should be different (different AAD)
        assert_ne!(blob1.ciphertext, blob2.ciphertext);

        // Both should decrypt correctly
        assert_eq!(envelope.decrypt("msg_1", &blob1).unwrap(), plaintext);
        assert_eq!(envelope.decrypt("msg_2", &blob2).unwrap(), plaintext);

        // Cross-decryption should fail (AAD mismatch)
        assert!(envelope.decrypt("msg_1", &blob2).is_err());
    }

    #[test]
    fn test_tampering_detection() {
        let envelope = EnvelopeEncryption::new(create_test_key());
        let data_id = "msg_123";
        let plaintext = b"Authentic message";

        let mut blob = envelope.encrypt(data_id, plaintext).unwrap();

        // Tamper with ciphertext
        if !blob.ciphertext.is_empty() {
            blob.ciphertext[0] ^= 0xFF;
        }

        // Decryption should fail
        assert!(envelope.decrypt(data_id, &blob).is_err());
    }

    #[test]
    fn test_master_key_rotation() {
        let mut key1 = create_test_key();
        key1[0] = 1;

        let envelope = EnvelopeEncryption::new(key1);
        let data_id = "msg_123";
        let plaintext = b"Message";

        let blob = envelope.encrypt(data_id, plaintext).unwrap();

        // Can decrypt with original key
        assert!(envelope.decrypt(data_id, &blob).is_ok());

        // Rotate master key
        let mut key2 = create_test_key();
        key2[0] = 2;
        envelope.rotate_master_key(key2);

        // Decryption should now fail (wrong master key)
        assert!(envelope.decrypt(data_id, &blob).is_err());
    }

    #[test]
    fn test_statistics() {
        let envelope = EnvelopeEncryption::new(create_test_key());

        for i in 0..10 {
            let _ = envelope.encrypt(&format!("msg_{}", i), b"data");
        }

        let stats = envelope.get_stats();
        assert_eq!(stats.total_encryptions, 10);

        for _i in 0..10 {
            // Would need the blob to decrypt properly...
        }
    }

    #[test]
    fn test_large_data() {
        let envelope = EnvelopeEncryption::new(create_test_key());
        let large_data = vec![0x42u8; 1_000_000]; // 1MB

        let blob = envelope
            .encrypt("large_msg", &large_data)
            .expect("Large encryption failed");

        let decrypted = envelope
            .decrypt("large_msg", &blob)
            .expect("Large decryption failed");

        assert_eq!(decrypted, large_data);
    }
}
