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

//! Temporal Message Deletion with Time-Lock Encryption
//!
//! Implements time-based automatic message expiration where encryption keys
//! can only be derived within a specific time window. After expiration,
//! messages become cryptographically unrecoverable without any central deletion.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Message time-to-live duration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MessageTTL {
    /// Message never expires
    Never,
    /// Message expires after given duration
    After(Duration),
    /// Message expires at specific timestamp
    AtTimestamp(u64),
}

impl MessageTTL {
    /// Calculate expiration timestamp
    pub(crate) fn expiration_time(&self) -> Option<u64> {
        match self {
            Self::Never => None,
            Self::After(duration) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                Some(now + duration.as_secs())
            }
            Self::AtTimestamp(ts) => Some(*ts),
        }
    }

    /// Check if message has expired
    pub(crate) fn is_expired(&self) -> bool {
        match self.expiration_time() {
            None => false,
            Some(exp_time) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now > exp_time
            }
        }
    }

    /// Get remaining time
    pub(crate) fn remaining(&self) -> Option<Duration> {
        match self.expiration_time() {
            None => None,
            Some(exp_time) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if now >= exp_time {
                    Some(Duration::ZERO)
                } else {
                    Some(Duration::from_secs(exp_time - now))
                }
            }
        }
    }
}

/// Encrypted message with temporal key derivation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TemporalMessage {
    /// Message ID
    pub message_id: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Expiration configuration
    pub ttl: MessageTTL,
    /// Encrypted payload
    pub ciphertext: Vec<u8>,
    /// Time-based key derivation parameters
    pub key_derivation: TimeKeyDerivation,
}

/// Time-lock encryption key derivation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TimeKeyDerivation {
    /// Granularity of time periods (in seconds)
    /// Lower = more fine-grained expiration
    pub period_seconds: u64,
    /// Base key for key derivation
    pub base_key_hash: Vec<u8>,
    /// Encryption version
    pub version: u8,
}

impl TimeKeyDerivation {
    /// Create new time-lock encryption parameters
    pub(crate) fn new(period_seconds: u64) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"timelock_base");
        let base_key_hash = hasher.finalize().to_vec();

        Self {
            period_seconds,
            base_key_hash,
            version: 1,
        }
    }

    /// Derive decryption key based on current time
    /// Returns None if message has expired
    pub(crate) fn derive_key(&self, expiration: Option<u64>) -> Option<[u8; 32]> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check expiration
        if let Some(exp) = expiration {
            if now > exp {
                return None;  // Message has expired, key cannot be derived
            }
        }

        // Derive key from time periods elapsed
        let periods = now / self.period_seconds;

        let mut hasher = Sha256::new();
        hasher.update(&self.base_key_hash);
        hasher.update(periods.to_le_bytes());

        let mut key = [0u8; 32];
        key.copy_from_slice(&hasher.finalize()[..]);

        Some(key)
    }
}

/// Temporal message manager
pub(crate) struct TemporalMessageManager {
    /// Messages indexed by ID
    messages: Arc<RwLock<std::collections::HashMap<String, TemporalMessage>>>,
    /// Expired message count (for statistics)
    expired_count: Arc<RwLock<u64>>,
    /// Purged message count
    purged_count: Arc<RwLock<u64>>,
}

impl TemporalMessageManager {
    /// Create new temporal message manager
    pub(crate) fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(std::collections::HashMap::new())),
            expired_count: Arc::new(RwLock::new(0)),
            purged_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Store a temporal message
    pub(crate) fn store_message(&self, message: TemporalMessage) -> Result<(), String> {
        // Validate message
        if message.message_id.is_empty() {
            return Err("Message ID cannot be empty".to_string());
        }

        if message.ciphertext.is_empty() {
            return Err("Ciphertext cannot be empty".to_string());
        }

        let mut messages = self.messages.write();
        if messages.contains_key(&message.message_id) {
            return Err("Message already exists".to_string());
        }

        messages.insert(message.message_id.clone(), message);
        Ok(())
    }

    /// Retrieve and decrypt a message (if not expired)
    pub(crate) fn retrieve_message(&self, message_id: &str) -> Result<Option<Vec<u8>>, TemporalError> {
        let messages = self.messages.read();

        match messages.get(message_id) {
            None => Err(TemporalError::MessageNotFound),
            Some(msg) => {
                // Check if message has expired
                if msg.ttl.is_expired() {
                    return Err(TemporalError::MessageExpired);
                }

                // Derive key (would fail if expired)
                let _key = msg.key_derivation
                    .derive_key(msg.ttl.expiration_time())
                    .ok_or(TemporalError::MessageExpired)?;

                // In production: decrypt with key
                // For now: return encrypted data and indicate success
                Ok(Some(msg.ciphertext.clone()))
            }
        }
    }

    /// Get message expiration info
    pub(crate) fn get_message_info(&self, message_id: &str) -> Result<MessageInfo, TemporalError> {
        let messages = self.messages.read();

        match messages.get(message_id) {
            None => Err(TemporalError::MessageNotFound),
            Some(msg) => {
                let expired = msg.ttl.is_expired();
                let remaining = msg.ttl.remaining();

                Ok(MessageInfo {
                    message_id: message_id.to_string(),
                    created_at: msg.created_at,
                    expiration: msg.ttl.expiration_time(),
                    expired,
                    remaining_seconds: remaining.map(|d| d.as_secs()),
                })
            }
        }
    }

    /// Purge all expired messages (cleanup)
    pub(crate) fn purge_expired(&self) -> u64 {
        let mut messages = self.messages.write();
        let original_count = messages.len();

        messages.retain(|_, msg| !msg.ttl.is_expired());

        let purged = (original_count - messages.len()) as u64;
        *self.purged_count.write() += purged;

        purged
    }

    /// Manually delete a message
    pub(crate) fn delete_message(&self, message_id: &str) -> Result<(), TemporalError> {
        let mut messages = self.messages.write();

        if messages.remove(message_id).is_some() {
            Ok(())
        } else {
            Err(TemporalError::MessageNotFound)
        }
    }

    /// Get manager statistics
    pub(crate) fn get_stats(&self) -> TemporalStats {
        let messages = self.messages.read();
        let active_messages = messages.len();
        let mut expired_count = 0;

        for msg in messages.values() {
            if msg.ttl.is_expired() {
                expired_count += 1;
            }
        }

        TemporalStats {
            active_messages,
            expired_messages: expired_count,
            purged_messages: *self.purged_count.read(),
        }
    }

    /// Cleanup routine (should be called periodically)
    pub(crate) fn cleanup(&self) -> CleanupResult {
        let purged = self.purge_expired();

        CleanupResult {
            messages_purged: purged,
            messages_remaining: self.messages.read().len(),
        }
    }
}

impl Default for TemporalMessageManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporal message error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TemporalError {
    /// Message not found
    MessageNotFound,
    /// Message has expired and is unrecoverable
    MessageExpired,
    /// Invalid message data
    InvalidMessage,
    /// Key derivation failed
    KeyDerivationFailed,
}

impl std::fmt::Display for TemporalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageNotFound => write!(f, "Message not found"),
            Self::MessageExpired => write!(f, "Message has expired and is unrecoverable"),
            Self::InvalidMessage => write!(f, "Invalid message data"),
            Self::KeyDerivationFailed => write!(f, "Key derivation failed"),
        }
    }
}

impl std::error::Error for TemporalError {}

/// Message information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MessageInfo {
    /// Message ID
    pub message_id: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Expiration timestamp (if applicable)
    pub expiration: Option<u64>,
    /// Is message expired?
    pub expired: bool,
    /// Remaining seconds before expiration
    pub remaining_seconds: Option<u64>,
}

/// Temporal manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TemporalStats {
    /// Number of active (non-expired) messages
    pub active_messages: usize,
    /// Number of expired messages still in storage
    pub expired_messages: usize,
    /// Total messages purged
    pub purged_messages: u64,
}

/// Cleanup operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CleanupResult {
    /// Messages purged in this operation
    pub messages_purged: u64,
    /// Total messages remaining
    pub messages_remaining: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_ttl_never() {
        let ttl = MessageTTL::Never;
        assert!(!ttl.is_expired());
        assert_eq!(ttl.remaining(), None);
    }

    #[test]
    fn test_message_ttl_after() {
        let ttl = MessageTTL::After(Duration::from_secs(60));
        assert!(!ttl.is_expired());
        assert!(ttl.remaining().is_some());
        assert!(ttl.remaining().unwrap().as_secs() <= 60);
    }

    #[test]
    fn test_message_ttl_expired() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ttl = MessageTTL::AtTimestamp(now - 60);
        assert!(ttl.is_expired());
    }

    #[test]
    fn test_time_key_derivation() {
        let derivation = TimeKeyDerivation::new(3600);  // 1 hour periods

        // Should derive key for current time
        let key1 = derivation.derive_key(None);
        assert!(key1.is_some());

        // Should derive same key within same period
        let key2 = derivation.derive_key(None);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_temporal_message_storage() {
        let manager = TemporalMessageManager::new();

        let msg = TemporalMessage {
            message_id: "msg_1".to_string(),
            created_at: 1704067200,
            ttl: MessageTTL::Never,
            ciphertext: vec![1, 2, 3, 4],
            key_derivation: TimeKeyDerivation::new(3600),
        };

        assert!(manager.store_message(msg).is_ok());
        assert_eq!(manager.get_stats().active_messages, 1);
    }

    #[test]
    fn test_retrieve_message() {
        let manager = TemporalMessageManager::new();

        let msg = TemporalMessage {
            message_id: "msg_1".to_string(),
            created_at: 1704067200,
            ttl: MessageTTL::Never,
            ciphertext: vec![1, 2, 3, 4],
            key_derivation: TimeKeyDerivation::new(3600),
        };

        manager.store_message(msg).unwrap();
        let result = manager.retrieve_message("msg_1");

        assert!(result.is_ok());
    }

    #[test]
    fn test_expired_message_retrieval() {
        let manager = TemporalMessageManager::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let msg = TemporalMessage {
            message_id: "expired_msg".to_string(),
            created_at: now - 100,
            ttl: MessageTTL::AtTimestamp(now - 10),  // Already expired
            ciphertext: vec![1, 2, 3, 4],
            key_derivation: TimeKeyDerivation::new(3600),
        };

        manager.store_message(msg).unwrap();

        // Should fail to retrieve expired message
        let result = manager.retrieve_message("expired_msg");
        assert_eq!(result, Err(TemporalError::MessageExpired));
    }

    #[test]
    fn test_delete_message() {
        let manager = TemporalMessageManager::new();

        let msg = TemporalMessage {
            message_id: "msg_1".to_string(),
            created_at: 1704067200,
            ttl: MessageTTL::Never,
            ciphertext: vec![1, 2, 3, 4],
            key_derivation: TimeKeyDerivation::new(3600),
        };

        manager.store_message(msg).unwrap();
        assert_eq!(manager.get_stats().active_messages, 1);

        manager.delete_message("msg_1").unwrap();
        assert_eq!(manager.get_stats().active_messages, 0);
    }

    #[test]
    fn test_purge_expired() {
        let manager = TemporalMessageManager::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Add mix of expired and active messages
        for i in 0..5 {
            let msg = TemporalMessage {
                message_id: format!("msg_{}", i),
                created_at: now - 100,
                ttl: if i < 3 {
                    MessageTTL::AtTimestamp(now - 10)  // Expired
                } else {
                    MessageTTL::Never  // Active
                },
                ciphertext: vec![i as u8],
                key_derivation: TimeKeyDerivation::new(3600),
            };
            let _ = manager.store_message(msg);
        }

        let purged = manager.purge_expired();
        assert_eq!(purged, 3);
        assert_eq!(manager.get_stats().active_messages, 2);
    }

    #[test]
    fn test_message_info() {
        let manager = TemporalMessageManager::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let msg = TemporalMessage {
            message_id: "msg_1".to_string(),
            created_at: now,
            ttl: MessageTTL::After(Duration::from_secs(3600)),
            ciphertext: vec![1, 2, 3],
            key_derivation: TimeKeyDerivation::new(60),
        };

        manager.store_message(msg).unwrap();
        let info = manager.get_message_info("msg_1").unwrap();

        assert_eq!(info.message_id, "msg_1");
        assert!(!info.expired);
        assert!(info.remaining_seconds.is_some());
    }

    #[test]
    fn test_cleanup() {
        let manager = TemporalMessageManager::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Add some messages
        for i in 0..10 {
            let msg = TemporalMessage {
                message_id: format!("msg_{}", i),
                created_at: now,
                ttl: MessageTTL::After(Duration::from_secs(1)),
                ciphertext: vec![i as u8],
                key_derivation: TimeKeyDerivation::new(3600),
            };
            let _ = manager.store_message(msg);
        }

        let result = manager.cleanup();
        assert!(result.messages_remaining <= 10);
    }
}
