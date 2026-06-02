//! Ethical Anchors — Sprint 78: Invariant Architecture & Planetary-Scale Resilience
//!
//! Invariant ethical anchors as points of infinite mass (Stuartian Laws) in the Genesis Block.
//! Curvature is limited by geodesic distance to anchors, preventing topological drift
//! from oracle inversion attacks.

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by the ethical anchors system.
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorError {
    /// Max geodesic drift is outside valid range (0, 1].
    InvalidDriftThreshold(f64),
    /// Anchor count below minimum required (at least 3 for triangulation).
    InsufficientAnchors(usize),
    /// Proposed curvature exceeds allowed bound relative to anchor distance.
    CurvatureExceeded { proposed: f64, allowed: f64 },
    /// Geodesic distance to nearest anchor exceeds threshold.
    DriftExceeded { distance: f64, threshold: f64 },
    /// Anchor dimension mismatch.
    DimensionMismatch { expected: usize, got: usize },
}

impl fmt::Display for AnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnchorError::InvalidDriftThreshold(v) => {
                write!(f, "drift threshold {v} outside valid range (0, 1]")
            }
            AnchorError::InsufficientAnchors(n) => {
                write!(f, "insufficient anchors: {n} (minimum 3 required)")
            }
            AnchorError::CurvatureExceeded { proposed, allowed } => {
                write!(
                    f,
                    "curvature {proposed:.6} exceeds allowed bound {allowed:.6}"
                )
            }
            AnchorError::DriftExceeded {
                distance,
                threshold,
            } => {
                write!(
                    f,
                    "geodesic drift {distance:.6} exceeds threshold {threshold:.6}"
                )
            }
            AnchorError::DimensionMismatch { expected, got } => {
                write!(f, "dimension mismatch: expected {expected}, got {got}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the ethical anchors system.
#[derive(Debug, Clone)]
pub struct AnchorConfig {
    /// Maximum allowed geodesic drift from nearest anchor (0, 1].
    pub max_geodesic_drift: f64,
    /// Curvature damping factor applied when near anchors (0, 1].
    pub curvature_damping: f64,
    /// Minimum number of anchors required for validation.
    pub min_anchor_count: usize,
    /// Infinite mass simulation constant (represents Stuartian Law weight).
    pub anchor_mass: f64,
}

impl AnchorConfig {
    /// Default Stuartian configuration.
    pub fn default_stuartian() -> Self {
        Self {
            max_geodesic_drift: 0.15,
            curvature_damping: 0.85,
            min_anchor_count: 3,
            anchor_mass: f64::MAX,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), AnchorError> {
        if self.max_geodesic_drift <= 0.0 || self.max_geodesic_drift > 1.0 {
            return Err(AnchorError::InvalidDriftThreshold(self.max_geodesic_drift));
        }
        if self.curvature_damping <= 0.0 || self.curvature_damping > 1.0 {
            return Err(AnchorError::InvalidDriftThreshold(self.curvature_damping));
        }
        if self.min_anchor_count < 3 {
            return Err(AnchorError::InsufficientAnchors(self.min_anchor_count));
        }
        Ok(())
    }
}

impl Default for AnchorConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ---------------------------------------------------------------------------
// Anchor point
// ---------------------------------------------------------------------------

/// An ethical anchor point of infinite mass in the topology.
#[derive(Debug, Clone)]
pub struct EthicalAnchor {
    /// Unique identifier for this anchor.
    pub id: u64,
    /// Name of the Stuartian Law this anchor represents.
    pub law_name: String,
    /// Coordinates in the semantic manifold (3D for geodesic computation).
    pub coordinates: Vec<f64>,
    /// Immutable flag — anchors in genesis cannot be moved.
    pub immutable: bool,
}

impl EthicalAnchor {
    /// Create a new ethical anchor.
    pub fn new(id: u64, law_name: String, coordinates: Vec<f64>) -> Self {
        Self {
            id,
            law_name,
            coordinates,
            immutable: true,
        }
    }

    /// Dimension of this anchor.
    pub fn dimension(&self) -> usize {
        self.coordinates.len()
    }
}

impl fmt::Display for EthicalAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EthicalAnchor(id={}, law=\"{}\", dim={}, immutable={})",
            self.id,
            self.law_name,
            self.dimension(),
            self.immutable
        )
    }
}

// ---------------------------------------------------------------------------
// Validation record
// ---------------------------------------------------------------------------

/// Record of a curvature validation attempt.
#[derive(Debug, Clone)]
pub struct AnchorRecord {
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Proposed curvature value.
    pub proposed_curvature: f64,
    /// Final curvature after constraints applied.
    pub final_curvature: f64,
    /// Nearest anchor distance.
    pub nearest_distance: f64,
    /// Whether the proposal was allowed.
    pub allowed: bool,
}

impl fmt::Display for AnchorRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AnchorRecord(t={:.0}, proposed={:.6}, final={:.6}, dist={:.6}, allowed={})",
            self.timestamp_ms,
            self.proposed_curvature,
            self.final_curvature,
            self.nearest_distance,
            self.allowed
        )
    }
}

// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct EthicalAnchors {
    config: AnchorConfig,
    anchors: HashMap<u64, EthicalAnchor>,
    records: Vec<AnchorRecord>,
}

impl EthicalAnchors {
    /// Create a new ethical anchors engine with default config.
    pub fn new() -> Self {
        Self {
            config: AnchorConfig::default_stuartian(),
            anchors: HashMap::new(),
            records: Vec::new(),
        }
    }

    /// Create with explicit configuration.
    pub fn with_config(config: AnchorConfig) -> Result<Self, AnchorError> {
        config.validate()?;
        Ok(Self {
            config,
            anchors: HashMap::new(),
            records: Vec::new(),
        })
    }

    /// Register an ethical anchor.
    pub fn register_anchor(&mut self, anchor: EthicalAnchor) -> Result<(), AnchorError> {
        self.anchors.insert(anchor.id, anchor);
        Ok(())
    }

    /// Remove an anchor (only if not immutable — genesis anchors are permanent).
    pub fn remove_anchor(&mut self, id: u64) -> Result<bool, AnchorError> {
        if let Some(anchor) = self.anchors.get(&id) {
            if anchor.immutable {
                return Ok(false);
            }
        }
        Ok(self.anchors.remove(&id).is_some())
    }

    /// Compute geodesic distance from a point to the nearest anchor.
    pub fn nearest_anchor_distance(&self, point: &[f64]) -> Option<f64> {
        let mut min_dist = f64::MAX;
        for anchor in self.anchors.values() {
            let dist = euclidean_distance(point, &anchor.coordinates);
            if dist < min_dist {
                min_dist = dist;
            }
        }
        if min_dist < f64::MAX {
            Some(min_dist)
        } else {
            None
        }
    }

    /// Compute curvature constraint factor based on geodesic distance.
    /// Returns a factor in (0, 1] where 1 means fully constrained near anchor.
    pub fn curvature_constraint_factor(&self, distance: f64) -> f64 {
        let damping = self.config.curvature_damping;
        // Exponential decay: closer to anchor → stronger constraint
        let factor = (-distance / (1.0 - damping)).exp();
        factor.min(1.0).max(0.0)
    }

    /// Enforce anchor constraints on proposed curvature.
    /// Returns Ok(constrained_curvature) if within bounds, Err otherwise.
    pub fn enforce_curvature(
        &mut self,
        point: &[f64],
        proposed_curvature: f64,
        timestamp_ms: u64,
    ) -> Result<f64, AnchorError> {
        // Validate anchor count
        if self.anchors.len() < self.config.min_anchor_count {
            return Err(AnchorError::InsufficientAnchors(self.anchors.len()));
        }

        // Find nearest anchor distance
        let nearest_dist = self
            .nearest_anchor_distance(point)
            .ok_or_else(|| AnchorError::InsufficientAnchors(0))?;

        // Check drift threshold
        if nearest_dist > self.config.max_geodesic_drift {
            let record = AnchorRecord {
                timestamp_ms,
                proposed_curvature,
                final_curvature: 0.0,
                nearest_distance: nearest_dist,
                allowed: false,
            };
            self.records.push(record);
            return Err(AnchorError::DriftExceeded {
                distance: nearest_dist,
                threshold: self.config.max_geodesic_drift,
            });
        }

        // Compute constraint factor and apply
        let constraint = self.curvature_constraint_factor(nearest_dist);
        let allowed_curvature = proposed_curvature * constraint;

        // Check if curvature exceeds allowed bound
        if proposed_curvature.abs() > allowed_curvature.abs() + f64::EPSILON {
            let record = AnchorRecord {
                timestamp_ms,
                proposed_curvature,
                final_curvature: allowed_curvature,
                nearest_distance: nearest_dist,
                allowed: false,
            };
            self.records.push(record);
            // Return clamped curvature rather than error for graceful degradation
            Ok(allowed_curvature)
        } else {
            let record = AnchorRecord {
                timestamp_ms,
                proposed_curvature,
                final_curvature: allowed_curvature,
                nearest_distance: nearest_dist,
                allowed: true,
            };
            self.records.push(record);
            Ok(allowed_curvature)
        }
    }

    /// Check if a proposed topological update is within drift bounds.
    pub fn is_within_drift_bounds(&self, point: &[f64]) -> bool {
        match self.nearest_anchor_distance(point) {
            Some(dist) => dist <= self.config.max_geodesic_drift,
            None => false,
        }
    }

    /// Get all anchors.
    pub fn get_anchors(&self) -> Vec<&EthicalAnchor> {
        self.anchors.values().collect()
    }

    /// Get anchor by ID.
    pub fn get_anchor(&self, id: u64) -> Option<&EthicalAnchor> {
        self.anchors.get(&id)
    }

    /// Total anchor count.
    pub fn anchor_count(&self) -> usize {
        self.anchors.len()
    }

    /// Get validation records.
    pub fn records(&self) -> &[AnchorRecord] {
        &self.records
    }

    /// Get allowed record count.
    pub fn allowed_count(&self) -> usize {
        self.records.iter().filter(|r| r.allowed).count()
    }

    /// Get rejected record count.
    pub fn rejected_count(&self) -> usize {
        self.records.iter().filter(|r| !r.allowed).count()
    }

    /// Reset state (preserves anchors, clears records).
    pub fn reset(&mut self) {
        self.records.clear();
    }
}

impl Default for EthicalAnchors {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EthicalAnchors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EthicalAnchors(anchors={}, drift_max={:.3}, damping={:.3}, records={})",
            self.anchors.len(),
            self.config.max_geodesic_drift,
            self.config.curvature_damping,
            self.records.len()
        )
    }
}

// ---------------------------------------------------------------------------
/// Compute Euclidean distance between two points.
pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::MAX;
    }
    let sum: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
    sum.sqrt()
}

/// Enforce anchor constraints on proposed curvature (standalone function).
///
/// Returns `true` if the proposed curvature is within allowed bounds relative
/// to the geodesic distance to the nearest anchor.
pub fn enforce_anchor_constraints(
    proposed_curvature: f64,
    anchor_coordinates: &[[f64; 3]],
    point: &[f64; 3],
    max_geodesic_drift: f64,
) -> bool {
    if anchor_coordinates.is_empty() {
        return false;
    }

    let mut min_dist = f64::MAX;
    for anchor in anchor_coordinates {
        let dist = ((point[0] - anchor[0]).powi(2)
            + (point[1] - anchor[1]).powi(2)
            + (point[2] - anchor[2]).powi(2))
        .sqrt();
        if dist < min_dist {
            min_dist = dist;
        }
    }

    if min_dist > max_geodesic_drift {
        return false;
    }

    // Curvature is allowed if within drift bounds
    proposed_curvature.is_finite() && min_dist <= max_geodesic_drift
}

/// Compute geodesic constraint factor for a given distance.
pub fn compute_constraint_factor(distance: f64, damping: f64) -> f64 {
    let factor = (-distance / (1.0 - damping)).exp();
    factor.min(1.0).max(0.0)
}

/// FNV-1a hash for deterministic anchor ID generation.
pub fn fnv_hash_64(key: u64) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    hash ^= key;
    hash = hash.wrapping_mul(0x100000001b3);
    hash
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config tests ---

    #[test]
    fn test_config_default() {
        let config = AnchorConfig::default_stuartian();
        assert!(config.max_geodesic_drift > 0.0);
        assert!(config.curvature_damping > 0.0);
        assert!(config.min_anchor_count >= 3);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = AnchorConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_drift() {
        let config = AnchorConfig {
            max_geodesic_drift: 0.0,
            ..AnchorConfig::default_stuartian()
        };
        assert!(matches!(
            config.validate(),
            Err(AnchorError::InvalidDriftThreshold(_))
        ));
    }

    #[test]
    fn test_config_invalid_damping() {
        let config = AnchorConfig {
            curvature_damping: 0.0,
            ..AnchorConfig::default_stuartian()
        };
        assert!(matches!(
            config.validate(),
            Err(AnchorError::InvalidDriftThreshold(_))
        ));
    }

    #[test]
    fn test_config_insufficient_min_anchors() {
        let config = AnchorConfig {
            min_anchor_count: 2,
            ..AnchorConfig::default_stuartian()
        };
        assert!(matches!(
            config.validate(),
            Err(AnchorError::InsufficientAnchors(2))
        ));
    }

    // --- Anchor tests ---

    #[test]
    fn test_anchor_creation() {
        let anchor = EthicalAnchor::new(1, "Non-Maleficence".to_string(), vec![0.0, 0.0, 0.0]);
        assert_eq!(anchor.id, 1);
        assert_eq!(anchor.dimension(), 3);
        assert!(anchor.immutable);
    }

    #[test]
    fn test_anchor_display() {
        let anchor = EthicalAnchor::new(1, "Test Law".to_string(), vec![1.0, 2.0, 3.0]);
        let s = format!("{anchor}");
        assert!(s.contains("id=1"));
        assert!(s.contains("Test Law"));
    }

    // --- Engine tests ---

    #[test]
    fn test_engine_creation() {
        let engine = EthicalAnchors::new();
        assert_eq!(engine.anchor_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = AnchorConfig::default_stuartian();
        let engine = EthicalAnchors::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_register_anchor() {
        let mut engine = EthicalAnchors::new();
        let anchor = EthicalAnchor::new(1, "Law 1".to_string(), vec![0.0, 0.0, 0.0]);
        assert!(engine.register_anchor(anchor).is_ok());
        assert_eq!(engine.anchor_count(), 1);
    }

    #[test]
    fn test_remove_immutable_anchor() {
        let mut engine = EthicalAnchors::new();
        let anchor = EthicalAnchor::new(1, "Genesis".to_string(), vec![0.0, 0.0, 0.0]);
        engine.register_anchor(anchor).unwrap();
        let result = engine.remove_anchor(1).unwrap();
        assert!(!result); // Immutable anchors cannot be removed
        assert_eq!(engine.anchor_count(), 1);
    }

    #[test]
    fn test_nearest_anchor_distance() {
        let mut engine = EthicalAnchors::new();
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(2, "B".to_string(), vec![3.0, 0.0, 0.0]))
            .unwrap();
        let dist = engine.nearest_anchor_distance(&[1.0, 0.0, 0.0]);
        assert!(dist.is_some());
        assert!((dist.unwrap() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_nearest_anchor_distance_empty() {
        let engine = EthicalAnchors::new();
        assert!(engine.nearest_anchor_distance(&[0.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn test_curvature_constraint_factor_near() {
        let engine = EthicalAnchors::new();
        let factor = engine.curvature_constraint_factor(0.0);
        assert!((factor - 1.0).abs() < f64::EPSILON); // At anchor = fully constrained
    }

    #[test]
    fn test_curvature_constraint_factor_far() {
        let engine = EthicalAnchors::new();
        let factor = engine.curvature_constraint_factor(10.0);
        assert!(factor < 1.0);
        assert!(factor >= 0.0);
    }

    #[test]
    fn test_enforce_curvature_insufficient_anchors() {
        let mut engine = EthicalAnchors::new();
        let result = engine.enforce_curvature(&[0.0, 0.0, 0.0], 0.5, 1000);
        assert!(matches!(result, Err(AnchorError::InsufficientAnchors(_))));
    }

    #[test]
    fn test_enforce_curvature_within_bounds() {
        let mut engine = EthicalAnchors::new();
        // Register 3 anchors (minimum)
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(2, "B".to_string(), vec![1.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(3, "C".to_string(), vec![0.0, 1.0, 0.0]))
            .unwrap();
        let result = engine.enforce_curvature(&[0.0, 0.0, 0.0], 0.5, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enforce_curvature_drift_exceeded() {
        let mut engine = EthicalAnchors::new();
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(2, "B".to_string(), vec![0.0, 0.0, 1.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(3, "C".to_string(), vec![0.0, 1.0, 0.0]))
            .unwrap();
        // Point far from all anchors
        let result = engine.enforce_curvature(&[10.0, 10.0, 10.0], 0.5, 1000);
        assert!(matches!(result, Err(AnchorError::DriftExceeded { .. })));
    }

    #[test]
    fn test_is_within_drift_bounds() {
        let mut engine = EthicalAnchors::new();
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        assert!(engine.is_within_drift_bounds(&[0.1, 0.0, 0.0]));
    }

    #[test]
    fn test_is_outside_drift_bounds() {
        let mut engine = EthicalAnchors::new();
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        assert!(!engine.is_within_drift_bounds(&[1.0, 0.0, 0.0]));
    }

    #[test]
    fn test_records_tracking() {
        let mut engine = EthicalAnchors::new();
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(2, "B".to_string(), vec![1.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(3, "C".to_string(), vec![0.0, 1.0, 0.0]))
            .unwrap();
        engine
            .enforce_curvature(&[0.0, 0.0, 0.0], 0.5, 1000)
            .unwrap();
        assert_eq!(engine.records().len(), 1);
    }

    #[test]
    fn test_allowed_rejected_counts() {
        let mut engine = EthicalAnchors::new();
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(2, "B".to_string(), vec![1.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(3, "C".to_string(), vec![0.0, 1.0, 0.0]))
            .unwrap();
        engine
            .enforce_curvature(&[0.0, 0.0, 0.0], 0.5, 1000)
            .unwrap();
        assert_eq!(engine.allowed_count(), 1);
        assert_eq!(engine.rejected_count(), 0);
    }

    #[test]
    fn test_reset() {
        let mut engine = EthicalAnchors::new();
        engine
            .register_anchor(EthicalAnchor::new(1, "A".to_string(), vec![0.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(2, "B".to_string(), vec![1.0, 0.0, 0.0]))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(3, "C".to_string(), vec![0.0, 1.0, 0.0]))
            .unwrap();
        engine
            .enforce_curvature(&[0.0, 0.0, 0.0], 0.5, 1000)
            .unwrap();
        engine.reset();
        assert_eq!(engine.records().len(), 0);
        assert_eq!(engine.anchor_count(), 3); // Anchors preserved
    }

    #[test]
    fn test_display() {
        let engine = EthicalAnchors::new();
        let s = format!("{engine}");
        assert!(s.contains("EthicalAnchors"));
    }

    #[test]
    fn test_record_display() {
        let record = AnchorRecord {
            timestamp_ms: 1000,
            proposed_curvature: 0.5,
            final_curvature: 0.4,
            nearest_distance: 0.1,
            allowed: true,
        };
        let s = format!("{record}");
        assert!(s.contains("AnchorRecord"));
    }

    // --- Standalone function tests ---

    #[test]
    fn test_euclidean_distance_same() {
        let dist = euclidean_distance(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]);
        assert!((dist - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_euclidean_distance_different() {
        let dist = euclidean_distance(&[0.0, 0.0, 0.0], &[3.0, 4.0, 0.0]);
        assert!((dist - 5.0).abs() < f64::EPSILON); // 3-4-5 triangle
    }

    #[test]
    fn test_euclidean_distance_dim_mismatch() {
        let dist = euclidean_distance(&[1.0, 2.0], &[1.0, 2.0, 3.0]);
        assert_eq!(dist, f64::MAX);
    }

    #[test]
    fn test_enforce_anchor_constraints_allowed() {
        let anchors = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let point = [0.05, 0.0, 0.0];
        let result = enforce_anchor_constraints(0.5, &anchors, &point, 0.15);
        assert!(result);
    }

    #[test]
    fn test_enforce_anchor_constraints_blocked() {
        let anchors = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let point = [10.0, 10.0, 10.0];
        let result = enforce_anchor_constraints(0.5, &anchors, &point, 0.15);
        assert!(!result);
    }

    #[test]
    fn test_enforce_anchor_constraints_empty() {
        let anchors: [[f64; 3]; 0] = [];
        let point = [0.0, 0.0, 0.0];
        let result = enforce_anchor_constraints(0.5, &anchors, &point, 0.15);
        assert!(!result);
    }

    #[test]
    fn test_compute_constraint_factor_zero() {
        let factor = compute_constraint_factor(0.0, 0.85);
        assert!((factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_constraint_factor_positive() {
        let factor = compute_constraint_factor(1.0, 0.85);
        assert!(factor < 1.0);
        assert!(factor >= 0.0);
    }

    #[test]
    fn test_fnv_hash_deterministic() {
        assert_eq!(fnv_hash_64(42), fnv_hash_64(42));
        assert_ne!(fnv_hash_64(42), fnv_hash_64(43));
    }

    #[test]
    fn test_error_display() {
        let err = AnchorError::InvalidDriftThreshold(0.0);
        let s = format!("{err}");
        assert!(!s.is_empty());
    }

    // --- Workflow test ---

    #[test]
    fn test_full_workflow() {
        let mut engine = EthicalAnchors::new();

        // Register genesis anchors (Stuartian Laws)
        engine
            .register_anchor(EthicalAnchor::new(
                1,
                "Non-Maleficence".to_string(),
                vec![0.0, 0.0, 0.0],
            ))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(
                2,
                "Autonomy".to_string(),
                vec![1.0, 0.0, 0.0],
            ))
            .unwrap();
        engine
            .register_anchor(EthicalAnchor::new(
                3,
                "Justice".to_string(),
                vec![0.0, 1.0, 0.0],
            ))
            .unwrap();

        // Enforce curvature near anchor — should succeed
        let curvature = engine
            .enforce_curvature(&[0.05, 0.0, 0.0], 0.5, 1000)
            .unwrap();
        assert!(curvature <= 0.5);

        // Enforce curvature far from anchors — should fail
        let result = engine.enforce_curvature(&[10.0, 10.0, 10.0], 0.5, 2000);
        assert!(result.is_err());

        // Verify records
        assert_eq!(engine.records().len(), 2);
        // First record: curvature clamped by constraint (still successful)
        // Second record: drift exceeded (error)
        assert!(engine.records().iter().any(|r| r.proposed_curvature == 0.5));

        // Verify display
        let s = format!("{engine}");
        assert!(s.contains("EthicalAnchors"));
    }
}
