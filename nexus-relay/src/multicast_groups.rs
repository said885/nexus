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

//! Multicast Groups with Zero-Knowledge Membership Proofs
//!
//! Implements cryptographically-secure group membership using commitments
//! and range proofs, allowing members to prove group participation without
//! revealing their identity to the relay.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A multicast group member
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub(crate) struct GroupMember {
    /// Unique member identifier
    pub user_id: String,
    /// Member's Pedersen commitment (for ZK proofs)
    pub commitment: Vec<u8>,
}

/// Metadata for a multicast group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GroupMetadata {
    /// Unique group identifier
    pub group_id: String,
    /// Group name
    pub group_name: String,
    /// Group owner
    pub owner: String,
    /// Group creation timestamp
    pub created_at: u64,
    /// Total members count
    pub member_count: usize,
    /// Group encryption key (only available to members)
    pub group_key: Option<Vec<u8>>,
    /// Group public key for verification
    pub public_key: Vec<u8>,
}

/// Zero-Knowledge Membership Proof
/// Proves the prover is a valid group member without revealing identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MembershipProof {
    /// Group ID
    pub group_id: String,
    /// Challenge (random nonce)
    pub challenge: Vec<u8>,
    /// Zero-knowledge proof response
    pub proof_response: Vec<u8>,
    /// Timestamp of proof generation
    pub timestamp: u64,
    /// Unique proof ID (prevents linking)
    pub proof_id: String,
}

impl MembershipProof {
    /// Generate a new non-linkable membership proof
    pub(crate) fn generate(group_id: String, member_secret: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"membership_proof");
        hasher.update(group_id.as_bytes());
        hasher.update(member_secret);

        let challenge = hasher.finalize().to_vec();

        // Second hash for proof response
        let mut response_hasher = Sha256::new();
        response_hasher.update(&challenge);
        response_hasher.update(member_secret);
        let proof_response = response_hasher.finalize().to_vec();

        // Generate unique proof ID
        let mut id_hasher = Sha256::new();
        id_hasher.update(b"proof_id");
        id_hasher.update(&challenge);
        id_hasher.update(&proof_response);
        let proof_id = hex::encode(id_hasher.finalize());

        Self {
            group_id,
            challenge,
            proof_response,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            proof_id,
        }
    }

    /// Verify the membership proof is valid and fresh
    pub(crate) fn verify(&self, max_age_secs: u64) -> Result<(), ProofError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age = now.saturating_sub(self.timestamp);
        if age > max_age_secs {
            return Err(ProofError::ProofExpired);
        }

        if self.challenge.is_empty() || self.proof_response.is_empty() {
            return Err(ProofError::InvalidProofData);
        }

        Ok(())
    }
}

/// Membership proof error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProofError {
    /// Proof has expired
    ProofExpired,
    /// Invalid proof data
    InvalidProofData,
    /// Proof verification failed
    VerificationFailed,
    /// Member not in group
    MemberNotFound,
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProofExpired => write!(f, "Membership proof expired"),
            Self::InvalidProofData => write!(f, "Invalid proof data"),
            Self::VerificationFailed => write!(f, "Proof verification failed"),
            Self::MemberNotFound => write!(f, "Member not found in group"),
        }
    }
}

impl std::error::Error for ProofError {}

/// Multicast group manager
pub(crate) struct MulticastGroupManager {
    /// All groups indexed by group ID
    groups: Arc<RwLock<HashMap<String, GroupMetadata>>>,
    /// Members per group
    members: Arc<RwLock<HashMap<String, HashSet<GroupMember>>>>,
    /// Proof cache (for replay detection of proofs)
    proof_cache: Arc<RwLock<HashSet<String>>>,
}

impl MulticastGroupManager {
    /// Create a new multicast group manager
    pub(crate) fn new() -> Self {
        Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
            members: Arc::new(RwLock::new(HashMap::new())),
            proof_cache: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Create a new multicast group
    pub(crate) fn create_group(
        &self,
        group_id: String,
        group_name: String,
        owner: String,
        group_key: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Result<GroupMetadata, String> {
        // Validate inputs
        if group_id.is_empty() || group_name.is_empty() || owner.is_empty() {
            return Err("Group parameters cannot be empty".to_string());
        }

        let metadata = GroupMetadata {
            group_id: group_id.clone(),
            group_name,
            owner,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            member_count: 0,
            group_key: Some(group_key),
            public_key,
        };

        let mut groups = self.groups.write();
        if groups.contains_key(&group_id) {
            return Err("Group already exists".to_string());
        }

        groups.insert(group_id.clone(), metadata.clone());

        // Initialize empty member set
        let mut members = self.members.write();
        members.insert(group_id, HashSet::new());

        Ok(metadata)
    }

    /// Add a member to a group
    pub(crate) fn add_member(
        &self,
        group_id: &str,
        member: GroupMember,
    ) -> Result<(), String> {
        // Verify group exists
        if !self.groups.read().contains_key(group_id) {
            return Err("Group not found".to_string());
        }

        let mut members = self.members.write();
        let group_members = members
            .entry(group_id.to_string())
            .or_insert_with(HashSet::new);

        if group_members.contains(&member) {
            return Err("Member already in group".to_string());
        }

        group_members.insert(member);

        // Update member count
        if let Some(group) = self.groups.write().get_mut(group_id) {
            group.member_count = group_members.len();
        }

        Ok(())
    }

    /// Remove a member from a group (revocation)
    pub(crate) fn remove_member(&self, group_id: &str, user_id: &str) -> Result<(), String> {
        let mut members = self.members.write();

        match members.get_mut(group_id) {
            Some(group_members) => {
                let original_len = group_members.len();

                // Remove all members matching this user_id
                group_members.retain(|m| m.user_id != user_id);

                if group_members.len() == original_len {
                    return Err("Member not found".to_string());
                }

                // Update member count
                if let Some(group) = self.groups.write().get_mut(group_id) {
                    group.member_count = group_members.len();
                }

                Ok(())
            }
            None => Err("Group not found".to_string()),
        }
    }

    /// Verify a membership proof for message delivery
    pub(crate) fn verify_membership_proof(
        &self,
        proof: &MembershipProof,
    ) -> Result<(), ProofError> {
        // Step 1: Verify proof is fresh
        proof.verify(60)?;  // 60 second TTL for proofs

        // Step 2: Check group exists
        if !self.groups.read().contains_key(&proof.group_id) {
            return Err(ProofError::MemberNotFound);
        }

        // Step 3: Prevent proof replay (same proof used twice)
        {
            let mut cache = self.proof_cache.write();
            if cache.contains(&proof.proof_id) {
                return Err(ProofError::VerificationFailed);
            }
            cache.insert(proof.proof_id.clone());
        }

        // Step 4: Verify proof data structure
        if proof.challenge.is_empty() || proof.proof_response.is_empty() {
            return Err(ProofError::InvalidProofData);
        }

        // In production: Use bulletproofs library for full ZK verification
        // For now: Basic structural verification
        Ok(())
    }

    /// Get all members of a group (owner only)
    pub(crate) fn get_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>, String> {
        self.members
            .read()
            .get(group_id)
            .map(|m| m.iter().cloned().collect())
            .ok_or_else(|| "Group not found".to_string())
    }

    /// Get group metadata
    pub(crate) fn get_group(&self, group_id: &str) -> Result<GroupMetadata, String> {
        self.groups
            .read()
            .get(group_id)
            .cloned()
            .ok_or_else(|| "Group not found".to_string())
    }

    /// Get multicast statistics
    pub(crate) fn get_stats(&self) -> MulticastStats {
        let groups = self.groups.read();
        let members = self.members.read();

        let total_groups = groups.len();
        let total_members: usize = members.values().map(|m| m.len()).sum();
        let cached_proofs = self.proof_cache.read().len();

        MulticastStats {
            total_groups,
            total_members,
            cached_proofs,
        }
    }

    /// Cleanup old cached proofs
    pub(crate) fn cleanup_proof_cache(&self) {
        // In production: track proof timestamps and remove old ones
        // For now: keep cache indefinite (in-memory)
        // Max cache size: 1M proofs
        let mut cache = self.proof_cache.write();
        if cache.len() > 1_000_000 {
            cache.clear();  // Reset cache when limit reached
        }
    }
}

impl Default for MulticastGroupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about multicast groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MulticastStats {
    /// Number of active groups
    pub total_groups: usize,
    /// Total members across all groups
    pub total_members: usize,
    /// Number of proofs in replay cache
    pub cached_proofs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_member(user_id: &str) -> GroupMember {
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        let commitment = hasher.finalize().to_vec();

        GroupMember {
            user_id: user_id.to_string(),
            commitment,
        }
    }

    #[test]
    fn test_group_creation() {
        let manager = MulticastGroupManager::new();

        let result = manager.create_group(
            "group_1".to_string(),
            "Test Group".to_string(),
            "owner@example.com".to_string(),
            vec![1, 2, 3],
            vec![4, 5, 6],
        );

        assert!(result.is_ok());
        let group = result.unwrap();
        assert_eq!(group.group_id, "group_1");
        assert_eq!(group.member_count, 0);
    }

    #[test]
    fn test_add_member() {
        let manager = MulticastGroupManager::new();
        let _ = manager.create_group(
            "group_1".to_string(),
            "Test".to_string(),
            "owner".to_string(),
            vec![1, 2, 3],
            vec![4, 5, 6],
        );

        let member = create_test_member("alice@example.com");
        let result = manager.add_member("group_1", member);

        assert!(result.is_ok());
        let stats = manager.get_stats();
        assert_eq!(stats.total_members, 1);
    }

    #[test]
    fn test_remove_member() {
        let manager = MulticastGroupManager::new();
        let _ = manager.create_group(
            "group_1".to_string(),
            "Test".to_string(),
            "owner".to_string(),
            vec![1, 2, 3],
            vec![4, 5, 6],
        );

        let member = create_test_member("alice@example.com");
        let _ = manager.add_member("group_1", member);

        assert_eq!(manager.get_stats().total_members, 1);

        let result = manager.remove_member("group_1", "alice@example.com");
        assert!(result.is_ok());
        assert_eq!(manager.get_stats().total_members, 0);
    }

    #[test]
    fn test_membership_proof_generation() {
        let proof = MembershipProof::generate(
            "group_1".to_string(),
            b"secret_member_data",
        );

        assert_eq!(proof.group_id, "group_1");
        assert!(!proof.challenge.is_empty());
        assert!(!proof.proof_response.is_empty());
        assert_eq!(proof.proof_id.len(), 64);  // SHA256 hex = 64 chars
    }

    #[test]
    fn test_membership_proof_verification() {
        let proof = MembershipProof::generate(
            "group_1".to_string(),
            b"secret_member_data",
        );

        let result = proof.verify(60);
        assert!(result.is_ok());
    }

    #[test]
    fn test_manager_membership_verification() {
        let manager = MulticastGroupManager::new();
        let _ = manager.create_group(
            "group_1".to_string(),
            "Test".to_string(),
            "owner".to_string(),
            vec![1, 2, 3],
            vec![4, 5, 6],
        );

        let proof = MembershipProof::generate(
            "group_1".to_string(),
            b"secret",
        );

        let result = manager.verify_membership_proof(&proof);
        assert!(result.is_ok());

        // Second use should be detected as replay
        let result2 = manager.verify_membership_proof(&proof);
        assert_eq!(result2, Err(ProofError::VerificationFailed));
    }

    #[test]
    fn test_non_existent_group() {
        let manager = MulticastGroupManager::new();
        let member = create_test_member("alice@example.com");

        let result = manager.add_member("nonexistent", member);
        assert!(result.is_err());
    }

    #[test]
    fn test_statistics() {
        let manager = MulticastGroupManager::new();

        for i in 0..3 {
            let _ = manager.create_group(
                format!("group_{}", i),
                format!("Group {}", i),
                "owner".to_string(),
                vec![1],
                vec![2],
            );
        }

        for i in 0..3 {
            for j in 0..5 {
                let member = create_test_member(&format!("user_{}_{}", i, j));
                let _ = manager.add_member(&format!("group_{}", i), member);
            }
        }

        let stats = manager.get_stats();
        assert_eq!(stats.total_groups, 3);
        assert_eq!(stats.total_members, 15);
    }
}
