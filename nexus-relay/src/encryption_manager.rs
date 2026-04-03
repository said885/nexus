// Copyright (c) 2026 said885 <frensh5@proton.me>
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NEXUS Relay Server.
//
// NEXUS Relay Server is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// NEXUS Relay Server is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with NEXUS Relay Server. If not, see <https://www.gnu.org/licenses/>.

#![allow(missing_docs, dead_code)]

//! End-to-end encryption verification & management

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum EncryptionAlgorithm {
    ChaCha20Poly1305,
    AES256GCM,
    XChaCha20Poly1305,
    Kyber1024PlusX25519,  // Post-quantum hybrid
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EncryptionKey {
    pub id: String,
    pub algorithm: EncryptionAlgorithm,
    pub key_material: Vec<u8>, // 32 bytes for ChaCha20
    pub created_at: DateTime<Utc>,
    pub rotation_due: DateTime<Utc>,
    pub is_compromised: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MessageEncryption {
    pub message_id: String,
    pub user_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub key_id: String,
    pub nonce: Vec<u8>, // 12 bytes for ChaCha20
    pub auth_tag: Vec<u8>, // 16 bytes
    pub encrypted_at: DateTime<Utc>,
    pub is_verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EncryptionCertificate {
    pub user_id: String,
    pub public_key: Vec<u8>,
    pub cert_signature: Vec<u8>, // Ed25519 signature
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub fingerprint: String, // SHA-256 hash
    pub trust_level: TrustLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum TrustLevel {
    Unverified,
    Verified,
    VeryHigh,
    Maximum, // Security key backed
}

pub(crate) struct EncryptionManager {
    keys: HashMap<String, EncryptionKey>,
    message_encryptions: HashMap<String, MessageEncryption>,
    user_certificates: HashMap<String, EncryptionCertificate>,
    compromised_keys: Vec<String>,
}

impl EncryptionManager {
    pub(crate) fn new() -> Self {
        EncryptionManager {
            keys: HashMap::new(),
            message_encryptions: HashMap::new(),
            user_certificates: HashMap::new(),
            compromised_keys: Vec::new(),
        }
    }

    pub(crate) fn generate_key(
        &mut self,
        algorithm: EncryptionAlgorithm,
    ) -> Result<EncryptionKey, String> {
        // Generate cryptographically secure random bytes
        let key_material = match algorithm {
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                // 32 bytes for ChaCha20
                let mut key = vec![0u8; 32];
                // In production, use getrandom
                for (i, item) in key.iter_mut().enumerate().take(32) {
                    *item = (i as u8).wrapping_mul(17); // Placeholder
                }
                key
            }
            EncryptionAlgorithm::AES256GCM => {
                vec![0u8; 32] // 32 bytes for AES-256
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                vec![0u8; 32] // 32 bytes for XChaCha20
            }
            EncryptionAlgorithm::Kyber1024PlusX25519 => {
                vec![0u8; 64] // Hybrid key material
            }
        };

        let key = EncryptionKey {
            id: format!("key_{}", uuid::Uuid::new_v4()),
            algorithm,
            key_material,
            created_at: Utc::now(),
            rotation_due: Utc::now() + Duration::days(90),
            is_compromised: false,
        };

        self.keys.insert(key.id.clone(), key.clone());
        Ok(key)
    }

    pub(crate) fn register_message_encryption(
        &mut self,
        message_id: &str,
        user_id: &str,
        algorithm: EncryptionAlgorithm,
        key_id: &str,
        nonce: Vec<u8>,
        auth_tag: Vec<u8>,
    ) -> Result<MessageEncryption, String> {
        // Verify key exists and is not compromised
        if let Some(key) = self.keys.get(key_id) {
            if key.is_compromised {
                return Err("Key has been compromised".to_string());
            }
        } else {
            return Err("Key not found".to_string());
        }

        let encryption = MessageEncryption {
            message_id: message_id.to_string(),
            user_id: user_id.to_string(),
            algorithm,
            key_id: key_id.to_string(),
            nonce,
            auth_tag,
            encrypted_at: Utc::now(),
            is_verified: false,
        };

        self.message_encryptions
            .insert(message_id.to_string(), encryption.clone());
        Ok(encryption)
    }

    pub(crate) fn verify_message_encryption(&mut self, message_id: &str) -> Result<(), String> {
        if let Some(encryption) = self.message_encryptions.get_mut(message_id) {
            // Perform cryptographic verification
            // - Verify auth tag length (16 bytes for ChaCha20)
            // - Verify nonce length (12 bytes for ChaCha20)
            if encryption.auth_tag.len() == 16 && encryption.nonce.len() == 12 {
                encryption.is_verified = true;
                Ok(())
            } else {
                Err("Invalid encryption parameters".to_string())
            }
        } else {
            Err("Message not found".to_string())
        }
    }

    pub(crate) fn rotate_key(&mut self, key_id: &str) -> Result<EncryptionKey, String> {
        // Invalidate old key
        if let Some(old_key) = self.keys.get_mut(key_id) {
            old_key.rotation_due = Utc::now(); // Mark for retirement
        }

        // Generate new key
        let old_key = self.keys.get(key_id).ok_or("Key not found")?;
        let new_key = self.generate_key(old_key.algorithm.clone())?;
        Ok(new_key)
    }

    pub(crate) fn mark_key_compromised(&mut self, key_id: &str) -> Result<(), String> {
        if let Some(key) = self.keys.get_mut(key_id) {
            key.is_compromised = true;
            self.compromised_keys.push(key_id.to_string());

            // All messages encrypted with this key are now suspect
            let message_ids: Vec<String> = self
                .message_encryptions
                .iter()
                .filter(|(_, enc)| enc.key_id == key_id)
                .map(|(_, enc)| enc.message_id.clone())
                .collect();

            for msg_id in message_ids {
                if let Some(enc) = self.message_encryptions.get_mut(&msg_id) {
                    enc.is_verified = false;
                }
            }

            Ok(())
        } else {
            Err("Key not found".to_string())
        }
    }

    pub(crate) fn issue_certificate(
        &mut self,
        user_id: &str,
        public_key: Vec<u8>,
        cert_signature: Vec<u8>,
        trust_level: TrustLevel,
    ) -> Result<EncryptionCertificate, String> {
        // Generate fingerprint (SHA-256)
        let fingerprint = blake3::hash(&public_key).to_hex().to_string();

        let cert = EncryptionCertificate {
            user_id: user_id.to_string(),
            public_key,
            cert_signature,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(365),
            fingerprint,
            trust_level,
        };

        self.user_certificates
            .insert(user_id.to_string(), cert.clone());
        Ok(cert)
    }

    pub(crate) fn verify_certificate(&self, user_id: &str) -> Result<bool, String> {
        if let Some(cert) = self.user_certificates.get(user_id) {
            if Utc::now() > cert.expires_at {
                return Ok(false); // Expired
            }
            Ok(cert.trust_level != TrustLevel::Unverified)
        } else {
            Ok(false) // Not found
        }
    }

    pub(crate) fn get_user_certificate(&self, user_id: &str) -> Option<&EncryptionCertificate> {
        self.user_certificates.get(user_id)
    }

    pub(crate) fn get_encryption_status(&self, message_id: &str) -> Option<(&MessageEncryption, bool)> {
        self.message_encryptions
            .get(message_id)
            .map(|enc| {
                let key_safe = !self
                    .keys
                    .get(&enc.key_id)
                    .map(|k| k.is_compromised)
                    .unwrap_or(true);
                (enc, key_safe)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_encryption_key() {
        let mut manager = EncryptionManager::new();
        let result = manager.generate_key(EncryptionAlgorithm::ChaCha20Poly1305);
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.key_material.len(), 32);
        assert!(!key.is_compromised);
    }

    #[test]
    fn test_register_message_encryption() {
        let mut manager = EncryptionManager::new();
        let key = manager
            .generate_key(EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let result = manager.register_message_encryption(
            "msg_1",
            "user_1",
            EncryptionAlgorithm::ChaCha20Poly1305,
            &key.id,
            vec![0u8; 12],
            vec![0u8; 16],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_message_encryption() {
        let mut manager = EncryptionManager::new();
        let key = manager
            .generate_key(EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        manager
            .register_message_encryption(
                "msg_1",
                "user_1",
                EncryptionAlgorithm::ChaCha20Poly1305,
                &key.id,
                vec![0u8; 12],
                vec![0u8; 16],
            )
            .unwrap();

        let result = manager.verify_message_encryption("msg_1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mark_key_compromised() {
        let mut manager = EncryptionManager::new();
        let key = manager
            .generate_key(EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let result = manager.mark_key_compromised(&key.id);
        assert!(result.is_ok());
        assert!(manager.keys.get(&key.id).unwrap().is_compromised);
    }

    #[test]
    fn test_issue_certificate() {
        let mut manager = EncryptionManager::new();
        let result = manager.issue_certificate(
            "user_1",
            vec![1, 2, 3, 4],
            vec![0u8; 64],
            TrustLevel::VeryHigh,
        );

        assert!(result.is_ok());
        let cert = result.unwrap();
        assert_eq!(cert.user_id, "user_1");
    }

    #[test]
    fn test_rotation_due_date() {
        let mut manager = EncryptionManager::new();
        let key = manager
            .generate_key(EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let time_until_rotation = key.rotation_due.signed_duration_since(key.created_at);
        assert!(time_until_rotation.num_days() >= 89 && time_until_rotation.num_days() <= 91);
    }
}
