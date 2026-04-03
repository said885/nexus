#![allow(missing_docs, dead_code)]

// Rate limiting and throttling
// nexus-relay/src/rate_limiting.rs

use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(crate) struct RateLimitConfig {
    pub max_requests: u32,
    pub window_duration_secs: u64,
    pub burst_limit: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            max_requests: 100,
            window_duration_secs: 60,
            burst_limit: 150,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RequestWindow {
    pub count: u32,
    pub start_time: DateTime<Utc>,
}

pub(crate) struct RateLimiter {
    user_requests: HashMap<String, Vec<DateTime<Utc>>>,
    config: RateLimitConfig,
    blocked_users: HashMap<String, DateTime<Utc>>, // user_id -> unblock_time
}

impl RateLimiter {
    pub(crate) fn new(config: RateLimitConfig) -> Self {
        RateLimiter {
            user_requests: HashMap::new(),
            config,
            blocked_users: HashMap::new(),
        }
    }

    pub(crate) fn is_allowed(&mut self, user_id: &str) -> bool {
        // Check if user is blocked
        if let Some(unblock_time) = self.blocked_users.get(user_id) {
            if Utc::now() < *unblock_time {
                return false;
            } else {
                self.blocked_users.remove(user_id);
            }
        }

        let now = Utc::now();
        let window_start = now
            - Duration::seconds(self.config.window_duration_secs as i64);

        let requests = self
            .user_requests
            .entry(user_id.to_string())
            .or_insert_with(Vec::new);

        // Clean old requests outside window
        requests.retain(|&req_time| req_time > window_start);

        if requests.len() >= self.config.max_requests as usize {
            // Block user for 5 minutes
            self.blocked_users.insert(
                user_id.to_string(),
                now + Duration::minutes(5),
            );
            false
        } else {
            requests.push(now);
            true
        }
    }

    pub(crate) fn get_remaining_requests(&mut self, user_id: &str) -> u32 {
        let now = Utc::now();
        let window_start = now
            - Duration::seconds(self.config.window_duration_secs as i64);

        if let Some(requests) = self.user_requests.get_mut(user_id) {
            requests.retain(|&req_time| req_time > window_start);
            (self.config.max_requests).saturating_sub(requests.len() as u32)
        } else {
            self.config.max_requests
        }
    }

    pub(crate) fn reset_user(&mut self, user_id: &str) {
        self.user_requests.remove(user_id);
        self.blocked_users.remove(user_id);
    }

    pub(crate) fn check_burst_limit(&self, user_id: &str) -> bool {
        let now = Utc::now();
        let burst_window_start = now - Duration::seconds(10);

        if let Some(requests) = self.user_requests.get(user_id) {
            let burst_count = requests
                .iter()
                .filter(|&req_time| *req_time > burst_window_start)
                .count();

            (burst_count as u32) < self.config.burst_limit
        } else {
            true
        }
    }

    pub(crate) fn cleanup_expired_blocks(&mut self) {
        let now = Utc::now();
        self.blocked_users.retain(|_, &mut unblock_time| unblock_time > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_requests() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_duration_secs: 60,
            burst_limit: 10,
        };
        let mut limiter = RateLimiter::new(config);

        for _ in 0..5 {
            assert!(limiter.is_allowed("user_1"));
        }
        assert!(!limiter.is_allowed("user_1"));
    }

    #[test]
    fn test_remaining_requests() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_duration_secs: 60,
            burst_limit: 15,
        };
        let mut limiter = RateLimiter::new(config);

        limiter.is_allowed("user_1");
        limiter.is_allowed("user_1");
        let remaining = limiter.get_remaining_requests("user_1");
        assert_eq!(remaining, 8);
    }

    #[test]
    fn test_burst_limit() {
        let config = RateLimitConfig {
            max_requests: 100,
            window_duration_secs: 60,
            burst_limit: 5,
        };
        let limiter = RateLimiter::new(config);
        assert!(limiter.check_burst_limit("user_1"));
    }

    #[test]
    fn test_reset_user() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_duration_secs: 60,
            burst_limit: 10,
        };
        let mut limiter = RateLimiter::new(config);

        for _ in 0..5 {
            assert!(limiter.is_allowed("user_1"));
        }
        assert!(!limiter.is_allowed("user_1"));

        limiter.reset_user("user_1");
        assert!(limiter.is_allowed("user_1"));
    }

    #[test]
    fn test_user_block_unblock() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration_secs: 60,
            burst_limit: 5,
        };
        let mut limiter = RateLimiter::new(config);

        assert!(limiter.is_allowed("user_1"));
        assert!(limiter.is_allowed("user_1"));
        assert!(!limiter.is_allowed("user_1")); // Now blocked

        // User should be blocked
        assert!(!limiter.is_allowed("user_1"));
    }
}
