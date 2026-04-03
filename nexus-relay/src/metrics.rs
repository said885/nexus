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

//! Prometheus metrics for monitoring
//!
//! Exposes /metrics endpoint for Prometheus scraping

use prometheus::{Counter, Gauge, Registry, TextEncoder};
use std::sync::Arc;

pub(crate) struct Metrics {
    pub registry: Registry,
    pub messages_sent: Counter,
    pub messages_queued: Counter,
    pub connected_clients: Gauge,
    pub errors_total: Counter,
}

impl Metrics {
    pub(crate) fn new() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let registry = Registry::new();

        let messages_sent = Counter::new("nexus_messages_sent_total", "Total messages sent")?;
        let messages_queued = Counter::new("nexus_messages_queued_total", "Total messages queued")?;
        let connected_clients = Gauge::new("nexus_connected_clients", "Currently connected clients")?;
        let errors_total = Counter::new("nexus_errors_total", "Total errors")?;

        registry.register(Box::new(messages_sent.clone()))?;
        registry.register(Box::new(messages_queued.clone()))?;
        registry.register(Box::new(connected_clients.clone()))?;
        registry.register(Box::new(errors_total.clone()))?;

        Ok(Arc::new(Self {
            registry,
            messages_sent,
            messages_queued,
            connected_clients,
            errors_total,
        }))
    }

    pub(crate) fn metrics_response(&self) -> String {
        let encoder = TextEncoder::new();
        encoder
            .encode_to_string(&self.registry.gather())
            .unwrap_or_default()
    }
}
