#![allow(missing_docs, dead_code)]

/**
 * NEXUS Behavioral Threat Detection
 * Real-time detection of compromised clients
 * Automatic isolation of suspicious activity
 * Observable via Prometheus metrics + Grafana alerts
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use tracing::{warn, error, debug};

const MAX_SIGNATURE_FAILURES: u32 = 5;
const MAX_REPLAY_ATTEMPTS: u32 = 2;
const MAX_MESSAGE_RATE: f64 = 100.0;
const MAX_AUTH_FAILURES: u32 = 10;
const MAX_CONNECTIONS_WITHOUT_AUTH: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThreatLevel {
    Normal,
    Suspicious,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub(crate) struct ClientBehavior {
    pub ip: String,
    pub signature_failures: u32,
    pub auth_failures: u32,
    pub replay_attempts: u32,
    pub message_rate: f64,
    pub challenge_responses: u32,
    pub connections: usize,
    pub first_seen: Instant,
    pub last_activity: Instant,
}

impl ClientBehavior {
    fn new(ip: String) -> Self {
        Self {
            ip,
            signature_failures: 0,
            auth_failures: 0,
            replay_attempts: 0,
            message_rate: 0.0,
            challenge_responses: 0,
            connections: 1,
            first_seen: Instant::now(),
            last_activity: Instant::now(),
        }
    }

    pub(crate) fn threat_score(&self) -> u32 {
        let mut score = 0u32;

        if self.signature_failures > MAX_SIGNATURE_FAILURES {
            score += 50;
        }

        if self.replay_attempts > MAX_REPLAY_ATTEMPTS {
            score += 80;
        }

        if self.message_rate > MAX_MESSAGE_RATE {
            score += 60;
        }

        if self.auth_failures > MAX_AUTH_FAILURES {
            score += 40;
        }

        if self.challenge_responses == 0 && self.connections > MAX_CONNECTIONS_WITHOUT_AUTH {
            score += 70;
        }

        score
    }

    pub(crate) fn threat_level(&self) -> ThreatLevel {
        let score = self.threat_score();
        match score {
            0..=20 => ThreatLevel::Normal,
            21..=50 => ThreatLevel::Suspicious,
            51..=79 => ThreatLevel::High,
            _ => ThreatLevel::Critical,
        }
    }
}

pub(crate) struct ThreatDetector {
    behaviors: Arc<RwLock<HashMap<String, ClientBehavior>>>,
    blocked_ips: Arc<RwLock<HashMap<String, Instant>>>,
    blocked_timeout_secs: u64,
}

impl ThreatDetector {
    pub(crate) fn new(blocked_timeout_secs: u64) -> Self {
        Self {
            behaviors: Arc::new(RwLock::new(HashMap::new())),
            blocked_ips: Arc::new(RwLock::new(HashMap::new())),
            blocked_timeout_secs,
        }
    }

    pub(crate) fn record_signature_failure(&self, ip: &str) {
        let mut behaviors = self.behaviors.write();
        let behavior = behaviors
            .entry(ip.to_string())
            .or_insert_with(|| ClientBehavior::new(ip.to_string()));

        behavior.signature_failures += 1;
        behavior.last_activity = Instant::now();

        if behavior.threat_level() == ThreatLevel::Critical {
            warn!("CRITICAL: {} signature failures from {}", behavior.signature_failures, ip);
            self.block_ip(ip);
        }
    }

    pub(crate) fn record_replay_attempt(&self, ip: &str) {
        let mut behaviors = self.behaviors.write();
        let behavior = behaviors
            .entry(ip.to_string())
            .or_insert_with(|| ClientBehavior::new(ip.to_string()));

        behavior.replay_attempts += 1;
        behavior.last_activity = Instant::now();

        if behavior.replay_attempts > MAX_REPLAY_ATTEMPTS {
            warn!("REPLAY ATTACK: {} attempts from {}", behavior.replay_attempts, ip);
            self.block_ip(ip);
        }
    }

    pub(crate) fn record_auth_failure(&self, ip: &str) {
        let mut behaviors = self.behaviors.write();
        let behavior = behaviors
            .entry(ip.to_string())
            .or_insert_with(|| ClientBehavior::new(ip.to_string()));

        behavior.auth_failures += 1;
        behavior.last_activity = Instant::now();

        if behavior.auth_failures > MAX_AUTH_FAILURES {
            warn!("BRUTE FORCE: {} auth failures from {}", behavior.auth_failures, ip);
            self.block_ip(ip);
        }
    }

    pub(crate) fn record_auth_success(&self, ip: &str) {
        let mut behaviors = self.behaviors.write();
        let behavior = behaviors
            .entry(ip.to_string())
            .or_insert_with(|| ClientBehavior::new(ip.to_string()));

        behavior.challenge_responses += 1;
        behavior.auth_failures = 0;
        behavior.signature_failures = 0;
        behavior.last_activity = Instant::now();

        debug!("Auth success for {}: {} valid responses", ip, behavior.challenge_responses);
    }

    pub(crate) fn record_message_activity(&self, ip: &str, message_count: u32) {
        let mut behaviors = self.behaviors.write();
        let behavior = behaviors
            .entry(ip.to_string())
            .or_insert_with(|| ClientBehavior::new(ip.to_string()));

        let elapsed = behavior.last_activity.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            behavior.message_rate = message_count as f64 / elapsed;
        }

        behavior.last_activity = Instant::now();

        if behavior.message_rate > MAX_MESSAGE_RATE {
            warn!("HIGH RATE: {:.2} msg/sec from {}", behavior.message_rate, ip);
        }
    }

    pub(crate) fn record_connection(&self, ip: &str) {
        let mut behaviors = self.behaviors.write();
        let behavior = behaviors
            .entry(ip.to_string())
            .or_insert_with(|| ClientBehavior::new(ip.to_string()));

        behavior.connections += 1;
        behavior.last_activity = Instant::now();
    }

    pub(crate) fn is_blocked(&self, ip: &str) -> bool {
        let blocked_ips = self.blocked_ips.read();

        if let Some(blocked_at) = blocked_ips.get(ip) {
            let elapsed = blocked_at.elapsed().as_secs();
            if elapsed < self.blocked_timeout_secs {
                return true;
            }
        }

        false
    }

    pub(crate) fn get_threat_level(&self, ip: &str) -> ThreatLevel {
        let behaviors = self.behaviors.read();
        behaviors
            .get(ip)
            .map(|b| b.threat_level())
            .unwrap_or(ThreatLevel::Normal)
    }

    pub(crate) fn get_behavior(&self, ip: &str) -> Option<ClientBehavior> {
        let behaviors = self.behaviors.read();
        behaviors.get(ip).cloned()
    }

    fn block_ip(&self, ip: &str) {
        let mut blocked_ips = self.blocked_ips.write();
        blocked_ips.insert(ip.to_string(), Instant::now());

        error!("BLOCKED IP: {} (timeout: {} sec)", ip, self.blocked_timeout_secs);
    }

    pub(crate) fn get_all_behaviors(&self) -> Vec<(String, ClientBehavior)> {
        let behaviors = self.behaviors.read();
        behaviors
            .iter()
            .map(|(ip, behavior)| (ip.clone(), behavior.clone()))
            .collect()
    }

    pub(crate) fn cleanup_old_entries(&self, age_secs: u64) {
        let mut behaviors = self.behaviors.write();
        let cutoff = Instant::now() - std::time::Duration::from_secs(age_secs);

        behaviors.retain(|_, behavior| behavior.last_activity > cutoff);

        let mut blocked_ips = self.blocked_ips.write();
        blocked_ips.retain(|_, blocked_at| blocked_at.elapsed().as_secs() < self.blocked_timeout_secs);

        debug!("Cleaned up old threat detection entries");
    }

    pub(crate) fn get_statistics(&self) -> ThreatStatistics {
        let behaviors = self.behaviors.read();
        let blocked_ips = self.blocked_ips.read();

        let mut stats = ThreatStatistics {
            total_clients: behaviors.len(),
            blocked_ips: blocked_ips.len(),
            normal_clients: 0,
            suspicious_clients: 0,
            high_threat_clients: 0,
            critical_threat_clients: 0,
            total_signature_failures: 0,
            total_replay_attempts: 0,
            total_auth_failures: 0,
        };

        for (_, behavior) in behaviors.iter() {
            match behavior.threat_level() {
                ThreatLevel::Normal => stats.normal_clients += 1,
                ThreatLevel::Suspicious => stats.suspicious_clients += 1,
                ThreatLevel::High => stats.high_threat_clients += 1,
                ThreatLevel::Critical => stats.critical_threat_clients += 1,
            }

            stats.total_signature_failures += behavior.signature_failures as usize;
            stats.total_replay_attempts += behavior.replay_attempts as usize;
            stats.total_auth_failures += behavior.auth_failures as usize;
        }

        stats
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ThreatStatistics {
    pub total_clients: usize,
    pub blocked_ips: usize,
    pub normal_clients: usize,
    pub suspicious_clients: usize,
    pub high_threat_clients: usize,
    pub critical_threat_clients: usize,
    pub total_signature_failures: usize,
    pub total_replay_attempts: usize,
    pub total_auth_failures: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_score_normal() {
        let behavior = ClientBehavior {
            ip: "192.168.1.1".to_string(),
            signature_failures: 0,
            auth_failures: 0,
            replay_attempts: 0,
            message_rate: 10.0,
            challenge_responses: 5,
            connections: 1,
            first_seen: Instant::now(),
            last_activity: Instant::now(),
        };

        assert_eq!(behavior.threat_level(), ThreatLevel::Normal);
    }

    #[test]
    fn test_threat_score_critical() {
        let behavior = ClientBehavior {
            ip: "192.168.1.1".to_string(),
            signature_failures: 10,
            auth_failures: 15,
            replay_attempts: 5,
            message_rate: 150.0,
            challenge_responses: 0,
            connections: 20,
            first_seen: Instant::now(),
            last_activity: Instant::now(),
        };

        assert_eq!(behavior.threat_level(), ThreatLevel::Critical);
    }

    #[test]
    fn test_threat_detector_blocking() {
        let detector = ThreatDetector::new(60);
        assert!(!detector.is_blocked("192.168.1.100"));
        detector.record_auth_failure("192.168.1.100");
        for _ in 0..10 {
            detector.record_auth_failure("192.168.1.100");
        }
        assert!(detector.is_blocked("192.168.1.100"));
    }
}
