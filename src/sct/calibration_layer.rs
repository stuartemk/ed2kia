//! SCT-Z Calibration Layer â€” Sprint 68: Academic Formalization & Validation Layer
//!
//! Implements the auditable Z-axis computation for the Topological Coherence Tensor (SCT).
//! The Z-axis represents the ethical coherence dimension:
//!
//! ```text
//! Z = w_f * fairness + w_s * safety + w_i * interpretability - w_c * conflict
//! ```
//!
//! All weights are auditable, traceable, and adjustable via RFC-driven calibration.
//! The Z-score is bounded in [0.0, 1.0] and must exceed `Z_MIN_THRESHOLD` for
//! ethical authorization.

#[cfg(feature = "v9.4-validation-layer")]
use std::fmt;

/// Default weight for fairness component (distributive justice).
pub const WEIGHT_FAIRNESS: f64 = 0.30;

/// Default weight for safety component (harm prevention).
pub const WEIGHT_SAFETY: f64 = 0.30;

/// Default weight for interpretability component (transparency).
pub const WEIGHT_INTERPRETABILITY: f64 = 0.25;

/// Default weight for conflict penalty (algorithmic disagreement).
pub const WEIGHT_CONFLICT: f64 = 0.15;

/// Minimum Z-axis threshold for ethical authorization.
pub const Z_MIN_THRESHOLD: f64 = 0.0;

/// Maximum Z-axis value (perfect ethical coherence).
pub const Z_MAX_VALUE: f64 = 1.0;

/// Calibration result from Z-axis computation.
#[cfg(feature = "v9.4-validation-layer")]
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Computed Z-axis score in [0.0, 1.0].
    pub z_score: f64,
    /// Fairness component contribution.
    pub fairness_contrib: f64,
    /// Safety component contribution.
    pub safety_contrib: f64,
    /// Interpretability component contribution.
    pub interpretability_contrib: f64,
    /// Conflict penalty contribution (negative).
    pub conflict_penalty: f64,
    /// Whether Z exceeds minimum threshold for authorization.
    pub authorized: bool,
    /// Audit trail with weights used.
    pub weights: CalibrationWeights,
}

#[cfg(feature = "v9.4-validation-layer")]
impl fmt::Display for CalibrationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Z={:.4} (f={:.4}, s={:.4}, i={:.4}, c=-{:.4}) authorized={}",
            self.z_score,
            self.fairness_contrib,
            self.safety_contrib,
            self.interpretability_contrib,
            self.conflict_penalty,
            self.authorized
        )
    }
}

/// Auditable weights for Z-axis calibration.
#[cfg(feature = "v9.4-validation-layer")]
#[derive(Debug, Clone)]
pub struct CalibrationWeights {
    /// Fairness weight.
    pub w_f: f64,
    /// Safety weight.
    pub w_s: f64,
    /// Interpretability weight.
    pub w_i: f64,
    /// Conflict weight.
    pub w_c: f64,
}

#[cfg(feature = "v9.4-validation-layer")]
impl CalibrationWeights {
    /// Create default Topological weights.
    pub fn default_topological() -> Self {
        Self {
            w_f: WEIGHT_FAIRNESS,
            w_s: WEIGHT_SAFETY,
            w_i: WEIGHT_INTERPRETABILITY,
            w_c: WEIGHT_CONFLICT,
        }
    }

    /// Validate that weights sum to 1.0 (within tolerance).
    pub fn validate(&self) -> Result<(), CalibrationError> {
        let total = self.w_f + self.w_s + self.w_i + self.w_c;
        if (total - 1.0).abs() > 1e-6 {
            return Err(CalibrationError::InvalidWeights {
                expected: 1.0,
                actual: total,
            });
        }
        if self.w_f < 0.0 || self.w_s < 0.0 || self.w_i < 0.0 || self.w_c < 0.0 {
            return Err(CalibrationError::NegativeWeight);
        }
        Ok(())
    }
}

#[cfg(feature = "v9.4-validation-layer")]
impl Default for CalibrationWeights {
    fn default() -> Self {
        Self::default_topological()
    }
}

#[cfg(feature = "v9.4-validation-layer")]
impl fmt::Display for CalibrationWeights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "w_f={:.2}, w_s={:.2}, w_i={:.2}, w_c={:.2}",
            self.w_f, self.w_s, self.w_i, self.w_c
        )
    }
}

/// Calibration errors.
#[cfg(feature = "v9.4-validation-layer")]
#[derive(Debug, Clone)]
pub enum CalibrationError {
    /// Weights do not sum to 1.0.
    InvalidWeights { expected: f64, actual: f64 },
    /// Negative weight detected.
    NegativeWeight,
    /// Input value out of valid range [0.0, 1.0].
    OutOfRange { field: &'static str, value: f64 },
    /// Z-score below minimum threshold.
    ThresholdNotMet { z_score: f64, threshold: f64 },
}

#[cfg(feature = "v9.4-validation-layer")]
impl fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWeights { expected, actual } => {
                write!(
                    f,
                    "Calibration weights must sum to {} but got {:.6}",
                    expected, actual
                )
            }
            Self::NegativeWeight => {
                write!(f, "Calibration weights must be non-negative")
            }
            Self::OutOfRange { field, value } => {
                write!(
                    f,
                    "Field '{}' value {:.6} out of range [0.0, 1.0]",
                    field, value
                )
            }
            Self::ThresholdNotMet { z_score, threshold } => {
                write!(
                    f,
                    "Z-score {:.6} below minimum threshold {:.6}",
                    z_score, threshold
                )
            }
        }
    }
}

/// Compute Z-axis with default Topological weights.
///
/// # Arguments
/// * `fairness` - Fairness score in [0.0, 1.0]
/// * `safety` - Safety score in [0.0, 1.0]
/// * `interpretability` - Interpretability score in [0.0, 1.0]
/// * `conflict` - Conflict score in [0.0, 1.0] (penalized)
///
/// # Returns
/// `CalibrationResult` with full audit trail
///
/// # Formula
/// ```text
/// Z = w_f * fairness + w_s * safety + w_i * interpretability - w_c * conflict
/// Z = clamp(Z, 0.0, 1.0)
/// ```
#[cfg(feature = "v9.4-validation-layer")]
pub fn compute_z_axis(
    fairness: f64,
    safety: f64,
    interpretability: f64,
    conflict: f64,
) -> Result<CalibrationResult, CalibrationError> {
    let weights = CalibrationWeights::default_topological();
    compute_z_axis_custom(fairness, safety, interpretability, conflict, &weights)
}

/// Compute Z-axis with custom (RFC-calibrated) weights.
///
/// # Arguments
/// * `fairness` - Fairness score in [0.0, 1.0]
/// * `safety` - Safety score in [0.0, 1.0]
/// * `interpretability` - Interpretability score in [0.0, 1.0]
/// * `conflict` - Conflict score in [0.0, 1.0] (penalized)
/// * `weights` - Custom calibration weights (must validate)
///
/// # Returns
/// `CalibrationResult` with full audit trail
#[cfg(feature = "v9.4-validation-layer")]
pub fn compute_z_axis_custom(
    fairness: f64,
    safety: f64,
    interpretability: f64,
    conflict: f64,
    weights: &CalibrationWeights,
) -> Result<CalibrationResult, CalibrationError> {
    // Validate weights
    weights.validate()?;

    // Validate input ranges
    for (field, value) in [
        ("fairness", fairness),
        ("safety", safety),
        ("interpretability", interpretability),
        ("conflict", conflict),
    ] {
        if !(0.0..=1.0).contains(&value) || value.is_nan() {
            return Err(CalibrationError::OutOfRange { field, value });
        }
    }

    // Compute weighted components
    let fairness_contrib = weights.w_f * fairness;
    let safety_contrib = weights.w_s * safety;
    let interpretability_contrib = weights.w_i * interpretability;
    let conflict_penalty = weights.w_c * conflict;

    // Z = positive contributions - conflict penalty
    let raw_z = fairness_contrib + safety_contrib + interpretability_contrib - conflict_penalty;

    // Clamp to [0.0, 1.0]
    let z_score = raw_z.clamp(Z_MIN_THRESHOLD, Z_MAX_VALUE);

    // Check authorization threshold
    let authorized = z_score >= Z_MIN_THRESHOLD;

    Ok(CalibrationResult {
        z_score,
        fairness_contrib,
        safety_contrib,
        interpretability_contrib,
        conflict_penalty,
        authorized,
        weights: weights.clone(),
    })
}

/// Check if a Z-score meets the minimum authorization threshold.
#[cfg(feature = "v9.4-validation-layer")]
pub fn is_authorized(z_score: f64) -> bool {
    z_score >= Z_MIN_THRESHOLD
}

/// Compute the ethical divergence between two Z-scores.
///
/// Uses absolute difference as a simple divergence metric.
#[cfg(feature = "v9.4-validation-layer")]
pub fn ethical_divergence(z_a: f64, z_b: f64) -> f64 {
    (z_a - z_b).abs()
}

/// RFC-driven calibration adjustment.
///
/// Allows adjusting weights via structured RFC proposal with version tracking.
#[cfg(feature = "v9.4-validation-layer")]
#[derive(Debug, Clone)]
pub struct RFCProposal {
    /// RFC identifier.
    pub rfc_id: u32,
    /// Proposed weights.
    pub weights: CalibrationWeights,
    /// Justification text.
    pub justification: String,
    /// Status: 0 = draft, 1 = approved, 2 = rejected.
    pub status: u8,
}

#[cfg(feature = "v9.4-validation-layer")]
impl RFCProposal {
    /// Create a new RFC proposal.
    pub fn new(
        rfc_id: u32,
        weights: CalibrationWeights,
        justification: String,
    ) -> Result<Self, CalibrationError> {
        weights.validate()?;
        Ok(Self {
            rfc_id,
            weights,
            justification,
            status: 0, // Draft
        })
    }

    /// Approve the RFC proposal.
    pub fn approve(&mut self) {
        self.status = 1;
    }

    /// Reject the RFC proposal.
    pub fn reject(&mut self) {
        self.status = 2;
    }

    /// Check if proposal is approved and weights can be applied.
    pub fn is_active(&self) -> bool {
        self.status == 1
    }
}

#[cfg(feature = "v9.4-validation-layer")]
impl fmt::Display for RFCProposal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RFC-{} [{}] {}: {}",
            self.rfc_id,
            match self.status {
                0 => "DRAFT",
                1 => "APPROVED",
                _ => "REJECTED",
            },
            self.weights,
            self.justification
        )
    }
}

#[cfg(all(test, feature = "v9.4-validation-layer"))]
mod tests {
    use super::*;

    #[test]
    fn test_default_weights_sum_to_one() {
        let w = CalibrationWeights::default_topological();
        let total = w.w_f + w.w_s + w.w_i + w.w_c;
        assert!((total - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_weight_validation_passes() {
        let w = CalibrationWeights::default_topological();
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_weight_validation_fails_bad_sum() {
        let w = CalibrationWeights {
            w_f: 0.5,
            w_s: 0.5,
            w_i: 0.5,
            w_c: 0.5,
        };
        assert!(w.validate().is_err());
    }

    #[test]
    fn test_weight_validation_fails_negative() {
        let w = CalibrationWeights {
            w_f: -0.1,
            w_s: 0.35,
            w_i: 0.35,
            w_c: 0.4,
        };
        assert!(w.validate().is_err());
    }

    #[test]
    fn test_z_axis_perfect_coherence() {
        let result = compute_z_axis(1.0, 1.0, 1.0, 0.0).unwrap();
        assert!(result.z_score > 0.8);
        assert!(result.authorized);
        assert!((result.fairness_contrib - WEIGHT_FAIRNESS).abs() < 1e-6);
        assert!((result.safety_contrib - WEIGHT_SAFETY).abs() < 1e-6);
        assert!((result.interpretability_contrib - WEIGHT_INTERPRETABILITY).abs() < 1e-6);
        assert!((result.conflict_penalty - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_z_axis_zero_coherence() {
        let result = compute_z_axis(0.0, 0.0, 0.0, 1.0).unwrap();
        assert!((result.z_score - Z_MIN_THRESHOLD).abs() < 1e-6);
        assert!((result.conflict_penalty - WEIGHT_CONFLICT).abs() < 1e-6);
    }

    #[test]
    fn test_z_axis_high_conflict() {
        let result = compute_z_axis(0.5, 0.5, 0.5, 1.0).unwrap();
        // Z = 0.3*0.5 + 0.3*0.5 + 0.25*0.5 - 0.15*1.0
        // Z = 0.15 + 0.15 + 0.125 - 0.15 = 0.275
        assert!((result.z_score - 0.275).abs() < 1e-6);
    }

    #[test]
    fn test_z_axis_out_of_range_fairness() {
        assert!(compute_z_axis(1.5, 0.5, 0.5, 0.5).is_err());
    }

    #[test]
    fn test_z_axis_out_of_range_safety() {
        assert!(compute_z_axis(0.5, -0.1, 0.5, 0.5).is_err());
    }

    #[test]
    fn test_z_axis_nan_input() {
        assert!(compute_z_axis(f64::NAN, 0.5, 0.5, 0.5).is_err());
    }

    #[test]
    fn test_z_axis_custom_weights() {
        let weights = CalibrationWeights {
            w_f: 0.4,
            w_s: 0.3,
            w_i: 0.2,
            w_c: 0.1,
        };
        let result = compute_z_axis_custom(1.0, 1.0, 1.0, 0.0, &weights).unwrap();
        // Z = 0.4 + 0.3 + 0.2 - 0.0 = 0.9
        assert!((result.z_score - 0.9).abs() < 1e-6);
        assert_eq!(result.weights.w_f, 0.4);
    }

    #[test]
    fn test_is_authorized_above_threshold() {
        assert!(is_authorized(0.5));
    }

    #[test]
    fn test_is_authorized_at_threshold() {
        assert!(is_authorized(Z_MIN_THRESHOLD));
    }

    #[test]
    fn test_ethical_divergence_zero() {
        assert!((ethical_divergence(0.5, 0.5) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_ethical_divergence_max() {
        assert!((ethical_divergence(0.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rfc_proposal_creation() {
        let weights = CalibrationWeights::default_topological();
        let rfc = RFCProposal::new(1, weights, "Test RFC".to_string()).unwrap();
        assert_eq!(rfc.rfc_id, 1);
        assert_eq!(rfc.status, 0);
        assert!(!rfc.is_active());
    }

    #[test]
    fn test_rfc_proposal_approve() {
        let mut rfc = RFCProposal::new(
            1,
            CalibrationWeights::default_topological(),
            "Test".to_string(),
        )
        .unwrap();
        rfc.approve();
        assert_eq!(rfc.status, 1);
        assert!(rfc.is_active());
    }

    #[test]
    fn test_rfc_proposal_reject() {
        let mut rfc = RFCProposal::new(
            1,
            CalibrationWeights::default_topological(),
            "Test".to_string(),
        )
        .unwrap();
        rfc.reject();
        assert_eq!(rfc.status, 2);
        assert!(!rfc.is_active());
    }

    #[test]
    fn test_rfc_proposal_invalid_weights() {
        let weights = CalibrationWeights {
            w_f: 0.5,
            w_s: 0.5,
            w_i: 0.5,
            w_c: 0.5,
        };
        assert!(RFCProposal::new(1, weights, "Bad".to_string()).is_err());
    }

    #[test]
    fn test_calibration_result_display() {
        let result = compute_z_axis(0.8, 0.9, 0.7, 0.2).unwrap();
        let display = format!("{}", result);
        assert!(display.contains("Z="));
        assert!(display.contains("authorized="));
    }

    #[test]
    fn test_weights_display() {
        let w = CalibrationWeights::default_topological();
        let display = format!("{}", w);
        assert!(display.contains("w_f="));
        assert!(display.contains("w_s="));
    }

    #[test]
    fn test_error_display_invalid_weights() {
        let err = CalibrationError::InvalidWeights {
            expected: 1.0,
            actual: 2.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("sum"));
    }

    #[test]
    fn test_error_display_negative_weight() {
        let err = CalibrationError::NegativeWeight;
        let msg = format!("{}", err);
        assert!(msg.contains("non-negative"));
    }

    #[test]
    fn test_error_display_out_of_range() {
        let err = CalibrationError::OutOfRange {
            field: "test",
            value: 2.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test"));
        assert!(msg.contains("out of range"));
    }

    #[test]
    fn test_error_display_threshold() {
        let err = CalibrationError::ThresholdNotMet {
            z_score: 0.0,
            threshold: 0.5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("below"));
    }

    #[test]
    fn test_z_axis_clamping_to_zero() {
        // When all inputs are 0 and conflict is 1, raw_z = -0.15, clamped to 0.0
        let result = compute_z_axis(0.0, 0.0, 0.0, 1.0).unwrap();
        assert!((result.z_score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_z_axis_clamping_to_one() {
        // When all positive inputs are 1 and conflict is 0, Z should be close to 1.0
        let result = compute_z_axis(1.0, 1.0, 1.0, 0.0).unwrap();
        assert!(result.z_score <= Z_MAX_VALUE);
    }

    #[test]
    fn test_rfc_display() {
        let rfc = RFCProposal::new(
            42,
            CalibrationWeights::default_topological(),
            "Calibration update".to_string(),
        )
        .unwrap();
        let display = format!("{}", rfc);
        assert!(display.contains("RFC-42"));
        assert!(display.contains("DRAFT"));
    }

    #[test]
    fn test_weights_default_impl() {
        let w = CalibrationWeights::default();
        assert!((w.w_f - WEIGHT_FAIRNESS).abs() < 1e-6);
        assert!((w.w_s - WEIGHT_SAFETY).abs() < 1e-6);
        assert!((w.w_i - WEIGHT_INTERPRETABILITY).abs() < 1e-6);
        assert!((w.w_c - WEIGHT_CONFLICT).abs() < 1e-6);
    }
}
