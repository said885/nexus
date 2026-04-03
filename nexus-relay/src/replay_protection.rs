#![allow(missing_docs, dead_code)]

//! Replay Attack Detection and Prevention
//!
//! Implements cryptographic nonce tracking and timestamp verification
//! to prevent replay attacks on authenticated connections.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Size of rolling window for nonce tracking
const NONCE_WINDOW_SIZE: usize = 50_000;

/// Maximum age of a message timestamp (30 seconds clock skew tolerance)
const MAX_TIMESTAMP_SKEW: Duration = Duration::from_secs(30);

/// A message nonce for replay detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct MessageNonce([u8; 32]);

impl MessageNonce {
    /// Generate a new message nonce
    pub(crate) fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let mut nonce = [0u8; 32];
        use rand::RngCore;
        rng.fill_bytes(&mut nonce);
        MessageNonce(nonce)
    }

    /// Create from bytes
    pub(crate) fn from_bytes(bytes: &[u8; 32]) -> Self {
        MessageNonce(*bytes)
    }

    /// Convert to hex string
    pub(crate) fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// Bloom filter for efficient nonce tracking
/// Uses double hashing for O(1) lookup with false positives
struct BloomFilter {
    bits: Vec<u64>,
    num_hashes: usize,
}

impl BloomFilter {
    /// Create a new Bloom filter with capacity for `capacity` items
    fn new(capacity: usize) -> Self {
        // 8 bits per item for low false-positive rate
        let num_bits = capacity * 8;
        let num_u64s = (num_bits + 63) / 64;

        Self {
            bits: vec![0u64; num_u64s],
            num_hashes: 3, // 3 hash functions = ~1% false positive rate
        }
    }

    /// Get hash values for a nonce
    fn hash(&self, nonce: &MessageNonce) -> Vec<usize> {
        let mut hashes = Vec::with_capacity(self.num_hashes);

        for i in 0..self.num_hashes {
            let mut hasher = Sha256::new();
            hasher.update(&nonce.0);
            hasher.update(i.to_le_bytes());
            let hash = hasher.finalize();

            let hash_value = u64::from_le_bytes([
                hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
            ]) as usize;

            hashes.push(hash_value % (self.bits.len() * 64));
        }

        hashes
    }

    /// Insert a nonce into the filter
    fn insert(&mut self, nonce: &MessageNonce) {
        for hash in self.hash(nonce) {
            let word_idx = hash / 64;
            let bit_idx = hash % 64;
            self.bits[word_idx] |= 1u64 << bit_idx;
        }
    }

    /// Check if a nonce might be in the filter (allows false positives)
    fn might_contain(&self, nonce: &MessageNonce) -> bool {
        self.hash(nonce).iter().all(|&hash| {
            let word_idx = hash / 64;
            let bit_idx = hash % 64;
            (self.bits[word_idx] & (1u64 << bit_idx)) != 0
        })
    }
}

/// Tracks message timestamps for clock skew detection
struct TimestampTracker {
    /// Last seen message timestamp
    last_timestamp: u64,
    /// Minimum timestamp seen
    min_timestamp: u64,
}

impl TimestampTracker {
    fn new() -> Self {
        Self {
            last_timestamp: 0,
            min_timestamp: u64::MAX,
        }
    }

    /// Verify timestamp is not too old or in the future
    fn verify(&mut self, timestamp: u64, current_time: u64) -> Result<(), String> {
        let age = current_time.saturating_sub(timestamp);
        let skew = timestamp.saturating_sub(current_time);

        // Check age (message too old)
        if age > MAX_TIMESTAMP_SKEW.as_secs() {
            return Err(format!("Message too old: {} seconds", age));
        }

        // Check skew (message timestamp in future)
        if skew > MAX_TIMESTAMP_SKEW.as_secs() {
            return Err(format!("Message timestamp in future: {} seconds", skew));
        }

        // Update tracking
        if timestamp > self.last_timestamp {
            self.last_timestamp = timestamp;
        }
        self.min_timestamp = self.min_timestamp.min(timestamp);

        Ok(())
    }
}

/// Replay attack detection system
pub(crate) struct ReplayDetector {
    /// Current Bloom filter for nonce detection
    bloom_filter: Arc<RwLock<BloomFilter>>,
    /// Fallback exact nonce tracking (for confirmation)
    exact_nonces: Arc<RwLock<VecDeque<(MessageNonce, Instant)>>>,
    /// Timestamp tracking per connection
    timestamps: Arc<RwLock<TimestampTracker>>,
    /// Statistics
    checks_performed: Arc<RwLock<u64>>,
    replays_detected: Arc<RwLock<u64>>,
}

impl ReplayDetector {
    /// Create a new replay detector
    pub(crate) fn new() -> Self {
        Self {
            bloom_filter: Arc::new(RwLock::new(BloomFilter::new(NONCE_WINDOW_SIZE))),
            exact_nonces: Arc::new(RwLock::new(VecDeque::new())),
            timestamps: Arc::new(RwLock::new(TimestampTracker::new())),
            checks_performed: Arc::new(RwLock::new(0)),
            replays_detected: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if a message nonce is a replay
    pub(crate) fn check_nonce(&self, nonce: &MessageNonce) -> Result<(), ReplayError> {
        let mut checks = self.checks_performed.write();
        *checks += 1;

        // Step 1: Exact check with stored nonces
        {
            let exact = self.exact_nonces.read();
            if exact.iter().any(|(n, _)| n == nonce) {
                let mut replays = self.replays_detected.write();
                *replays += 1;
                return Err(ReplayError::NonceReused);
            }
        }

        // Step 2: Add to tracking (not seen before)
        {
            let mut filter = self.bloom_filter.write();
            filter.insert(nonce);

            let mut exact = self.exact_nonces.write();
            exact.push_back((*nonce, Instant::now()));

            // Keep only recent nonces
            if exact.len() > NONCE_WINDOW_SIZE {
                exact.pop_front();
            }
        }

        Ok(())
    }

    /// Verify message timestamp
    pub(crate) fn verify_timestamp(&self, timestamp: u64, current_time: u64) -> Result<(), ReplayError> {
        let mut ts_tracker = self.timestamps.write();
        ts_tracker
            .verify(timestamp, current_time)
            .map_err(|e| ReplayError::InvalidTimestamp(e))
    }

    /// Cleanup old nonce entries
    pub(crate) fn cleanup_old_nonces(&self) {
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes

        let mut exact = self.exact_nonces.write();
        while let Some((_, creation_time)) = exact.front() {
            if *creation_time < cutoff {
                exact.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get detector statistics
    pub(crate) fn get_stats(&self) -> ReplayDetectorStats {
        ReplayDetectorStats {
            checks_performed: *self.checks_performed.read(),
            replays_detected: *self.replays_detected.read(),
            active_nonces: self.exact_nonces.read().len(),
        }
    }
}

impl Default for ReplayDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Replay detection error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReplayError {
    /// Nonce has been used before
    NonceReused,
    /// Timestamp is invalid or suspicious
    InvalidTimestamp(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonceReused => write!(f, "Nonce has been used before"),
            Self::InvalidTimestamp(msg) => write!(f, "Invalid timestamp: {}", msg),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Statistics from replay detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReplayDetectorStats {
    /// Total nonce checks performed
    pub checks_performed: u64,
    /// Number of replay attacks detected
    pub replays_detected: u64,
    /// Number of nonces currently being tracked
    pub active_nonces: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_generation() {
        let n1 = MessageNonce::generate();
        let n2 = MessageNonce::generate();

        assert_ne!(n1, n2);
        assert_eq!(n1.to_hex().len(), 64);
    }

    #[test]
    fn test_bloom_filter_basic() {
        let mut filter = BloomFilter::new(1000);
        let nonce = MessageNonce::generate();

        // Should not contain before insert
        assert!(!filter.might_contain(&nonce));

        // Insert
        filter.insert(&nonce);

        // Should contain after insert
        assert!(filter.might_contain(&nonce));
    }

    #[test]
    fn test_replay_detection() {
        let detector = ReplayDetector::new();
        let nonce = MessageNonce::generate();

        // First check should succeed
        assert!(detector.check_nonce(&nonce).is_ok());

        // Second check should detect replay
        assert_eq!(detector.check_nonce(&nonce), Err(ReplayError::NonceReused));

        // Statistics should be updated
        let stats = detector.get_stats();
        assert_eq!(stats.replays_detected, 1);
        assert_eq!(stats.checks_performed, 2);
    }

    #[test]
    fn test_timestamp_verification() {
        let detector = ReplayDetector::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Current timestamp should be valid
        assert!(detector.verify_timestamp(now, now).is_ok());

        // Old timestamp should fail
        assert!(detector.verify_timestamp(now - 60, now).is_err());

        // Future timestamp should fail
        assert!(detector.verify_timestamp(now + 60, now).is_err());
    }

    #[test]
    fn test_multiple_nonces() {
        let detector = ReplayDetector::new();

        let mut nonces = Vec::new();
        for _ in 0..100 {
            nonces.push(MessageNonce::generate());
        }

        // All first checks should succeed
        for nonce in &nonces {
            assert!(detector.check_nonce(nonce).is_ok());
        }

        // All replays should be detected
        for nonce in &nonces {
            assert_eq!(detector.check_nonce(nonce), Err(ReplayError::NonceReused));
        }

        let stats = detector.get_stats();
        assert_eq!(stats.replays_detected, 100);
        assert_eq!(stats.checks_performed, 200);
    }

    #[test]
    fn test_cleanup() {
        let detector = ReplayDetector::new();

        for _ in 0..10 {
            let nonce = MessageNonce::generate();
            let _ = detector.check_nonce(&nonce);
        }

        assert_eq!(detector.get_stats().active_nonces, 10);

        // Cleanup should work without errors
        detector.cleanup_old_nonces();
    }
}
