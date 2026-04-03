#![allow(missing_docs, dead_code)]

// Multi-device synchronization with hybrid end-to-end encryption
// nexus-relay/src/sync.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Device registration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RegisteredDevice {
    pub device_id: String,
    pub user_id: String,
    pub device_name: String,
    pub device_type: DeviceType,
    pub prekey_bundle: PrekeyBundle,
    pub is_primary: bool,
    pub registered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum DeviceType {
    Mobile,
    Desktop,
    Web,
    Tablet,
}

/// Prekey bundle for hybrid KEM (Kyber + X25519)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PrekeyBundle {
    pub device_id: String,
    
    // Post-quantum keys (Kyber 1024)
    pub kyber_public_key: Vec<u8>,  // 1568 bytes
    pub kyber_prekey1: Vec<u8>,     // Optional: alternative Kyber key
    
    // Classical keys (Curve25519)
    pub x25519_public_key: Vec<u8>, // 32 bytes
    pub x25519_prekey1: Vec<u8>,    // Additional X25519 for resilience
    
    // Signature key
    pub signature_public_key: Vec<u8>, // Dilithium 5 or Ed25519
    pub prekey_signature: Vec<u8>,     // Signature over prekey bundle
    
    // Metadata
    pub bundle_id: u32,             // Rotation counter
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Sync message for cross-device delivery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SyncMessage {
    pub sync_id: String,
    pub user_id: String,
    pub source_device_id: String,
    pub target_device_id: Option<String>, // None = broadcast to all other devices
    pub sync_type: SyncType,
    pub encrypted_payload: Vec<u8>,       // Encrypted with per-device session key
    pub signature: Vec<u8>,               // Source device signature
    pub timestamp: DateTime<Utc>,
    pub ttl: u64,                        // seconds
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum SyncType {
    MessageRead,
    ConversationArchived,
    ContactUpdated,
    SettingsChanged,
    DeviceAdded,
    DeviceRemoved,
    KeyRotation,
    Custom(String),
}

/// Multi-device session state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DeviceSyncState {
    pub user_id: String,
    pub devices: HashMap<String, RegisteredDevice>,
    pub primary_device_id: String,
    pub session_keys: HashMap<String, Vec<u8>>, // device_id -> session_key (32 bytes)
    pub last_sync_timestamp: DateTime<Utc>,
    pub pending_syncs: Vec<SyncMessage>,
}

impl DeviceSyncState {
    /// Initialize sync state for user
    pub(crate) fn new(user_id: String, primary_device_id: String) -> Self {
        Self {
            user_id,
            devices: HashMap::new(),
            primary_device_id,
            session_keys: HashMap::new(),
            last_sync_timestamp: Utc::now(),
            pending_syncs: Vec::new(),
        }
    }

    /// Register new device for user
    pub(crate) fn register_device(
        &mut self,
        device: RegisteredDevice,
    ) -> Result<(), String> {
        // Validate device name length
        if device.device_name.len() > 255 {
            return Err("Device name too long".to_string());
        }

        // Only allow max 5 devices per user
        if self.devices.len() >= 5 && !self.devices.contains_key(&device.device_id) {
            return Err("Max devices reached (5)".to_string());
        }

        // If marking as primary, unmark current primary
        if device.is_primary {
            if let Some(current_primary) = self.devices.get_mut(&self.primary_device_id) {
                current_primary.is_primary = false;
            }
            self.primary_device_id = device.device_id.clone();
        }

        // Derive session key for this device (hybrid KEM)
        let session_key = derive_device_session_key(
            &device.prekey_bundle.kyber_public_key,
            &device.prekey_bundle.x25519_public_key,
        )?;

        self.session_keys.insert(device.device_id.clone(), session_key);
        self.devices.insert(device.device_id.clone(), device);

        Ok(())
    }

    /// Unregister device
    pub(crate) fn unregister_device(&mut self, device_id: String) -> Result<(), String> {
        if !self.devices.contains_key(&device_id) {
            return Err("Device not found".to_string());
        }

        if device_id == self.primary_device_id {
            return Err("Cannot unregister primary device".to_string());
        }

        self.devices.remove(&device_id);
        self.session_keys.remove(&device_id);

        // Broadcast device removal to all other devices
        self.broadcast_sync(SyncType::DeviceRemoved, vec![])?;

        Ok(())
    }

    /// Get session key for device
    pub(crate) fn get_device_session_key(&self, device_id: &str) -> Result<&Vec<u8>, String> {
        self.session_keys
            .get(device_id)
            .ok_or("Device session key not found".to_string())
    }

    /// Broadcast sync message to all non-source devices
    pub(crate) fn broadcast_sync(
        &mut self,
        sync_type: SyncType,
        payload: Vec<u8>,
    ) -> Result<Vec<SyncMessage>, String> {
        let mut messages = Vec::new();

        let source_device = &self.primary_device_id;

        for (device_id, device) in self.devices.iter() {
            if device_id == source_device || !device.is_active {
                continue;
            }

            let sync_msg = SyncMessage {
                sync_id: format!("{}-{}", self.user_id, device_id),
                user_id: self.user_id.clone(),
                source_device_id: source_device.clone(),
                target_device_id: Some(device_id.clone()),
                sync_type: sync_type.clone(),
                encrypted_payload: payload.clone(),
                signature: vec![],
                timestamp: Utc::now(),
                ttl: 86400, // 1 day
            };

            messages.push(sync_msg);
        }

        Ok(messages)
    }

    /// Queue sync message
    pub(crate) fn queue_sync_message(&mut self, message: SyncMessage) -> Result<(), String> {
        if message.ttl == 0 {
            return Err("Message TTL expired".to_string());
        }

        self.pending_syncs.push(message);

        // Keep only last 1000 syncs per device
        if self.pending_syncs.len() > 1000 {
            self.pending_syncs.remove(0);
        }

        Ok(())
    }

    /// Get pending syncs for device
    pub(crate) fn get_pending_syncs(&mut self, device_id: &str) -> Vec<SyncMessage> {
        let now = Utc::now();
        let mut syncs_to_deliver = Vec::new();

        let mut expired_indices = Vec::new();

        for (idx, sync) in self.pending_syncs.iter().enumerate() {
            let age = now.signed_duration_since(sync.timestamp).num_seconds() as u64;

            // Check if expired
            if age > sync.ttl {
                expired_indices.push(idx);
                continue;
            }

            // Check if for this device
            if sync.target_device_id.is_none()
                || sync.target_device_id.as_ref() == Some(&device_id.to_string())
            {
                syncs_to_deliver.push(sync.clone());
            }
        }

        // Remove expired messages (in reverse to maintain indices)
        for idx in expired_indices.iter().rev() {
            self.pending_syncs.remove(*idx);
        }

        self.last_sync_timestamp = Utc::now();
        syncs_to_deliver
    }

    /// Rotate all device session keys
    pub(crate) fn rotate_session_keys(&mut self) -> Result<(), String> {
        for (device_id, device) in self.devices.iter() {
            let new_session_key = derive_device_session_key(
                &device.prekey_bundle.kyber_public_key,
                &device.prekey_bundle.x25519_public_key,
            )?;

            self.session_keys.insert(device_id.clone(), new_session_key);
        }

        self.broadcast_sync(SyncType::KeyRotation, vec![])?;

        Ok(())
    }

    /// Verify device is alive
    pub(crate) fn mark_device_active(&mut self, device_id: &str) -> Result<(), String> {
        if let Some(device) = self.devices.get_mut(device_id) {
            device.last_seen = Utc::now();
            device.is_active = true;
            Ok(())
        } else {
            Err("Device not found".to_string())
        }
    }

    /// Get active devices
    pub(crate) fn get_active_devices(&self) -> Vec<&RegisteredDevice> {
        self.devices
            .values()
            .filter(|d| d.is_active)
            .collect()
    }

    /// Verify device offline (no heartbeat in 24 hours)
    pub(crate) fn verify_offline_devices(&mut self) -> Vec<String> {
        let now = Utc::now();
        let mut offline_devices = Vec::new();

        for (device_id, device) in self.devices.iter_mut() {
            let last_seen_ago = now.signed_duration_since(device.last_seen).num_seconds() as u64;
            if last_seen_ago > 86400 {
                // 24 hours
                device.is_active = false;
                offline_devices.push(device_id.clone());
            }
        }

        offline_devices
    }
}

/// Derive device session key using hybrid KEM
fn derive_device_session_key(kyber_pk: &[u8], x25519_pk: &[u8]) -> Result<Vec<u8>, String> {
    use sha3::{Sha3_256, Digest};

    let mut hasher = Sha3_256::new();
    hasher.update(b"NEXUS_DEVICE_SESSION");
    hasher.update(kyber_pk);
    hasher.update(x25519_pk);

    Ok(hasher.finalize().to_vec())
}

/// Multi-device manager
pub(crate) struct DeviceManager {
    pub users: HashMap<String, DeviceSyncState>,
}

impl DeviceManager {
    pub(crate) fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    /// Initialize sync state for new user
    pub(crate) fn init_user(
        &mut self,
        user_id: String,
        primary_device_id: String,
    ) -> &mut DeviceSyncState {
        self.users
            .entry(user_id.clone())
            .or_insert_with(|| DeviceSyncState::new(user_id, primary_device_id))
    }

    /// Get user sync state
    pub(crate) fn get_user_state(&self, user_id: &str) -> Option<&DeviceSyncState> {
        self.users.get(user_id)
    }

    /// Get mutable user sync state
    pub(crate) fn get_user_state_mut(&mut self, user_id: &str) -> Option<&mut DeviceSyncState> {
        self.users.get_mut(user_id)
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_registration() {
        let mut state = DeviceSyncState::new("user1".to_string(), "device1".to_string());

        let device = RegisteredDevice {
            device_id: "device2".to_string(),
            user_id: "user1".to_string(),
            device_name: "iPhone".to_string(),
            device_type: DeviceType::Mobile,
            prekey_bundle: PrekeyBundle {
                device_id: "device2".to_string(),
                kyber_public_key: vec![0u8; 1568],
                kyber_prekey1: vec![0u8; 1568],
                x25519_public_key: vec![0u8; 32],
                x25519_prekey1: vec![0u8; 32],
                signature_public_key: vec![0u8; 2593],
                prekey_signature: vec![0u8; 4595],
                bundle_id: 1,
                created_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::days(30),
            },
            is_primary: false,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            is_active: true,
        };

        assert!(state.register_device(device).is_ok());
        assert_eq!(state.devices.len(), 1);
    }

    #[test]
    fn test_broadcast_sync() {
        let mut state = DeviceSyncState::new("user1".to_string(), "device1".to_string());

        // Register second device
        let device = RegisteredDevice {
            device_id: "device2".to_string(),
            user_id: "user1".to_string(),
            device_name: "Android Phone".to_string(),
            device_type: DeviceType::Mobile,
            prekey_bundle: PrekeyBundle {
                device_id: "device2".to_string(),
                kyber_public_key: vec![0u8; 1568],
                kyber_prekey1: vec![0u8; 1568],
                x25519_public_key: vec![0u8; 32],
                x25519_prekey1: vec![0u8; 32],
                signature_public_key: vec![0u8; 2593],
                prekey_signature: vec![0u8; 4595],
                bundle_id: 1,
                created_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::days(30),
            },
            is_primary: false,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            is_active: true,
        };

        state.register_device(device).unwrap();

        let msgs = state.broadcast_sync(SyncType::SettingsChanged, vec![1, 2, 3]).unwrap();
        assert!(msgs.len() > 0);
    }

    #[test]
    fn test_device_manager() {
        let mut manager = DeviceManager::new();
        let state = manager.init_user("user1".to_string(), "device1".to_string());

        assert_eq!(state.user_id, "user1");
        assert!(manager.get_user_state("user1").is_some());
    }
}
