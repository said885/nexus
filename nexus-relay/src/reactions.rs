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

// Advanced message reactions system
// nexus-relay/src/reactions.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum ReactionType {
    Emoji(String),
    Custom(String),
    Thumbs,
    Heart,
    Laugh,
    Surprise,
    Sad,
    Angry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Reaction {
    pub id: String,
    pub message_id: String,
    pub user_id: String,
    pub reaction_type: ReactionType,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MessageReactions {
    pub message_id: String,
    pub reactions: HashMap<String, Vec<Reaction>>,
    pub total_reactions: usize,
}

pub(crate) struct ReactionManager {
    reactions: HashMap<String, MessageReactions>,
}

impl ReactionManager {
    pub(crate) fn new() -> Self {
        ReactionManager {
            reactions: HashMap::new(),
        }
    }

    pub(crate) fn add_reaction(
        &mut self,
        message_id: &str,
        user_id: &str,
        reaction_type: ReactionType,
    ) -> Result<Reaction, String> {
        let reaction_id = format!("reaction_{}", uuid::Uuid::new_v4());
        let reaction = Reaction {
            id: reaction_id.clone(),
            message_id: message_id.to_string(),
            user_id: user_id.to_string(),
            reaction_type: reaction_type.clone(),
            created_at: Utc::now(),
        };

        let message_reactions = self
            .reactions
            .entry(message_id.to_string())
            .or_insert_with(|| MessageReactions {
                message_id: message_id.to_string(),
                reactions: HashMap::new(),
                total_reactions: 0,
            });

        let key = format!("{:?}", reaction_type);
        message_reactions
            .reactions
            .entry(key)
            .or_default()
            .push(reaction.clone());
        message_reactions.total_reactions += 1;

        Ok(reaction)
    }

    pub(crate) fn remove_reaction(&mut self, message_id: &str, reaction_id: &str) -> Result<(), String> {
        if let Some(message_reactions) = self.reactions.get_mut(message_id) {
            for reactions in message_reactions.reactions.values_mut() {
                if let Some(pos) = reactions.iter().position(|r| r.id == reaction_id) {
                    reactions.remove(pos);
                    message_reactions.total_reactions = message_reactions.total_reactions.saturating_sub(1);
                    return Ok(());
                }
            }
            Err("Reaction not found".to_string())
        } else {
            Err("Message not found".to_string())
        }
    }

    pub(crate) fn get_reactions(&self, message_id: &str) -> Option<&MessageReactions> {
        self.reactions.get(message_id)
    }

    pub(crate) fn get_reaction_summary(&self, message_id: &str) -> Option<Vec<(String, usize)>> {
        self.reactions.get(message_id).map(|mr| {
            mr.reactions
                .iter()
                .map(|(k, v)| (k.clone(), v.len()))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_reaction() {
        let mut manager = ReactionManager::new();
        let result = manager.add_reaction("msg_1", "user_1", ReactionType::Heart);
        assert!(result.is_ok());
        let reaction = result.unwrap();
        assert_eq!(reaction.message_id, "msg_1");
        assert_eq!(reaction.user_id, "user_1");
    }

    #[test]
    fn test_remove_reaction() {
        let mut manager = ReactionManager::new();
        let reaction = manager
            .add_reaction("msg_1", "user_1", ReactionType::Heart)
            .unwrap();
        let result = manager.remove_reaction("msg_1", &reaction.id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_reactions() {
        let mut manager = ReactionManager::new();
        manager
            .add_reaction("msg_1", "user_1", ReactionType::Heart)
            .unwrap();
        manager
            .add_reaction("msg_1", "user_2", ReactionType::Laugh)
            .unwrap();

        let reactions = manager.get_reactions("msg_1");
        assert!(reactions.is_some());
        assert_eq!(reactions.unwrap().total_reactions, 2);
    }

    #[test]
    fn test_emoji_reaction() {
        let mut manager = ReactionManager::new();
        let result = manager.add_reaction("msg_1", "user_1", ReactionType::Emoji("🚀".to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_reaction_summary() {
        let mut manager = ReactionManager::new();
        manager
            .add_reaction("msg_1", "user_1", ReactionType::Heart)
            .unwrap();
        manager
            .add_reaction("msg_1", "user_2", ReactionType::Heart)
            .unwrap();

        let summary = manager.get_reaction_summary("msg_1");
        assert!(summary.is_some());
        assert_eq!(summary.unwrap().len(), 1);
    }
}
