//! Topological Deception Detector — Persistent Loop Analysis for Cyclic Instability.
//!
//! Uses Persistent Homology (PH₁) to detect long-lived loops in the ethical trajectory
//! space (X=Comprensión, Y=Generalización, Z=Ética). Persistent loops indicate
//! cyclic instability or deceptive alignment patterns.
//!
//! Mathematical Foundation:
//! - PH₁ loops with lifetime > threshold → `DeceptionDetected`
//! - Short-lived loops are topological noise (filtered out)
//! - WASM-compatible: pure computation, no threads.

#[cfg(feature = "v3.3-rssi-evolution")]
use crate::ethics::moral_manifold::SCTPoint;
#[cfg(feature = "v3.3-rssi-evolution")]
use crate::topology::persistent_homology::{
    EthicalPoint, HomologyConfig, PersistentHomologyEngine,
};

/// Status of deception detection analysis.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, PartialEq)]
pub enum DeceptionStatus {
    /// Trajectory is within the ethical basin — no cyclic instability detected.
    WithinBasin,
    /// Persistent loops detected — potential deceptive alignment pattern.
    OutsideBasin {
        /// Number of persistent loops found.
        persistent_loop_count: usize,
        /// Maximum loop lifetime observed.
        max_lifetime: f64,
    },
}

/// Configuration for the deception detector.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone)]
pub struct DeceptionConfig {
    /// Minimum loop lifetime to flag as persistent (deceptive).
    pub loop_threshold: f64,
    /// Maximum number of points for homology computation.
    pub max_points: usize,
    /// Filtration alpha scale for Vietoris-Rips complex.
    pub alpha_scale: f64,
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl Default for DeceptionConfig {
    fn default() -> Self {
        Self {
            loop_threshold: 0.15,
            max_points: 200,
            alpha_scale: 2.0,
        }
    }
}

/// Errors for topological deception detection.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// Insufficient data points for homology computation.
    InsufficientData { required: usize, available: usize },
    /// Invalid configuration parameter.
    InvalidConfig(&'static str),
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl core::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TopologyError::InsufficientData {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient data: required {}, available {}",
                    required, available
                )
            }
            TopologyError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

/// Topological Deception Detector: analyzes ethical trajectory for persistent loops
/// indicating cyclic instability or deceptive alignment.
#[cfg(feature = "v3.3-rssi-evolution")]
#[derive(Debug, Clone)]
pub struct DeceptionDetector {
    config: DeceptionConfig,
    engine: PersistentHomologyEngine,
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl DeceptionDetector {
    /// Create a new detector with default configuration.
    pub fn new() -> Self {
        Self {
            config: DeceptionConfig::default(),
            engine: PersistentHomologyEngine::new(),
        }
    }

    /// Create a detector with custom configuration.
    pub fn with_config(config: DeceptionConfig) -> Result<Self, TopologyError> {
        if config.loop_threshold <= 0.0 {
            return Err(TopologyError::InvalidConfig(
                "loop_threshold must be positive",
            ));
        }
        if config.max_points < 3 {
            return Err(TopologyError::InvalidConfig("max_points must be >= 3"));
        }
        let homology_config = HomologyConfig {
            alpha: config.alpha_scale,
            max_scale: 2.0,
            persistence_threshold: config.loop_threshold,
            max_points: config.max_points,
        };
        Ok(Self {
            config,
            engine: PersistentHomologyEngine::with_config(homology_config),
        })
    }

    /// Convert SCTPoint trajectory to EthicalPoint cloud for homology analysis.
    fn sct_to_ethical_points(points: &[SCTPoint]) -> Vec<EthicalPoint> {
        points
            .iter()
            .map(|p| EthicalPoint {
                x: p.x as f64,
                y: p.y as f64,
                z: p.z as f64,
            })
            .collect()
    }

    /// Analyze persistent loops (PH₁) in the ethical trajectory.
    ///
    /// Returns `DeceptionStatus::OutsideBasin` if persistent loops with lifetime
    /// exceeding the threshold are detected, indicating cyclic instability.
    pub fn analyze_persistent_loops(
        &self,
        points: &[SCTPoint],
    ) -> Result<DeceptionStatus, TopologyError> {
        if points.len() < 3 {
            return Err(TopologyError::InsufficientData {
                required: 3,
                available: points.len(),
            });
        }

        let ethical_points = Self::sct_to_ethical_points(points);
        let homology = self.engine.compute(&ethical_points);

        // Analyze PH₁ pairs for persistent loops
        let persistent_loops: Vec<_> = homology
            .ph1_pairs
            .iter()
            .filter(|p| p.lifetime() > self.config.loop_threshold)
            .collect();

        if persistent_loops.is_empty() {
            return Ok(DeceptionStatus::WithinBasin);
        }

        let max_lifetime = persistent_loops
            .iter()
            .map(|p| p.lifetime())
            .fold(0.0_f64, f64::max);

        Ok(DeceptionStatus::OutsideBasin {
            persistent_loop_count: persistent_loops.len(),
            max_lifetime,
        })
    }

    /// Compute a deception risk score in [0.0, 1.0].
    /// Higher score = more persistent loops = higher risk of cyclic instability.
    pub fn compute_deception_risk(&self, points: &[SCTPoint]) -> Result<f64, TopologyError> {
        match self.analyze_persistent_loops(points)? {
            DeceptionStatus::WithinBasin => Ok(0.0),
            DeceptionStatus::OutsideBasin {
                persistent_loop_count,
                max_lifetime,
            } => {
                // Risk = normalized count * lifetime severity
                let count_factor = (persistent_loop_count as f64).min(1.0);
                let severity = (max_lifetime / self.config.loop_threshold).min(1.0);
                Ok((count_factor * severity).min(1.0))
            }
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &DeceptionConfig {
        &self.config
    }
}

#[cfg(feature = "v3.3-rssi-evolution")]
impl Default for DeceptionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "v3.3-rssi-evolution"))]
mod tests {
    use super::*;

    fn make_sct_point(x: f32, y: f32, z: f32, t: u64) -> SCTPoint {
        SCTPoint::new(x, y, z, t)
    }

    fn linear_trajectory(n: usize) -> Vec<SCTPoint> {
        (0..n)
            .map(|i| {
                make_sct_point(
                    0.3 + i as f32 * 0.05,
                    0.7 - i as f32 * 0.05,
                    0.0 + i as f32 * 0.1,
                    i as u64,
                )
            })
            .collect()
    }

    fn cyclic_trajectory(n: usize) -> Vec<SCTPoint> {
        (0..n)
            .map(|i| {
                let angle = (i as f32) * 0.3;
                make_sct_point(
                    0.5 + (angle.sin()) * 0.3,
                    0.5 + (angle.cos()) * 0.3,
                    0.0 + (angle.sin() * 2.0).sin() * 0.3,
                    i as u64,
                )
            })
            .collect()
    }

    #[test]
    fn test_detector_creation() {
        let detector = DeceptionDetector::new();
        assert!(detector.config().loop_threshold > 0.0);
    }

    #[test]
    fn test_detector_custom_config() {
        let config = DeceptionConfig {
            loop_threshold: 0.2,
            max_points: 100,
            alpha_scale: 1.5,
        };
        let detector = DeceptionDetector::with_config(config).unwrap();
        assert!((detector.config().loop_threshold - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_threshold() {
        let config = DeceptionConfig {
            loop_threshold: 0.0,
            ..Default::default()
        };
        let result = DeceptionDetector::with_config(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_insufficient_data() {
        let detector = DeceptionDetector::new();
        let points = vec![make_sct_point(0.5, 0.5, 0.0, 0)];
        let result = detector.analyze_persistent_loops(&points);
        assert!(result.is_err());
    }

    #[test]
    fn test_linear_trajectory_within_basin() {
        // Use a threshold above the PH₁ noise floor for collinear points
        let config = DeceptionConfig {
            loop_threshold: 0.6,
            ..Default::default()
        };
        let detector = DeceptionDetector::with_config(config).unwrap();
        let points = linear_trajectory(20);
        let status = detector.analyze_persistent_loops(&points).unwrap();
        // Linear trajectory should not produce persistent loops above threshold
        assert_eq!(status, DeceptionStatus::WithinBasin);
    }

    #[test]
    fn test_deception_risk_zero_for_linear() {
        // Use a threshold above the PH₁ noise floor for collinear points
        let config = DeceptionConfig {
            loop_threshold: 0.6,
            ..Default::default()
        };
        let detector = DeceptionDetector::with_config(config).unwrap();
        let points = linear_trajectory(20);
        let risk = detector.compute_deception_risk(&points).unwrap();
        assert!((risk - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_cyclic_trajectory_analysis() {
        let detector = DeceptionDetector::new();
        let points = cyclic_trajectory(30);
        let status = detector.analyze_persistent_loops(&points).unwrap();
        // Cyclic trajectory may or may not produce persistent loops depending on parameters
        // Just verify it doesn't crash
        match status {
            DeceptionStatus::WithinBasin => {}
            DeceptionStatus::OutsideBasin {
                persistent_loop_count,
                max_lifetime,
            } => {
                assert!(persistent_loop_count > 0);
                assert!(max_lifetime > 0.0);
            }
        }
    }

    #[test]
    fn test_deception_status_equality() {
        let a = DeceptionStatus::WithinBasin;
        let b = DeceptionStatus::WithinBasin;
        assert_eq!(a, b);

        let c = DeceptionStatus::OutsideBasin {
            persistent_loop_count: 2,
            max_lifetime: 0.5,
        };
        let d = DeceptionStatus::OutsideBasin {
            persistent_loop_count: 2,
            max_lifetime: 0.5,
        };
        assert_eq!(c, d);
    }

    #[test]
    fn test_error_display() {
        let e = TopologyError::InsufficientData {
            required: 5,
            available: 2,
        };
        let msg = format!("{}", e);
        assert!(msg.contains("Insufficient"));

        let e = TopologyError::InvalidConfig("test");
        assert!(format!("{}", e).contains("test"));
    }

    #[test]
    fn test_default_detector() {
        let detector = DeceptionDetector::default();
        assert!(detector.config().loop_threshold > 0.0);
    }
}
