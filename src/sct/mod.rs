//! SCT — Stuartian Coherence Tensor module.
//!
//! Contains the Z-axis calibration layer for auditable ethical coherence computation.

#[cfg(feature = "v9.4-validation-layer")]
pub mod calibration_layer;

#[cfg(feature = "v9.4-validation-layer")]
pub use calibration_layer::{
    compute_z_axis, compute_z_axis_custom, ethical_divergence, is_authorized, CalibrationError,
    CalibrationResult, CalibrationWeights, RFCProposal, WEIGHT_CONFLICT, WEIGHT_FAIRNESS,
    WEIGHT_INTERPRETABILITY, WEIGHT_SAFETY, Z_MAX_VALUE, Z_MIN_THRESHOLD,
};
