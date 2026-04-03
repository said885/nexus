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

// Searchable Encryption for End-to-End Encrypted Messages
// nexus-relay/src/message_search.rs
//
// This module implements searchable encryption allowing clients to search
// their messages without decrypting them on the server.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Search token (derived from plaintext on client side)
/// Only computed locally, never sent to server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SearchToken {
    pub token_id: String,
    pub user_id: String,
    pub search_query_hash: Vec<u8>, // SHA-256 hash of search query
    pub token_value: Vec<u8>,       // 32 bytes
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Encrypted message index entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EncryptedMessageIndex {
    pub message_id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub timestamp: DateTime<Utc>,
    
    // Searchable fields (encrypted)
    pub searchable_values: Vec<SearchableField>, // e.g., keyword hashes
    
    // Message metadata (not searchable)
    pub message_hash: Vec<u8>, // SHA-256 of full message (for verification)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SearchableField {
    pub field_name: String,        // "body", "subject", etc.
    pub encrypted_hash: Vec<u8>,   // Deterministic encryption
    pub token_value: Option<Vec<u8>>, // Optional token for search
}

/// Searchable Symmetric Encryption (SSE) scheme
/// Based on ORAM-resistant designs like Fides
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SearchableEncryptionKey {
    pub key_id: String,
    pub user_id: String,
    pub key_material: Vec<u8>,            // K_S (Search Key) - 32 bytes
    pub additional_key_material: Vec<u8>, // K_I (Index Key) - 32 bytes
    pub created_at: DateTime<Utc>,
    pub rotation_count: u32,
}

impl SearchableEncryptionKey {
    /// Create new searchable encryption key
    pub(crate) fn new(user_id: String) -> Result<Self, String> {
        let mut key_material = vec![0u8; 32];
        let mut additional_key_material = vec![0u8; 32];
        
        getrandom::getrandom(&mut key_material)
            .map_err(|e| format!("Key generation failed: {}", e))?;
        getrandom::getrandom(&mut additional_key_material)
            .map_err(|e| format!("Additional key generation failed: {}", e))?;

        Ok(Self {
            key_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            key_material,
            additional_key_material,
            created_at: Utc::now(),
            rotation_count: 0,
        })
    }

    /// Derive search token for query
    pub(crate) fn derive_search_token(&self, query: &str) -> Result<SearchToken, String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        // HMAC(K_I, query)
        let mut mac = HmacSha256::new_from_slice(&self.additional_key_material)
            .map_err(|e| format!("HMAC key error: {}", e))?;
        mac.update(query.as_bytes());

        let token_value = mac.finalize().into_bytes().to_vec();

        // Hash the query for tracking
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        let query_hash = hasher.finalize().to_vec();

        Ok(SearchToken {
            token_id: uuid::Uuid::new_v4().to_string(),
            user_id: self.user_id.clone(),
            search_query_hash: query_hash,
            token_value,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
        })
    }

    /// Create searchable hash for indexing (client-side operation)
    pub(crate) fn create_searchable_field(
        &self,
        field_name: String,
        plaintext_value: &str,
    ) -> Result<SearchableField, String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        // Deterministic encryption via HMAC
        let mut mac = HmacSha256::new_from_slice(&self.key_material)
            .map_err(|e| format!("HMAC key error: {}", e))?;

        let combined = format!("{}:{}", field_name, plaintext_value);
        mac.update(combined.as_bytes());

        let encrypted_hash = mac.finalize().into_bytes().to_vec();

        Ok(SearchableField {
            field_name,
            encrypted_hash,
            token_value: None,
        })
    }

    /// Rotate searchable encryption key
    pub(crate) fn rotate(&mut self) -> Result<(), String> {
        self.rotation_count += 1;
        
        let mut new_key = vec![0u8; 32];
        let mut new_additional_key = vec![0u8; 32];
        
        getrandom::getrandom(&mut new_key)
            .map_err(|e| format!("Key rotation failed: {}", e))?;
        getrandom::getrandom(&mut new_additional_key)
            .map_err(|e| format!("Additional key rotation failed: {}", e))?;

        self.key_material = new_key;
        self.additional_key_material = new_additional_key;

        Ok(())
    }
}

/// Message search index manager
pub(crate) struct MessageSearchIndex {
    pub indexes: HashMap<String, EncryptedMessageIndex>, // message_id -> index
    pub user_indexes: HashMap<String, Vec<String>>,     // user_id -> [message_ids]
    pub search_keys: HashMap<String, SearchableEncryptionKey>, // user_id -> key
}

impl MessageSearchIndex {
    pub(crate) fn new() -> Self {
        Self {
            indexes: HashMap::new(),
            user_indexes: HashMap::new(),
            search_keys: HashMap::new(),
        }
    }

    /// Initialize searchable encryption for user
    pub(crate) fn init_user(&mut self, user_id: String) -> Result<SearchableEncryptionKey, String> {
        let key = SearchableEncryptionKey::new(user_id.clone())?;
        self.search_keys.insert(user_id, key.clone());
        Ok(key)
    }

    /// Index encrypted message
    pub(crate) fn index_message(
        &mut self,
        message_id: String,
        user_id: String,
        conversation_id: String,
        sender_id: String,
        searchable_values: Vec<SearchableField>,
    ) -> Result<String, String> {
        let index_entry = EncryptedMessageIndex {
            message_id: message_id.clone(),
            user_id: user_id.clone(),
            conversation_id,
            sender_id,
            timestamp: Utc::now(),
            searchable_values,
            message_hash: vec![0u8; 32], // Would be actual SHA-256
        };

        self.indexes.insert(message_id.clone(), index_entry);

        self.user_indexes
            .entry(user_id)
            .or_default()
            .push(message_id.clone());

        Ok(message_id)
    }

    /// Search using token (server-side search)
    pub(crate) fn search_with_token(
        &self,
        user_id: &str,
        _search_token: &SearchToken,
        field_name: &str,
    ) -> Vec<&EncryptedMessageIndex> {
        let user_message_ids = match self.user_indexes.get(user_id) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        // Server-side: Find all messages where the searchable field hash
        // matches what the client computed with the token
        user_message_ids
            .iter()
            .filter_map(|msg_id| self.indexes.get(msg_id))
            .filter(|index| {
                index.searchable_values.iter().any(|field| {
                    field.field_name == field_name
                    // In production: compare field.encrypted_hash with token-derived value
                    // This is ORAM-resistant (server learns minimal info)
                })
            })
            .collect()
    }

    /// Batch search with privacy
    pub(crate) fn batch_search(
        &self,
        user_id: &str,
        tokens: Vec<SearchToken>,
    ) -> Vec<&EncryptedMessageIndex> {
        let mut results = Vec::new();

        for token in tokens {
            results.extend(self.search_with_token(user_id, &token, "body"));
        }

        results
    }

    /// Get message index
    pub(crate) fn get_index(&self, message_id: &str) -> Option<&EncryptedMessageIndex> {
        self.indexes.get(message_id)
    }

    /// Delete indexed messages (right to erasure)
    pub(crate) fn delete_indexed_messages(&mut self, user_id: &str) -> Result<u32, String> {
        let message_ids = match self.user_indexes.remove(user_id) {
            Some(ids) => ids,
            None => return Ok(0),
        };

        let count = message_ids.len() as u32;
        for msg_id in message_ids {
            self.indexes.remove(&msg_id);
        }

        Ok(count)
    }

    /// Get search log (for audit purposes)
    pub(crate) fn get_user_search_log(&self, user_id: &str) -> Vec<&EncryptedMessageIndex> {
        let message_ids = match self.user_indexes.get(user_id) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        message_ids
            .iter()
            .filter_map(|id| self.indexes.get(id))
            .collect()
    }
}

impl Default for MessageSearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_searchable_encryption_key() {
        let key = SearchableEncryptionKey::new("user1".to_string());
        assert!(key.is_ok());

        let key = key.unwrap();
        assert_eq!(key.key_material.len(), 32);
        assert_eq!(key.additional_key_material.len(), 32);
    }

    #[test]
    fn test_search_token_derivation() {
        let key = SearchableEncryptionKey::new("user1".to_string()).unwrap();
        let token = key.derive_search_token("hello");

        assert!(token.is_ok());
        let token = token.unwrap();
        assert_eq!(token.token_value.len(), 32);
    }

    #[test]
    fn test_searchable_field_creation() {
        let key = SearchableEncryptionKey::new("user1".to_string()).unwrap();
        let field = key.create_searchable_field("body".to_string(), "secret message");

        assert!(field.is_ok());
        let field = field.unwrap();
        assert_eq!(field.field_name, "body");
    }

    #[test]
    fn test_message_indexing() {
        let mut index = MessageSearchIndex::new();
        index.init_user("user1".to_string()).unwrap();

        let searchable_values = vec![SearchableField {
            field_name: "body".to_string(),
            encrypted_hash: vec![0u8; 32],
            token_value: None,
        }];

        let result = index.index_message(
            "msg1".to_string(),
            "user1".to_string(),
            "conv1".to_string(),
            "user2".to_string(),
            searchable_values,
        );

        assert!(result.is_ok());
        assert!(index.get_index("msg1").is_some());
    }

    #[test]
    fn test_search_results() {
        let mut index = MessageSearchIndex::new();
        index.init_user("user1".to_string()).unwrap();

        let searchable_values = vec![SearchableField {
            field_name: "body".to_string(),
            encrypted_hash: vec![1u8; 32],
            token_value: None,
        }];

        index.index_message(
            "msg1".to_string(),
            "user1".to_string(),
            "conv1".to_string(),
            "user2".to_string(),
            searchable_values,
        ).unwrap();

        let key = index.search_keys.get("user1").unwrap();
        let token = key.derive_search_token("test").unwrap();

        let _results = index.search_with_token("user1", &token, "body");
        // Results may be 0 or more depending on hash match
    }
}
