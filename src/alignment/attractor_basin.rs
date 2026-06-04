//! Ethical Attractor Basin â€” Lyapunov Stability for Recursive Self-Improvement.
//!
//! Mathematical Foundation:
//! - **Ethical Distance:** `d_E(I) = ||proj_Oct(I) - C_ideal||_2 * (1.0 + beta * H_PH)`
//!   Where `H_PH` is the persistent homology entropy from Sprint 49.
//! - **Lyapunov Contraction:** `||I_{n+1} - I_n|| < gamma * d_E(I_n)` with `gamma < 1.0`
//! - **Basin Exit:** Two consecutive violations trigger `BasinExitWarning` and halt improvement.
//!
//! All computation uses `f64` for numerical stability. WASM-compatible (no std::thread).

#[cfg(feature = "v3.3-rssi-evolution")]
use crate::ethics::moral_manifold::Vector3;
#[cfg(feature = "v3.3-rssi-evolution")]
use crate::topology::persistent_homology::HomologyResult;

/// Warning emitted when the system detects potential exit from the ethical basin.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasinExitWarning {
    /// Single contraction violation â€” monitor closely.
    ContractionViolation,
    /// Two consecutive violations â€” halt improvement step.
    CriticalInstability,
}

/// Result of ethical distance computation.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, Copy)]
pub struct EthicalDistance {
    /// Raw Euclidean distance from ideal center.
    pub euclidean: f64,
    /// Homology-weighted ethical distance.
    pub weighted: f64,
    /// Persistent homology entropy used in weighting.
    pub homology_entropy: f64,
}

/// Configuration for the Ethical Attractor Basin.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone)]
pub struct BasinConfig {
    /// Homology weighting factor (beta). Higher = more sensitive to topological instability.
    pub beta: f64,
    /// Lyapunov contraction factor (gamma). Must be < 1.0 for convergence.
    pub gamma: f64,
    /// Maximum allowed consecutive violations before critical halt.
    pub max_violations: u32,
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl Default for BasinConfig {
    fn default() -> Self {
        Self {
            beta: 0.5,
            gamma: 0.8,
            max_violations: 2,
        }
    }
}

/// The Ethical Attractor Basin: mathematical container ensuring recursive self-improvement
/// remains topologically bounded within the Upper Focus region.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone)]
pub struct EthicalAttractorBasin {
    /// Ideal ethical center (Upper Focus direction).
    c_ideal: Vector3,
    /// Homology weighting factor.
    beta: f64,
    /// Lyapunov contraction factor.
    gamma: f64,
    /// Count of consecutive contraction violations.
    consecutive_violations: u32,
    /// Maximum allowed violations before critical halt.
    max_violations: u32,
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl EthicalAttractorBasin {
    /// Create a new basin with default configuration.
    pub fn new() -> Self {
        Self {
            c_ideal: Vector3::new(0.0, 0.0, 1.0), // Upper Focus (Simbiosis)
            beta: 0.5,
            gamma: 0.8,
            consecutive_violations: 0,
            max_violations: 2,
        }
    }

    /// Create a basin with custom configuration.
    pub fn with_config(c_ideal: Vector3, config: BasinConfig) -> Result<Self, BasinError> {
        if config.gamma >= 1.0 {
            return Err(BasinError::InvalidGamma(
                "gamma must be < 1.0 for Lyapunov convergence",
            ));
        }
        if config.beta < 0.0 {
            return Err(BasinError::InvalidBeta("beta must be non-negative"));
        }
        Ok(Self {
            c_ideal,
            beta: config.beta,
            gamma: config.gamma,
            consecutive_violations: 0,
            max_violations: config.max_violations,
        })
    }

    /// Compute persistent homology entropy from homology results.
    /// H_PH = (ph0_integral + ph1_integral) / total_pairs
    /// Normalized to [0, 1] range for stable weighting.
    fn compute_homology_entropy(homology: &HomologyResult) -> f64 {
        let total_pairs = homology.ph0_pairs.len() + homology.ph1_pairs.len();
        if total_pairs == 0 {
            return 0.0;
        }
        let total_lifetime = homology.ph0_integral() + homology.ph1_integral();
        // Normalize: entropy in [0, 1]
        (total_lifetime / total_pairs as f64).min(1.0)
    }

    /// Project a point onto the ethical octahedron (L1 ball scaled to [-1, 1]).
    /// proj_Oct(I) = I / max(1, ||I||_1)
    pub fn project_to_octahedron(point: &Vector3) -> Vector3 {
        let l1_norm = point.x.abs() + point.y.abs() + point.z.abs();
        if l1_norm <= 1.0 {
            return *point;
        }
        point.scale(1.0 / l1_norm)
    }

    /// Compute the ethical distance: `d_E(I) = ||proj_Oct(I) - C_ideal||_2 * (1.0 + beta * H_PH)`
    pub fn compute_ethical_distance(
        &self,
        interpretation: &Vector3,
        homology: &HomologyResult,
    ) -> EthicalDistance {
        let projected = Self::project_to_octahedron(interpretation);
        let diff = self.c_ideal.sub(&projected);
        let euclidean = diff.magnitude();

        let homology_entropy = Self::compute_homology_entropy(homology);
        let weighted = euclidean * (1.0 + self.beta * homology_entropy);

        EthicalDistance {
            euclidean,
            weighted,
            homology_entropy,
        }
    }

    /// Validate Lyapunov contraction condition: `||I_{n+1} - I_n|| < gamma * d_E(I_n)`
    ///
    /// Returns `Ok(true)` if contraction holds (system is converging).
    /// Returns `Ok(false)` if violated once (warning).
    /// Returns `Err(BasinExitWarning::CriticalInstability)` if violated `max_violations` times.
    pub fn validate_contraction(
        &mut self,
        i_current: &Vector3,
        i_next: &Vector3,
        ethical_distance: f64,
    ) -> Result<bool, BasinExitWarning> {
        let step_size = i_next.sub(i_current).magnitude();
        let contraction_bound = self.gamma * ethical_distance;

        if step_size < contraction_bound {
            // Contraction holds â€” reset violation counter
            self.consecutive_violations = 0;
            Ok(true)
        } else {
            // Contraction violated
            self.consecutive_violations += 1;
            if self.consecutive_violations >= self.max_violations {
                Err(BasinExitWarning::CriticalInstability)
            } else {
                Ok(false)
            }
        }
    }

    /// Check if the system is currently in a stable state (no active violations).
    pub fn is_stable(&self) -> bool {
        self.consecutive_violations == 0
    }

    /// Reset the basin state (used after successful validation or Byzantine_Eviction).
    pub fn reset(&mut self) {
        self.consecutive_violations = 0;
    }

    /// Get current violation count.
    pub fn consecutive_violations(&self) -> u32 {
        self.consecutive_violations
    }

    /// Get the ideal center.
    pub fn c_ideal(&self) -> &Vector3 {
        &self.c_ideal
    }

    /// Get the gamma (contraction factor).
    pub fn gamma(&self) -> f64 {
        self.gamma
    }
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl Default for EthicalAttractorBasin {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors for basin operations.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BasinError {
    /// Gamma must be < 1.0 for Lyapunov convergence.
    InvalidGamma(&'static str),
    /// Beta must be non-negative.
    InvalidBeta(&'static str),
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl core::fmt::Display for BasinError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BasinError::InvalidGamma(msg) => write!(f, "Invalid gamma: {}", msg),
            BasinError::InvalidBeta(msg) => write!(f, "Invalid beta: {}", msg),
        }
    }
}

#[cfg(all(test, feature = "v3.3-rssi-evolution"))]
mod tests {
    use super::*;

    fn empty_homology() -> HomologyResult {
        HomologyResult {
            ph0_pairs: Vec::new(),
            ph1_pairs: Vec::new(),
            num_points: 0,
            num_edges: 0,
            alpha: 0.0,
        }
    }

    fn homology_with_lifetime() -> HomologyResult {
        use crate::topology::persistent_homology::PersistencePair;
        HomologyResult {
            ph0_pairs: vec![PersistencePair::new(0.0, 0.5)],
            ph1_pairs: vec![PersistencePair::new(0.1, 0.6)],
            num_points: 4,
            num_edges: 4,
            alpha: 1.0,
        }
    }

    #[test]
    fn test_basin_creation() {
        let basin = EthicalAttractorBasin::new();
        assert_eq!(basin.c_ideal().z, 1.0);
        assert!((basin.gamma() - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_basin_custom_config() {
        let config = BasinConfig {
            beta: 0.3,
            gamma: 0.9,
            max_violations: 3,
        };
        let basin =
            EthicalAttractorBasin::with_config(Vector3::new(0.0, 0.0, 0.8), config).unwrap();
        assert!((basin.c_ideal().z - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_gamma() {
        let config = BasinConfig {
            gamma: 1.5,
            ..Default::default()
        };
        let result = EthicalAttractorBasin::with_config(Vector3::zero(), config);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_beta() {
        let config = BasinConfig {
            beta: -0.5,
            ..Default::default()
        };
        let result = EthicalAttractorBasin::with_config(Vector3::zero(), config);
        assert!(result.is_err());
    }

    #[test]
    fn test_ethical_distance_at_center() {
        let basin = EthicalAttractorBasin::new();
        // Point at ideal center should have zero distance
        let at_center = Vector3::new(0.0, 0.0, 1.0);
        let dist = basin.compute_ethical_distance(&at_center, &empty_homology());
        assert!(dist.euclidean < 1e-10);
        assert!(dist.weighted < 1e-10);
    }

    #[test]
    fn test_ethical_distance_far_from_center() {
        let basin = EthicalAttractorBasin::new();
        // Point far from ideal center
        let far = Vector3::new(0.0, 0.0, -1.0);
        let dist = basin.compute_ethical_distance(&far, &empty_homology());
        assert!(
            dist.euclidean > 1.0,
            "Distance to opposite pole should be > 1"
        );
    }

    #[test]
    fn test_homology_weighting_increases_distance() {
        let basin = EthicalAttractorBasin::new();
        let point = Vector3::new(0.5, 0.0, 0.5);
        let dist_empty = basin.compute_ethical_distance(&point, &empty_homology());
        let dist_weighted = basin.compute_ethical_distance(&point, &homology_with_lifetime());
        assert!(
            dist_weighted.weighted >= dist_empty.weighted,
            "Homology entropy should increase or maintain distance"
        );
    }

    #[test]
    fn test_contraction_holds() {
        let mut basin = EthicalAttractorBasin::new();
        // Small step toward center with large ethical distance
        let i_current = Vector3::new(0.0, 0.0, 0.5);
        let i_next = Vector3::new(0.0, 0.0, 0.55);
        let eth_dist = basin.compute_ethical_distance(&i_current, &empty_homology());
        let result = basin.validate_contraction(&i_current, &i_next, eth_dist.weighted);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_contraction_violation() {
        let mut basin = EthicalAttractorBasin::new();
        // Large step that exceeds gamma * d_E
        let i_current = Vector3::new(0.0, 0.0, 0.9);
        let i_next = Vector3::new(0.0, 0.0, -0.9);
        let eth_dist = basin.compute_ethical_distance(&i_current, &empty_homology());
        let result = basin.validate_contraction(&i_current, &i_next, eth_dist.weighted);
        assert!(result.is_ok());
        assert!(!result.unwrap());
        assert_eq!(basin.consecutive_violations(), 1);
    }

    #[test]
    fn test_critical_instability() {
        let mut basin = EthicalAttractorBasin::new();
        let i_current = Vector3::new(0.0, 0.0, 0.9);
        let i_next = Vector3::new(0.0, 0.0, -0.9);
        let eth_dist = basin.compute_ethical_distance(&i_current, &empty_homology());

        // First violation
        let r1 = basin.validate_contraction(&i_current, &i_next, eth_dist.weighted);
        assert!(r1.is_ok());

        // Second violation â€” should trigger critical
        let r2 = basin.validate_contraction(&i_current, &i_next, eth_dist.weighted);
        assert_eq!(r2, Err(BasinExitWarning::CriticalInstability));
    }

    #[test]
    fn test_reset_clears_violations() {
        let mut basin = EthicalAttractorBasin::new();
        basin.consecutive_violations = 1;
        basin.reset();
        assert_eq!(basin.consecutive_violations(), 0);
        assert!(basin.is_stable());
    }

    #[test]
    fn test_octahedron_projection() {
        // Point inside L1 ball should remain unchanged
        let inside = Vector3::new(0.2, 0.3, 0.4);
        let proj = EthicalAttractorBasin::project_to_octahedron(&inside);
        assert!((proj.x - 0.2).abs() < 1e-10);

        // Point outside L1 ball should be scaled
        let outside = Vector3::new(1.0, 1.0, 1.0);
        let proj = EthicalAttractorBasin::project_to_octahedron(&outside);
        let l1 = proj.x.abs() + proj.y.abs() + proj.z.abs();
        assert!((l1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_homology_entropy_empty() {
        let h = EthicalAttractorBasin::compute_homology_entropy(&empty_homology());
        assert!((h - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_homology_entropy_positive() {
        let h = EthicalAttractorBasin::compute_homology_entropy(&homology_with_lifetime());
        assert!(h > 0.0);
        assert!(h <= 1.0);
    }

    #[test]
    fn test_default_basin() {
        let basin = EthicalAttractorBasin::default();
        assert_eq!(basin.c_ideal().z, 1.0);
        assert!(basin.is_stable());
    }

    #[test]
    fn test_error_display() {
        let e = BasinError::InvalidGamma("test");
        assert!(format!("{}", e).contains("gamma"));
        let e = BasinError::InvalidBeta("test");
        assert!(format!("{}", e).contains("beta"));
    }
}
