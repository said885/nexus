#![allow(missing_docs, dead_code)]

// Typing indicators and user presence
// nexus-relay/src/presence.rs

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum UserStatus {
    Online,
    Away,
    DoNotDisturb,
    Offline,
    Invisible,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TypingIndicator {
    pub user_id: String,
    pub conversation_id: String,
    pub started_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct UserPresence {
    pub user_id: String,
    pub status: UserStatus,
    pub last_seen: DateTime<Utc>,
    pub status_message: Option<String>,
    pub current_device: Option<String>,
}

pub(crate) struct PresenceManager {
    typing_indicators: HashMap<String, TypingIndicator>, // Key: user_id:conversation_id
    user_presence: HashMap<String, UserPresence>,
    typing_timeout_ms: u64,
}

impl PresenceManager {
    pub(crate) fn new() -> Self {
        PresenceManager {
            typing_indicators: HashMap::new(),
            user_presence: HashMap::new(),
            typing_timeout_ms: 3000, // 3 seconds
        }
    }

    pub(crate) fn set_user_status(
        &mut self,
        user_id: &str,
        status: UserStatus,
        status_message: Option<String>,
    ) -> Result<UserPresence, String> {
        let presence = UserPresence {
            user_id: user_id.to_string(),
            status,
            last_seen: Utc::now(),
            status_message,
            current_device: None,
        };

        self.user_presence
            .insert(user_id.to_string(), presence.clone());
        Ok(presence)
    }

    pub(crate) fn set_typing(
        &mut self,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<TypingIndicator, String> {
        let key = format!("{}:{}", user_id, conversation_id);
        
        if let Some(indicator) = self.typing_indicators.get_mut(&key) {
            indicator.last_update = Utc::now();
        } else {
            let indicator = TypingIndicator {
                user_id: user_id.to_string(),
                conversation_id: conversation_id.to_string(),
                started_at: Utc::now(),
                last_update: Utc::now(),
            };
            self.typing_indicators.insert(key.clone(), indicator);
        }

        Ok(self.typing_indicators.get(&key).unwrap().clone())
    }

    pub(crate) fn clear_typing(&mut self, user_id: &str, conversation_id: &str) -> Result<(), String> {
        let key = format!("{}:{}", user_id, conversation_id);
        if self.typing_indicators.remove(&key).is_some() {
            Ok(())
        } else {
            Err("Typing indicator not found".to_string())
        }
    }

    pub(crate) fn get_typing_users(&self, conversation_id: &str) -> Vec<TypingIndicator> {
        let now = Utc::now();
        let timeout = Duration::milliseconds(self.typing_timeout_ms as i64);

        self.typing_indicators
            .values()
            .filter(|ti| {
                ti.conversation_id == conversation_id
                    && now.signed_duration_since(ti.last_update) < timeout
            })
            .cloned()
            .collect()
    }

    pub(crate) fn get_user_presence(&self, user_id: &str) -> Option<&UserPresence> {
        self.user_presence.get(user_id)
    }

    pub(crate) fn cleanup_stale_typing(&mut self) {
        let now = Utc::now();
        let timeout = Duration::milliseconds(self.typing_timeout_ms as i64);

        self.typing_indicators.retain(|_, ti| {
            now.signed_duration_since(ti.last_update) < timeout
        });
    }

    pub(crate) fn get_all_user_statuses(&self) -> Vec<UserPresence> {
        self.user_presence.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_user_status() {
        let mut manager = PresenceManager::new();
        let result = manager.set_user_status("user_1", UserStatus::Online, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, UserStatus::Online);
    }

    #[test]
    fn test_set_typing() {
        let mut manager = PresenceManager::new();
        let result = manager.set_typing("user_1", "conv_1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().user_id, "user_1");
    }

    #[test]
    fn test_clear_typing() {
        let mut manager = PresenceManager::new();
        manager.set_typing("user_1", "conv_1").unwrap();
        let result = manager.clear_typing("user_1", "conv_1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_typing_users() {
        let mut manager = PresenceManager::new();
        manager.set_typing("user_1", "conv_1").unwrap();
        manager.set_typing("user_2", "conv_1").unwrap();

        let typing_users = manager.get_typing_users("conv_1");
        assert_eq!(typing_users.len(), 2);
    }

    #[test]
    fn test_user_presence_status_message() {
        let mut manager = PresenceManager::new();
        let result = manager.set_user_status(
            "user_1",
            UserStatus::Away,
            Some("On vacation".to_string()),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().status_message,
            Some("On vacation".to_string())
        );
    }

    #[test]
    fn test_get_user_presence() {
        let mut manager = PresenceManager::new();
        manager
            .set_user_status("user_1", UserStatus::Online, None)
            .unwrap();

        let presence = manager.get_user_presence("user_1");
        assert!(presence.is_some());
        assert_eq!(presence.unwrap().status, UserStatus::Online);
    }
}
