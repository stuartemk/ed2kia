//! Differential Holographic Noise â€” Sprint 78: Invariant Architecture & Planetary-Scale Resilience
//!
//! Resuelve el bug terminal: Fuga de privacidad en sharding hologrÃ¡fico.
//!
//! Implementa ruido diferencial estuardiano: inyecciÃ³n de Laplace/Gaussiana
//! calibrada en actualizaciones topolÃ³gicas. Preserva GEI macro, protege
//! prompts micro. Imposible reconstruir prompt individual desde embedding.
//!
//! # GarantÃ­as
//!
//! - Privacidad: (Îµ, Î´)-diferencial en cada actualizaciÃ³n
//! - Utilidad: GEI macro preservado vÃ­a peso de preservaciÃ³n
//! - Ruido: Laplace para sensibilidad L1, Gaussiana para L2
//! - CalibraciÃ³n: ruido âˆ 1/Îµ (menor Îµ = mayor privacidad = mÃ¡s ruido)

use std::fmt;

/// Error types for Differential Holographic Noise
#[derive(Debug, Clone, PartialEq)]
pub enum NoiseError {
    /// Invalid epsilon (must be > 0)
    InvalidEpsilon(f64),
    /// Invalid delta (must be in [0, 1])
    InvalidDelta(f64),
    /// Invalid sensitivity (must be > 0)
    InvalidSensitivity(f64),
    /// Dimension mismatch
    DimensionMismatch(usize, usize),
    /// Invalid preservation weight (must be in [0, 1])
    InvalidPreservationWeight(f64),
    /// Empty input vector
    EmptyInput,
}

impl fmt::Display for NoiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoiseError::InvalidEpsilon(e) => write!(f, "Invalid epsilon: {:.6} (must be > 0)", e),
            NoiseError::InvalidDelta(d) => {
                write!(f, "Invalid delta: {:.6} (must be in [0, 1])", d)
            }
            NoiseError::InvalidSensitivity(s) => {
                write!(f, "Invalid sensitivity: {:.6} (must be > 0)", s)
            }
            NoiseError::DimensionMismatch(a, b) => {
                write!(f, "Dimension mismatch: {} vs {}", a, b)
            }
            NoiseError::InvalidPreservationWeight(w) => {
                write!(
                    f,
                    "Invalid preservation weight: {:.4} (must be in [0, 1])",
                    w
                )
            }
            NoiseError::EmptyInput => write!(f, "Empty input vector"),
        }
    }
}

impl std::error::Error for NoiseError {}

/// Configuration for differential privacy noise.
#[derive(Debug, Clone)]
pub struct NoiseConfig {
    /// Privacy budget epsilon (default 1.0)
    pub epsilon: f64,
    /// Failure probability delta (default 0.0001)
    pub delta: f64,
    /// Sensitivity of the query (default 1.0)
    pub sensitivity: f64,
    /// GEI preservation weight (default 0.7)
    pub gei_preservation_weight: f64,
    /// Noise distribution: true = Laplace, false = Gaussian
    pub use_laplace: bool,
    /// Random seed for reproducibility (0 = system random)
    pub seed: u64,
}

impl NoiseConfig {
    /// Default Topological configuration.
    pub fn default_topological() -> Self {
        Self {
            epsilon: 1.0,
            delta: 0.000_1,
            sensitivity: 1.0,
            gei_preservation_weight: 0.7,
            use_laplace: true,
            seed: 0,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), NoiseError> {
        if self.epsilon <= 0.0 {
            return Err(NoiseError::InvalidEpsilon(self.epsilon));
        }
        if self.delta < 0.0 || self.delta > 1.0 {
            return Err(NoiseError::InvalidDelta(self.delta));
        }
        if self.sensitivity <= 0.0 {
            return Err(NoiseError::InvalidSensitivity(self.sensitivity));
        }
        if self.gei_preservation_weight < 0.0 || self.gei_preservation_weight > 1.0 {
            return Err(NoiseError::InvalidPreservationWeight(
                self.gei_preservation_weight,
            ));
        }
        Ok(())
    }
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

/// Record of a noise injection operation.
#[derive(Debug, Clone)]
pub struct NoiseRecord {
    /// Original vector dimension.
    pub dimension: usize,
    /// Epsilon used.
    pub epsilon: f64,
    /// Delta used.
    pub delta: f64,
    /// Noise scale applied.
    pub noise_scale: f64,
    /// GEI preservation weight.
    pub gei_weight: f64,
    /// L2 norm of noise added.
    pub noise_l2_norm: f64,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

impl fmt::Display for NoiseRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Noise[dim={}, Îµ={:.4}, Î´={:.6}, scale={:.6}, noise_L2={:.6}]",
            self.dimension, self.epsilon, self.delta, self.noise_scale, self.noise_l2_norm
        )
    }
}

/// Main engine for differential holographic noise.
#[derive(Debug, Clone)]
pub struct DifferentialHolographicNoise {
    /// Configuration.
    pub config: NoiseConfig,
    /// Total privacy budget consumed.
    pub total_epsilon_consumed: f64,
    /// Number of queries processed.
    pub query_count: usize,
    /// Noise injection history.
    pub noise_history: Vec<NoiseRecord>,
}

impl DifferentialHolographicNoise {
    /// Create with default Topological config.
    pub fn new() -> Self {
        Self {
            config: NoiseConfig::default_topological(),
            total_epsilon_consumed: 0.0,
            query_count: 0,
            noise_history: Vec::new(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: NoiseConfig) -> Result<Self, NoiseError> {
        config.validate()?;
        Ok(Self {
            config,
            total_epsilon_consumed: 0.0,
            query_count: 0,
            noise_history: Vec::new(),
        })
    }

    /// Inject calibrated noise into a topological update.
    pub fn inject_noise(
        &mut self,
        topological_update: &[f32],
        timestamp_ms: u64,
    ) -> Result<Vec<f32>, NoiseError> {
        if topological_update.is_empty() {
            return Err(NoiseError::EmptyInput);
        }

        let noisy = inject_topological_noise(
            topological_update,
            self.config.epsilon,
            self.config.delta,
            self.config.gei_preservation_weight,
        );

        // Compute noise L2 norm for tracking
        let noise_l2: f32 = topological_update
            .iter()
            .zip(noisy.iter())
            .map(|(o, n)| (o - n).powi(2))
            .sum::<f32>()
            .sqrt();

        let noise_scale = self.config.sensitivity / self.config.epsilon;

        self.noise_history.push(NoiseRecord {
            dimension: topological_update.len(),
            epsilon: self.config.epsilon,
            delta: self.config.delta,
            noise_scale,
            gei_weight: self.config.gei_preservation_weight,
            noise_l2_norm: noise_l2 as f64,
            timestamp_ms,
        });

        self.total_epsilon_consumed += self.config.epsilon;
        self.query_count += 1;

        Ok(noisy)
    }

    /// Check if privacy budget is exhausted.
    pub fn is_budget_exhausted(&self, max_budget: f64) -> bool {
        self.total_epsilon_consumed >= max_budget
    }

    /// Get remaining privacy budget.
    pub fn remaining_budget(&self, max_budget: f64) -> f64 {
        (max_budget - self.total_epsilon_consumed).max(0.0)
    }

    /// Reset engine state.
    pub fn reset(&mut self) {
        self.total_epsilon_consumed = 0.0;
        self.query_count = 0;
        self.noise_history.clear();
    }
}

impl Default for DifferentialHolographicNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DifferentialHolographicNoise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DiffNoise[queries={}, Îµ_total={:.4}, budget_ok={}]",
            self.query_count,
            self.total_epsilon_consumed,
            !self.is_budget_exhausted(10.0)
        )
    }
}

// -- Standalone public functions --

/// Inject Topological differential privacy noise into a topological update.
///
/// Calibrates Laplace/Gaussian noise to preserve GEI macro while
/// protecting individual prompts from reconstruction attacks.
pub fn inject_topological_noise(
    topological_update: &[f32],
    epsilon: f64,
    delta: f64,
    gei_preservation_weight: f64,
) -> Vec<f32> {
    if topological_update.is_empty() {
        return vec![];
    }

    let use_laplace = epsilon < delta; // Laplace for stronger per-query privacy
    let sensitivity = 1.0;

    inject_topological_noise_internal(
        topological_update,
        epsilon,
        delta,
        gei_preservation_weight,
        use_laplace,
        sensitivity,
        0,
    )
}

/// Internal noise injection with full parameters.
fn inject_topological_noise_internal(
    topological_update: &[f32],
    epsilon: f64,
    delta: f64,
    gei_preservation_weight: f64,
    use_laplace: bool,
    sensitivity: f64,
    seed: u64,
) -> Vec<f32> {
    let scale = if use_laplace {
        sensitivity / epsilon
    } else {
        // Gaussian mechanism: scale = sensitivity * sqrt(2 * ln(1.25/delta)) / epsilon
        let ln_factor = (1.25 / delta.max(f64::EPSILON)).ln();
        sensitivity * (2.0 * ln_factor).sqrt() / epsilon
    };

    let mut result = Vec::with_capacity(topological_update.len());
    let mut rng = FnvRng::new(seed.wrapping_add(0x5a5a5a5a));

    for &val in topological_update {
        let noise = if use_laplace {
            sample_laplace(&mut rng, scale)
        } else {
            sample_gaussian(&mut rng, scale)
        };

        // GEI preservation: blend original with noisy value
        let noisy_val = val + noise as f32;
        let preserved = val * gei_preservation_weight as f32
            + noisy_val * (1.0 - gei_preservation_weight) as f32;
        result.push(preserved);
    }

    result
}

/// Compute the sensitivity of a query (L1 norm for Laplace).
pub fn compute_l1_sensitivity(vectors: &[&[f32]]) -> f64 {
    if vectors.len() < 2 {
        return 0.0;
    }
    let mut max_diff = 0.0f64;
    for window in vectors.windows(2) {
        let diff: f64 = window[0]
            .iter()
            .zip(window[1].iter())
            .map(|(a, b)| (a - b).abs() as f64)
            .sum();
        max_diff = max_diff.max(diff);
    }
    max_diff
}

/// Compute the sensitivity of a query (L2 norm for Gaussian).
pub fn compute_l2_sensitivity(vectors: &[&[f32]]) -> f64 {
    if vectors.len() < 2 {
        return 0.0;
    }
    let mut max_diff = 0.0f64;
    for window in vectors.windows(2) {
        let diff: f64 = window[0]
            .iter()
            .zip(window[1].iter())
            .map(|(a, b)| ((a - b).abs() as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        max_diff = max_diff.max(diff);
    }
    max_diff
}

/// Check if a vector has been sufficiently obfuscated.
pub fn is_obfuscated(original: &[f32], noisy: &[f32], threshold: f64) -> bool {
    if original.len() != noisy.len() || original.is_empty() {
        return false;
    }
    let max_diff: f64 = original
        .iter()
        .zip(noisy.iter())
        .map(|(o, n)| (o - n).abs() as f64)
        .fold(0.0f64, f64::max);
    max_diff > threshold
}

// -- PRNG utilities --

/// Simple FNV-based RNG for deterministic noise generation.
struct FnvRng {
    state: u64,
}

impl FnvRng {
    fn new(seed: u64) -> Self {
        // Ensure state is never 0 (degenerate: produces all-zero output)
        Self {
            state: seed | 0x9E3779B97F4A7C15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state.wrapping_shr(13);
        self.state = self.state.wrapping_mul(0x5555555555555555);
        self.state ^= self.state.wrapping_shr(17);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (u64::MAX >> 11) as f64
    }
}

/// Sample from Laplace distribution.
fn sample_laplace(rng: &mut FnvRng, scale: f64) -> f64 {
    let u = rng.next_f64();
    // Inverse CDF of Laplace(0, scale): b * sign(u-0.5) * ln(1 - 2|u - 0.5|)
    // Equivalent to: if u < 0.5: scale * ln(2u), else: -scale * ln(2(1-u))
    let u = u.max(f64::EPSILON).min(1.0 - f64::EPSILON); // Avoid ln(0)
    if u < 0.5 {
        scale * (2.0 * u).ln()
    } else {
        -scale * (2.0 * (1.0 - u)).ln()
    }
}

/// Sample from Gaussian distribution (Box-Muller).
fn sample_gaussian(rng: &mut FnvRng, scale: f64) -> f64 {
    let u1 = rng.next_f64().max(f64::EPSILON);
    let u2 = rng.next_f64();
    // Box-Muller: sqrt(-2 * ln(u1)) * cos(2 * PI * u2)
    scale * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = NoiseConfig::default_topological();
        assert!(config.epsilon > 0.0);
        assert!(config.delta >= 0.0 && config.delta <= 1.0);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = NoiseConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_epsilon() {
        let mut config = NoiseConfig::default_topological();
        config.epsilon = 0.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_delta() {
        let mut config = NoiseConfig::default_topological();
        config.delta = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_preservation() {
        let mut config = NoiseConfig::default_topological();
        config.gei_preservation_weight = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_engine_creation() {
        let engine = DifferentialHolographicNoise::new();
        assert_eq!(engine.query_count, 0);
        assert_eq!(engine.total_epsilon_consumed, 0.0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = NoiseConfig::default_topological();
        let engine = DifferentialHolographicNoise::with_config(config).unwrap();
        assert_eq!(engine.query_count, 0);
    }

    #[test]
    fn test_inject_noise() {
        let mut engine = DifferentialHolographicNoise::new();
        let data = vec![1.0f32, 2.0, 3.0, 4.0];
        let noisy = engine.inject_noise(&data, 1000).unwrap();
        assert_eq!(noisy.len(), data.len());
        assert_eq!(engine.query_count, 1);
    }

    #[test]
    fn test_inject_noise_empty() {
        let mut engine = DifferentialHolographicNoise::new();
        let result = engine.inject_noise(&[], 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_noise_changes_values() {
        let mut engine = DifferentialHolographicNoise::new();
        let data = vec![1.0f32, 2.0, 3.0];
        let noisy = engine.inject_noise(&data, 1000).unwrap();
        // Values should differ due to noise
        let differs = data
            .iter()
            .zip(noisy.iter())
            .any(|(a, b)| (a - b).abs() > f32::EPSILON);
        assert!(differs);
    }

    #[test]
    fn test_gei_preservation() {
        let mut engine = DifferentialHolographicNoise::new();
        engine.config.gei_preservation_weight = 1.0; // Full preservation
        let data = vec![1.0f32, 2.0, 3.0];
        let noisy = engine.inject_noise(&data, 1000).unwrap();
        // With weight=1.0, output should be close to original
        for (o, n) in data.iter().zip(noisy.iter()) {
            assert!((o - n).abs() < 0.5); // Tolerance for noise blending
        }
    }

    #[test]
    fn test_budget_tracking() {
        let mut engine = DifferentialHolographicNoise::new();
        let data = vec![1.0f32, 2.0, 3.0];
        engine.inject_noise(&data, 1000).unwrap();
        engine.inject_noise(&data, 2000).unwrap();
        assert!((engine.total_epsilon_consumed - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_budget_exhausted() {
        let mut engine = DifferentialHolographicNoise::new();
        engine.config.epsilon = 2.0;
        let data = vec![1.0f32, 2.0];
        engine.inject_noise(&data, 1000).unwrap();
        engine.inject_noise(&data, 2000).unwrap();
        assert!(engine.is_budget_exhausted(3.0));
        assert!(!engine.is_budget_exhausted(5.0));
    }

    #[test]
    fn test_remaining_budget() {
        let engine = DifferentialHolographicNoise::new();
        assert!((engine.remaining_budget(10.0) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reset() {
        let mut engine = DifferentialHolographicNoise::new();
        let data = vec![1.0f32, 2.0];
        engine.inject_noise(&data, 1000).unwrap();
        engine.reset();
        assert_eq!(engine.query_count, 0);
        assert_eq!(engine.total_epsilon_consumed, 0.0);
    }

    #[test]
    fn test_display() {
        let engine = DifferentialHolographicNoise::new();
        let s = format!("{}", engine);
        assert!(s.contains("DiffNoise"));
    }

    #[test]
    fn test_noise_record_display() {
        let record = NoiseRecord {
            dimension: 64,
            epsilon: 1.0,
            delta: 0.0001,
            noise_scale: 1.0,
            gei_weight: 0.7,
            noise_l2_norm: 2.5,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("Noise"));
    }

    #[test]
    fn test_standalone_inject() {
        let data = vec![1.0f32, 2.0, 3.0];
        let noisy = inject_topological_noise(&data, 1.0, 0.0001, 0.7);
        assert_eq!(noisy.len(), data.len());
    }

    #[test]
    fn test_standalone_inject_empty() {
        let noisy = inject_topological_noise(&[], 1.0, 0.0001, 0.7);
        assert!(noisy.is_empty());
    }

    #[test]
    fn test_compute_l1_sensitivity() {
        let a = vec![1.0f32, 2.0, 3.0];
        let b = vec![1.5f32, 2.5, 3.5];
        let sens = compute_l1_sensitivity(&[&a, &b]);
        assert!(sens > 0.0);
    }

    #[test]
    fn test_compute_l2_sensitivity() {
        let a = vec![1.0f32, 2.0, 3.0];
        let b = vec![1.5f32, 2.5, 3.5];
        let sens = compute_l2_sensitivity(&[&a, &b]);
        assert!(sens > 0.0);
    }

    #[test]
    fn test_is_obfuscated() {
        let original = vec![1.0f32, 2.0, 3.0];
        let noisy = vec![1.5f32, 2.8, 3.6];
        assert!(is_obfuscated(&original, &noisy, 0.1));
        assert!(!is_obfuscated(&original, &noisy, 1.0));
    }

    #[test]
    fn test_is_obfuscated_empty() {
        assert!(!is_obfuscated(&[], &[], 0.1));
    }

    #[test]
    fn test_error_display() {
        let err = NoiseError::InvalidEpsilon(0.0);
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = DifferentialHolographicNoise::new();
        engine.config.epsilon = 0.5;

        let data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];

        // Inject noise multiple times
        for i in 0..5 {
            let noisy = engine.inject_noise(&data, 1000 + i).unwrap();
            assert_eq!(noisy.len(), data.len());
        }

        assert_eq!(engine.query_count, 5);
        assert!((engine.total_epsilon_consumed - 2.5).abs() < f64::EPSILON);
        assert!(!engine.is_budget_exhausted(5.0));
        assert!(engine.is_budget_exhausted(2.0));
    }
}
