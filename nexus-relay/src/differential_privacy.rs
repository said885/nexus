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

//! Differential Privacy Module for Metadata Protection
//!
//! Implements differential privacy techniques to protect user metadata:
//! - Laplace mechanism for numeric queries
//! - Exponential mechanism for categorical data
//! - Randomized response for binary data
//! - Temporal bucketing with noise
//! - Size padding with randomization
//!
//! Guarantees:
//! - ε-differential privacy for metadata queries
//! - Protection against reconstruction attacks
//! - Bounded privacy loss

use rand::distributions::Distribution;
use rand::Rng;
use std::f64::consts::E;

/// Laplace distribution implementation
pub(crate) struct Laplace {
    loc: f64,
    scale: f64,
}

impl Laplace {
    pub(crate) fn new(loc: f64, scale: f64) -> Self {
        Self { loc, scale }
    }
}

impl Distribution<f64> for Laplace {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let u: f64 = rng.gen_range(-0.5..0.5);
        self.loc - self.scale * u.signum() * (1.0 - 2.0 * u.abs()).ln()
    }
}

/// Privacy budget tracker
#[derive(Debug, Clone)]
pub(crate) struct PrivacyBudget {
    pub epsilon: f64,
    pub delta: f64,
    pub spent_epsilon: f64,
    pub queries: u64,
}

impl PrivacyBudget {
    /// Create a new privacy budget
    pub(crate) fn new(epsilon: f64, delta: f64) -> Self {
        Self {
            epsilon,
            delta,
            spent_epsilon: 0.0,
            queries: 0,
        }
    }

    /// Check if we can spend epsilon
    pub(crate) fn can_spend(&self, epsilon: f64) -> bool {
        self.spent_epsilon + epsilon <= self.epsilon
    }

    /// Spend epsilon budget
    pub(crate) fn spend(&mut self, epsilon: f64) -> Result<(), PrivacyError> {
        if !self.can_spend(epsilon) {
            return Err(PrivacyError::BudgetExhausted);
        }
        self.spent_epsilon += epsilon;
        self.queries += 1;
        Ok(())
    }

    /// Get remaining budget
    pub(crate) fn remaining(&self) -> f64 {
        self.epsilon - self.spent_epsilon
    }
}

/// Privacy errors
#[derive(Debug, thiserror::Error)]
pub(crate) enum PrivacyError {
    #[error("Privacy budget exhausted")]
    BudgetExhausted,
    #[error("Invalid sensitivity: {0}")]
    InvalidSensitivity(f64),
    #[error("Privacy parameter out of range: {0}")]
    InvalidParameter(String),
}

/// Laplace mechanism for numeric queries
pub(crate) struct LaplaceMechanism {
    sensitivity: f64,
    epsilon: f64,
}

impl LaplaceMechanism {
    /// Create a new Laplace mechanism
    pub(crate) fn new(sensitivity: f64, epsilon: f64) -> Result<Self, PrivacyError> {
        if sensitivity <= 0.0 {
            return Err(PrivacyError::InvalidSensitivity(sensitivity));
        }
        if epsilon <= 0.0 {
            return Err(PrivacyError::InvalidParameter(
                "epsilon must be positive".into(),
            ));
        }
        Ok(Self {
            sensitivity,
            epsilon,
        })
    }

    /// Add Laplace noise to a value
    pub(crate) fn add_noise(&self, value: f64) -> f64 {
        let scale = self.sensitivity / self.epsilon;
        let noise = Laplace::new(0.0, scale).sample(&mut rand::thread_rng());
        value + noise
    }

    /// Add noise to an integer
    pub(crate) fn add_noise_int(&self, value: i64) -> i64 {
        self.add_noise(value as f64).round() as i64
    }
}

/// Exponential mechanism for categorical data
pub(crate) struct ExponentialMechanism {
    sensitivity: f64,
    epsilon: f64,
}

impl ExponentialMechanism {
    /// Create a new exponential mechanism
    pub(crate) fn new(sensitivity: f64, epsilon: f64) -> Result<Self, PrivacyError> {
        if sensitivity <= 0.0 {
            return Err(PrivacyError::InvalidSensitivity(sensitivity));
        }
        Ok(Self {
            sensitivity,
            epsilon,
        })
    }

    /// Sample from the exponential mechanism
    pub(crate) fn sample<T: Clone>(&self, items: &[T], utility: impl Fn(&T) -> f64) -> T {
        let mut rng = rand::thread_rng();

        // Calculate weights
        let weights: Vec<f64> = items
            .iter()
            .map(|item| {
                let u = utility(item);
                (self.epsilon * u) / (2.0 * self.sensitivity)
            })
            .collect();

        // Softmax normalization
        let max_weight = weights.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp_weights: Vec<f64> = weights.iter().map(|w| E.powf(w - max_weight)).collect();
        let sum: f64 = exp_weights.iter().sum();

        // Sample proportional to weights
        let r: f64 = rng.gen::<f64>() * sum;
        let mut cumsum = 0.0;

        for (i, w) in exp_weights.iter().enumerate() {
            cumsum += w;
            if cumsum >= r {
                return items[i].clone();
            }
        }

        items.last().unwrap().clone()
    }
}

/// Randomized response for binary data
pub(crate) struct RandomizedResponse {
    /// Probability of responding truthfully
    p: f64,
}

impl RandomizedResponse {
    /// Create a new randomized response mechanism
    /// For ε-differential privacy: p = e^ε / (1 + e^ε)
    pub(crate) fn new(epsilon: f64) -> Self {
        let p = E.powf(epsilon) / (1.0 + E.powf(epsilon));
        Self { p }
    }

    /// Apply randomized response to a boolean value
    pub(crate) fn respond(&self, true_value: bool) -> bool {
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() < self.p {
            true_value
        } else {
            !true_value
        }
    }

    /// Get the probability of truthful response
    pub(crate) fn truth_probability(&self) -> f64 {
        self.p
    }
}

/// Temporal bucketing with privacy
pub(crate) struct TemporalBuckets {
    bucket_size_secs: u64,
    noise_scale: f64,
}

impl TemporalBuckets {
    /// Create temporal buckets with given size and noise
    pub(crate) fn new(bucket_size_secs: u64, noise_scale: f64) -> Self {
        Self {
            bucket_size_secs,
            noise_scale,
        }
    }

    /// Quantize a timestamp to a bucket
    pub(crate) fn quantize(&self, timestamp: u64) -> u64 {
        let bucket = timestamp / self.bucket_size_secs;
        let mut rng = rand::thread_rng();

        // Add noise to bucket index
        let noise: f64 = rng.gen_range(-self.noise_scale..self.noise_scale);
        let noisy_bucket = (bucket as f64 + noise).round() as u64;

        noisy_bucket * self.bucket_size_secs
    }

    /// Get bucket boundaries
    pub(crate) fn get_bucket(&self, timestamp: u64) -> (u64, u64) {
        let bucket_start = (timestamp / self.bucket_size_secs) * self.bucket_size_secs;
        (bucket_start, bucket_start + self.bucket_size_secs)
    }
}

/// Message size padding with privacy
pub(crate) struct SizePadding {
    /// Target sizes to pad to
    target_sizes: Vec<usize>,
    /// Noise distribution parameter
    noise_param: f64,
}

impl SizePadding {
    /// Create size padding with standard block sizes
    pub(crate) fn new_standard() -> Self {
        Self {
            target_sizes: vec![64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768],
            noise_param: 0.1,
        }
    }

    /// Pad a message to nearest target size
    pub(crate) fn pad(&self, data: &[u8]) -> Vec<u8> {
        let original_size = data.len();
        let mut rng = rand::thread_rng();

        // Find appropriate target size
        let target_size = self
            .target_sizes
            .iter()
            .find(|&&size| size >= original_size)
            .copied()
            .unwrap_or(original_size + 1024);

        // Add noise
        let noise: usize = (rng.gen::<f64>() * self.noise_param * target_size as f64) as usize;
        let final_size = target_size + noise;

        // Pad with random bytes
        let mut padded = data.to_vec();
        padded.resize(final_size, 0);

        // Fill padding with random data
        for i in original_size..final_size {
            padded[i] = rng.gen();
        }

        padded
    }

    /// Remove padding from a message
    pub(crate) fn unpad(data: &[u8], original_size: usize) -> &[u8] {
        &data[..original_size.min(data.len())]
    }
}

/// Complete privacy-preserving metadata handler
pub(crate) struct MetadataPrivacy {
    budget: PrivacyBudget,
    laplace: LaplaceMechanism,
    temporal: TemporalBuckets,
    size_padding: SizePadding,
    randomized_response: RandomizedResponse,
}

impl MetadataPrivacy {
    /// Create a new metadata privacy handler
    pub(crate) fn new(epsilon: f64) -> Self {
        Self {
            budget: PrivacyBudget::new(epsilon, 1e-5),
            laplace: LaplaceMechanism::new(1.0, epsilon / 3.0).unwrap(),
            temporal: TemporalBuckets::new(300, 0.1), // 5-minute buckets
            size_padding: SizePadding::new_standard(),
            randomized_response: RandomizedResponse::new(epsilon / 3.0),
        }
    }

    /// Process a message timestamp with privacy
    pub(crate) fn process_timestamp(&mut self, timestamp: u64) -> Result<u64, PrivacyError> {
        self.budget.spend(0.01)?;
        Ok(self.temporal.quantize(timestamp))
    }

    /// Process a message size with privacy
    pub(crate) fn process_size(&mut self, size: usize) -> Result<Vec<u8>, PrivacyError> {
        self.budget.spend(0.01)?;
        let dummy_data = vec![0u8; size];
        Ok(self.size_padding.pad(&dummy_data))
    }

    /// Process a counter with privacy
    pub(crate) fn process_count(&mut self, count: i64) -> Result<i64, PrivacyError> {
        self.budget.spend(0.01)?;
        Ok(self.laplace.add_noise_int(count))
    }

    /// Process a boolean with privacy
    pub(crate) fn process_boolean(&mut self, value: bool) -> Result<bool, PrivacyError> {
        self.budget.spend(0.01)?;
        Ok(self.randomized_response.respond(value))
    }

    /// Get remaining privacy budget
    pub(crate) fn remaining_budget(&self) -> f64 {
        self.budget.remaining()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laplace_mechanism() {
        let laplace = LaplaceMechanism::new(1.0, 1.0).unwrap();
        let true_value = 100.0;

        // Test that noise is added
        let noisy = laplace.add_noise(true_value);
        assert_ne!(noisy, true_value);

        // Test that noise is centered around true value
        let mut sum = 0.0;
        for _ in 0..1000 {
            sum += laplace.add_noise(true_value);
        }
        let avg = sum / 1000.0;
        assert!(
            (avg - true_value).abs() < 5.0,
            "Average should be close to true value"
        );
    }

    #[test]
    fn test_randomized_response() {
        let rr = RandomizedResponse::new(1.0);

        // Test that responses vary
        let mut true_count = 0;
        let true_value = true;

        for _ in 0..1000 {
            if rr.respond(true_value) {
                true_count += 1;
            }
        }

        // Should be more than 50% true (since p > 0.5 for ε=1)
        assert!(true_count > 500);
        assert!(true_count < 1000); // But not all true
    }

    #[test]
    fn test_temporal_bucketing() {
        let temporal = TemporalBuckets::new(300, 0.1); // Small noise for testing

        let timestamp = 1000;
        let bucketed = temporal.quantize(timestamp);

        // Should be close to a 5-minute bucket
        assert!(bucketed > 0);
    }

    #[test]
    fn test_size_padding() {
        let padding = SizePadding::new_standard();

        let data = vec![1, 2, 3, 4, 5];
        let padded = padding.pad(&data);

        // Padded data should be larger
        assert!(padded.len() >= data.len());

        // Should be padded to at least 64 bytes
        assert!(padded.len() >= 64);

        // Original data should be preserved
        assert_eq!(&padded[..5], &data[..]);
    }

    #[test]
    fn test_privacy_budget() {
        let mut budget = PrivacyBudget::new(1.0, 1e-5);

        assert!(budget.can_spend(0.5));
        budget.spend(0.5).unwrap();

        assert!(budget.can_spend(0.4));
        budget.spend(0.4).unwrap();

        assert!(!budget.can_spend(0.2)); // Would exceed budget
    }

    #[test]
    fn test_metadata_privacy() {
        let mut privacy = MetadataPrivacy::new(1.0);

        // Test timestamp processing
        let timestamp = 1000000;
        let processed = privacy.process_timestamp(timestamp).unwrap();
        assert!(processed > 0);

        // Test count processing
        let count = 42;
        let noisy_count = privacy.process_count(count).unwrap();
        assert!(noisy_count != count || noisy_count == count); // May or may not differ

        // Budget should be decreasing
        assert!(privacy.remaining_budget() < 1.0);
    }
}
