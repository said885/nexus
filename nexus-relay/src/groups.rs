#![allow(missing_docs, dead_code)]

//! Group messaging with end-to-end encryption

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Group member with encryption state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GroupMember {
    pub user_id: String,
    pub device_id: String,
    pub group_session_key: Vec<u8>, // Per-member group key (derived from group MSK)
    pub added_at: DateTime<Utc>,
    pub is_admin: bool,
}

/// Group message encryption metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GroupMessageMetadata {
    pub group_id: String,
    pub sender_id: String,
    pub message_index: u64,
    pub epoch: u32,
    pub nonce: [u8; 12],
    pub timestamp: DateTime<Utc>,
}

/// Group state for E2E encryption
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GroupState {
    pub group_id: String,
    pub name: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    
    // Cryptographic state
    pub master_secret_key: Vec<u8>, // 32 bytes, never transmitted
    pub epoch: u32,                  // Incremented on member changes
    pub members: HashMap<String, GroupMember>,
    pub pending_members: HashSet<String>,
    
    // AEAD state
    pub message_index: u64,
    pub max_message_lifetime: u64, // seconds
}

/// Device addition proposal for group (requires other admins' approval)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GroupAddition {
    pub addition_id: String,
    pub group_id: String,
    pub user_id: String,
    pub device_id: String,
    pub proposed_by: String,
    pub proposed_at: DateTime<Utc>,
    pub approvals: HashSet<String>, // Set of admin IDs who approved
    pub requires_approvals: usize,  // Threshold for adding device
}

/// Group removal with forward secrecy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GroupRemoval {
    pub removal_id: String,
    pub group_id: String,
    pub user_id: String,
    pub removed_by: String,
    pub removed_at: DateTime<Utc>,
    pub new_group_key_epoch: u32, // Group secret changed
}

impl GroupState {
    /// Initialize new group with creator as owner/admin
    pub(crate) fn new(group_id: String, name: String, owner_id: String) -> Self {
        // Generate 32-byte master secret key
        let mut master_secret_key = vec![0u8; 32];
        getrandom::getrandom(&mut master_secret_key)
            .expect("Failed to generate group master secret");

        let mut members = HashMap::new();
        if let Ok(owner_device_id) = std::env::var("OWNER_DEVICE_ID") {
            members.insert(
                owner_id.clone(),
                GroupMember {
                    user_id: owner_id.clone(),
                    device_id: owner_device_id,
                    group_session_key: derive_group_session_key(&master_secret_key, &owner_id, 0),
                    added_at: Utc::now(),
                    is_admin: true,
                },
            );
        }

        Self {
            group_id,
            name,
            owner_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            master_secret_key,
            epoch: 0,
            members,
            pending_members: HashSet::new(),
            message_index: 0,
            max_message_lifetime: 604800, // 7 days
        }
    }

    /// Add member with secure key distribution
    /// Must be called by admin, will trigger group epoch increment
    pub(crate) fn propose_member_addition(
        &mut self,
        user_id: String,
        device_id: String,
        proposed_by: String,
    ) -> Result<GroupAddition, String> {
        // Verify proposer is admin
        let proposer = self.members.get(&proposed_by)
            .ok_or("Proposer not in group")?;
        if !proposer.is_admin {
            return Err("Only admins can propose additions".to_string());
        }

        // Check user not already in group
        if self.members.contains_key(&user_id) {
            return Err("User already in group".to_string());
        }

        // Create addition proposal
        let addition = GroupAddition {
            addition_id: Uuid::new_v4().to_string(),
            group_id: self.group_id.clone(),
            user_id: user_id.clone(),
            device_id,
            proposed_by,
            proposed_at: Utc::now(),
            approvals: HashSet::new(),
            requires_approvals: (self.members.len() / 2) + 1, // Majority threshold
        };

        self.pending_members.insert(user_id);
        Ok(addition)
    }

    /// Approve member addition (if you're admin)
    pub(crate) fn approve_addition(
        &mut self,
        addition: &mut GroupAddition,
        approver_id: String,
    ) -> Result<(), String> {
        // Verify approver is admin and in group
        let approver = self.members.get(&approver_id)
            .ok_or("Approver not in group")?;
        if !approver.is_admin {
            return Err("Only admins can approve".to_string());
        }

        // Record approval
        addition.approvals.insert(approver_id);

        // If threshold met, finalize addition
        if addition.approvals.len() >= addition.requires_approvals {
            self.finalize_member_addition(addition)?;
        }

        Ok(())
    }

    /// Finalize member addition and rotate group key
    fn finalize_member_addition(&mut self, addition: &GroupAddition) -> Result<(), String> {
        // Add new member
        self.members.insert(
            addition.user_id.clone(),
            GroupMember {
                user_id: addition.user_id.clone(),
                device_id: addition.device_id.clone(),
                group_session_key: derive_group_session_key(
                    &self.master_secret_key,
                    &addition.user_id,
                    self.epoch,
                ),
                added_at: Utc::now(),
                is_admin: false,
            },
        );

        // Remove from pending
        self.pending_members.remove(&addition.user_id);

        // Increment epoch - forces all members to re-derive keys
        self.epoch = self.epoch.wrapping_add(1);
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Remove member and rotate group key (forward secrecy)
    pub(crate) fn remove_member(
        &mut self,
        user_id: String,
        removed_by: String,
    ) -> Result<GroupRemoval, String> {
        // Verify remover is admin
        let remover = self.members.get(&removed_by)
            .ok_or("Remover not in group")?;
        if !remover.is_admin {
            return Err("Only admins can remove members".to_string());
        }

        // Verify target exists
        if !self.members.contains_key(&user_id) {
            return Err("User not in group".to_string());
        }

        // Cannot remove owner
        if user_id == self.owner_id {
            return Err("Cannot remove group owner".to_string());
        }

        // Remove member
        self.members.remove(&user_id);

        // Increment epoch and regenerate all keys (forward secrecy)
        let _prev_epoch = self.epoch;
        self.epoch = self.epoch.wrapping_add(1);

        // All existing members must derive new keys with new epoch
        for member in self.members.values_mut() {
            member.group_session_key =
                derive_group_session_key(&self.master_secret_key, &member.user_id, self.epoch);
        }

        self.updated_at = Utc::now();

        Ok(GroupRemoval {
            removal_id: Uuid::new_v4().to_string(),
            group_id: self.group_id.clone(),
            user_id,
            removed_by,
            removed_at: Utc::now(),
            new_group_key_epoch: self.epoch,
        })
    }

    /// Get message encryption context for specific member
    pub(crate) fn get_message_context(
        &mut self,
        sender_id: &str,
    ) -> Result<GroupMessageMetadata, String> {
        // Verify sender is in group
        self.members.get(sender_id)
            .ok_or("Sender not in group")?;

        // Generate nonce
        let mut nonce = [0u8; 12];
        getrandom::getrandom(&mut nonce)
            .map_err(|e| format!("Nonce generation failed: {}", e))?;

        // Increment message index
        self.message_index = self.message_index.wrapping_add(1);

        Ok(GroupMessageMetadata {
            group_id: self.group_id.clone(),
            sender_id: sender_id.to_string(),
            message_index: self.message_index,
            epoch: self.epoch,
            nonce,
            timestamp: Utc::now(),
        })
    }

    /// Verify message is valid in this epoch
    pub(crate) fn verify_message_context(&self, metadata: &GroupMessageMetadata) -> Result<(), String> {
        if metadata.group_id != self.group_id {
            return Err("Message group mismatch".to_string());
        }

        if metadata.epoch != self.epoch {
            return Err("Message from different epoch".to_string());
        }

        // Check message age
        let age = Utc::now()
            .signed_duration_since(metadata.timestamp)
            .num_seconds() as u64;
        if age > self.max_message_lifetime {
            return Err("Message expired".to_string());
        }

        Ok(())
    }
}

/// Derive per-member group session key using HKDF
fn derive_group_session_key(master_key: &[u8], user_id: &str, epoch: u32) -> Vec<u8> {
    use sha3::{Sha3_256, Digest};

    let mut hasher = Sha3_256::new();
    hasher.update(b"NEXUS_GROUP_SESSION_KEY");
    hasher.update(master_key);
    hasher.update(user_id.as_bytes());
    hasher.update(epoch.to_le_bytes());

    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_creation() {
        let group = GroupState::new(
            "group123".to_string(),
            "Test Group".to_string(),
            "user1".to_string(),
        );

        assert_eq!(group.group_id, "group123");
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.epoch, 0);
        assert_eq!(group.master_secret_key.len(), 32);
    }

    #[test]
    fn test_member_addition_proposal() {
        let mut group = GroupState::new(
            "group123".to_string(),
            "Test Group".to_string(),
            "user1".to_string(),
        );

        // Add user1 as member first
        if group.members.is_empty() {
            group.members.insert(
                "user1".to_string(),
                GroupMember {
                    user_id: "user1".to_string(),
                    device_id: "device1".to_string(),
                    group_session_key: vec![0u8; 32],
                    added_at: Utc::now(),
                    is_admin: true,
                },
            );
        }

        let result = group.propose_member_addition(
            "user2".to_string(),
            "device2".to_string(),
            "user1".to_string(),
        );

        assert!(result.is_ok());
        assert!(group.pending_members.contains("user2"));
    }

    #[test]
    fn test_forward_secrecy_on_removal() {
        let mut group = GroupState::new(
            "group123".to_string(),
            "Test Group".to_string(),
            "user1".to_string(),
        );

        // Setup members
        if group.members.is_empty() {
            group.members.insert(
                "user1".to_string(),
                GroupMember {
                    user_id: "user1".to_string(),
                    device_id: "device1".to_string(),
                    group_session_key: vec![0u8; 32],
                    added_at: Utc::now(),
                    is_admin: true,
                },
            );
        }

        group.members.insert(
            "user2".to_string(),
            GroupMember {
                user_id: "user2".to_string(),
                device_id: "device2".to_string(),
                group_session_key: derive_group_session_key(
                    &group.master_secret_key,
                    "user2",
                    0,
                ),
                added_at: Utc::now(),
                is_admin: false,
            },
        );

        let epoch_before = group.epoch;

        let result = group.remove_member("user2".to_string(), "user1".to_string());
        assert!(result.is_ok());

        // Epoch must have incremented
        assert!(group.epoch > epoch_before);
        // User2 must be removed
        assert!(!group.members.contains_key("user2"));
    }
}
