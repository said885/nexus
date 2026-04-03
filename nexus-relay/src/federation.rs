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

//! Federation support - Multi-relay coordination
//!
//! Allows multiple NEXUS relay servers to federate and route messages across domains

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Federated relay peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FederatedRelay {
    pub domain: String,
    pub url: String,
    pub public_key: String,
    pub priority: u8,
    pub status: RelayStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) enum RelayStatus {
    #[serde(rename = "online")]
    Online,
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "degraded")]
    Degraded,
}

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FederationConfig {
    pub enabled: bool,
    pub domain_name: String,
    pub peer_relays: Vec<FederatedRelay>,
    pub federation_timeout_secs: u64,
}

/// Message for federation (cross-relay routing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FederatedMessage {
    pub message_id: String,
    pub sender_hash: String,
    pub sender_domain: String,
    pub recipient_hash: String,
    pub recipient_domain: String,
    pub content: Vec<u8>,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

pub(crate) struct FederationManager {
    config: FederationConfig,
    peer_status: HashMap<String, RelayStatus>,
}

impl FederationManager {
    pub(crate) fn new(config: FederationConfig) -> Self {
        let peer_status = config
            .peer_relays
            .iter()
            .map(|relay| (relay.domain.clone(), relay.status.clone()))
            .collect();

        Self { config, peer_status }
    }

    pub(crate) fn route_message(&self, msg: &FederatedMessage) -> Result<(), String> {
        // Check if recipient domain is local or remote
        if msg.recipient_domain == self.config.domain_name {
            // Local delivery
            info!(msg_id = %msg.message_id, "Local delivery");
            Ok(())
        } else {
            // Remote delivery - find peer
            let relay = self
                .config
                .peer_relays
                .iter()
                .find(|r| r.domain == msg.recipient_domain)
                .ok_or("Recipient domain not found")?;

            if relay.status != RelayStatus::Online {
                return Err("Recipient relay unavailable".to_string());
            }

            info!(
                msg_id = %msg.message_id,
                recipient_domain = %msg.recipient_domain,
                "Federated delivery"
            );
            Ok(())
        }
    }

    pub(crate) fn add_peer(&mut self, relay: FederatedRelay) {
        self.peer_status.insert(relay.domain.clone(), relay.status.clone());
        self.config.peer_relays.push(relay);
    }

    pub(crate) fn update_peer_status(&mut self, domain: &str, status: RelayStatus) {
        self.peer_status.insert(domain.to_string(), status);
    }

    pub(crate) fn get_online_peers(&self) -> Vec<&FederatedRelay> {
        self.config
            .peer_relays
            .iter()
            .filter(|r| self.peer_status.get(&r.domain) == Some(&RelayStatus::Online))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federation_routing() {
        let config = FederationConfig {
            enabled: true,
            domain_name: "relay1.nexus.org".to_string(),
            peer_relays: vec![
                FederatedRelay {
                    domain: "relay2.nexus.org".to_string(),
                    url: "https://relay2.nexus.org".to_string(),
                    public_key: "pk2".to_string(),
                    priority: 1,
                    status: RelayStatus::Online,
                },
            ],
            federation_timeout_secs: 10,
        };

        let manager = FederationManager::new(config);

        let msg = FederatedMessage {
            message_id: "msg1".to_string(),
            sender_hash: "alice".to_string(),
            sender_domain: "relay1.nexus.org".to_string(),
            recipient_hash: "bob".to_string(),
            recipient_domain: "relay2.nexus.org".to_string(),
            content: vec![],
            timestamp: 0,
            signature: vec![],
        };

        assert!(manager.route_message(&msg).is_ok());
    }
}
