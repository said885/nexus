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

/**
 * NEXUS Threat Detection Metrics
 * Prometheus metrics for real-time threat monitoring
 * Grafana-compatible alerts
 */

use prometheus::{Counter, Gauge, IntCounter, IntGauge, Registry};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SIGNATURE_FAILURES: Counter = Counter::new("nexus_signature_failures_total", "Total signature verification failures")
        .expect("Failed to create signature_failures metric");

    pub static ref REPLAY_ATTEMPTS: Counter = Counter::new("nexus_replay_attempts_total", "Total replay attack attempts detected")
        .expect("Failed to create replay_attempts metric");

    pub static ref AUTH_FAILURES: Counter = Counter::new("nexus_auth_failures_total", "Total authentication failures")
        .expect("Failed to create auth_failures metric");

    pub static ref BLOCKED_IPS: Gauge = Gauge::new("nexus_blocked_ips_total", "Total IPs currently blocked")
        .expect("Failed to create blocked_ips metric");

    pub static ref THREAT_LEVEL: IntGauge = IntGauge::new("nexus_threat_level", "Current threat level (0=normal, 1=suspicious, 2=high, 3=critical)")
        .expect("Failed to create threat_level metric");

    pub static ref SUSPICIOUS_CLIENTS: Gauge = Gauge::new("nexus_suspicious_clients", "Number of suspicious clients")
        .expect("Failed to create suspicious_clients metric");

    pub static ref HIGH_THREAT_CLIENTS: Gauge = Gauge::new("nexus_high_threat_clients", "Number of high threat clients")
        .expect("Failed to create high_threat_clients metric");

    pub static ref CRITICAL_THREAT_CLIENTS: Gauge = Gauge::new("nexus_critical_threat_clients", "Number of critical threat clients")
        .expect("Failed to create critical_threat_clients metric");
}

pub fn register_metrics(registry: &Registry) -> prometheus::Result<()> {
    registry.register(Box::new(SIGNATURE_FAILURES.clone()))?;
    registry.register(Box::new(REPLAY_ATTEMPTS.clone()))?;
    registry.register(Box::new(AUTH_FAILURES.clone()))?;
    registry.register(Box::new(BLOCKED_IPS.clone()))?;
    registry.register(Box::new(THREAT_LEVEL.clone()))?;
    registry.register(Box::new(SUSPICIOUS_CLIENTS.clone()))?;
    registry.register(Box::new(HIGH_THREAT_CLIENTS.clone()))?;
    registry.register(Box::new(CRITICAL_THREAT_CLIENTS.clone()))?;
    Ok(())
}
