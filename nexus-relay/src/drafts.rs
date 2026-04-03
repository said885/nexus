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

// Draft messages system
// nexus-relay/src/drafts.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DraftMessage {
    pub id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
    pub attachments: Vec<String>, // File IDs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MessageThread {
    pub id: String,
    pub root_message_id: String,
    pub conversation_id: String,
    pub participant_count: usize,
    pub reply_count: usize,
    pub created_at: DateTime<Utc>,
    pub last_reply_at: DateTime<Utc>,
}

pub(crate) struct DraftManager {
    drafts: HashMap<String, DraftMessage>,
    threads: HashMap<String, MessageThread>,
}

impl DraftManager {
    pub(crate) fn new() -> Self {
        DraftManager {
            drafts: HashMap::new(),
            threads: HashMap::new(),
        }
    }

    pub(crate) fn create_draft(
        &mut self,
        user_id: &str,
        conversation_id: &str,
        content: &str,
    ) -> Result<DraftMessage, String> {
        let draft_id = format!("draft_{}", uuid::Uuid::new_v4());
        let draft = DraftMessage {
            id: draft_id.clone(),
            user_id: user_id.to_string(),
            conversation_id: conversation_id.to_string(),
            content: content.to_string(),
            created_at: Utc::now(),
            last_updated_at: Utc::now(),
            attachments: Vec::new(),
        };

        self.drafts.insert(draft_id, draft.clone());
        Ok(draft)
    }

    pub(crate) fn update_draft(
        &mut self,
        draft_id: &str,
        content: &str,
    ) -> Result<DraftMessage, String> {
        if let Some(draft) = self.drafts.get_mut(draft_id) {
            draft.content = content.to_string();
            draft.last_updated_at = Utc::now();
            Ok(draft.clone())
        } else {
            Err("Draft not found".to_string())
        }
    }

    pub(crate) fn add_attachment_to_draft(
        &mut self,
        draft_id: &str,
        file_id: &str,
    ) -> Result<(), String> {
        if let Some(draft) = self.drafts.get_mut(draft_id) {
            draft.attachments.push(file_id.to_string());
            draft.last_updated_at = Utc::now();
            Ok(())
        } else {
            Err("Draft not found".to_string())
        }
    }

    pub(crate) fn remove_attachment_from_draft(
        &mut self,
        draft_id: &str,
        file_id: &str,
    ) -> Result<(), String> {
        if let Some(draft) = self.drafts.get_mut(draft_id) {
            draft.attachments.retain(|id| id != file_id);
            draft.last_updated_at = Utc::now();
            Ok(())
        } else {
            Err("Draft not found".to_string())
        }
    }

    pub(crate) fn get_draft(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> Option<&DraftMessage> {
        self.drafts
            .values()
            .find(|d| d.user_id == user_id && d.conversation_id == conversation_id)
    }

    pub(crate) fn delete_draft(&mut self, draft_id: &str) -> Result<(), String> {
        if self.drafts.remove(draft_id).is_some() {
            Ok(())
        } else {
            Err("Draft not found".to_string())
        }
    }

    pub(crate) fn create_thread(
        &mut self,
        root_message_id: &str,
        conversation_id: &str,
    ) -> Result<MessageThread, String> {
        let thread_id = format!("thread_{}", uuid::Uuid::new_v4());
        let thread = MessageThread {
            id: thread_id.clone(),
            root_message_id: root_message_id.to_string(),
            conversation_id: conversation_id.to_string(),
            participant_count: 1,
            reply_count: 0,
            created_at: Utc::now(),
            last_reply_at: Utc::now(),
        };

        self.threads.insert(thread_id, thread.clone());
        Ok(thread)
    }

    pub(crate) fn add_reply_to_thread(&mut self, thread_id: &str) -> Result<(), String> {
        if let Some(thread) = self.threads.get_mut(thread_id) {
            thread.reply_count += 1;
            thread.last_reply_at = Utc::now();
            Ok(())
        } else {
            Err("Thread not found".to_string())
        }
    }

    pub(crate) fn get_thread(&self, thread_id: &str) -> Option<&MessageThread> {
        self.threads.get(thread_id)
    }

    pub(crate) fn get_thread_by_root_message(&self, root_message_id: &str) -> Option<&MessageThread> {
        self.threads
            .values()
            .find(|t| t.root_message_id == root_message_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_draft() {
        let mut manager = DraftManager::new();
        let result = manager.create_draft("user_1", "conv_1", "Hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Hello");
    }

    #[test]
    fn test_update_draft() {
        let mut manager = DraftManager::new();
        let draft = manager.create_draft("user_1", "conv_1", "Hello").unwrap();
        let result = manager.update_draft(&draft.id, "Hello World");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Hello World");
    }

    #[test]
    fn test_add_attachment() {
        let mut manager = DraftManager::new();
        let draft = manager.create_draft("user_1", "conv_1", "Check this").unwrap();
        let result = manager.add_attachment_to_draft(&draft.id, "file_123");
        assert!(result.is_ok());

        let updated = manager.get_draft("user_1", "conv_1").unwrap();
        assert_eq!(updated.attachments.len(), 1);
    }

    #[test]
    fn test_get_draft() {
        let mut manager = DraftManager::new();
        manager.create_draft("user_1", "conv_1", "Hello").unwrap();
        let draft = manager.get_draft("user_1", "conv_1");
        assert!(draft.is_some());
    }

    #[test]
    fn test_create_thread() {
        let mut manager = DraftManager::new();
        let result = manager.create_thread("msg_1", "conv_1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().reply_count, 0);
    }

    #[test]
    fn test_add_reply_to_thread() {
        let mut manager = DraftManager::new();
        let thread = manager.create_thread("msg_1", "conv_1").unwrap();
        let result = manager.add_reply_to_thread(&thread.id);
        assert!(result.is_ok());

        let updated = manager.get_thread(&thread.id).unwrap();
        assert_eq!(updated.reply_count, 1);
    }
}
