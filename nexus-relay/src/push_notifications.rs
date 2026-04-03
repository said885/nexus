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

// Encrypted push notifications with end-to-end encryption
// nexus-relay/src/push_notifications.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Push notification token
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PushToken {
    pub token_id: String,
    pub user_id: String,
    pub device_id: String,
    pub platform: PushPlatform,
    pub token_value: String,       // Device token from FCM/APNs
    pub encryption_key: Vec<u8>,   // Public key for E2E encryption
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum PushPlatform {
    #[serde(rename = "fcm")]
    Firebase,  // Firebase Cloud Messaging (Android, Web)
    #[serde(rename = "apns")]
    Apns,      // Apple Push Notification service (iOS, macOS)
    #[serde(rename = "web")]
    WebPush,   // Web Push API
}

/// Encrypted push notification payload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EncryptedPushNotification {
    pub notification_id: String,
    pub push_token_id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    
    // Encrypted data
    pub encryption_algorithm: String, // "ChaCha20-Poly1305"
    pub encrypted_payload: Vec<u8>,
    pub nonce: [u8; 12],
    pub authentication_tag: [u8; 16],
    
    // Metadata (NOT encrypted, metadata only)
    pub title: String,  // Generic title, no PII
    pub badge: u32,
    pub priority: NotificationPriority,
    
    // TTL and expiry
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub time_to_live_seconds: u32,
    
    // Delivery tracking
    pub sent_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub failed: bool,
    pub failure_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum NotificationType {
    IncomingMessage,
    IncomingCall,
    GroupInvite,
    GroupMessage,
    DeviceAdded,
    SecurityAlert,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Decrypted push notification data (server-side only)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PushNotificationPayload {
    pub sender_id: String,
    pub message: String,
    pub action: Option<String>,
    pub deep_link: Option<String>,
    pub custom_data: HashMap<String, String>,
}

impl PushToken {
    /// Create new push token
    pub(crate) fn new(
        user_id: String,
        device_id: String,
        platform: PushPlatform,
        token_value: String,
        encryption_key: Vec<u8>,
    ) -> Self {
        Self {
            token_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            device_id,
            platform,
            token_value,
            encryption_key,
            created_at: Utc::now(),
            last_used: Utc::now(),
            is_active: true,
        }
    }

    /// Refresh token timestamp
    pub(crate) fn mark_used(&mut self) {
        self.last_used = Utc::now();
    }

    /// Invalidate token (revoke from device)
    pub(crate) fn invalidate(&mut self) {
        self.is_active = false;
    }
}

impl EncryptedPushNotification {
    /// Create new encrypted push notification
    pub(crate) fn new(
        push_token_id: String,
        user_id: String,
        notification_type: NotificationType,
        title: String,
        payload: PushNotificationPayload,
        encryption_key: &[u8],
        priority: NotificationPriority,
    ) -> Result<Self, String> {
        use chacha20poly1305::{
            aead::{Aead, KeyInit, Payload},
            ChaCha20Poly1305,
        };

        // Serialize payload
        let payload_json = serde_json::to_vec(&payload)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Generate nonce
        let mut nonce_arr = [0u8; 12];
        getrandom::getrandom(&mut nonce_arr)
            .map_err(|e| format!("Nonce generation failed: {}", e))?;

        let nonce = chacha20poly1305::Nonce::from(nonce_arr);

        // Encrypt with ChaCha20-Poly1305
        let cipher = ChaCha20Poly1305::new_from_slice(encryption_key)
            .map_err(|_| "Invalid encryption key size".to_string())?;

        let (encrypted_payload, tag) = {
            let ciphertext = cipher
                .encrypt(&nonce, Payload::from(payload_json.as_slice()))
                .map_err(|e| format!("Encryption failed: {}", e))?;

            let mut auth_tag = [0u8; 16];
            auth_tag.copy_from_slice(&ciphertext[ciphertext.len() - 16..]);
            let encrypted = ciphertext[..ciphertext.len() - 16].to_vec();

            (encrypted, auth_tag)
        };

        let now = Utc::now();

        Ok(Self {
            notification_id: uuid::Uuid::new_v4().to_string(),
            push_token_id,
            user_id,
            notification_type,
            encryption_algorithm: "ChaCha20-Poly1305".to_string(),
            encrypted_payload,
            nonce: nonce_arr,
            authentication_tag: tag,
            title,
            badge: 1,
            priority,
            created_at: now,
            expires_at: now + chrono::Duration::days(1),
            time_to_live_seconds: 86400,
            sent_at: None,
            delivered_at: None,
            failed: false,
            failure_reason: None,
        })
    }

    /// Mark as sent
    pub(crate) fn mark_sent(&mut self) {
        self.sent_at = Some(Utc::now());
    }

    /// Mark as delivered
    pub(crate) fn mark_delivered(&mut self) {
        self.delivered_at = Some(Utc::now());
    }

    /// Mark as failed
    pub(crate) fn mark_failed(&mut self, reason: String) {
        self.failed = true;
        self.failure_reason = Some(reason);
    }

    /// Check if expired
    pub(crate) fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Push notification manager
pub(crate) struct PushNotificationManager {
    pub tokens: HashMap<String, PushToken>,
    pub notifications: HashMap<String, EncryptedPushNotification>,
    pub user_tokens: HashMap<String, Vec<String>>, // user_id -> [token_ids]
}

impl PushNotificationManager {
    pub(crate) fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            notifications: HashMap::new(),
            user_tokens: HashMap::new(),
        }
    }

    /// Register push token
    pub(crate) fn register_token(&mut self, token: PushToken) -> Result<String, String> {
        let token_id = token.token_id.clone();
        let user_id = token.user_id.clone();

        self.tokens.insert(token_id.clone(), token);

        self.user_tokens
            .entry(user_id)
            .or_default()
            .push(token_id.clone());

        Ok(token_id)
    }

    /// Unregister push token
    pub(crate) fn unregister_token(&mut self, token_id: &str) -> Result<(), String> {
        if let Some(token) = self.tokens.remove(token_id) {
            if let Some(user_tokens) = self.user_tokens.get_mut(&token.user_id) {
                user_tokens.retain(|id| id != token_id);
            }
            Ok(())
        } else {
            Err("Token not found".to_string())
        }
    }

    /// Queue encrypted notification
    pub(crate) fn queue_notification(
        &mut self,
        notification: EncryptedPushNotification,
    ) -> Result<String, String> {
        let notification_id = notification.notification_id.clone();
        self.notifications.insert(notification_id.clone(), notification);
        Ok(notification_id)
    }

    /// Get pending notifications
    pub(crate) fn get_pending_notifications(&self, user_id: &str) -> Vec<&EncryptedPushNotification> {
        self.notifications
            .values()
            .filter(|n| {
                n.user_id == user_id
                    && !n.is_expired()
                    && n.sent_at.is_none()
                    && !n.failed
            })
            .collect()
    }

    /// Get user's active tokens
    pub(crate) fn get_user_tokens(&self, user_id: &str) -> Vec<&PushToken> {
        let token_ids = match self.user_tokens.get(user_id) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        token_ids
            .iter()
            .filter_map(|id| self.tokens.get(id))
            .filter(|t| t.is_active)
            .collect()
    }

    /// Cleanup expired notifications
    pub(crate) fn cleanup_expired(&mut self) {
        self.notifications
            .retain(|_, n| !n.is_expired());
    }
}

impl Default for PushNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_token_creation() {
        let token = PushToken::new(
            "user1".to_string(),
            "device1".to_string(),
            PushPlatform::Firebase,
            "fcm_token_123".to_string(),
            vec![0u8; 32],
        );

        assert_eq!(token.user_id, "user1");
        assert!(token.is_active);
    }

    #[test]
    fn test_encrypted_notification_creation() {
        let encryption_key = vec![0u8; 32];
        let payload = PushNotificationPayload {
            sender_id: "user2".to_string(),
            message: "Hello!".to_string(),
            action: None,
            deep_link: None,
            custom_data: HashMap::new(),
        };

        let notification = EncryptedPushNotification::new(
            "token123".to_string(),
            "user1".to_string(),
            NotificationType::IncomingMessage,
            "New Message".to_string(),
            payload,
            &encryption_key,
            NotificationPriority::High,
        );

        assert!(notification.is_ok());
        let notif = notification.unwrap();
        assert!(!notif.is_expired());
    }

    #[test]
    fn test_notification_manager() {
        let mut manager = PushNotificationManager::new();

        let token = PushToken::new(
            "user1".to_string(),
            "device1".to_string(),
            PushPlatform::Firebase,
            "token123".to_string(),
            vec![0u8; 32],
        );

        assert!(manager.register_token(token).is_ok());
        assert_eq!(manager.get_user_tokens("user1").len(), 1);
    }
}
