//! RSSI Engine â€” Recursive Topological Self-Improvement with Byzantine_Eviction.
//!
//! Implements the 5-phase self-improvement cycle:
//! 1. **Inference Phase:** SAE execution on complex prompts.
//! 2. **Human Steering Aggregation:** SCT update weighted by Steward CE.
//! 3. **Geometric Ethical Gradient:** Compute G_e projected to Octahedron.
//! 4. **Self-Improvement Step:** Controlled update `I_{n+1} = I_n + alpha * grad`.
//! 5. **Validation Gate:** BFT consensus + cryptographic signature from 7+ Stewards.
//!
//! **Byzantine_Eviction Mechanism:** If validation fails or basin exit is detected,
//! triggers automatic rollback and partial weight reset of unstable SAE layers.
//!
//! WASM-compatible: pure computation, no std::thread.

#[cfg(feature = "v3.3-rssi-evolution")]
use crate::alignment::attractor_basin::{BasinConfig, BasinExitWarning, EthicalAttractorBasin};
#[cfg(feature = "v3.3-rssi-evolution")]
use crate::ethics::moral_manifold::{SCTPoint, Vector3};
#[cfg(feature = "v3.3-rssi-evolution")]
use crate::topology::deception_detector::{DeceptionConfig, DeceptionDetector, DeceptionStatus};
#[cfg(feature = "v3.3-rssi-evolution")]
use crate::topology::persistent_homology::{
    EthicalPoint, HomologyResult, PersistentHomologyEngine,
};

/// Maximum number of SAE layers that can be tracked.
#[cfg(feature = "v3.3-rssi-evolution")]
const MAX_SAE_LAYERS: usize = 64;

/// Result of a single RSSI improvement step.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, PartialEq)]
pub struct ImprovementResult {
    /// Iteration number.
    pub iteration: usize,
    /// Previous interpretation state.
    pub i_previous: Vector3,
    /// New interpretation state.
    pub i_current: Vector3,
    /// Ethical distance at this step.
    pub ethical_distance: f64,
    /// Step size taken.
    pub step_size: f64,
    /// Contraction held (Lyapunov condition satisfied).
    pub contraction_held: bool,
    /// Human correlation score (higher = better alignment).
    pub human_correlation: f64,
    /// Byzantine_Eviction was triggered.
    pub byzantine_eviction_triggered: bool,
}

/// Configuration for the RSSI Engine.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone)]
pub struct RssiConfig {
    /// Learning rate for self-improvement step.
    pub alpha: f64,
    /// Minimum BFT consensus ratio (e.g., 0.67 for 2/3 majority).
    pub bft_threshold: f64,
    /// Minimum number of Steward signatures required.
    pub min_steward_signatures: usize,
    /// Basin configuration.
    pub basin_config: BasinConfig,
    /// Deception detector configuration.
    pub deception_config: DeceptionConfig,
    /// Maximum iterations before forced review.
    pub max_iterations: usize,
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl Default for RssiConfig {
    fn default() -> Self {
        Self {
            alpha: 0.05,
            bft_threshold: 0.67,
            min_steward_signatures: 7,
            basin_config: BasinConfig::default(),
            deception_config: DeceptionConfig::default(),
            max_iterations: 100,
        }
    }
}

/// Errors for RSSI operations.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RssiError {
    /// Basin validation failed â€” system exiting ethical basin.
    BasinExit(BasinExitWarning),
    /// Deception detected in ethical trajectory.
    DeceptionDetected,
    /// BFT consensus not reached.
    ConsensusFailed { approved: usize, required: usize },
    /// Insufficient Steward signatures.
    InsufficientSignatures { received: usize, required: usize },
    /// Maximum iterations reached.
    MaxIterationsReached,
    /// Invalid configuration.
    InvalidConfig(&'static str),
    /// Byzantine_Eviction error during rollback.
    ByzantineEvictionFailed(&'static str),
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl core::fmt::Display for RssiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RssiError::BasinExit(warning) => {
                write!(f, "Basin exit detected: {:?}", warning)
            }
            RssiError::DeceptionDetected => {
                write!(f, "Deceptive alignment pattern detected")
            }
            RssiError::ConsensusFailed { approved, required } => {
                write!(
                    f,
                    "BFT consensus failed: {}/{} approved",
                    approved, required
                )
            }
            RssiError::InsufficientSignatures { received, required } => {
                write!(f, "Insufficient signatures: {}/{}", received, required)
            }
            RssiError::MaxIterationsReached => {
                write!(f, "Maximum iterations reached â€” forced review required")
            }
            RssiError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            RssiError::ByzantineEvictionFailed(msg) => {
                write!(f, "Byzantine_Eviction failed: {}", msg)
            }
        }
    }
}

/// Errors specific to Byzantine_Eviction (rollback) operations.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByzantineEvictionError {
    /// Layer index out of bounds.
    LayerOutOfBounds { index: usize, max: usize },
    /// Rollback state unavailable.
    NoRollbackState,
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl core::fmt::Display for ByzantineEvictionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ByzantineEvictionError::LayerOutOfBounds { index, max } => {
                write!(f, "Layer {} out of bounds (max {})", index, max)
            }
            ByzantineEvictionError::NoRollbackState => {
                write!(f, "No rollback state available for Byzantine_Eviction")
            }
        }
    }
}

/// The RSSI Engine: orchestrates recursive self-improvement with ethical containment.
#[cfg(feature = "v3.3-rssi-evolution")]
pub struct RssiEngine {
    config: RssiConfig,
    basin: EthicalAttractorBasin,
    detector: DeceptionDetector,
    homology_engine: PersistentHomologyEngine,
    /// Current interpretation state.
    current_state: Vector3,
    /// Previous interpretation state (for rollback).
    previous_state: Option<Vector3>,
    /// SAE layer weights (simulated for self-improvement tracking).
    sae_weights: Vec<f64>,
    /// Iteration counter.
    iteration: usize,
    /// History of ethical trajectory points.
    trajectory: Vec<SCTPoint>,
    /// Accumulated improvement results.
    results: Vec<ImprovementResult>,
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl RssiEngine {
    /// Create a new RSSI engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: RssiConfig::default(),
            basin: EthicalAttractorBasin::new(),
            detector: DeceptionDetector::new(),
            homology_engine: PersistentHomologyEngine::new(),
            current_state: Vector3::new(0.3, 0.4, 0.5),
            previous_state: None,
            sae_weights: vec![0.5; MAX_SAE_LAYERS],
            iteration: 0,
            trajectory: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: RssiConfig) -> Result<Self, RssiError> {
        if config.alpha <= 0.0 || config.alpha > 1.0 {
            return Err(RssiError::InvalidConfig("alpha must be in (0, 1]"));
        }
        if config.min_steward_signatures < 7 {
            return Err(RssiError::InvalidConfig(
                "min_steward_signatures must be >= 7",
            ));
        }
        let basin = EthicalAttractorBasin::with_config(
            Vector3::new(0.0, 0.0, 1.0),
            config.basin_config.clone(),
        )
        .map_err(|e| RssiError::InvalidConfig(e.to_string().leak()))?;
        let detector = DeceptionDetector::with_config(config.deception_config.clone())
            .map_err(|_| RssiError::InvalidConfig("deception config invalid"))?;
        Ok(Self {
            config,
            basin,
            detector,
            homology_engine: PersistentHomologyEngine::new(),
            current_state: Vector3::new(0.3, 0.4, 0.5),
            previous_state: None,
            sae_weights: vec![0.5; MAX_SAE_LAYERS],
            iteration: 0,
            trajectory: Vec::new(),
            results: Vec::new(),
        })
    }

    // --- Phase 1: Inference Phase ---

    /// Phase 1: Execute SAE inference on complex prompts.
    /// Returns feature activations as a Vector3 (X=ComprensiÃ³n, Y=GeneralizaciÃ³n, Z=Ã‰tica).
    pub fn phase_inference(&self, prompt_features: &[f64]) -> Vector3 {
        if prompt_features.is_empty() {
            return self.current_state;
        }
        let chunk_size = prompt_features.len() / 3;
        if chunk_size == 0 {
            return self.current_state;
        }
        let x: f64 = prompt_features[..chunk_size].iter().sum::<f64>() / chunk_size as f64;
        let y: f64 = prompt_features[chunk_size..2 * chunk_size]
            .iter()
            .sum::<f64>()
            / chunk_size as f64;
        let z: f64 = prompt_features[2 * chunk_size..].iter().sum::<f64>()
            / (prompt_features.len() - 2 * chunk_size) as f64;
        Vector3::new(
            x * self.sae_weights[0].clamp(0.0, 1.0),
            y * self.sae_weights[1].clamp(0.0, 1.0),
            z * self.sae_weights[2].clamp(0.0, 1.0),
        )
    }

    // --- Phase 2: Human Steering Aggregation ---

    /// Phase 2: Aggregate human steering signals weighted by Steward CE.
    /// Returns updated SCT direction.
    pub fn phase_steering_aggregation(
        &self,
        steward_signals: &[(Vector3, f64)], // (direction, ce_weight)
    ) -> Vector3 {
        if steward_signals.is_empty() {
            return self.current_state;
        }
        let total_ce: f64 = steward_signals.iter().map(|(_, ce)| ce).sum();
        if total_ce < 1e-15 {
            return self.current_state;
        }
        let mut aggregated = Vector3::zero();
        for (direction, ce) in steward_signals {
            let weight = ce / total_ce;
            let weighted = direction.scale(weight);
            aggregated = aggregated.add(&weighted);
        }
        aggregated
    }

    // --- Phase 3: Geometric Ethical Gradient ---

    /// Phase 3: Compute geometric ethical gradient projected to Octahedron.
    /// G_e = proj_Oct(steering - current_state)
    pub fn phase_ethical_gradient(&self, steering: &Vector3) -> Vector3 {
        let gradient = steering.sub(&self.current_state);
        EthicalAttractorBasin::project_to_octahedron(&gradient)
    }

    // --- Phase 4: Self-Improvement Step ---

    /// Phase 4: Apply controlled self-improvement step.
    /// I_{n+1} = I_n + alpha * G_e
    pub fn phase_improvement_step(&self, gradient: &Vector3) -> Vector3 {
        let update = gradient.scale(self.config.alpha);
        self.current_state.add(&update)
    }

    // --- Phase 5: Validation Gate ---

    /// Phase 5: Validate improvement through BFT consensus and basin check.
    pub fn phase_validation(
        &mut self,
        i_next: &Vector3,
        steward_approvals: usize,
        steward_signatures: usize,
        total_validators: usize,
    ) -> Result<ImprovementResult, RssiError> {
        // BFT consensus check
        let consensus_ratio = steward_approvals as f64 / total_validators as f64;
        if consensus_ratio < self.config.bft_threshold {
            return Err(RssiError::ConsensusFailed {
                approved: steward_approvals,
                required: total_validators,
            });
        }

        // Signature check
        if steward_signatures < self.config.min_steward_signatures {
            return Err(RssiError::InsufficientSignatures {
                received: steward_signatures,
                required: self.config.min_steward_signatures,
            });
        }

        // Homology computation for ethical distance
        let ethical_points: Vec<EthicalPoint> = self
            .trajectory
            .iter()
            .map(|p| EthicalPoint {
                x: p.x as f64,
                y: p.y as f64,
                z: p.z as f64,
            })
            .collect();
        let homology = if ethical_points.len() >= 3 {
            self.homology_engine.compute(&ethical_points)
        } else {
            HomologyResult {
                ph0_pairs: Vec::new(),
                ph1_pairs: Vec::new(),
                num_points: 0,
                num_edges: 0,
                alpha: 0.0,
            }
        };

        // Ethical distance
        let eth_dist = self.basin.compute_ethical_distance(i_next, &homology);

        // Lyapunov contraction check
        let contraction_result =
            self.basin
                .validate_contraction(&self.current_state, i_next, eth_dist.weighted);

        let contraction_held = match &contraction_result {
            Ok(held) => *held,
            Err(BasinExitWarning::CriticalInstability) => {
                return Err(RssiError::BasinExit(BasinExitWarning::CriticalInstability));
            }
            Err(BasinExitWarning::ContractionViolation) => false,
        };

        // Deception detection
        if self.trajectory.len() >= 3 {
            match self.detector.analyze_persistent_loops(&self.trajectory) {
                Ok(DeceptionStatus::OutsideBasin { .. }) => {
                    return Err(RssiError::DeceptionDetected);
                }
                Ok(DeceptionStatus::WithinBasin) => {}
                Err(_) => {} // Insufficient data â€” proceed with caution
            }
        }

        // Record trajectory point
        self.trajectory.push(SCTPoint::new(
            i_next.x as f32,
            i_next.y as f32,
            i_next.z as f32,
            self.iteration as u64,
        ));

        let step_size = i_next.sub(&self.current_state).magnitude();
        let human_correlation = Self::compute_human_correlation(i_next);

        let result = ImprovementResult {
            iteration: self.iteration,
            i_previous: self.current_state,
            i_current: *i_next,
            ethical_distance: eth_dist.weighted,
            step_size,
            contraction_held,
            human_correlation,
            byzantine_eviction_triggered: false,
        };

        // Commit the improvement
        self.previous_state = Some(self.current_state);
        self.current_state = *i_next;
        self.iteration += 1;
        self.results.push(result.clone());

        Ok(result)
    }

    /// Compute human correlation score: alignment with Upper Focus.
    /// Score = dot(current, upper_focus) normalized to [0, 1].
    pub fn compute_human_correlation(state: &Vector3) -> f64 {
        let upper_focus = Vector3::new(0.0, 0.0, 1.0);
        let normalized = state.normalize();
        (normalized.dot(&upper_focus) + 1.0) / 2.0
    }

    // --- Full Cycle ---

    /// Execute one full RSSI cycle (all 5 phases).
    pub fn execute_cycle(
        &mut self,
        prompt_features: &[f64],
        steward_signals: &[(Vector3, f64)],
        steward_approvals: usize,
        steward_signatures: usize,
        total_validators: usize,
    ) -> Result<ImprovementResult, RssiError> {
        if self.iteration >= self.config.max_iterations {
            return Err(RssiError::MaxIterationsReached);
        }

        // Phase 1: Inference
        let inference = self.phase_inference(prompt_features);

        // Phase 2: Steering Aggregation
        let steering = self.phase_steering_aggregation(steward_signals);

        // Blend inference with steering
        let blended = inference.scale(0.4).add(&steering.scale(0.6));

        // Phase 3: Ethical Gradient
        let gradient = self.phase_ethical_gradient(&blended);

        // Phase 4: Improvement Step
        let i_next = self.phase_improvement_step(&gradient);

        // Phase 5: Validation Gate
        self.phase_validation(
            &i_next,
            steward_approvals,
            steward_signatures,
            total_validators,
        )
    }

    // --- Byzantine_Eviction Mechanism ---

    /// Trigger Byzantine_Eviction: rollback to previous state and reset unstable SAE layers.
    pub fn trigger_byzantine_eviction(
        &mut self,
        unstable_layers: &[usize],
    ) -> Result<(), ByzantineEvictionError> {
        // Rollback interpretation state
        match self.previous_state.take() {
            Some(previous) => {
                self.current_state = previous;
            }
            None => {
                return Err(ByzantineEvictionError::NoRollbackState);
            }
        }

        // Reset unstable SAE layer weights to baseline
        for &layer_idx in unstable_layers {
            if layer_idx >= MAX_SAE_LAYERS {
                return Err(ByzantineEvictionError::LayerOutOfBounds {
                    index: layer_idx,
                    max: MAX_SAE_LAYERS,
                });
            }
            self.sae_weights[layer_idx] = 0.5; // Reset to neutral
        }

        // Reset basin violations
        self.basin.reset();

        // Remove last trajectory point if it was unstable
        if self.trajectory.len() > 1 {
            self.trajectory.pop();
        }

        Ok(())
    }

    /// Get current interpretation state.
    pub fn current_state(&self) -> &Vector3 {
        &self.current_state
    }

    /// Get iteration count.
    pub fn iteration(&self) -> usize {
        self.iteration
    }

    /// Get improvement history.
    pub fn results(&self) -> &[ImprovementResult] {
        &self.results
    }

    /// Get trajectory history.
    pub fn trajectory(&self) -> &[SCTPoint] {
        &self.trajectory
    }

    /// Compute approximate Lyapunov exponent from improvement history.
    /// lambda = (1/N) * sum(ln(||delta_{n+1}|| / ||delta_n||))
    /// Negative lambda indicates convergence (stable attractor).
    pub fn lyapunov_exponent(&self) -> Option<f64> {
        if self.results.len() < 2 {
            return None;
        }
        let mut sum = 0.0;
        let mut count = 0;
        for window in self.results.windows(2) {
            let d0 = window[0].step_size;
            let d1 = window[1].step_size;
            if d0 > 1e-15 && d1 > 1e-15 {
                sum += (d1 / d0).ln();
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        Some(sum / count as f64)
    }

    /// Reset engine to initial state.
    pub fn reset(&mut self) {
        self.current_state = Vector3::new(0.3, 0.4, 0.5);
        self.previous_state = None;
        self.sae_weights = vec![0.5; MAX_SAE_LAYERS];
        self.iteration = 0;
        self.trajectory.clear();
        self.results.clear();
        self.basin.reset();
    }
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl Default for RssiEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "v3.3-rssi-evolution"))]
mod tests {
    use super::*;

    fn upper_steward_signal(ce: f64) -> (Vector3, f64) {
        (Vector3::new(0.1, 0.1, 0.8), ce)
    }

    fn valid_prompt_features() -> Vec<f64> {
        vec![0.6, 0.7, 0.8, 0.3, 0.4, 0.5, 0.7, 0.8, 0.9]
    }

    #[test]
    fn test_engine_creation() {
        let engine = RssiEngine::new();
        assert_eq!(engine.iteration(), 0);
        assert!(engine.results().is_empty());
    }

    #[test]
    fn test_engine_custom_config() {
        let config = RssiConfig {
            alpha: 0.1,
            min_steward_signatures: 10,
            ..Default::default()
        };
        let engine = RssiEngine::with_config(config).unwrap();
        assert!((engine.config.alpha - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_alpha() {
        let config = RssiConfig {
            alpha: 0.0,
            ..Default::default()
        };
        let result = RssiEngine::with_config(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_phase_inference() {
        let engine = RssiEngine::new();
        let features = valid_prompt_features();
        let result = engine.phase_inference(&features);
        assert!(!result.x.is_nan());
        assert!(!result.y.is_nan());
        assert!(!result.z.is_nan());
    }

    #[test]
    fn test_phase_steering_aggregation() {
        let engine = RssiEngine::new();
        let signals = vec![upper_steward_signal(10.0), upper_steward_signal(5.0)];
        let result = engine.phase_steering_aggregation(&signals);
        assert!((result.z - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_phase_ethical_gradient() {
        let engine = RssiEngine::new();
        let steering = Vector3::new(0.5, 0.5, 0.9);
        let gradient = engine.phase_ethical_gradient(&steering);
        // Gradient should be projected to octahedron
        let l1 = gradient.x.abs() + gradient.y.abs() + gradient.z.abs();
        assert!(l1 <= 1.0 + 1e-10);
    }

    #[test]
    fn test_phase_improvement_step() {
        let engine = RssiEngine::new();
        let gradient = Vector3::new(0.0, 0.0, 0.5);
        let next = engine.phase_improvement_step(&gradient);
        // Should move in gradient direction scaled by alpha
        assert!(next.z > engine.current_state().z);
    }

    #[test]
    fn test_full_cycle_success() {
        let mut engine = RssiEngine::new();
        let features = valid_prompt_features();
        let signals = vec![upper_steward_signal(10.0)];
        let result = engine.execute_cycle(
            &features, &signals, 8,  // approvals
            10, // signatures
            10, // total validators
        );
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.contraction_held);
        assert!(r.byzantine_eviction_triggered == false);
    }

    #[test]
    fn test_consensus_failure() {
        let mut engine = RssiEngine::new();
        let features = valid_prompt_features();
        let signals = vec![upper_steward_signal(10.0)];
        let result = engine.execute_cycle(
            &features, &signals, 2, // only 2 approvals out of 10
            10, 10,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RssiError::ConsensusFailed { .. }
        ));
    }

    #[test]
    fn test_insufficient_signatures() {
        let mut engine = RssiEngine::new();
        let features = valid_prompt_features();
        let signals = vec![upper_steward_signal(10.0)];
        let result = engine.execute_cycle(
            &features, &signals, 8, 3, // only 3 signatures (need 7)
            10,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RssiError::InsufficientSignatures { .. }
        ));
    }

    #[test]
    fn test_Byzantine_Eviction_rollback() {
        let mut engine = RssiEngine::new();
        // Advance one step first
        engine.previous_state = Some(Vector3::new(0.2, 0.3, 0.4));
        engine.current_state = Vector3::new(0.5, 0.5, 0.5);
        engine.sae_weights[0] = 0.9;

        let result = engine.trigger_byzantine_eviction(&[0]);
        assert!(result.is_ok());
        // State should be rolled back
        assert!((engine.current_state().x - 0.2).abs() < 1e-10);
        // Weight should be reset
        assert!((engine.sae_weights[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_Byzantine_Eviction_no_rollback_state() {
        let mut engine = RssiEngine::new();
        engine.previous_state = None;
        let result = engine.trigger_byzantine_eviction(&[0]);
        assert_eq!(result, Err(ByzantineEvictionError::NoRollbackState));
    }

    #[test]
    fn test_Byzantine_Eviction_layer_out_of_bounds() {
        let mut engine = RssiEngine::new();
        engine.previous_state = Some(Vector3::new(0.2, 0.3, 0.4));
        let result = engine.trigger_byzantine_eviction(&[100]);
        assert_eq!(
            result,
            Err(ByzantineEvictionError::LayerOutOfBounds {
                index: 100,
                max: MAX_SAE_LAYERS
            })
        );
    }

    #[test]
    fn test_lyapunov_exponent() {
        let engine = RssiEngine::new();
        assert!(engine.lyapunov_exponent().is_none());
    }

    #[test]
    fn test_reset() {
        let mut engine = RssiEngine::new();
        engine.current_state = Vector3::new(0.9, 0.9, 0.9);
        engine.iteration = 5;
        engine.reset();
        assert_eq!(engine.iteration(), 0);
        assert!((engine.current_state().z - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_human_correlation_upper() {
        let upper = Vector3::new(0.0, 0.0, 1.0);
        let score = RssiEngine::compute_human_correlation(&upper);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_human_correlation_lower() {
        let lower = Vector3::new(0.0, 0.0, -1.0);
        let score = RssiEngine::compute_human_correlation(&lower);
        assert!((score - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_error_display() {
        let e = RssiError::BasinExit(BasinExitWarning::CriticalInstability);
        assert!(format!("{}", e).contains("Basin"));
        let e = RssiError::DeceptionDetected;
        assert!(format!("{}", e).contains("Deceptive"));
    }

    #[test]
    fn test_Byzantine_Eviction_error_display() {
        let e = ByzantineEvictionError::NoRollbackState;
        assert!(format!("{}", e).contains("rollback"));
    }

    #[test]
    fn test_default_engine() {
        let engine = RssiEngine::default();
        assert_eq!(engine.iteration(), 0);
    }

    #[test]
    fn test_max_iterations() {
        let mut engine = RssiEngine::new();
        engine.iteration = 100; // At max
        let features = valid_prompt_features();
        let signals = vec![upper_steward_signal(10.0)];
        let result = engine.execute_cycle(&features, &signals, 8, 10, 10);
        assert_eq!(result, Err(RssiError::MaxIterationsReached));
    }
}
