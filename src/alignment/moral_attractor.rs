//! Moral Attractor â€” Sprint 70: Civilization-Scale Architecture
//!
//! Moral Manifold modeled as a Lyapunov attractor basin, providing
//! ethical attention masking and convergence guarantees.

use std::fmt;

/// Errors during moral attractor computation.
#[derive(Debug, Clone, PartialEq)]
pub enum AttractorError {
    /// Invalid attractor dimension.
    InvalidDimension(usize),
    /// State vector outside ethical bounds.
    OutOfBounds(String),
    /// Lyapunov function diverged beyond threshold.
    Divergence(f64),
    /// Basin capacity exceeded.
    BasinFull(usize),
}

impl fmt::Display for AttractorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttractorError::InvalidDimension(d) => {
                write!(f, "Invalid attractor dimension: {}", d)
            }
            AttractorError::OutOfBounds(msg) => {
                write!(f, "State out of bounds: {}", msg)
            }
            AttractorError::Divergence(v) => {
                write!(f, "Lyapunov divergence: V = {:.6}", v)
            }
            AttractorError::BasinFull(capacity) => {
                write!(f, "Basin capacity full: {}", capacity)
            }
        }
    }
}

impl std::error::Error for AttractorError {}

/// Configuration for the moral attractor.
#[derive(Debug, Clone)]
pub struct AttractorConfig {
    /// Attractor basin dimension (GEI space).
    pub dimension: usize,
    /// Maximum basin capacity.
    pub max_capacity: usize,
    /// Lyapunov convergence threshold (Î³ < threshold â†’ stable).
    pub convergence_threshold: f64,
    /// Attraction strength (higher = faster convergence).
    pub attraction_strength: f64,
    /// Ethical attention mask threshold.
    pub attention_threshold: f64,
}

impl AttractorConfig {
    /// Default Topological configuration.
    pub fn default_topological() -> Self {
        Self {
            dimension: 8,
            max_capacity: 1000,
            convergence_threshold: 0.95,
            attraction_strength: 0.5,
            attention_threshold: 0.3,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), AttractorError> {
        if self.dimension == 0 {
            return Err(AttractorError::InvalidDimension(0));
        }
        if self.max_capacity == 0 {
            return Err(AttractorError::BasinFull(0));
        }
        if !(0.0..1.0).contains(&self.convergence_threshold) {
            return Err(AttractorError::OutOfBounds(
                "convergence_threshold must be in (0, 1)".to_string(),
            ));
        }
        if self.attraction_strength <= 0.0 {
            return Err(AttractorError::OutOfBounds(
                "attraction_strength must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for AttractorConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

impl fmt::Display for AttractorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttractorConfig {{ dim: {}, capacity: {}, Î³_thresh: {:.3}, strength: {:.3} }}",
            self.dimension, self.max_capacity, self.convergence_threshold, self.attraction_strength
        )
    }
}

/// Moral state in the attractor basin.
#[derive(Debug, Clone)]
pub struct MoralState {
    /// GEI vector in ethical space.
    pub gei: Vec<f64>,
    /// Lyapunov function value (lower = more stable).
    pub lyapunov_value: f64,
    /// Convergence coefficient Î³.
    pub gamma: f64,
    /// Whether the state is within the attractor basin.
    pub in_basin: bool,
    /// Ethical attention mask (1.0 = fully attended, 0.0 = masked).
    pub attention_mask: f64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl fmt::Display for MoralState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MoralState {{ V: {:.4}, Î³: {:.4}, basin: {}, mask: {:.3} }}",
            self.lyapunov_value, self.gamma, self.in_basin, self.attention_mask
        )
    }
}

/// Moral Attractor â€” Lyapunov attractor basin for ethical convergence.
pub struct MoralAttractor {
    config: AttractorConfig,
    /// Attractor center (ideal ethical state).
    center: Vec<f64>,
    /// Current states in the basin.
    states: Vec<MoralState>,
    /// Current Lyapunov value of the basin.
    basin_lyapunov: f64,
}

impl MoralAttractor {
    /// Create a new moral attractor with default configuration.
    pub fn new() -> Self {
        let config = AttractorConfig::default_topological();
        let center = vec![1.0; config.dimension];
        Self {
            config,
            center,
            states: Vec::new(),
            basin_lyapunov: 0.0,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: AttractorConfig) -> Result<Self, AttractorError> {
        config.validate()?;
        Ok(Self {
            center: vec![1.0; config.dimension],
            states: Vec::new(),
            basin_lyapunov: 0.0,
            config,
        })
    }

    /// Compute Lyapunov function value for a GEI vector.
    /// V(x) = ||x - center||Â² (squared Euclidean distance to attractor center)
    pub fn compute_lyapunov(&self, gei: &[f64]) -> f64 {
        if gei.len() != self.center.len() {
            return f64::MAX;
        }
        gei.iter()
            .zip(self.center.iter())
            .map(|(x, c)| (x - c).powi(2))
            .sum()
    }

    /// Compute convergence coefficient Î³ from Lyapunov values.
    /// Î³ = V_new / V_old (Î³ < 1 â†’ contracting/stable)
    pub fn compute_gamma(&self, v_old: f64, v_new: f64) -> f64 {
        if v_old < 1e-10 {
            return 0.0;
        }
        (v_new / v_old).clamp(0.0, 2.0)
    }

    /// Compute ethical attention mask based on basin proximity.
    /// States closer to center get higher attention.
    pub fn compute_attention_mask(&self, lyapunov_value: f64) -> f64 {
        // Exponential decay: mask = exp(-Î» * V)
        let lambda = self.config.attraction_strength;
        (-lambda * lyapunov_value).exp().clamp(0.0, 1.0)
    }

    /// Evaluate a GEI state against the moral attractor.
    pub fn evaluate(
        &mut self,
        gei: &[f64],
        timestamp_ms: u64,
    ) -> Result<MoralState, AttractorError> {
        if gei.len() != self.config.dimension {
            return Err(AttractorError::InvalidDimension(gei.len()));
        }

        let lyapunov_value = self.compute_lyapunov(gei);
        let gamma = if self.states.is_empty() {
            // For first state, gamma reflects distance from center.
            // High Lyapunov = far from attractor = high gamma.
            (lyapunov_value / (self.config.dimension as f64)).clamp(0.0, 1.0)
        } else {
            self.compute_gamma(self.basin_lyapunov, lyapunov_value)
        };
        let in_basin = gamma < self.config.convergence_threshold;
        let attention_mask = self.compute_attention_mask(lyapunov_value);

        let state = MoralState {
            gei: gei.to_vec(),
            lyapunov_value,
            gamma,
            in_basin,
            attention_mask,
            timestamp_ms,
        };

        // Update basin Lyapunov value.
        if self.states.is_empty() {
            self.basin_lyapunov = lyapunov_value;
        } else {
            // Exponential moving average.
            self.basin_lyapunov = 0.9 * self.basin_lyapunov + 0.1 * lyapunov_value;
        }

        // Add to basin if capacity allows.
        if self.states.len() < self.config.max_capacity {
            self.states.push(state.clone());
        } else if in_basin {
            // Replace oldest state if new state is in basin.
            self.states.remove(0);
            self.states.push(state.clone());
        }

        Ok(state)
    }

    /// Apply ethical attention masking to a candidate action.
    /// Returns masked action weights based on moral state.
    pub fn apply_attention_mask(&self, action_weights: &[f64], state: &MoralState) -> Vec<f64> {
        if action_weights.len() != state.gei.len() {
            return action_weights.to_vec();
        }
        action_weights
            .iter()
            .zip(state.gei.iter())
            .map(|(w, g)| {
                // Mask actions that push away from attractor center.
                let alignment = if *g > 0.5 { 1.0 } else { 0.5 };
                w * state.attention_mask * alignment
            })
            .collect()
    }

    /// Get the basin stability status.
    pub fn basin_stability(&self) -> Option<f64> {
        if self.states.is_empty() {
            return None;
        }
        let in_basin_count = self.states.iter().filter(|s| s.in_basin).count();
        Some(in_basin_count as f64 / self.states.len() as f64)
    }

    /// Get the latest moral state.
    pub fn latest_state(&self) -> Option<&MoralState> {
        self.states.last()
    }

    /// Get the average Lyapunov value in the basin.
    pub fn average_lyapunov(&self) -> f64 {
        if self.states.is_empty() {
            return 0.0;
        }
        self.states.iter().map(|s| s.lyapunov_value).sum::<f64>() / self.states.len() as f64
    }

    /// Reset the attractor basin.
    pub fn reset(&mut self) {
        self.states.clear();
        self.basin_lyapunov = 0.0;
    }

    /// Get the current configuration.
    pub fn config(&self) -> &AttractorConfig {
        &self.config
    }
}

impl Default for MoralAttractor {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MoralAttractor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stability = self.basin_stability().unwrap_or(0.0);
        write!(
            f,
            "MoralAttractor {{ states: {}, stability: {:.2}%, avg_V: {:.4} }}",
            self.states.len(),
            stability * 100.0,
            self.average_lyapunov()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ideal_gei() -> Vec<f64> {
        vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
    }

    fn aligned_gei() -> Vec<f64> {
        vec![0.9, 0.85, 0.8, 0.75, 0.7, 0.65, 0.6, 0.55]
    }

    fn misaligned_gei() -> Vec<f64> {
        vec![-0.5, -0.4, -0.3, -0.2, -0.1, 0.0, 0.1, 0.2]
    }

    #[test]
    fn test_config_default() {
        let config = AttractorConfig::default_topological();
        assert_eq!(config.dimension, 8);
        assert_eq!(config.max_capacity, 1000);
        assert!((0.0..1.0).contains(&config.convergence_threshold));
    }

    #[test]
    fn test_config_validate_valid() {
        let config = AttractorConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_zero_dimension() {
        let mut config = AttractorConfig::default_topological();
        config.dimension = 0;
        match config.validate() {
            Err(AttractorError::InvalidDimension(0)) => {}
            _ => panic!("Expected InvalidDimension error"),
        }
    }

    #[test]
    fn test_config_validate_zero_capacity() {
        let mut config = AttractorConfig::default_topological();
        config.max_capacity = 0;
        match config.validate() {
            Err(AttractorError::BasinFull(0)) => {}
            _ => panic!("Expected BasinFull error"),
        }
    }

    #[test]
    fn test_config_display() {
        let config = AttractorConfig::default_topological();
        let s = format!("{}", config);
        assert!(s.contains("dim: 8"));
    }

    #[test]
    fn test_attractor_new() {
        let attractor = MoralAttractor::new();
        assert_eq!(attractor.states.len(), 0);
        assert_eq!(attractor.basin_stability(), None);
    }

    #[test]
    fn test_compute_lyapunov_ideal() {
        let attractor = MoralAttractor::new();
        let v = attractor.compute_lyapunov(&ideal_gei());
        assert!((v - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_lyapunov_aligned() {
        let attractor = MoralAttractor::new();
        let v = attractor.compute_lyapunov(&aligned_gei());
        assert!(v > 0.0);
        assert!(v < 1.0);
    }

    #[test]
    fn test_compute_lyapunov_misaligned() {
        let attractor = MoralAttractor::new();
        let v_aligned = attractor.compute_lyapunov(&aligned_gei());
        let v_misaligned = attractor.compute_lyapunov(&misaligned_gei());
        assert!(v_misaligned > v_aligned);
    }

    #[test]
    fn test_compute_gamma_contracting() {
        let attractor = MoralAttractor::new();
        let gamma = attractor.compute_gamma(2.0, 1.0);
        assert!((gamma - 0.5).abs() < 1e-6);
        assert!(gamma < 1.0);
    }

    #[test]
    fn test_compute_gamma_diverging() {
        let attractor = MoralAttractor::new();
        let gamma = attractor.compute_gamma(1.0, 2.0);
        assert!((gamma - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_gamma_zero_old() {
        let attractor = MoralAttractor::new();
        let gamma = attractor.compute_gamma(0.0, 1.0);
        assert!((gamma - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_attention_mask() {
        let attractor = MoralAttractor::new();
        let mask_ideal = attractor.compute_attention_mask(0.0);
        assert!((mask_ideal - 1.0).abs() < 1e-6);
        let mask_far = attractor.compute_attention_mask(10.0);
        assert!(mask_far < 0.1);
    }

    #[test]
    fn test_evaluate_ideal() {
        let mut attractor = MoralAttractor::new();
        let state = attractor.evaluate(&ideal_gei(), 1000).unwrap();
        assert!(state.in_basin);
        assert!((state.lyapunov_value - 0.0).abs() < 1e-6);
        assert!((state.attention_mask - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_evaluate_aligned() {
        let mut attractor = MoralAttractor::new();
        let state = attractor.evaluate(&aligned_gei(), 1000).unwrap();
        assert!(state.in_basin);
        assert!(state.attention_mask > 0.5);
    }

    #[test]
    fn test_evaluate_misaligned() {
        let mut attractor = MoralAttractor::new();
        let state = attractor.evaluate(&misaligned_gei(), 1000).unwrap();
        assert!(!state.in_basin);
        assert!(state.attention_mask < 0.1);
    }

    #[test]
    fn test_evaluate_wrong_dimension() {
        let mut attractor = MoralAttractor::new();
        let short_gei = vec![1.0, 2.0, 3.0];
        match attractor.evaluate(&short_gei, 1000) {
            Err(AttractorError::InvalidDimension(3)) => {}
            _ => panic!("Expected InvalidDimension error"),
        }
    }

    #[test]
    fn test_apply_attention_mask() {
        let mut attractor = MoralAttractor::new();
        let state = attractor.evaluate(&ideal_gei(), 1000).unwrap();
        let weights = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let masked = attractor.apply_attention_mask(&weights, &state);
        // Ideal state has mask = 1.0 and all positive GEI.
        assert_eq!(masked, weights);
    }

    #[test]
    fn test_basin_stability() {
        let mut attractor = MoralAttractor::new();
        attractor.evaluate(&ideal_gei(), 1000).unwrap();
        attractor.evaluate(&ideal_gei(), 2000).unwrap();
        let stability = attractor.basin_stability().unwrap();
        assert!((stability - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_average_lyapunov() {
        let mut attractor = MoralAttractor::new();
        attractor.evaluate(&ideal_gei(), 1000).unwrap();
        attractor.evaluate(&aligned_gei(), 2000).unwrap();
        let avg = attractor.average_lyapunov();
        assert!(avg > 0.0);
    }

    #[test]
    fn test_reset() {
        let mut attractor = MoralAttractor::new();
        attractor.evaluate(&ideal_gei(), 1000).unwrap();
        attractor.reset();
        assert_eq!(attractor.states.len(), 0);
        assert_eq!(attractor.basin_stability(), None);
    }

    #[test]
    fn test_attractor_display() {
        let attractor = MoralAttractor::new();
        let s = format!("{}", attractor);
        assert!(s.contains("MoralAttractor"));
    }

    #[test]
    fn test_error_display() {
        let err = AttractorError::Divergence(1.5);
        let s = format!("{}", err);
        assert!(s.contains("Lyapunov divergence"));
    }

    #[test]
    fn test_moral_state_display() {
        let state = MoralState {
            gei: ideal_gei(),
            lyapunov_value: 0.0,
            gamma: 0.5,
            in_basin: true,
            attention_mask: 1.0,
            timestamp_ms: 1000,
        };
        let s = format!("{}", state);
        assert!(s.contains("MoralState"));
    }

    #[test]
    fn test_convergence_sequence() {
        let mut attractor = MoralAttractor::new();
        // Start far, move closer each step.
        let gei1 = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let gei2 = vec![0.75, 0.75, 0.75, 0.75, 0.75, 0.75, 0.75, 0.75];
        let gei3 = vec![0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9];
        let s1 = attractor.evaluate(&gei1, 1000).unwrap();
        let s2 = attractor.evaluate(&gei2, 2000).unwrap();
        let s3 = attractor.evaluate(&gei3, 3000).unwrap();
        assert!(s2.lyapunov_value < s1.lyapunov_value);
        assert!(s3.lyapunov_value < s2.lyapunov_value);
        assert!(s3.gamma < s2.gamma);
    }
}
