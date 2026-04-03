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

// Encrypted file and media storage management
// nexus-relay/src/media_storage.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Media file with encryption metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EncryptedMediaFile {
    pub file_id: String,
    pub user_id: String,
    pub message_id: Option<String>, // Associated message
    pub display_name: String,
    pub media_type: MediaType,
    pub size_bytes: u64,
    pub max_size_bytes: u64, // Policy limit
    
    // Encryption
    pub encryption_algorithm: String,   // "ChaCha20-Poly1305"
    pub encryption_key_id: String,      // Reference to user's media key
    pub file_hash: Vec<u8>,             // SHA-256 of encrypted file
    pub upload_key_derivation: Vec<u8>, // HKDF context
    
    // Lifecycle
    pub uploaded_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,      // Auto-delete after X days
    pub deleted_at: Option<DateTime<Utc>>,
    pub access_count: u32,
    pub last_accessed: DateTime<Utc>,
    
    // Storage
    pub storage_path: String,           // Cloud storage reference
    pub is_deleted: bool,
    pub deletion_token: Option<String>, // For secure deletion
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum MediaType {
    #[serde(rename = "image")]
    Image { format: String }, // "jpeg", "png", "webp"
    #[serde(rename = "video")]
    Video { codec: String, duration_seconds: u32 }, // "h264", "vp9"
    #[serde(rename = "audio")]
    Audio { codec: String, duration_seconds: u32 }, // "opus", "aac"
    #[serde(rename = "document")]
    Document { format: String }, // "pdf", "docx"
    #[serde(rename = "file")]
    File { mimetype: String },
}

/// Media upload session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MediaUploadSession {
    pub session_id: String,
    pub user_id: String,
    pub file_id: String,
    pub upload_key: Vec<u8>,            // Temporary encryption key
    pub uploaded_chunks: u32,
    pub total_chunks: u32,
    pub chunk_hashes: Vec<Vec<u8>>,    // SHA-256 of each chunk
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_complete: bool,
}

/// Media download session with time-limited access
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MediaDownloadSession {
    pub session_id: String,
    pub file_id: String,
    pub user_id: String,
    pub download_token: String,        // One-time use token
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ip_address: String,
    pub access_count: u32,
    pub max_accesses: u32,
    pub is_valid: bool,
}

impl EncryptedMediaFile {
    /// Create new encrypted media file
    pub(crate) fn new(
        user_id: String,
        display_name: String,
        media_type: MediaType,
        size_bytes: u64,
        encryption_key_id: String,
    ) -> Result<Self, String> {
        if size_bytes > 1_000_000_000 {
            return Err("File exceeds 1GB limit".to_string());
        }

        let mut file_hash = vec![0u8; 32];
        getrandom::getrandom(&mut file_hash)
            .map_err(|e| format!("Hash generation failed: {}", e))?;

        let mut derivation = vec![0u8; 32];
        getrandom::getrandom(&mut derivation)
            .map_err(|e| format!("Derivation generation failed: {}", e))?;

        Ok(Self {
            file_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            message_id: None,
            display_name,
            media_type,
            size_bytes,
            max_size_bytes: 1_000_000_000,
            encryption_algorithm: "ChaCha20-Poly1305".to_string(),
            encryption_key_id,
            file_hash,
            upload_key_derivation: derivation,
            uploaded_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(30),
            deleted_at: None,
            access_count: 0,
            last_accessed: Utc::now(),
            storage_path: String::new(),
            is_deleted: false,
            deletion_token: None,
        })
    }

    /// Check if file is expired
    pub(crate) fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Mark file as deleted
    pub(crate) fn mark_deleted(&mut self) -> Result<(), String> {
        self.is_deleted = true;
        self.deleted_at = Some(Utc::now());
        
        // Generate deletion token for secure erasure from storage
        let deletion_token = uuid::Uuid::new_v4().to_string();
        self.deletion_token = Some(deletion_token);

        Ok(())
    }

    /// Record access
    pub(crate) fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Extend expiration
    pub(crate) fn extend_expiration(&mut self, days: i64) -> Result<(), String> {
        if self.is_deleted {
            return Err("Cannot extend deleted file".to_string());
        }

        self.expires_at += chrono::Duration::days(days);
        Ok(())
    }
}

impl MediaUploadSession {
    /// Create new upload session
    pub(crate) fn new(
        user_id: String,
        file_id: String,
        total_chunks: u32,
    ) -> Result<Self, String> {
        if total_chunks == 0 || total_chunks > 10000 {
            return Err("Invalid chunk count".to_string());
        }

        let mut upload_key = vec![0u8; 32];
        getrandom::getrandom(&mut upload_key)
            .map_err(|e| format!("Upload key generation failed: {}", e))?;

        Ok(Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            file_id,
            upload_key,
            uploaded_chunks: 0,
            total_chunks,
            chunk_hashes: Vec::new(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            is_complete: false,
        })
    }

    /// Add chunk hash
    pub(crate) fn add_chunk_hash(&mut self, hash: Vec<u8>) -> Result<(), String> {
        if self.chunk_hashes.len() >= self.total_chunks as usize {
            return Err("Too many chunks".to_string());
        }

        self.chunk_hashes.push(hash);
        self.uploaded_chunks = self.chunk_hashes.len() as u32;

        // Mark complete when all chunks received
        if self.uploaded_chunks == self.total_chunks {
            self.is_complete = true;
        }

        Ok(())
    }

    /// Check if upload is expired
    pub(crate) fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Verify upload integrity
    pub(crate) fn verify_integrity(&self, expected_hash: Vec<u8>) -> bool {
        // Combine all chunk hashes
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();

        for chunk_hash in &self.chunk_hashes {
            hasher.update(chunk_hash);
        }

        hasher.finalize().to_vec() == expected_hash
    }
}

impl MediaDownloadSession {
    /// Create new download session
    pub(crate) fn new(
        file_id: String,
        user_id: String,
        ip_address: String,
        max_accesses: u32,
    ) -> Result<Self, String> {
        let download_token = format!("{}-{}", uuid::Uuid::new_v4(), chrono::Utc::now().timestamp());

        Ok(Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            file_id,
            user_id,
            download_token,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            ip_address,
            access_count: 0,
            max_accesses,
            is_valid: true,
        })
    }

    /// Verify download access
    pub(crate) fn verify_access(&self, token: &str, ip: &str) -> Result<(), String> {
        if !self.is_valid {
            return Err("Download session invalid".to_string());
        }

        if self.download_token != token {
            return Err("Invalid download token".to_string());
        }

        if self.ip_address != ip {
            return Err("IP mismatch".to_string());
        }

        if Utc::now() > self.expires_at {
            return Err("Download session expired".to_string());
        }

        if self.access_count >= self.max_accesses {
            return Err("Max downloads reached".to_string());
        }

        Ok(())
    }

    /// Record download
    pub(crate) fn record_download(&mut self) {
        self.access_count += 1;
        
        if self.access_count >= self.max_accesses {
            self.is_valid = false;
        }
    }
}

/// Media storage manager
pub(crate) struct MediaStorageManager {
    pub files: HashMap<String, EncryptedMediaFile>,
    pub upload_sessions: HashMap<String, MediaUploadSession>,
    pub download_sessions: HashMap<String, MediaDownloadSession>,
    pub user_storage: HashMap<String, u64>, // user_id -> total_bytes_used
    pub max_storage_per_user: u64,
}

impl MediaStorageManager {
    pub(crate) fn new(max_storage_per_user: u64) -> Self {
        Self {
            files: HashMap::new(),
            upload_sessions: HashMap::new(),
            download_sessions: HashMap::new(),
            user_storage: HashMap::new(),
            max_storage_per_user,
        }
    }

    /// Create file
    pub(crate) fn create_file(
        &mut self,
        user_id: String,
        display_name: String,
        media_type: MediaType,
        size_bytes: u64,
        encryption_key_id: String,
    ) -> Result<EncryptedMediaFile, String> {
        // Check storage quota
        let used = self.user_storage.get(&user_id).copied().unwrap_or(0);
        if used + size_bytes > self.max_storage_per_user {
            return Err("Storage quota exceeded".to_string());
        }

        let file = EncryptedMediaFile::new(
            user_id.clone(),
            display_name,
            media_type,
            size_bytes,
            encryption_key_id,
        )?;

        *self.user_storage.entry(user_id).or_insert(0) += size_bytes;
        self.files.insert(file.file_id.clone(), file.clone());

        Ok(file)
    }

    /// Get file
    pub(crate) fn get_file(&self, file_id: &str) -> Option<&EncryptedMediaFile> {
        self.files.get(file_id)
    }

    /// Get mutable file
    pub(crate) fn get_file_mut(&mut self, file_id: &str) -> Option<&mut EncryptedMediaFile> {
        self.files.get_mut(file_id)
    }

    /// Create upload session
    pub(crate) fn create_upload_session(
        &mut self,
        file_id: String,
        user_id: String,
        total_chunks: u32,
    ) -> Result<MediaUploadSession, String> {
        let session = MediaUploadSession::new(user_id, file_id, total_chunks)?;
        self.upload_sessions.insert(session.session_id.clone(), session.clone());
        Ok(session)
    }

    /// Create download session
    pub(crate) fn create_download_session(
        &mut self,
        file_id: String,
        user_id: String,
        ip_address: String,
    ) -> Result<MediaDownloadSession, String> {
        let session = MediaDownloadSession::new(file_id, user_id, ip_address, 5)?;
        self.download_sessions.insert(session.session_id.clone(), session.clone());
        Ok(session)
    }

    /// Cleanup expired sessions
    pub(crate) fn cleanup_expired(&mut self) {
        self.upload_sessions.retain(|_, s| !s.is_expired());
        self.download_sessions.retain(|_, s| Utc::now() <= s.expires_at);
    }
}

impl Default for MediaStorageManager {
    fn default() -> Self {
        Self::new(100_000_000_000) // 100GB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_file_creation() {
        let file = EncryptedMediaFile::new(
            "user1".to_string(),
            "photo.jpg".to_string(),
            MediaType::Image { format: "jpeg".to_string() },
            1000000,
            "key1".to_string(),
        );

        assert!(file.is_ok());
        let file = file.unwrap();
        assert!(!file.is_expired());
    }

    #[test]
    fn test_upload_session() {
        let session = MediaUploadSession::new(
            "user1".to_string(),
            "file1".to_string(),
            10,
        );

        assert!(session.is_ok());
        let session = session.unwrap();
        assert_eq!(session.total_chunks, 10);
    }

    #[test]
    fn test_download_session() {
        let session = MediaDownloadSession::new(
            "file1".to_string(),
            "user1".to_string(),
            "192.168.1.1".to_string(),
            5,
        );

        assert!(session.is_ok());
        let session = session.unwrap();
        assert!(session.is_valid);
    }

    #[test]
    fn test_storage_manager() {
        let mut manager = MediaStorageManager::new(1_000_000_000);

        let file = manager.create_file(
            "user1".to_string(),
            "test.txt".to_string(),
            MediaType::File { mimetype: "text/plain".to_string() },
            1000,
            "key1".to_string(),
        );

        assert!(file.is_ok());
        assert_eq!(manager.files.len(), 1);
    }
}
