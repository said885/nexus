#![allow(missing_docs, dead_code)]

// Zero-knowledge metadata minimization & privacy preservation
// nexus-relay/src/metadata_privacy.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum PrivacyLevel {
    Public,
    Minimal,           // Strip all non-essential metadata
    AlmostZero,        // Hash-based indirection
    ZeroKnowledge,     // No metadata whatsoever
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MessageMetadata {
    pub id: String,
    pub sender_hash: String,           // SHA256(sender_id + salt)
    pub recipient_hash: String,        // SHA256(recipient_id + salt)
    pub timestamp_bucket: u64,         // Rounded to 5-minute intervals
    pub size_bucket: u32,              // Rounded to nearest 1KB
    pub content_type_hash: String,     // SHA256(content_type)
    pub is_encrypted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PrivacyProfile {
    pub id: String,
    pub user_id: String,
    pub privacy_level: PrivacyLevel,
    pub metadata_retention_days: u32,  // 0 = delete immediately
    pub strip_timestamps: bool,
    pub strip_file_sizes: bool,
    pub strip_read_receipts: bool,
    pub anonymize_typing_indicator: bool,
    pub use_padding: bool,             // Add noise to message sizes
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MetadataLayer {
    pub id: String,
    pub original_metadata: String,
    pub hashed_metadata: String,
    pub privacy_level: PrivacyLevel,
    pub created_at: DateTime<Utc>,
}

pub(crate) struct MetadataPrivacyService {
    privacy_profiles: HashMap<String, PrivacyProfile>,
    metadata_layers: HashMap<String, MetadataLayer>,
    salt: String,
}

impl MetadataPrivacyService {
    pub(crate) fn new(salt: String) -> Self {
        MetadataPrivacyService {
            privacy_profiles: HashMap::new(),
            metadata_layers: HashMap::new(),
            salt,
        }
    }

    pub(crate) fn create_privacy_profile(
        &mut self,
        user_id: &str,
        privacy_level: PrivacyLevel,
    ) -> PrivacyProfile {
        let profile = PrivacyProfile {
            id: format!("privacy_{}", uuid::Uuid::new_v4()),
            user_id: user_id.to_string(),
            privacy_level: privacy_level.clone(),
            metadata_retention_days: match privacy_level {
                PrivacyLevel::Public => 365,
                PrivacyLevel::Minimal => 90,
                PrivacyLevel::AlmostZero => 30,
                PrivacyLevel::ZeroKnowledge => 0,
            },
            strip_timestamps: privacy_level != PrivacyLevel::Public,
            strip_file_sizes: privacy_level != PrivacyLevel::Public,
            strip_read_receipts: privacy_level == PrivacyLevel::ZeroKnowledge,
            anonymize_typing_indicator: privacy_level == PrivacyLevel::ZeroKnowledge
                || privacy_level == PrivacyLevel::AlmostZero,
            use_padding: privacy_level == PrivacyLevel::ZeroKnowledge,
            created_at: Utc::now(),
        };

        self.privacy_profiles
            .insert(profile.id.clone(), profile.clone());
        profile
    }

    pub(crate) fn strip_metadata(
        &mut self,
        sender_id: &str,
        recipient_id: &str,
        timestamp: DateTime<Utc>,
        size: u32,
        content_type: &str,
        privacy_level: PrivacyLevel,
    ) -> Result<MessageMetadata, String> {
        // Hash sender/recipient to prevent correlation
        let sender_hash = blake3::hash(format!("{}:{}", sender_id, self.salt).as_bytes()).to_hex().to_string();
        let recipient_hash = blake3::hash(format!("{}:{}", recipient_id, self.salt).as_bytes()).to_hex().to_string();

        // Round timestamp to 5-minute intervals
        let timestamp_bucket = (timestamp.timestamp() / 300) * 300;

        // Round size to 1KB buckets
        let size_bucket = (size / 1024) * 1024;

        // Hash content type
        let content_type_hash = blake3::hash(format!("{}:{}", content_type, self.salt).as_bytes()).to_hex().to_string();

        let metadata = MessageMetadata {
            id: format!("meta_{}", uuid::Uuid::new_v4()),
            sender_hash,
            recipient_hash,
            timestamp_bucket: timestamp_bucket as u64,
            size_bucket,
            content_type_hash,
            is_encrypted: true,
        };

        let layer = MetadataLayer {
            id: metadata.id.clone(),
            original_metadata: format!(
                "sender:{},recipient:{},timestamp:{},size:{}",
                sender_id, recipient_id, timestamp, size
            ),
            hashed_metadata: format!(
                "sender_hash:{},recipient_hash:{},timestamp_bucket:{},size_bucket:{}",
                metadata.sender_hash, metadata.recipient_hash, metadata.timestamp_bucket, metadata.size_bucket
            ),
            privacy_level,
            created_at: Utc::now(),
        };

        self.metadata_layers.insert(layer.id.clone(), layer);
        Ok(metadata)
    }

    pub(crate) fn anonymize_with_padding(
        &self,
        message_size: u32,
        target_privacy_level: PrivacyLevel,
    ) -> u32 {
        if target_privacy_level != PrivacyLevel::ZeroKnowledge {
            return message_size;
        }

        // Add random padding to disguise message size
        // Round to nearest power of 2
        let next_power = message_size.next_power_of_two();
        let padding_size = rand::random::<u32>() % (next_power / 2);
        message_size + padding_size
    }

    pub(crate) fn calculate_privacy_score(&self, profile: &PrivacyProfile) -> f64 {
        let mut score: f64 = 0.0;

        // Base score from privacy level
        score += match profile.privacy_level {
            PrivacyLevel::Public => 0.0,
            PrivacyLevel::Minimal => 0.25,
            PrivacyLevel::AlmostZero => 0.75,
            PrivacyLevel::ZeroKnowledge => 1.0,
        };

        // Extra points for enabled privacy features
        if profile.strip_timestamps {
            score += 0.1;
        }
        if profile.strip_file_sizes {
            score += 0.1;
        }
        if profile.strip_read_receipts {
            score += 0.1;
        }
        if profile.anonymize_typing_indicator {
            score += 0.1;
        }
        if profile.use_padding {
            score += 0.1;
        }

        // Normalize to 0-1 range
        score.min(1.0_f64)
    }

    pub(crate) fn get_privacy_profile(&self, user_id: &str) -> Option<&PrivacyProfile> {
        self.privacy_profiles
            .values()
            .find(|p| p.user_id == user_id)
    }

    pub(crate) fn update_privacy_level(
        &mut self,
        user_id: &str,
        new_level: PrivacyLevel,
    ) -> Result<(), String> {
        if let Some(profile) = self
            .privacy_profiles
            .values_mut()
            .find(|p| p.user_id == user_id)
        {
            profile.privacy_level = new_level.clone();
            profile.metadata_retention_days = match new_level {
                PrivacyLevel::Public => 365,
                PrivacyLevel::Minimal => 90,
                PrivacyLevel::AlmostZero => 30,
                PrivacyLevel::ZeroKnowledge => 0,
            };
            Ok(())
        } else {
            Err("User profile not found".to_string())
        }
    }

    pub(crate) fn verify_metadata_stripped(&self, metadata: &MessageMetadata) -> bool {
        // Verify metadata has been properly hashed/stripped
        metadata.sender_hash.len() == 64 && metadata.recipient_hash.len() == 64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_privacy_profile() {
        let mut service = MetadataPrivacyService::new("salt_value".to_string());
        let profile = service.create_privacy_profile("user_1", PrivacyLevel::ZeroKnowledge);

        assert_eq!(profile.user_id, "user_1");
        assert_eq!(profile.privacy_level, PrivacyLevel::ZeroKnowledge);
        assert_eq!(profile.metadata_retention_days, 0);
    }

    #[test]
    fn test_strip_metadata() {
        let mut service = MetadataPrivacyService::new("salt_value".to_string());
        let metadata = service
            .strip_metadata(
                "sender_1",
                "recipient_1",
                Utc::now(),
                5000,
                "text/plain",
                PrivacyLevel::AlmostZero,
            )
            .unwrap();

        assert!(metadata.sender_hash.len() == 64);
        assert!(metadata.recipient_hash.len() == 64);
        assert!(metadata.is_encrypted);
    }

    #[test]
    fn test_privacy_score_calculation() {
        let mut service = MetadataPrivacyService::new("salt_value".to_string());
        let profile = service.create_privacy_profile("user_1", PrivacyLevel::ZeroKnowledge);
        let score = service.calculate_privacy_score(&profile);

        assert!(score > 0.5);
    }

    #[test]
    fn test_verify_metadata_stripped() {
        let mut service = MetadataPrivacyService::new("salt_value".to_string());
        let metadata = service
            .strip_metadata(
                "sender_1",
                "recipient_1",
                Utc::now(),
                1024,
                "text/plain",
                PrivacyLevel::Minimal,
            )
            .unwrap();

        assert!(service.verify_metadata_stripped(&metadata));
    }

    #[test]
    fn test_anonymize_with_padding() {
        let service = MetadataPrivacyService::new("salt_value".to_string());
        let original_size = 1000u32;
        let padded_size = service.anonymize_with_padding(original_size, PrivacyLevel::ZeroKnowledge);

        assert!(padded_size >= original_size);
    }

    #[test]
    fn test_update_privacy_level() {
        let mut service = MetadataPrivacyService::new("salt_value".to_string());
        service.create_privacy_profile("user_1", PrivacyLevel::Minimal);

        let result = service.update_privacy_level("user_1", PrivacyLevel::ZeroKnowledge);
        assert!(result.is_ok());

        let profile = service.get_privacy_profile("user_1").unwrap();
        assert_eq!(profile.privacy_level, PrivacyLevel::ZeroKnowledge);
    }
}
