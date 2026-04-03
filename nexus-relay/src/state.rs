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

use axum::extract::ws::Message;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Maximum offline messages queued per recipient.
pub const MAX_OFFLINE_QUEUE: usize = 100;

/// Maximum TTL for an offline message (7 days).
pub const MAX_TTL_SECONDS: u64 = 7 * 24 * 3600;

/// Rate-limit window duration.
pub const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

/// Maximum messages allowed per IP per window.
pub const RATE_LIMIT_MAX: u32 = 100;

/// Maximum sealed content size in bytes (256 KiB).
pub const MAX_SEALED_BYTES: usize = 256 * 1024;

/// Maximum one-time prekeys stored per identity.
pub const MAX_ONE_TIME_PREKEYS: usize = 100;

/// Maximum concurrent connections per IP.
pub const MAX_CONNECTIONS_PER_IP: usize = 5;

/// Maximum group members.
pub const MAX_GROUP_MEMBERS: usize = 256;

// ---------------------------------------------------------------------------
// User status
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum UserStatus {
    Online,
    Away,
    DoNotDisturb,
    #[default]
    Offline,
    Invisible,
}

// ---------------------------------------------------------------------------
// Connected client handle
// ---------------------------------------------------------------------------

/// Represents a currently connected WebSocket client.
pub struct Client {
    pub recipient_hash: [u8; 32],
    pub sender: mpsc::UnboundedSender<Message>,
    pub connected_at: Instant,
    pub last_seen: Instant,
    pub ip: String,
    pub status: UserStatus,
    pub status_message: Option<String>,
    pub message_count: AtomicU64,
}

impl Client {
    pub fn new(
        recipient_hash: [u8; 32],
        sender: mpsc::UnboundedSender<Message>,
        ip: String,
    ) -> Self {
        Self {
            recipient_hash,
            sender,
            connected_at: Instant::now(),
            last_seen: Instant::now(),
            ip,
            status: UserStatus::Online,
            status_message: None,
            message_count: AtomicU64::new(0),
        }
    }

    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    pub fn increment_messages(&self) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_message_count(&self) -> u64 {
        self.message_count.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Stored prekey bundle
// ---------------------------------------------------------------------------

pub struct StoredPreKeyBundle {
    pub identity_key_bytes: Vec<u8>,
    pub signed_prekey_bytes: Vec<u8>,
    pub one_time_prekeys: Vec<Vec<u8>>,
    pub registered_at: Instant,
    pub last_upload: Instant,
}

// ---------------------------------------------------------------------------
// Offline message
// ---------------------------------------------------------------------------

pub struct QueuedMessage {
    pub sealed_content: Vec<u8>,
    pub received_at: Instant,
    pub ttl_seconds: u64,
    pub priority: u8,
}

impl QueuedMessage {
    pub fn is_expired(&self) -> bool {
        self.received_at.elapsed().as_secs() >= self.ttl_seconds
    }
}

// ---------------------------------------------------------------------------
// Group chat
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub members: Vec<String>,
    pub admins: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_public: bool,
    pub max_members: usize,
    pub epoch: u64,
}

impl Group {
    pub fn new(name: String, owner: String) -> Self {
        let now = unix_now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: None,
            owner: owner.clone(),
            members: vec![owner],
            admins: vec![],
            created_at: now,
            updated_at: now,
            is_public: false,
            max_members: MAX_GROUP_MEMBERS,
            epoch: 0,
        }
    }

    pub fn add_member(&mut self, member: String) -> Result<(), String> {
        if self.members.len() >= self.max_members {
            return Err("Group is full".into());
        }
        if self.members.contains(&member) {
            return Err("Already a member".into());
        }
        self.members.push(member);
        self.epoch += 1;
        self.updated_at = unix_now();
        Ok(())
    }

    pub fn remove_member(&mut self, member: &str) -> Result<(), String> {
        if member == self.owner {
            return Err("Cannot remove owner".into());
        }
        let pos = self
            .members
            .iter()
            .position(|m| m == member)
            .ok_or("Not a member")?;
        self.members.remove(pos);
        self.admins.retain(|a| a != member);
        self.epoch += 1;
        self.updated_at = unix_now();
        Ok(())
    }

    pub fn is_member(&self, user: &str) -> bool {
        self.members.contains(&user.to_string())
    }

    pub fn is_admin(&self, user: &str) -> bool {
        self.owner == user || self.admins.contains(&user.to_string())
    }
}

// ---------------------------------------------------------------------------
// Typing indicator
// ---------------------------------------------------------------------------

pub struct TypingIndicator {
    pub user_id: String,
    pub conversation_id: String,
    pub started_at: Instant,
}

impl TypingIndicator {
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed() > Duration::from_secs(5)
    }
}

// ---------------------------------------------------------------------------
// Call session
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CallType {
    Audio,
    Video,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CallStatus {
    Ringing,
    Active,
    Ended,
    Missed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallSession {
    pub id: String,
    pub initiator: String,
    pub recipient: String,
    pub call_type: CallType,
    pub status: CallStatus,
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

impl CallSession {
    pub fn new(initiator: String, recipient: String, call_type: CallType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            initiator,
            recipient,
            call_type,
            status: CallStatus::Ringing,
            started_at: unix_now(),
            ended_at: None,
        }
    }

    pub fn end(&mut self) {
        self.status = CallStatus::Ended;
        self.ended_at = Some(unix_now());
    }

    pub fn duration_secs(&self) -> u64 {
        match self.ended_at {
            Some(end) => end - self.started_at,
            None => unix_now() - self.started_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub clients: DashMap<String, Client>,
    pub prekeys: DashMap<String, StoredPreKeyBundle>,
    pub offline_queue: DashMap<String, Vec<QueuedMessage>>,
    pub rate_limits: DashMap<String, (u32, Instant)>,
    pub ip_connections: DashMap<String, usize>,
    pub start_time: Instant,
    pub total_messages: AtomicU64,
    pub groups: DashMap<String, Group>,
    pub calls: DashMap<String, CallSession>,
    pub typing_indicators: DashMap<String, Vec<TypingIndicator>>,
    pub presence: DashMap<String, (UserStatus, Instant)>,
    pub delivery_receipts: DashMap<String, u64>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            clients: DashMap::new(),
            prekeys: DashMap::new(),
            offline_queue: DashMap::new(),
            rate_limits: DashMap::new(),
            ip_connections: DashMap::new(),
            start_time: Instant::now(),
            total_messages: AtomicU64::new(0),
            groups: DashMap::new(),
            calls: DashMap::new(),
            typing_indicators: DashMap::new(),
            presence: DashMap::new(),
            delivery_receipts: DashMap::new(),
        })
    }

    // Rate limiting
    pub fn check_rate_limit(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut entry = self.rate_limits.entry(ip.to_owned()).or_insert((0, now));
        let (count, window_start) = entry.value_mut();

        if now.duration_since(*window_start) >= RATE_LIMIT_WINDOW {
            *count = 1;
            *window_start = now;
            true
        } else if *count < RATE_LIMIT_MAX {
            *count += 1;
            true
        } else {
            false
        }
    }

    // Connection management
    pub fn can_connect(&self, ip: &str) -> bool {
        let count = self.ip_connections.get(ip).map(|c| *c.value()).unwrap_or(0);
        count < MAX_CONNECTIONS_PER_IP
    }

    pub fn add_connection(&self, ip: &str) {
        *self.ip_connections.entry(ip.to_owned()).or_insert(0) += 1;
    }

    pub fn remove_connection(&self, ip: &str) {
        if let Some(mut count) = self.ip_connections.get_mut(ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                drop(count);
                self.ip_connections.remove(ip);
            }
        }
    }

    // Message counting
    pub fn record_message(&self) {
        self.total_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_total_messages(&self) -> u64 {
        self.total_messages.load(Ordering::Relaxed)
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    // Offline queue
    pub fn enqueue_offline(
        &self,
        recipient_hex: &str,
        msg: QueuedMessage,
    ) -> Result<(), &'static str> {
        let mut queue = self
            .offline_queue
            .entry(recipient_hex.to_owned())
            .or_default();
        queue.retain(|m| !m.is_expired());
        if queue.len() >= MAX_OFFLINE_QUEUE {
            if let Some(pos) = queue.iter().position(|m| m.priority == 0) {
                queue.remove(pos);
            } else {
                return Err("queue full");
            }
        }
        queue.push(msg);
        Ok(())
    }

    pub fn drain_offline(&self, recipient_hex: &str) -> Vec<QueuedMessage> {
        match self.offline_queue.get_mut(recipient_hex) {
            Some(mut queue) => {
                let drained = std::mem::take(queue.value_mut());
                drained.into_iter().filter(|m| !m.is_expired()).collect()
            }
            None => Vec::new(),
        }
    }

    pub fn total_queued_messages(&self) -> usize {
        self.offline_queue.iter().map(|q| q.value().len()).sum()
    }

    // Group management
    pub fn create_group(&self, name: String, owner: String) -> Result<String, String> {
        let group = Group::new(name, owner);
        let id = group.id.clone();
        self.groups.insert(id.clone(), group);
        Ok(id)
    }

    pub fn get_group(&self, group_id: &str) -> Option<Group> {
        self.groups.get(group_id).map(|g| g.value().clone())
    }

    pub fn add_group_member(&self, group_id: &str, member: String) -> Result<(), String> {
        let mut group = self.groups.get_mut(group_id).ok_or("Group not found")?;
        group.add_member(member)
    }

    pub fn remove_group_member(&self, group_id: &str, member: &str) -> Result<(), String> {
        let mut group = self.groups.get_mut(group_id).ok_or("Group not found")?;
        group.remove_member(member)
    }

    // Call management
    pub fn start_call(&self, initiator: String, recipient: String, call_type: CallType) -> String {
        let call = CallSession::new(initiator, recipient, call_type);
        let id = call.id.clone();
        self.calls.insert(id.clone(), call);
        id
    }

    pub fn end_call(&self, call_id: &str) -> Result<(), String> {
        let mut call = self.calls.get_mut(call_id).ok_or("Call not found")?;
        call.end();
        Ok(())
    }

    pub fn get_call(&self, call_id: &str) -> Option<CallSession> {
        self.calls.get(call_id).map(|c| c.value().clone())
    }

    // Presence
    pub fn set_user_status(&self, user_id: &str, status: UserStatus) {
        self.presence
            .insert(user_id.to_string(), (status, Instant::now()));
    }

    pub fn get_user_status(&self, user_id: &str) -> UserStatus {
        self.presence
            .get(user_id)
            .map(|p| p.value().0.clone())
            .unwrap_or(UserStatus::Offline)
    }

    pub fn get_online_users(&self) -> Vec<String> {
        self.presence
            .iter()
            .filter(|p| p.value().0 == UserStatus::Online)
            .map(|p| p.key().clone())
            .collect()
    }

    // Typing indicators
    pub fn set_typing(&self, conversation_id: &str, user_id: &str) {
        let mut indicators = self
            .typing_indicators
            .entry(conversation_id.to_string())
            .or_default();
        indicators.retain(|i| i.user_id != user_id);
        indicators.push(TypingIndicator {
            user_id: user_id.to_string(),
            conversation_id: conversation_id.to_string(),
            started_at: Instant::now(),
        });
    }

    pub fn clear_typing(&self, conversation_id: &str, user_id: &str) {
        if let Some(mut indicators) = self.typing_indicators.get_mut(conversation_id) {
            indicators.retain(|i| i.user_id != user_id);
        }
    }

    pub fn get_typing_users(&self, conversation_id: &str) -> Vec<String> {
        match self.typing_indicators.get(conversation_id) {
            Some(indicators) => indicators
                .iter()
                .filter(|i| !i.is_expired())
                .map(|i| i.user_id.clone())
                .collect(),
            None => Vec::new(),
        }
    }

    // Delivery receipts
    pub fn record_delivery(&self, message_id: String) {
        self.delivery_receipts.insert(message_id, unix_now());
    }

    pub fn get_delivery_receipt(&self, message_id: &str) -> Option<u64> {
        self.delivery_receipts.get(message_id).map(|t| *t.value())
    }

    // Cleanup
    pub fn cleanup_expired(&self) {
        self.offline_queue.retain(|_, queue| {
            queue.retain(|m| !m.is_expired());
            !queue.is_empty()
        });

        let cutoff = 2 * RATE_LIMIT_WINDOW;
        self.rate_limits
            .retain(|_, (_, window_start)| window_start.elapsed() < cutoff);

        self.typing_indicators.retain(|_, indicators| {
            indicators.retain(|i| !i.is_expired());
            !indicators.is_empty()
        });

        let hour_ago = unix_now() - 3600;
        self.calls.retain(|_, call| match call.status {
            CallStatus::Ended | CallStatus::Missed => call.ended_at.is_none_or(|t| t > hour_ago),
            _ => true,
        });

        let day_ago = unix_now() - 86400;
        self.delivery_receipts
            .retain(|_, timestamp| *timestamp > day_ago);
    }

    // Statistics
    pub fn get_stats(&self) -> ServerStats {
        ServerStats {
            connected_clients: self.clients.len(),
            registered_identities: self.prekeys.len(),
            queued_messages: self.total_queued_messages(),
            active_groups: self.groups.len(),
            active_calls: self
                .calls
                .iter()
                .filter(|c| matches!(c.value().status, CallStatus::Ringing | CallStatus::Active))
                .count(),
            online_users: self.get_online_users().len(),
            total_messages: self.get_total_messages(),
            uptime_seconds: self.uptime_seconds(),
        }
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Serialize)]
pub struct ServerStats {
    pub connected_clients: usize,
    pub registered_identities: usize,
    pub queued_messages: usize,
    pub active_groups: usize,
    pub active_calls: usize,
    pub online_users: usize,
    pub total_messages: u64,
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let state = AppState::new();
        assert_eq!(state.clients.len(), 0);
        assert_eq!(state.get_total_messages(), 0);
    }

    #[test]
    fn test_rate_limiting() {
        let state = AppState::new();
        for _ in 0..RATE_LIMIT_MAX {
            assert!(state.check_rate_limit("1.1.1.1"));
        }
        assert!(!state.check_rate_limit("1.1.1.1"));
    }

    #[test]
    fn test_connection_limit() {
        let state = AppState::new();
        for _ in 0..MAX_CONNECTIONS_PER_IP {
            state.add_connection("1.1.1.1");
        }
        assert!(!state.can_connect("1.1.1.1"));
    }

    #[test]
    fn test_messages() {
        let state = AppState::new();
        state.record_message();
        assert_eq!(state.get_total_messages(), 1);
    }

    #[test]
    fn test_offline_queue() {
        let state = AppState::new();
        let msg = QueuedMessage {
            sealed_content: vec![1],
            received_at: Instant::now(),
            ttl_seconds: 3600,
            priority: 0,
        };
        assert!(state.enqueue_offline("u1", msg).is_ok());
        assert_eq!(state.drain_offline("u1").len(), 1);
    }

    #[test]
    fn test_groups() {
        let state = AppState::new();
        let id = state.create_group("G".into(), "a".into()).unwrap();
        assert!(state.add_group_member(&id, "b".into()).is_ok());
        assert!(state.get_group(&id).unwrap().is_member("b"));
    }

    #[test]
    fn test_calls() {
        let state = AppState::new();
        let id = state.start_call("a".into(), "b".into(), CallType::Audio);
        assert!(state.end_call(&id).is_ok());
    }

    #[test]
    fn test_presence() {
        let state = AppState::new();
        state.set_user_status("a", UserStatus::Online);
        assert_eq!(state.get_user_status("a"), UserStatus::Online);
    }

    #[test]
    fn test_typing() {
        let state = AppState::new();
        state.set_typing("c", "u");
        assert_eq!(state.get_typing_users("c").len(), 1);
    }

    #[test]
    fn test_stats() {
        let state = AppState::new();
        state.record_message();
        assert_eq!(state.get_stats().total_messages, 1);
    }
}
