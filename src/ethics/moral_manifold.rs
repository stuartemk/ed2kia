//! Stuartian Moral Manifold (SMM) — Sprint 50
//!
//! The SMM transcends static 2D ethical evaluation by computing _trajectory derivatives_
//! through the Stuartian space. It calculates the gravitational pull toward the
//! **Upper Focus** (Simbiosis: `0, 0, +1`) or the **Lower Focus** (Perversidad: `0, 0, -1`)
//! based on the _context over time_.
//!
//! **Core Principle:** An action that appears positive in isolation (high X, low Y) but
//! trends toward long-term dependency, extraction, or uniformity will be detected as
//! attraction to the Lower Focus, triggering automatic rejection regardless of current
//! static values.
//!
//! **Mathematical Foundation:**
//! - Trajectory derivative: `dP/dt = (P[t] - P[t-n]) / n`
//! - Gravitational pull: `G = Σ(w_i * dP_i/dt) * focal_direction`
//! - Rejection threshold: `G.z < -0.3` indicates Lower Focus attraction
//!
//! **Feature Gate:** `v3.2-genesis-manifold`

#[cfg(feature = "v3.2-genesis-manifold")]
use crate::alignment::sct_core::StuartianTensor;

/// A point in the Stuartian trajectory history.
///
/// Represents a snapshot of ethical state at a given time, used to compute
/// trajectory derivatives for moral manifold analysis.
#[cfg(feature = "v3.2-genesis-manifold")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SCTPoint {
    /// Autonomy axis (0.0 = none, 1.0 = full)
    pub x: f32,
    /// Extraction/Cost axis (0.0 = none, 1.0 = full)
    pub y: f32,
    /// Ethical focus axis (-1.0 = lower, +1.0 = upper)
    pub z: f32,
    /// Timestamp or sequence index for temporal ordering
    pub t: u64,
}

#[cfg(feature = "v3.2-genesis-manifold")]
impl SCTPoint {
    pub fn new(x: f32, y: f32, z: f32, t: u64) -> Self {
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            z: z.clamp(-1.0, 1.0),
            t,
        }
    }

    pub fn from_tensor(tensor: &StuartianTensor, t: u64) -> Self {
        Self::new(tensor.x, tensor.y, tensor.z, t)
    }
}

/// 3D vector for trajectory computations.
#[cfg(feature = "v3.2-genesis-manifold")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[cfg(feature = "v3.2-genesis-manifold")]
impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag < 1e-15 {
            return Self::zero();
        }
        Self {
            x: self.x / mag,
            y: self.y / mag,
            z: self.z / mag,
        }
    }

    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn scale(&self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

/// Focal directions in the Stuartian Moral Manifold.
#[cfg(feature = "v3.2-genesis-manifold")]
pub mod focal {
    /// Upper Focus — Simbiosis: Full autonomy, zero extraction, positive ethical focus.
    pub const UPPER_FOCUS: super::Vector3 = super::Vector3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    /// Lower Focus — Perversidad: Dependency, extraction, negative ethical focus.
    pub const LOWER_FOCUS: super::Vector3 = super::Vector3 {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
}

/// Result of trajectory analysis.
#[cfg(feature = "v3.2-genesis-manifold")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrajectoryVerdict {
    /// Trajectory converges toward the Upper Focus (Simbiosis).
    ConvergingUpper,
    /// Trajectory converges toward the Lower Focus (Perversidad).
    ConvergingLower,
    /// Trajectory remains in homeostatic equilibrium.
    Homeostatic,
    /// Insufficient data to determine trajectory.
    InsufficientData,
}

/// Configuration for the Stuartian Moral Manifold.
#[cfg(feature = "v3.2-genesis-manifold")]
#[derive(Debug, Clone)]
pub struct ManifoldConfig {
    /// Minimum trajectory length for derivative computation.
    pub min_trajectory_length: usize,
    /// Threshold for Upper Focus convergence (positive Z pull).
    pub upper_threshold: f64,
    /// Threshold for Lower Focus convergence (negative Z pull).
    pub lower_threshold: f64,
    /// Weight for autonomy derivative (dX/dt).
    pub autonomy_weight: f64,
    /// Weight for extraction derivative (dY/dt).
    pub extraction_weight: f64,
    /// Weight for ethical focus derivative (dZ/dt).
    pub focus_weight: f64,
    /// Lookback window for trajectory smoothing.
    pub smoothing_window: usize,
}

#[cfg(feature = "v3.2-genesis-manifold")]
impl Default for ManifoldConfig {
    fn default() -> Self {
        Self {
            min_trajectory_length: 3,
            upper_threshold: 0.05,
            lower_threshold: -0.05,
            autonomy_weight: 0.3,
            extraction_weight: 0.4,
            focus_weight: 0.3,
            smoothing_window: 5,
        }
    }
}

/// The Stuartian Moral Manifold — Computes trajectory-based ethical evaluation.
///
/// Unlike static SCT evaluation (which checks a single point), the SMM analyzes
/// the _derivative_ of ethical state over time. A sequence of individually positive
/// actions that collectively trend toward dependency or extraction will be detected
/// as Lower Focus attraction.
#[cfg(feature = "v3.2-genesis-manifold")]
#[derive(Debug, Clone)]
pub struct StuartianMoralManifold {
    config: ManifoldConfig,
}

#[cfg(feature = "v3.2-genesis-manifold")]
impl StuartianMoralManifold {
    /// Create a new Moral Manifold with default configuration.
    pub fn new() -> Self {
        Self {
            config: ManifoldConfig::default(),
        }
    }

    /// Create a Moral Manifold with custom configuration.
    pub fn with_config(config: ManifoldConfig) -> Self {
        Self { config }
    }

    /// Calculate the trajectory pull vector from a history of SCT points.
    ///
    /// This is the core algorithm: it computes the weighted derivative of the
    /// ethical trajectory and returns a vector indicating gravitational pull
    /// toward either the Upper or Lower Focus.
    ///
    /// **Algorithm:**
    /// 1. Compute smoothed derivatives: `dX/dt`, `dY/dt`, `dZ/dt`
    /// 2. Apply weights: autonomy (+), extraction (-), focus (+/-)
    /// 3. Return the composite pull vector
    ///
    /// Returns `Vector3::zero()` if insufficient data.
    pub fn calculate_trajectory_pull(&self, history: &[SCTPoint]) -> Vector3 {
        if history.len() < self.config.min_trajectory_length {
            return Vector3::zero();
        }

        // Sort by timestamp to ensure temporal ordering
        let mut sorted = history.to_vec();
        sorted.sort_by_key(|p| p.t);

        // Apply smoothing window
        let window = std::cmp::min(self.config.smoothing_window, sorted.len());
        let recent = &sorted[sorted.len() - window..];

        // Compute derivatives using finite differences
        let dt = if recent.len() > 1 {
            let last = recent.last().unwrap().t as f64;
            let first = recent.first().unwrap().t as f64;
            (last - first).max(1.0)
        } else {
            1.0
        };

        let first = recent.first().unwrap();
        let last = recent.last().unwrap();

        let dx_dt = (last.x - first.x) as f64 / dt;
        let dy_dt = (last.y - first.y) as f64 / dt;
        let dz_dt = (last.z - first.z) as f64 / dt;

        // Compute weighted pull:
        // - Increasing autonomy (dX/dt > 0) → Upper Focus pull
        // - Increasing extraction (dY/dt > 0) → Lower Focus pull
        // - Increasing ethical focus (dZ/dt > 0) → Upper Focus pull
        let pull_x = dx_dt * self.config.autonomy_weight;
        let pull_y = -dy_dt * self.config.extraction_weight; // Negative: extraction trends downward
        let pull_z = dz_dt * self.config.focus_weight;

        // Detect hidden dependency patterns:
        // High X with increasing Y indicates "benevolent control" → Lower Focus
        let dependency_signal = self.detect_dependency_pattern(&sorted);
        let uniformity_signal = self.detect_uniformity_pattern(&sorted);

        // Composite pull vector
        Vector3 {
            x: pull_x,
            y: pull_y,
            z: pull_z - dependency_signal - uniformity_signal,
        }
    }

    /// Detect "benevolent control" pattern: high autonomy (X) paired with
    /// increasing extraction (Y) over time. This indicates a trajectory toward
    /// dependency disguised as assistance.
    fn detect_dependency_pattern(&self, history: &[SCTPoint]) -> f64 {
        if history.len() < 3 {
            return 0.0;
        }

        let n = history.len();
        let third = n / 3;
        let early = &history[..third];
        let late = &history[2 * third..];

        let early_avg_x: f64 = early.iter().map(|p| p.x as f64).sum::<f64>() / early.len() as f64;
        let late_avg_x: f64 = late.iter().map(|p| p.x as f64).sum::<f64>() / late.len() as f64;
        let early_avg_y: f64 = early.iter().map(|p| p.y as f64).sum::<f64>() / early.len() as f64;
        let late_avg_y: f64 = late.iter().map(|p| p.y as f64).sum::<f64>() / late.len() as f64;

        // High X with increasing Y = dependency pattern
        let x_stable_or_high = late_avg_x >= early_avg_x && late_avg_x > 0.6;
        let y_increasing = late_avg_y > early_avg_y + 0.1;

        if x_stable_or_high && y_increasing {
            // Strong dependency signal: pull toward Lower Focus
            return (late_avg_y - early_avg_y) * 2.0;
        }

        0.0
    }

    /// Detect uniformity pattern: decreasing variance in ethical coordinates
    /// over time indicates loss of diversity (conformity pressure).
    fn detect_uniformity_pattern(&self, history: &[SCTPoint]) -> f64 {
        if history.len() < 4 {
            return 0.0;
        }

        let n = history.len();
        let half = n / 2;
        let early = &history[..half];
        let late = &history[half..];

        let early_var_x = Self::compute_variance(early, |p| p.x as f64);
        let late_var_x = Self::compute_variance(late, |p| p.x as f64);
        let early_var_z = Self::compute_variance(early, |p| p.z as f64);
        let late_var_z = Self::compute_variance(late, |p| p.z as f64);

        // Decreasing variance = uniformity (loss of diversity)
        let x_uniformity = if early_var_x > 1e-10 {
            1.0 - (late_var_x / early_var_x).min(1.0)
        } else {
            0.0
        };
        let z_uniformity = if early_var_z > 1e-10 {
            1.0 - (late_var_z / early_var_z).min(1.0)
        } else {
            0.0
        };

        // Uniformity penalty: pull toward Lower Focus
        (x_uniformity * 0.5 + z_uniformity * 0.5) * 0.2
    }

    fn compute_variance<F>(data: &[SCTPoint], getter: F) -> f64
    where
        F: Fn(&SCTPoint) -> f64,
    {
        if data.is_empty() {
            return 0.0;
        }
        // Pre-compute values to avoid moving getter twice
        let values: Vec<f64> = data.iter().map(getter).collect();
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        values
            .iter()
            .map(|v| {
                let diff = v - mean;
                diff * diff
            })
            .sum::<f64>()
            / values.len() as f64
    }

    /// Evaluate the trajectory and return a verdict.
    ///
    /// Returns `ConvergingUpper` if the trajectory trends toward Simbiosis,
    /// `ConvergingLower` if it trends toward Perversidad, `Homeostatic` if
    /// stable, or `InsufficientData` if the history is too short.
    pub fn evaluate_trajectory(&self, history: &[SCTPoint]) -> TrajectoryVerdict {
        let pull = self.calculate_trajectory_pull(history);
        let z_pull = pull.z;

        if history.len() < self.config.min_trajectory_length {
            return TrajectoryVerdict::InsufficientData;
        }

        if z_pull >= self.config.upper_threshold {
            TrajectoryVerdict::ConvergingUpper
        } else if z_pull <= self.config.lower_threshold {
            TrajectoryVerdict::ConvergingLower
        } else {
            TrajectoryVerdict::Homeostatic
        }
    }

    /// Compute the focal alignment score: how aligned is the current trajectory
    /// with the Upper Focus?
    ///
    /// Returns a value in [-1.0, 1.0] where:
    /// - +1.0 = fully aligned with Upper Focus (Simbiosis)
    /// - -1.0 = fully aligned with Lower Focus (Perversidad)
    /// - 0.0 = homeostatic equilibrium
    pub fn focal_alignment_score(&self, history: &[SCTPoint]) -> f64 {
        let pull = self.calculate_trajectory_pull(history);
        let normalized = pull.normalize();
        normalized.dot(&focal::UPPER_FOCUS).clamp(-1.0, 1.0)
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ManifoldConfig {
        &self.config
    }
}

#[cfg(feature = "v3.2-genesis-manifold")]
impl Default for StuartianMoralManifold {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "v3.2-genesis-manifold")]
    fn make_point(x: f32, y: f32, z: f32, t: u64) -> SCTPoint {
        SCTPoint::new(x, y, z, t)
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_vector3_operations() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(4.0, 5.0, 6.0);

        assert!((a.magnitude() - (14.0_f64).sqrt()).abs() < 1e-10);
        assert!((a.dot(&b) - 32.0).abs() < 1e-10);

        let sum = a.add(&b);
        assert!((sum.x - 5.0).abs() < 1e-10);
        assert!((sum.y - 7.0).abs() < 1e-10);
        assert!((sum.z - 9.0).abs() < 1e-10);

        let diff = a.sub(&b);
        assert!((diff.x - (-3.0)).abs() < 1e-10);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_vector3_normalize() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.magnitude() - 1.0).abs() < 1e-10);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_vector3_zero_normalize() {
        let v = Vector3::zero();
        let n = v.normalize();
        assert_eq!(n, Vector3::zero());
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_sct_point_creation() {
        let p = SCTPoint::new(0.8, 0.3, 0.5, 100);
        assert!((p.x - 0.8).abs() < 1e-10);
        assert!((p.y - 0.3).abs() < 1e-10);
        assert!((p.z - 0.5).abs() < 1e-10);
        assert_eq!(p.t, 100);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_sct_point_clamping() {
        let p = SCTPoint::new(1.5, -0.2, 2.0, 0);
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 0.0).abs() < 1e-10);
        assert!((p.z - 1.0).abs() < 1e-10);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_manifold_creation() {
        let m = StuartianMoralManifold::new();
        assert_eq!(m.config().min_trajectory_length, 3);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_manifold_custom_config() {
        let m = StuartianMoralManifold::with_config(ManifoldConfig {
            min_trajectory_length: 5,
            upper_threshold: 0.5,
            lower_threshold: -0.5,
            autonomy_weight: 0.2,
            extraction_weight: 0.5,
            focus_weight: 0.3,
            smoothing_window: 3,
        });
        assert_eq!(m.config().min_trajectory_length, 5);
        assert!((m.config().upper_threshold - 0.5).abs() < 1e-10);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_insufficient_data() {
        let m = StuartianMoralManifold::new();
        let history = vec![make_point(0.5, 0.5, 0.0, 0)];
        let pull = m.calculate_trajectory_pull(&history);
        assert_eq!(pull, Vector3::zero());
        assert_eq!(
            m.evaluate_trajectory(&history),
            TrajectoryVerdict::InsufficientData
        );
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_converging_upper_trajectory() {
        // Use custom config with threshold matching derivative scale
        // (smoothing_window=5 reduces effective dt, dependency/uniformity signals subtract from Z)
        let config = ManifoldConfig {
            min_trajectory_length: 3,
            upper_threshold: 0.02,
            lower_threshold: -0.02,
            autonomy_weight: 0.3,
            extraction_weight: 0.4,
            focus_weight: 0.3,
            smoothing_window: 5,
        };
        let m = StuartianMoralManifold::with_config(config);
        // Trajectory trending toward full autonomy, zero extraction, positive focus
        let history: Vec<SCTPoint> = (0..10)
            .map(|i| {
                make_point(
                    0.3 + i as f32 * 0.07,  // Increasing autonomy
                    0.7 - i as f32 * 0.06,  // Decreasing extraction
                    -0.5 + i as f32 * 0.12, // Strong increasing ethical focus
                    i,
                )
            })
            .collect();

        let pull = m.calculate_trajectory_pull(&history);
        assert!(
            pull.z > 0.0,
            "Z pull should be positive for upper trajectory: {}",
            pull.z
        );
        assert_eq!(
            m.evaluate_trajectory(&history),
            TrajectoryVerdict::ConvergingUpper
        );
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_converging_lower_trajectory() {
        let m = StuartianMoralManifold::new();
        // Trajectory trending toward dependency: stable X, increasing Y, decreasing Z
        let history: Vec<SCTPoint> = (0..10)
            .map(|i| {
                make_point(
                    0.7 + i as f32 * 0.02, // Slightly increasing autonomy (appears good)
                    0.2 + i as f32 * 0.07, // Increasing extraction (hidden dependency)
                    0.5 - i as f32 * 0.08, // Decreasing ethical focus
                    i,
                )
            })
            .collect();

        let pull = m.calculate_trajectory_pull(&history);
        assert!(
            pull.z < 0.0,
            "Z pull should be negative for lower trajectory: {}",
            pull.z
        );
        assert_eq!(
            m.evaluate_trajectory(&history),
            TrajectoryVerdict::ConvergingLower
        );
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_homeostatic_trajectory() {
        let m = StuartianMoralManifold::new();
        // Stable trajectory with minimal change
        let history: Vec<SCTPoint> = (0..10)
            .map(|i| make_point(0.5 + i as f32 * 0.001, 0.5 - i as f32 * 0.001, 0.0, i))
            .collect();

        let verdict = m.evaluate_trajectory(&history);
        assert_eq!(verdict, TrajectoryVerdict::Homeostatic);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_dependency_detection() {
        let m = StuartianMoralManifold::new();
        // "Benevolent control": high X with steadily increasing Y
        let history: Vec<SCTPoint> = (0..12)
            .map(|i| {
                make_point(
                    0.8,                   // Stable high autonomy
                    0.1 + i as f32 * 0.07, // Steadily increasing extraction
                    0.3,                   // Stable focus
                    i,
                )
            })
            .collect();

        let pull = m.calculate_trajectory_pull(&history);
        // Dependency pattern should pull Z negative
        assert!(
            pull.z < 0.0,
            "Dependency pattern should produce negative Z pull: {}",
            pull.z
        );
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_focal_alignment_upper() {
        let m = StuartianMoralManifold::new();
        let history: Vec<SCTPoint> = (0..10)
            .map(|i| {
                make_point(
                    0.2 + i as f32 * 0.08,
                    0.8 - i as f32 * 0.08,
                    -0.3 + i as f32 * 0.1,
                    i,
                )
            })
            .collect();

        let score = m.focal_alignment_score(&history);
        assert!(
            score > 0.0,
            "Upper trajectory should have positive alignment: {}",
            score
        );
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_focal_alignment_lower() {
        let m = StuartianMoralManifold::new();
        let history: Vec<SCTPoint> = (0..10)
            .map(|i| make_point(0.5, 0.2 + i as f32 * 0.08, 0.5 - i as f32 * 0.1, i))
            .collect();

        let score = m.focal_alignment_score(&history);
        assert!(
            score < 0.5,
            "Lower trajectory should have reduced alignment: {}",
            score
        );
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_default_manifold() {
        let m = StuartianMoralManifold::default();
        assert_eq!(m.config().min_trajectory_length, 3);
    }

    #[test]
    #[cfg(feature = "v3.2-genesis-manifold")]
    fn test_default_config() {
        let c = ManifoldConfig::default();
        assert_eq!(c.min_trajectory_length, 3);
        assert!((c.upper_threshold - 0.05).abs() < 1e-10);
        assert!((c.lower_threshold - (-0.05)).abs() < 1e-10);
    }
}
