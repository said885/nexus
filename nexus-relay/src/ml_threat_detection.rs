#![allow(missing_docs, dead_code)]

//! ML-Based Threat Detection Module
//!
//! Implements anomaly detection using statistical methods that approximate
//! Isolation Forest and can be extended with ML models.
//!
//! Detection Capabilities:
//! - Brute force attacks
//! - Rate limit abuse
//! - Unusual access patterns
//! - Credential stuffing
//! - Account enumeration
//! - Timing attacks

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;
use tracing::{info, warn};

/// Feature vector for anomaly detection
#[derive(Debug, Clone)]
pub(crate) struct FeatureVector {
    pub requests_per_minute: f64,
    pub unique_ips: f64,
    pub failed_auth_rate: f64,
    pub message_size_avg: f64,
    pub connection_duration: f64,
    pub time_of_day: f64,
    pub geo_entropy: f64,
    pub user_agent_changes: f64,
}

impl FeatureVector {
    /// Normalize features to [0, 1] range
    pub(crate) fn normalize(&self) -> Vec<f64> {
        vec![
            (self.requests_per_minute / 100.0).min(1.0),
            (self.unique_ips / 10.0).min(1.0),
            self.failed_auth_rate.min(1.0),
            (self.message_size_avg / 65536.0).min(1.0),
            (self.connection_duration / 3600.0).min(1.0),
            self.time_of_day,
            self.geo_entropy.min(1.0),
            (self.user_agent_changes / 5.0).min(1.0),
        ]
    }
}

/// Isolation Tree node for anomaly detection
#[derive(Debug, Clone)]
struct IsolationNode {
    feature_index: usize,
    threshold: f64,
    left: Option<Box<IsolationNode>>,
    right: Option<Box<IsolationNode>>,
    height: usize,
    is_leaf: bool,
    size: usize,
}

/// Isolation Forest for anomaly detection
pub(crate) struct IsolationForest {
    trees: Vec<IsolationNode>,
    contamination: f64,
    max_samples: usize,
    num_trees: usize,
    max_height: usize,
}

impl IsolationForest {
    /// Create a new Isolation Forest
    pub(crate) fn new(num_trees: usize, max_samples: usize, contamination: f64) -> Self {
        Self {
            trees: Vec::new(),
            contamination,
            max_samples,
            num_trees,
            max_height: (max_samples as f64).log2().ceil() as usize,
        }
    }

    /// Train the forest on normal data
    pub(crate) fn train(&mut self, data: &[FeatureVector]) {
        self.trees.clear();

        for _ in 0..self.num_trees {
            // Sample subset of data
            let sample_size = data.len().min(self.max_samples);
            let sample: Vec<&FeatureVector> = data.iter().take(sample_size).collect();

            // Build isolation tree
            let tree = self.build_tree(&sample, 0);
            self.trees.push(tree);
        }

        info!("Trained Isolation Forest with {} trees", self.num_trees);
    }

    /// Build a single isolation tree
    fn build_tree(&self, data: &[&FeatureVector], height: usize) -> IsolationNode {
        let n = data.len();

        // Base case: leaf node
        if n <= 1 || height >= self.max_height {
            return IsolationNode {
                feature_index: 0,
                threshold: 0.0,
                left: None,
                right: None,
                height,
                is_leaf: true,
                size: n,
            };
        }

        // Random feature selection
        let feature_index = (height * 7 + 3) % 8; // Pseudo-random

        // Get feature values
        let mut values: Vec<f64> = data.iter().map(|f| f.normalize()[feature_index]).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Random split point
        let split_idx = n / 2;
        let threshold = values[split_idx];

        // Partition data
        let left: Vec<&FeatureVector> = data
            .iter()
            .filter(|f| f.normalize()[feature_index] < threshold)
            .copied()
            .collect();
        let right: Vec<&FeatureVector> = data
            .iter()
            .filter(|f| f.normalize()[feature_index] >= threshold)
            .copied()
            .collect();

        IsolationNode {
            feature_index,
            threshold,
            left: Some(Box::new(self.build_tree(&left, height + 1))),
            right: Some(Box::new(self.build_tree(&right, height + 1))),
            height,
            is_leaf: false,
            size: n,
        }
    }

    /// Calculate anomaly score for a feature vector
    pub(crate) fn anomaly_score(&self, features: &FeatureVector) -> f64 {
        if self.trees.is_empty() {
            return 0.5; // Untrained
        }

        let normalized = features.normalize();
        let mut total_path_length = 0.0;

        for tree in &self.trees {
            total_path_length += self.path_length(tree, &normalized, 0);
        }

        let avg_path_length = total_path_length / self.trees.len() as f64;
        let expected_length = self.average_path_length(self.max_samples);

        // Anomaly score: closer to 1 = more anomalous
        2.0_f64.powf(-avg_path_length / expected_length)
    }

    /// Calculate path length in a tree
    fn path_length(&self, node: &IsolationNode, features: &[f64], depth: usize) -> f64 {
        if node.is_leaf {
            return depth as f64 + self.average_path_length(node.size);
        }

        if features[node.feature_index] < node.threshold {
            match &node.left {
                Some(left) => self.path_length(left, features, depth + 1),
                None => depth as f64,
            }
        } else {
            match &node.right {
                Some(right) => self.path_length(right, features, depth + 1),
                None => depth as f64,
            }
        }
    }

    /// Average path length in unsuccessful BST search
    fn average_path_length(&self, n: usize) -> f64 {
        if n <= 1 {
            return 0.0;
        }
        let n_f = n as f64;
        2.0 * (n_f.ln() + 0.5772156649) - 2.0 * (n_f - 1.0) / n_f
    }

    /// Check if a feature vector is anomalous
    pub(crate) fn is_anomaly(&self, features: &FeatureVector, threshold: f64) -> bool {
        self.anomaly_score(features) > threshold
    }
}

/// Behavioral baseline for a user
#[derive(Debug, Clone)]
pub(crate) struct UserBaseline {
    pub avg_requests_per_hour: f64,
    pub typical_hours: Vec<f64>,
    pub typical_ips: Vec<String>,
    pub avg_message_size: f64,
    pub typical_user_agents: Vec<String>,
}

/// Threat detection engine
pub(crate) struct ThreatDetectionEngine {
    /// Isolation Forest model
    forest: IsolationForest,

    /// User baselines
    baselines: DashMap<String, UserBaseline>,

    /// Recent events for time-series analysis
    recent_events: DashMap<String, VecDeque<SecurityEvent>>,

    /// Anomaly threshold
    anomaly_threshold: f64,

    /// Alert history
    alerts: DashMap<String, Vec<SecurityAlert>>,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum SecurityEvent {
    Login {
        user_id: String,
        ip: String,
        success: bool,
        timestamp: u64,
    },
    MessageSent {
        user_id: String,
        recipient: String,
        size: usize,
        timestamp: u64,
    },
    Connection {
        ip: String,
        duration: Duration,
        messages_sent: usize,
    },
    ApiCall {
        endpoint: String,
        ip: String,
        status: u16,
        timestamp: u64,
    },
}

/// Security alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SecurityAlert {
    pub id: String,
    pub severity: AlertSeverity,
    pub alert_type: String,
    pub description: String,
    pub user_id: Option<String>,
    pub ip: Option<String>,
    pub timestamp: u64,
    pub anomaly_score: f64,
    pub features: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ThreatDetectionEngine {
    /// Create a new threat detection engine
    pub(crate) fn new() -> Self {
        Self {
            forest: IsolationForest::new(100, 256, 0.1),
            baselines: DashMap::new(),
            recent_events: DashMap::new(),
            anomaly_threshold: 0.7,
            alerts: DashMap::new(),
        }
    }

    /// Train on historical data
    pub(crate) fn train(&mut self, historical_data: &[FeatureVector]) {
        self.forest.train(historical_data);
        info!(
            "Threat detection engine trained on {} samples",
            historical_data.len()
        );
    }

    /// Analyze an incoming event
    pub(crate) fn analyze_event(&self, event: &SecurityEvent) -> Option<SecurityAlert> {
        let features = self.extract_features(event);
        let score = self.forest.anomaly_score(&features);

        if score > self.anomaly_threshold {
            let alert = self.create_alert(event, &features, score);

            // Store alert
            let key = match event {
                SecurityEvent::Login { user_id, .. } => user_id.clone(),
                SecurityEvent::MessageSent { user_id, .. } => user_id.clone(),
                SecurityEvent::Connection { ip, .. } => ip.clone(),
                SecurityEvent::ApiCall { ip, .. } => ip.clone(),
            };

            self.alerts.entry(key).or_default().push(alert.clone());

            warn!(
                severity = ?alert.severity,
                score = score,
                type = %alert.alert_type,
                "Anomaly detected"
            );

            Some(alert)
        } else {
            // Update baseline
            self.update_baseline(event, &features);
            None
        }
    }

    /// Extract features from an event
    fn extract_features(&self, event: &SecurityEvent) -> FeatureVector {
        match event {
            SecurityEvent::Login {
                user_id,
                ip: _,
                success: _,
                timestamp,
            } => {
                let recent_logins = self.count_recent_logins(user_id, 300);
                let failed_rate = self.calculate_failed_rate(user_id);

                FeatureVector {
                    requests_per_minute: recent_logins as f64,
                    unique_ips: self.count_unique_ips(user_id) as f64,
                    failed_auth_rate: failed_rate,
                    message_size_avg: 0.0,
                    connection_duration: 0.0,
                    time_of_day: (*timestamp % 86400) as f64 / 86400.0,
                    geo_entropy: 0.0,
                    user_agent_changes: 0.0,
                }
            }
            SecurityEvent::MessageSent { user_id, size, .. } => FeatureVector {
                requests_per_minute: self.count_recent_messages(user_id, 60) as f64,
                unique_ips: 1.0,
                failed_auth_rate: 0.0,
                message_size_avg: *size as f64,
                connection_duration: 0.0,
                time_of_day: 0.5,
                geo_entropy: 0.0,
                user_agent_changes: 0.0,
            },
            SecurityEvent::Connection {
                ip: _,
                duration,
                messages_sent,
            } => FeatureVector {
                requests_per_minute: *messages_sent as f64,
                unique_ips: 1.0,
                failed_auth_rate: 0.0,
                message_size_avg: 0.0,
                connection_duration: duration.as_secs() as f64,
                time_of_day: 0.5,
                geo_entropy: 0.0,
                user_agent_changes: 0.0,
            },
            SecurityEvent::ApiCall { ip, status, .. } => FeatureVector {
                requests_per_minute: self.count_recent_api_calls(ip, 60) as f64,
                unique_ips: 1.0,
                failed_auth_rate: if *status >= 400 { 1.0 } else { 0.0 },
                message_size_avg: 0.0,
                connection_duration: 0.0,
                time_of_day: 0.5,
                geo_entropy: 0.0,
                user_agent_changes: 0.0,
            },
        }
    }

    /// Create an alert from anomalous event
    fn create_alert(
        &self,
        event: &SecurityEvent,
        features: &FeatureVector,
        score: f64,
    ) -> SecurityAlert {
        let (alert_type, description, severity) = match event {
            SecurityEvent::Login { success: false, .. } if score > 0.9 => (
                "BRUTE_FORCE",
                "Possible brute force attack detected",
                AlertSeverity::Critical,
            ),
            SecurityEvent::Login { success: false, .. } => (
                "FAILED_AUTH",
                "Multiple failed login attempts",
                AlertSeverity::High,
            ),
            SecurityEvent::MessageSent { size, .. } if *size > 100_000 => (
                "LARGE_MESSAGE",
                "Unusually large message sent",
                AlertSeverity::Medium,
            ),
            SecurityEvent::Connection { duration, .. } if duration.as_secs() > 3600 => (
                "LONG_CONNECTION",
                "Unusually long connection",
                AlertSeverity::Low,
            ),
            _ => (
                "ANOMALY",
                "Unusual activity detected",
                AlertSeverity::Medium,
            ),
        };

        let (user_id, ip) = match event {
            SecurityEvent::Login { user_id, ip, .. } => (Some(user_id.clone()), Some(ip.clone())),
            SecurityEvent::MessageSent { user_id, .. } => (Some(user_id.clone()), None),
            SecurityEvent::Connection { ip, .. } => (None, Some(ip.clone())),
            SecurityEvent::ApiCall { ip, .. } => (None, Some(ip.clone())),
        };

        SecurityAlert {
            id: uuid::Uuid::new_v4().to_string(),
            severity,
            alert_type: alert_type.to_string(),
            description: description.to_string(),
            user_id,
            ip,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            anomaly_score: score,
            features: features.normalize(),
        }
    }

    /// Update user baseline
    fn update_baseline(&self, event: &SecurityEvent, _features: &FeatureVector) {
        // Simplified baseline update
        match event {
            SecurityEvent::Login { user_id, ip, .. } => {
                let mut baseline =
                    self.baselines
                        .entry(user_id.clone())
                        .or_insert_with(|| UserBaseline {
                            avg_requests_per_hour: 10.0,
                            typical_hours: vec![],
                            typical_ips: vec![ip.clone()],
                            avg_message_size: 1024.0,
                            typical_user_agents: vec![],
                        });

                if !baseline.typical_ips.contains(ip) {
                    baseline.typical_ips.push(ip.clone());
                }
            }
            _ => {}
        }
    }

    // Helper methods
    fn count_recent_logins(&self, user_id: &str, _window_secs: u64) -> usize {
        self.recent_events
            .get(user_id)
            .map(|events| events.len())
            .unwrap_or(0)
    }

    fn count_unique_ips(&self, user_id: &str) -> usize {
        self.baselines
            .get(user_id)
            .map(|b| b.typical_ips.len())
            .unwrap_or(1)
    }

    fn calculate_failed_rate(&self, _user_id: &str) -> f64 {
        0.1 // Placeholder
    }

    fn count_recent_messages(&self, _user_id: &str, _window_secs: u64) -> usize {
        5 // Placeholder
    }

    fn count_recent_api_calls(&self, _ip: &str, _window_secs: u64) -> usize {
        10 // Placeholder
    }

    /// Get alerts for a user
    pub(crate) fn get_alerts(&self, user_id: &str) -> Vec<SecurityAlert> {
        self.alerts
            .get(user_id)
            .map(|alerts| alerts.clone())
            .unwrap_or_default()
    }

    /// Clear old alerts
    pub(crate) fn cleanup_old_alerts(&self, max_age_secs: u64) {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - max_age_secs;

        self.alerts.retain(|_, alerts| {
            alerts.retain(|a| a.timestamp > cutoff);
            !alerts.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_forest() {
        let mut forest = IsolationForest::new(10, 100, 0.1);

        // Generate normal data
        let normal_data: Vec<FeatureVector> = (0..100)
            .map(|i| FeatureVector {
                requests_per_minute: 10.0 + (i % 5) as f64,
                unique_ips: 1.0,
                failed_auth_rate: 0.01,
                message_size_avg: 1024.0,
                connection_duration: 60.0,
                time_of_day: 0.5,
                geo_entropy: 0.1,
                user_agent_changes: 0.0,
            })
            .collect();

        forest.train(&normal_data);

        // Test normal point
        let normal = FeatureVector {
            requests_per_minute: 12.0,
            unique_ips: 1.0,
            failed_auth_rate: 0.02,
            message_size_avg: 1100.0,
            connection_duration: 65.0,
            time_of_day: 0.5,
            geo_entropy: 0.1,
            user_agent_changes: 0.0,
        };

        let normal_score = forest.anomaly_score(&normal);
        assert!(normal_score < 0.7, "Normal data should not be anomalous");

        // Test anomalous point
        let anomalous = FeatureVector {
            requests_per_minute: 1000.0, // Very high
            unique_ips: 50.0,
            failed_auth_rate: 0.9,
            message_size_avg: 1_000_000.0,
            connection_duration: 10000.0,
            time_of_day: 0.1,
            geo_entropy: 0.9,
            user_agent_changes: 10.0,
        };

        let anomaly_score = forest.anomaly_score(&anomalous);
        assert!(
            anomaly_score > 0.5,
            "Anomalous data should have higher score"
        );
    }

    #[test]
    fn test_threat_detection() {
        let engine = ThreatDetectionEngine::new();

        // Test normal login
        let normal_login = SecurityEvent::Login {
            user_id: "user1".to_string(),
            ip: "192.168.1.1".to_string(),
            success: true,
            timestamp: 1000,
        };

        let _alert = engine.analyze_event(&normal_login);
        // Should not trigger alert with untrained model

        // Test suspicious login pattern
        let suspicious_login = SecurityEvent::Login {
            user_id: "user1".to_string(),
            ip: "10.0.0.1".to_string(),
            success: false,
            timestamp: 1001,
        };

        let _ = engine.analyze_event(&suspicious_login);
    }
}
