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

// Notification service module
// nexus-relay/src/notifications.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum NotificationChannel {
    Email,
    Sms,
    PushNotification,
    InApp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate)  struct Notification {
    pub id: String,
    pub recipient_id: String,
    pub channel: NotificationChannel,
    pub title: String,
    pub body: String,
    pub action_url: Option<String>,
    pub sent_at: DateTime<Utc>,
    pub read: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NotificationPreferences {
    pub user_id: String,
    pub email_notifications: bool,
    pub sms_notifications: bool,
    pub push_notifications: bool,
    pub frequency: NotificationFrequency,
    pub quiet_hours_start: String, // Format: "HH:MM"
    pub quiet_hours_end: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum NotificationFrequency {
    Instant,
    Daily,
    Weekly,
    Never,
}

pub(crate) struct NotificationService {
    notifications: HashMap<String, Vec<Notification>>,
    preferences: HashMap<String, NotificationPreferences>,
}

impl NotificationService {
    pub(crate) fn new() -> Self {
        NotificationService {
            notifications: HashMap::new(),
            preferences: HashMap::new(),
        }
    }

    pub(crate) fn send_notification(
        &mut self,
        recipient_id: &str,
        channel: NotificationChannel,
        title: &str,
        body: &str,
        action_url: Option<String>,
    ) -> Result<Notification, String> {
        // Check user preferences
        if let Some(prefs) = self.preferences.get(recipient_id) {
            match channel {
                NotificationChannel::Email if !prefs.email_notifications => {
                    return Err("Email notifications disabled".to_string());
                }
                NotificationChannel::Sms if !prefs.sms_notifications => {
                    return Err("SMS notifications disabled".to_string());
                }
                NotificationChannel::PushNotification if !prefs.push_notifications => {
                    return Err("Push notifications disabled".to_string());
                }
                _ => {}
            }
        }

        let notification = Notification {
            id: format!("notif_{}", uuid::Uuid::new_v4()),
            recipient_id: recipient_id.to_string(),
            channel,
            title: title.to_string(),
            body: body.to_string(),
            action_url,
            sent_at: Utc::now(),
            read: false,
        };

        self.notifications
            .entry(recipient_id.to_string())
            .or_default()
            .push(notification.clone());

        Ok(notification)
    }

    pub(crate) fn set_preferences(
        &mut self,
        user_id: &str,
        preferences: NotificationPreferences,
    ) -> Result<(), String> {
        self.preferences.insert(user_id.to_string(), preferences);
        Ok(())
    }

    pub(crate) fn get_preferences(&self, user_id: &str) -> Option<&NotificationPreferences> {
        self.preferences.get(user_id)
    }

    pub(crate) fn mark_as_read(&mut self, user_id: &str, notification_id: &str) -> Result<(), String> {
        if let Some(notifications) = self.notifications.get_mut(user_id) {
            if let Some(notif) = notifications
                .iter_mut()
                .find(|n| n.id == notification_id)
            {
                notif.read = true;
                return Ok(());
            }
        }
        Err("Notification not found".to_string())
    }

    pub(crate) fn get_unread_count(&self, user_id: &str) -> usize {
        self.notifications
            .get(user_id)
            .map(|notifs| notifs.iter().filter(|n| !n.read).count())
            .unwrap_or(0)
    }

    pub(crate) fn get_notifications(&self, user_id: &str) -> Vec<&Notification> {
        self.notifications
            .get(user_id)
            .map(|notifs| notifs.iter().collect())
            .unwrap_or_default()
    }

    pub(crate) fn delete_notification(
        &mut self,
        user_id: &str,
        notification_id: &str,
    ) -> Result<(), String> {
        if let Some(notifications) = self.notifications.get_mut(user_id) {
            if let Some(pos) = notifications.iter().position(|n| n.id == notification_id) {
                notifications.remove(pos);
                return Ok(());
            }
        }
        Err("Notification not found".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_notification() {
        let mut service = NotificationService::new();
        let result = service.send_notification(
            "user_1",
            NotificationChannel::InApp,
            "Hello",
            "You have a new message",
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_disabled_notification_channel() {
        let mut service = NotificationService::new();
        let prefs = NotificationPreferences {
            user_id: "user_1".to_string(),
            email_notifications: false,
            sms_notifications: true,
            push_notifications: true,
            frequency: NotificationFrequency::Instant,
            quiet_hours_start: "22:00".to_string(),
            quiet_hours_end: "08:00".to_string(),
        };
        service.set_preferences("user_1", prefs).unwrap();

        let result = service.send_notification(
            "user_1",
            NotificationChannel::Email,
            "Test",
            "Body",
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_mark_as_read() {
        let mut service = NotificationService::new();
        let notif = service
            .send_notification("user_1", NotificationChannel::InApp, "Hi", "Body", None)
            .unwrap();

        let result = service.mark_as_read("user_1", &notif.id);
        assert!(result.is_ok());
        assert!(service
            .get_notifications("user_1")
            .first()
            .unwrap()
            .read);
    }

    #[test]
    fn test_unread_count() {
        let mut service = NotificationService::new();
        service
            .send_notification("user_1", NotificationChannel::InApp, "Hi", "Body", None)
            .unwrap();
        service
            .send_notification("user_1", NotificationChannel::InApp, "Hi", "Body", None)
            .unwrap();

        assert_eq!(service.get_unread_count("user_1"), 2);
    }

    #[test]
    fn test_delete_notification() {
        let mut service = NotificationService::new();
        let notif = service
            .send_notification("user_1", NotificationChannel::InApp, "Hi", "Body", None)
            .unwrap();

        let result = service.delete_notification("user_1", &notif.id);
        assert!(result.is_ok());
        assert_eq!(service.get_unread_count("user_1"), 0);
    }
}
