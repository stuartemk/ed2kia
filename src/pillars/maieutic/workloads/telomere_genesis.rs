//! Telomere Regeneration Workload â€” Distributed Bio-Mathematical Simulation.
//!
//! **Sprint 50:** Implements the Genesis Telomere Workload as a `DistributedWorkload`
//! that models cellular aging as epigenetic noise (information theory of aging).
//!
//! **Core Concepts:**
//! - **Epigenetic Noise:** Cellular aging as progressive loss of information fidelity.
//! - **Syntax Correction:** Simulated cellular reprogramming that restores information
//!   clarity without altering cellular identity (DNA signature preserved).
//! - **Distributed Workload:** BFT-consensus-ready task distribution for WASM nodes.
//!
//! **Mathematical Foundation:**
//! - Shannon entropy measures information loss: `H(X) = -Î£ p(x) log2(p(x))`
//! - Kullback-Leibler divergence measures identity drift: `D_KL(P||Q) = Î£ P(x) log(P(x)/Q(x))`
//! - Telomere length modeled as information capacity: `L(t) = L_0 * exp(-Î» * t)`
//! - Regeneration score: `R = (H_initial - H_current) / H_initial`
//!
//! **WASM Compatibility:** Pure Rust, no native dependencies, platform-agnostic.
//!
//! **Feature Gate:** `v3.2-genesis-manifold`
//!
//! **Reference:** Sprint 50 â€” The Topological Moral Manifold & Genesis Telomere Workload

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Distributed Workload Trait
// ---------------------------------------------------------------------------

/// Trait for distributed computational workloads that can be executed across
/// a network of WASM-compatible nodes with BFT consensus validation.
///
/// **Requirements:**
/// - Platform-agnostic (WASM + native)
/// - Deterministic results for consensus validation
/// - Bounded computational complexity
/// - Zero side effects (pure computation)
pub trait DistributedWorkload: Send + Sync {
    /// Unique identifier for the workload type.
    fn workload_id(&self) -> &str;

    /// Execute the workload computation.
    fn execute(&self, context: &WorkloadContext) -> Result<WorkloadResult, WorkloadError>;

    /// Validate a result produced by another node (for BFT consensus).
    fn validate_result(
        &self,
        context: &WorkloadContext,
        result: &WorkloadResult,
    ) -> Result<(), WorkloadError>;

    /// Estimate computational cost (for load balancing).
    fn estimated_cost(&self) -> WorkloadCost;
}

// ---------------------------------------------------------------------------
// Workload Context & Result
// ---------------------------------------------------------------------------

/// Execution context for distributed workloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadContext {
    /// Node identifier executing the workload.
    pub node_id: u64,
    /// Total number of validators in the network.
    pub total_validators: usize,
    /// Required consensus threshold (2f+1).
    pub consensus_threshold: usize,
    /// Random seed for reproducible noise generation.
    pub seed: u64,
    /// Maximum iterations allowed.
    pub max_iterations: usize,
}

impl Default for WorkloadContext {
    fn default() -> Self {
        Self {
            node_id: 0,
            total_validators: 7,
            consensus_threshold: 5, // 2f+1 for f=2
            seed: 42,
            max_iterations: 100,
        }
    }
}

/// Result of executing a distributed workload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadResult {
    /// Primary result value.
    pub value: f64,
    /// Confidence score [0.0, 1.0].
    pub confidence: f64,
    /// Number of iterations executed.
    pub iterations: usize,
    /// Metadata for validation.
    pub metadata: Vec<u8>,
}

impl WorkloadResult {
    pub fn new(value: f64, confidence: f64, iterations: usize, metadata: Vec<u8>) -> Self {
        Self {
            value,
            confidence: confidence.clamp(0.0, 1.0),
            iterations,
            metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// Workload Cost & Error
// ---------------------------------------------------------------------------

/// Estimated computational cost for load balancing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkloadCost {
    /// Estimated CPU cycles (relative units).
    pub cpu_cycles: u64,
    /// Estimated memory usage (bytes).
    pub memory_bytes: u64,
    /// Complexity class description.
    pub complexity: &'static str,
}

/// Errors that can occur during workload execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadError {
    /// Input parameters out of valid range.
    InvalidParameter { name: &'static str, value: f64 },
    /// Computation exceeded maximum iterations.
    MaxIterationsExceeded { limit: usize },
    /// Result validation failed.
    ValidationFailed { reason: &'static str },
    /// Identity drift exceeds tolerance (cellular identity compromised).
    IdentityDriftExceeded { drift: f64, tolerance: f64 },
    /// Numerical instability detected.
    NumericalInstability { metric: &'static str },
}

impl core::fmt::Display for WorkloadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidParameter { name, value } => {
                write!(f, "Invalid parameter '{}' with value {}", name, value)
            }
            Self::MaxIterationsExceeded { limit } => {
                write!(f, "Max iterations exceeded (limit: {})", limit)
            }
            Self::ValidationFailed { reason } => {
                write!(f, "Validation failed: {}", reason)
            }
            Self::IdentityDriftExceeded { drift, tolerance } => {
                write!(
                    f,
                    "Identity drift {:.4} exceeds tolerance {:.4}",
                    drift, tolerance
                )
            }
            Self::NumericalInstability { metric } => {
                write!(f, "Numerical instability in '{}'", metric)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Epigenetic Noise Model
// ---------------------------------------------------------------------------

/// Models epigenetic noise as information-theoretic degradation of cellular state.
///
/// **Theory:** Cellular aging results from progressive accumulation of epigenetic
/// noise â€” random deviations from the optimal gene expression pattern. This is
/// modeled as increasing Shannon entropy in the cellular information channel.
///
/// **Mathematical Model:**
/// - Initial state: Low entropy (high information fidelity)
/// - Aging: Entropy increases as `H(t) = H_0 + Î± * log(1 + Î² * t)`
/// - Telomere length correlates with information capacity: `L âˆ (H_max - H_current)`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpigeneticNoiseModel {
    /// Initial entropy level (young cell).
    pub h0: f64,
    /// Noise accumulation rate.
    pub alpha: f64,
    /// Noise acceleration factor.
    pub beta: f64,
    /// Maximum entropy (complete information loss).
    pub h_max: f64,
    /// Cellular identity signature (preserved during regeneration).
    pub identity_signature: [f64; 8],
}

impl EpigeneticNoiseModel {
    /// Create a new epigenetic noise model with default biological parameters.
    pub fn new() -> Self {
        Self {
            h0: 0.1,                        // Low initial entropy
            alpha: 0.05,                    // Moderate accumulation rate
            beta: 0.02,                     // Slow acceleration
            h_max: 2.0,                     // Maximum entropy bound
            identity_signature: [0.125; 8], // Uniform identity signature
        }
    }

    /// Create a custom noise model.
    pub fn with_params(h0: f64, alpha: f64, beta: f64, h_max: f64) -> Self {
        Self {
            h0,
            alpha,
            beta,
            h_max,
            identity_signature: [0.125; 8],
        }
    }

    /// Calculate entropy at time t: `H(t) = H_0 + Î± * log(1 + Î² * t)`
    pub fn entropy_at_time(&self, t: f64) -> f64 {
        self.h0 + self.alpha * (1.0 + self.beta * t).ln().max(0.0)
    }

    /// Calculate telomere length as function of entropy: `L(t) = L_0 * (1 - H(t)/H_max)`
    pub fn telomere_length(&self, t: f64, initial_length: f64) -> f64 {
        let entropy = self.entropy_at_time(t);
        let capacity_ratio = 1.0 - (entropy / self.h_max).min(1.0);
        initial_length * capacity_ratio.max(0.0)
    }

    /// Calculate Shannon entropy for a probability distribution.
    pub fn shannon_entropy(distribution: &[f64]) -> f64 {
        distribution
            .iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.log2())
            .sum()
    }

    /// Calculate Kullback-Leibler divergence: `D_KL(P||Q) = Î£ P(x) log(P(x)/Q(x))`
    /// Measures how much the current distribution diverges from the identity.
    pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
        p.iter()
            .zip(q.iter())
            .filter(|(pi, _)| **pi > 0.0)
            .map(|(pi, qi)| {
                if *qi > 0.0 {
                    pi * (pi / qi).ln()
                } else {
                    f64::INFINITY
                }
            })
            .sum()
    }

    /// Generate noisy cellular state based on time and seed.
    pub fn generate_noisy_state(&self, t: f64, seed: u64) -> Vec<f64> {
        let entropy = self.entropy_at_time(t);
        let noise_scale = (entropy / self.h_max).min(1.0);

        // Deterministic noise generation using seed
        let mut state = self.identity_signature.to_vec();
        for (i, val) in state.iter_mut().enumerate() {
            let noise = Self::deterministic_noise(seed, i) * noise_scale * 0.1;
            *val = (*val + noise).max(0.0);
        }

        // Normalize to probability distribution
        let sum: f64 = state.iter().sum();
        if sum > 0.0 {
            state.iter_mut().for_each(|v| *v /= sum);
        }

        state
    }

    /// Deterministic pseudo-random noise generator (WASM-compatible).
    fn deterministic_noise(seed: u64, index: usize) -> f64 {
        let mut state = seed
            .wrapping_add(index as u64)
            .wrapping_mul(6364136223846793005);
        state = state.wrapping_mul(6364136223846793005);
        state ^= state >> 16;
        let normalized = ((state & 0xFFFFFFFF) as f64) / (u32::MAX as f64);
        (normalized - 0.5) * 2.0 // Range: [-1.0, 1.0]
    }
}

impl Default for EpigeneticNoiseModel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Syntax Correction Sequence
// ---------------------------------------------------------------------------

/// Represents a syntax correction sequence â€” simulated cellular reprogramming
/// that restores information clarity without altering cellular identity.
///
/// **Process:**
/// 1. Measure current entropy (information loss)
/// 2. Calculate correction vector toward optimal state
/// 3. Apply correction while monitoring identity drift
/// 4. Verify identity preservation (KL divergence < tolerance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxCorrection {
    /// Correction iteration number.
    pub iteration: usize,
    /// Entropy before correction.
    pub entropy_before: f64,
    /// Entropy after correction.
    pub entropy_after: f64,
    /// Identity drift (KL divergence from original signature).
    pub identity_drift: f64,
    /// Correction strength applied [0.0, 1.0].
    pub correction_strength: f64,
    /// Success flag (identity preserved).
    pub success: bool,
}

impl SyntaxCorrection {
    /// Regeneration improvement from this correction.
    pub fn improvement(&self) -> f64 {
        (self.entropy_before - self.entropy_after).max(0.0)
    }
}

// ---------------------------------------------------------------------------
// Telomere Regeneration Task
// ---------------------------------------------------------------------------

/// Distributed workload for telomere regeneration simulation.
///
/// **Objective:** Find syntax correction sequences that restore information
/// fidelity (reduce epigenetic noise) while preserving cellular identity.
///
/// **Algorithm:**
/// 1. Generate noisy cellular state at time t
/// 2. Iteratively apply gradient-descent-like corrections toward identity
/// 3. Monitor identity drift (KL divergence) at each step
/// 4. Calculate regeneration score based on entropy reduction
/// 5. Return result with confidence based on convergence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelomereRegenerationTask {
    /// Biological time point (aging level).
    pub time_point: f64,
    /// Initial telomere length.
    pub initial_telomere_length: f64,
    /// Maximum allowed identity drift.
    pub max_identity_drift: f64,
    /// Correction learning rate.
    pub learning_rate: f64,
    /// Noise model configuration.
    pub noise_model: EpigeneticNoiseModel,
}

impl TelomereRegenerationTask {
    pub fn new(time_point: f64) -> Self {
        Self {
            time_point,
            initial_telomere_length: 10000.0, // Base pairs
            max_identity_drift: 0.1,
            learning_rate: 0.1,
            noise_model: EpigeneticNoiseModel::new(),
        }
    }

    pub fn with_params(
        time_point: f64,
        initial_length: f64,
        max_drift: f64,
        learning_rate: f64,
    ) -> Self {
        Self {
            time_point,
            initial_telomere_length: initial_length,
            max_identity_drift: max_drift,
            learning_rate,
            noise_model: EpigeneticNoiseModel::new(),
        }
    }

    /// Core regeneration algorithm: iterative syntax correction.
    fn regenerate(&self, context: &WorkloadContext) -> Result<WorkloadResult, WorkloadError> {
        // Validate parameters
        if self.time_point < 0.0 {
            return Err(WorkloadError::InvalidParameter {
                name: "time_point",
                value: self.time_point,
            });
        }

        if self.learning_rate <= 0.0 || self.learning_rate > 1.0 {
            return Err(WorkloadError::InvalidParameter {
                name: "learning_rate",
                value: self.learning_rate,
            });
        }

        // Generate noisy state at current time
        let noisy_state = self
            .noise_model
            .generate_noisy_state(self.time_point, context.seed);
        let initial_entropy = EpigeneticNoiseModel::shannon_entropy(&noisy_state);

        // Iterative correction
        let mut current_state = noisy_state;
        let mut corrections = Vec::new();
        let mut current_entropy = initial_entropy;

        for iter in 0..context.max_iterations {
            // Calculate correction vector toward identity
            let correction: Vec<f64> = self
                .noise_model
                .identity_signature
                .iter()
                .zip(current_state.iter())
                .map(|(target, current)| self.learning_rate * (target - current))
                .collect();

            // Apply correction
            let mut new_state: Vec<f64> = current_state
                .iter()
                .zip(correction.iter())
                .map(|(s, c)| (s + c).max(0.0))
                .collect();

            // Normalize
            let sum: f64 = new_state.iter().sum();
            if sum > 0.0 {
                new_state.iter_mut().for_each(|v| *v /= sum);
            }

            // Calculate metrics
            let new_entropy = EpigeneticNoiseModel::shannon_entropy(&new_state);
            let identity_drift = EpigeneticNoiseModel::kl_divergence(
                &new_state,
                &self.noise_model.identity_signature,
            );

            // Check identity constraint
            if identity_drift > self.max_identity_drift {
                // Correction would compromise identity â€” stop
                corrections.push(SyntaxCorrection {
                    iteration: iter,
                    entropy_before: current_entropy,
                    entropy_after: new_entropy,
                    identity_drift,
                    correction_strength: self.learning_rate,
                    success: false,
                });
                break;
            }

            // Record successful correction
            corrections.push(SyntaxCorrection {
                iteration: iter,
                entropy_before: current_entropy,
                entropy_after: new_entropy,
                identity_drift,
                correction_strength: self.learning_rate,
                success: true,
            });

            current_state = new_state;
            current_entropy = new_entropy;

            // Convergence check
            if initial_entropy - current_entropy < 1e-6 {
                break;
            }
        }

        // Calculate regeneration score
        let regeneration_score = if initial_entropy > 0.0 {
            (initial_entropy - current_entropy) / initial_entropy
        } else {
            0.0
        };

        // Calculate restored telomere length
        let restored_length = self.noise_model.telomere_length(
            self.time_point * (1.0 - regeneration_score),
            self.initial_telomere_length,
        );

        // Confidence based on successful corrections ratio
        let successful_corrections = corrections.iter().filter(|c| c.success).count();
        let confidence = if !corrections.is_empty() {
            successful_corrections as f64 / corrections.len() as f64
        } else {
            0.0
        };

        // Serialize metadata for validation
        let metadata = bincode::serialize(&corrections).unwrap_or_else(|_| Vec::new());

        Ok(WorkloadResult::new(
            restored_length,
            confidence,
            corrections.len(),
            metadata,
        ))
    }
}

impl DistributedWorkload for TelomereRegenerationTask {
    fn workload_id(&self) -> &str {
        "telomere_regeneration_v1"
    }

    fn execute(&self, context: &WorkloadContext) -> Result<WorkloadResult, WorkloadError> {
        self.regenerate(context)
    }

    fn validate_result(
        &self,
        _context: &WorkloadContext,
        result: &WorkloadResult,
    ) -> Result<(), WorkloadError> {
        // Validate result is within expected bounds
        if result.value.is_nan() || result.value.is_infinite() {
            return Err(WorkloadError::NumericalInstability {
                metric: "restored_length",
            });
        }

        if result.value < 0.0 || result.value > self.initial_telomere_length * 1.1 {
            return Err(WorkloadError::ValidationFailed {
                reason: "restored length out of bounds",
            });
        }

        if result.confidence < 0.0 || result.confidence > 1.0 {
            return Err(WorkloadError::ValidationFailed {
                reason: "confidence out of bounds",
            });
        }

        // Validate metadata contains correction records
        if result.metadata.is_empty() && result.iterations > 0 {
            return Err(WorkloadError::ValidationFailed {
                reason: "missing correction metadata",
            });
        }

        Ok(())
    }

    fn estimated_cost(&self) -> WorkloadCost {
        let state_size = self.noise_model.identity_signature.len() as u64;
        let max_iterations = 100u64; // Default estimate
        WorkloadCost {
            cpu_cycles: state_size
                .saturating_mul(max_iterations)
                .saturating_mul(1000),
            memory_bytes: state_size.saturating_mul(8),
            complexity: "O(n * iterations)",
        }
    }
}

impl Default for TelomereRegenerationTask {
    fn default() -> Self {
        Self::new(0.0)
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // EpigeneticNoiseModel tests
    #[test]
    fn test_noise_model_creation() {
        let model = EpigeneticNoiseModel::new();
        assert!(model.h0 > 0.0);
        assert!(model.alpha > 0.0);
        assert!(model.h_max > model.h0);
    }

    #[test]
    fn test_noise_model_custom_params() {
        let model = EpigeneticNoiseModel::with_params(0.05, 0.03, 0.01, 1.5);
        assert_eq!(model.h0, 0.05);
        assert_eq!(model.alpha, 0.03);
        assert_eq!(model.beta, 0.01);
        assert_eq!(model.h_max, 1.5);
    }

    #[test]
    fn test_entropy_increases_with_time() {
        let model = EpigeneticNoiseModel::new();
        let h0 = model.entropy_at_time(0.0);
        let h10 = model.entropy_at_time(10.0);
        let h100 = model.entropy_at_time(100.0);
        assert!(h0 < h10);
        assert!(h10 < h100);
    }

    #[test]
    fn test_entropy_at_time_zero() {
        let model = EpigeneticNoiseModel::new();
        let h0 = model.entropy_at_time(0.0);
        // ln(1) = 0, so H(0) = H_0
        assert!((h0 - model.h0).abs() < 1e-10);
    }

    #[test]
    fn test_telomere_length_decreases_with_time() {
        let model = EpigeneticNoiseModel::new();
        let l0 = model.telomere_length(0.0, 10000.0);
        let l100 = model.telomere_length(100.0, 10000.0);
        assert!(l0 > l100);
        assert!(l0 <= 10000.0);
        assert!(l100 >= 0.0);
    }

    #[test]
    fn test_shannon_entropy_uniform() {
        // Uniform distribution has maximum entropy
        let uniform = vec![0.25, 0.25, 0.25, 0.25];
        let entropy = EpigeneticNoiseModel::shannon_entropy(&uniform);
        assert!((entropy - 2.0).abs() < 1e-10); // log2(4) = 2
    }

    #[test]
    fn test_shannon_entropy_deterministic() {
        // Deterministic distribution has zero entropy
        let deterministic = vec![1.0, 0.0, 0.0, 0.0];
        let entropy = EpigeneticNoiseModel::shannon_entropy(&deterministic);
        assert!(entropy.abs() < 1e-10);
    }

    #[test]
    fn test_kl_divergence_same_distribution() {
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let kl = EpigeneticNoiseModel::kl_divergence(&p, &p);
        assert!(kl.abs() < 1e-10);
    }

    #[test]
    fn test_kl_divergence_different_distribution() {
        let p = vec![1.0, 0.0, 0.0, 0.0];
        let q = vec![0.25, 0.25, 0.25, 0.25];
        let kl = EpigeneticNoiseModel::kl_divergence(&p, &q);
        assert!(kl > 0.0);
    }

    #[test]
    fn test_generate_noisy_state_normalizes() {
        let model = EpigeneticNoiseModel::new();
        let state = model.generate_noisy_state(50.0, 12345);
        let sum: f64 = state.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert_eq!(state.len(), 8);
    }

    #[test]
    fn test_noisy_state_diverges_with_time() {
        let model = EpigeneticNoiseModel::new();
        let state_young = model.generate_noisy_state(0.0, 42);
        let state_old = model.generate_noisy_state(100.0, 42);

        let drift_young =
            EpigeneticNoiseModel::kl_divergence(&state_young, &model.identity_signature);
        let drift_old = EpigeneticNoiseModel::kl_divergence(&state_old, &model.identity_signature);

        assert!(drift_old > drift_young);
    }

    // TelomereRegenerationTask tests
    #[test]
    fn test_task_creation() {
        let task = TelomereRegenerationTask::new(50.0);
        assert_eq!(task.time_point, 50.0);
        assert_eq!(task.initial_telomere_length, 10000.0);
    }

    #[test]
    fn test_task_custom_params() {
        let task = TelomereRegenerationTask::with_params(100.0, 8000.0, 0.2, 0.05);
        assert_eq!(task.time_point, 100.0);
        assert_eq!(task.initial_telomere_length, 8000.0);
        assert_eq!(task.max_identity_drift, 0.2);
        assert_eq!(task.learning_rate, 0.05);
    }

    #[test]
    fn test_task_default() {
        let task = TelomereRegenerationTask::default();
        assert_eq!(task.time_point, 0.0);
    }

    #[test]
    fn test_workload_id() {
        let task = TelomereRegenerationTask::new(50.0);
        assert_eq!(task.workload_id(), "telomere_regeneration_v1");
    }

    #[test]
    fn test_execute_valid() {
        let task = TelomereRegenerationTask::new(50.0);
        let context = WorkloadContext::default();
        let result = task.execute(&context);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.value > 0.0);
        assert!(result.value <= task.initial_telomere_length * 1.1);
        assert!(result.confidence >= 0.0);
        assert!(result.confidence <= 1.0);
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_execute_invalid_time() {
        let mut task = TelomereRegenerationTask::new(-1.0);
        let context = WorkloadContext::default();
        let result = task.execute(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_invalid_learning_rate() {
        let task = TelomereRegenerationTask::with_params(50.0, 10000.0, 0.1, 0.0);
        let context = WorkloadContext::default();
        let result = task.execute(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_result_valid() {
        let task = TelomereRegenerationTask::new(50.0);
        let context = WorkloadContext::default();

        let result = task.execute(&context).unwrap();
        let validation = task.validate_result(&context, &result);
        assert!(validation.is_ok());
    }

    #[test]
    fn test_validate_result_nan() {
        let task = TelomereRegenerationTask::new(50.0);
        let context = WorkloadContext::default();
        let result = WorkloadResult::new(f64::NAN, 0.5, 10, vec![1, 2, 3]);
        let validation = task.validate_result(&context, &result);
        assert!(validation.is_err());
    }

    #[test]
    fn test_validate_result_out_of_bounds() {
        let task = TelomereRegenerationTask::new(50.0);
        let context = WorkloadContext::default();
        let result = WorkloadResult::new(99999.0, 0.5, 10, vec![1, 2, 3]);
        let validation = task.validate_result(&context, &result);
        assert!(validation.is_err());
    }

    #[test]
    fn test_validate_result_confidence_out_of_bounds() {
        let task = TelomereRegenerationTask::new(50.0);
        let context = WorkloadContext::default();
        // Use struct literal to bypass confidence clamping in WorkloadResult::new
        let result = WorkloadResult {
            value: 5000.0,
            confidence: 1.5, // Out of bounds, not clamped
            iterations: 10,
            metadata: vec![1, 2, 3],
        };
        let validation = task.validate_result(&context, &result);
        assert!(validation.is_err());
    }

    #[test]
    fn test_estimated_cost() {
        let task = TelomereRegenerationTask::new(50.0);
        let cost = task.estimated_cost();
        assert!(cost.cpu_cycles > 0);
        assert!(cost.memory_bytes > 0);
        assert_eq!(cost.complexity, "O(n * iterations)");
    }

    #[test]
    fn test_regeneration_improves_with_younger_cells() {
        let task_young = TelomereRegenerationTask::new(10.0);
        let task_old = TelomereRegenerationTask::new(100.0);
        let context = WorkloadContext::default();

        let result_young = task_young.execute(&context).unwrap();
        let result_old = task_old.execute(&context).unwrap();

        // Younger cells should have higher restored telomere length
        assert!(result_young.value > result_old.value);
    }

    #[test]
    fn test_deterministic_results_with_same_seed() {
        let task = TelomereRegenerationTask::new(50.0);
        let context1 = WorkloadContext {
            seed: 42,
            ..Default::default()
        };
        let context2 = WorkloadContext {
            seed: 42,
            ..Default::default()
        };

        let result1 = task.execute(&context1).unwrap();
        let result2 = task.execute(&context2).unwrap();

        assert_eq!(result1.value, result2.value);
        assert_eq!(result1.confidence, result2.confidence);
    }

    // WorkloadContext tests
    #[test]
    fn test_context_default() {
        let ctx = WorkloadContext::default();
        assert_eq!(ctx.node_id, 0);
        assert_eq!(ctx.total_validators, 7);
        assert_eq!(ctx.consensus_threshold, 5);
        assert_eq!(ctx.seed, 42);
        assert_eq!(ctx.max_iterations, 100);
    }

    // SyntaxCorrection tests
    #[test]
    fn test_correction_improvement() {
        let correction = SyntaxCorrection {
            iteration: 1,
            entropy_before: 1.5,
            entropy_after: 1.0,
            identity_drift: 0.05,
            correction_strength: 0.1,
            success: true,
        };
        assert!((correction.improvement() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_correction_negative_improvement_clamped() {
        let correction = SyntaxCorrection {
            iteration: 1,
            entropy_before: 1.0,
            entropy_after: 1.5,
            identity_drift: 0.05,
            correction_strength: 0.1,
            success: false,
        };
        assert_eq!(correction.improvement(), 0.0);
    }

    // WorkloadResult tests
    #[test]
    fn test_result_confidence_clamped() {
        let result = WorkloadResult::new(1000.0, 1.5, 10, vec![]);
        assert_eq!(result.confidence, 1.0);

        let result = WorkloadResult::new(1000.0, -0.5, 10, vec![]);
        assert_eq!(result.confidence, 0.0);
    }

    // Error display tests
    #[test]
    fn test_error_display_invalid_parameter() {
        let err = WorkloadError::InvalidParameter {
            name: "test",
            value: 1.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test"));
    }

    #[test]
    fn test_error_display_identity_drift() {
        let err = WorkloadError::IdentityDriftExceeded {
            drift: 0.5,
            tolerance: 0.1,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("0.5"));
        assert!(msg.contains("0.1"));
    }
}
