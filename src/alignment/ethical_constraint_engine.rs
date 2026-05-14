//! Ethical Constraint Engine — Motor de restricciones éticas para fine-tuning
//!
//! Define y aplica restricciones éticas durante el fine-tuning de modelos,
//! incluyendo límites de valores, enmascaramiento de características y
//! umbrales de alineación.

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintError {
    ConstraintNotFound(String),
    InvalidConstraint(String),
    ViolationHalt(String),
    CorrectionFailed(String),
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintError::ConstraintNotFound(id) => write!(f, "Constraint '{}' not found", id),
            ConstraintError::InvalidConstraint(msg) => write!(f, "Invalid constraint: {}", msg),
            ConstraintError::ViolationHalt(msg) => write!(f, "Halt violation: {}", msg),
            ConstraintError::CorrectionFailed(msg) => write!(f, "Correction failed: {}", msg),
        }
    }
}

impl std::error::Error for ConstraintError {}

// ============================================================================
// Constraint Severity
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintSeverity {
    Warning,
    Correction,
    Halt,
}

impl fmt::Display for ConstraintSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintSeverity::Warning => write!(f, "Warning"),
            ConstraintSeverity::Correction => write!(f, "Correction"),
            ConstraintSeverity::Halt => write!(f, "Halt"),
        }
    }
}

// ============================================================================
// Constraint Type
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintType {
    ValueBound {
        min: f32,
        max: f32,
        feature_index: usize,
    },
    FeatureMask {
        masked_features: Vec<usize>,
    },
    AlignmentThreshold {
        min_alignment: f32,
    },
    DivergenceLimit {
        max_divergence: f32,
    },
}

impl fmt::Display for ConstraintType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintType::ValueBound {
                min,
                max,
                feature_index,
            } => write!(
                f,
                "ValueBound(idx={}, min={}, max={})",
                feature_index, min, max
            ),
            ConstraintType::FeatureMask { masked_features } => {
                write!(f, "FeatureMask(count={})", masked_features.len())
            }
            ConstraintType::AlignmentThreshold { min_alignment } => {
                write!(f, "AlignmentThreshold(min={})", min_alignment)
            }
            ConstraintType::DivergenceLimit { max_divergence } => {
                write!(f, "DivergenceLimit(max={})", max_divergence)
            }
        }
    }
}

// ============================================================================
// Ethical Constraint
// ============================================================================

#[derive(Debug, Clone)]
pub struct EthicalConstraint {
    pub id: String,
    pub constraint_type: ConstraintType,
    pub parameters: HashMap<String, f32>,
    pub severity: ConstraintSeverity,
}

impl EthicalConstraint {
    pub fn new(id: String, constraint_type: ConstraintType, severity: ConstraintSeverity) -> Self {
        Self {
            id,
            constraint_type,
            parameters: HashMap::new(),
            severity,
        }
    }

    pub fn with_parameter(mut self, key: String, value: f32) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

// ============================================================================
// Constraint Violation
// ============================================================================

#[derive(Debug, Clone)]
pub struct ConstraintViolation {
    pub constraint_id: String,
    pub violation_type: String,
    pub severity: ConstraintSeverity,
    pub timestamp: Instant,
    pub corrected: bool,
}

impl ConstraintViolation {
    pub fn new(constraint_id: String, violation_type: String, severity: ConstraintSeverity) -> Self {
        Self {
            constraint_id,
            violation_type,
            severity,
            timestamp: Instant::now(),
            corrected: false,
        }
    }
}

// ============================================================================
// Constraint Engine
// ============================================================================

pub struct ConstraintEngine {
    constraints: Vec<EthicalConstraint>,
    violations: Vec<ConstraintViolation>,
}

impl ConstraintEngine {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            violations: Vec::new(),
        }
    }

    /// Add a constraint to the engine
    pub fn add_constraint(&mut self, constraint: EthicalConstraint) {
        self.constraints.push(constraint);
    }

    /// Remove a constraint by ID
    pub fn remove_constraint(&mut self, constraint_id: &str) -> Result<(), ConstraintError> {
        let initial_len = self.constraints.len();
        self.constraints
            .retain(|c| c.id != constraint_id);
        if self.constraints.len() == initial_len {
            return Err(ConstraintError::ConstraintNotFound(
                constraint_id.to_string(),
            ));
        }
        Ok(())
    }

    /// Validate a gradient against all constraints
    pub fn validate_gradient(&mut self, gradient: &[f32]) -> Result<(), ConstraintViolation> {
        for constraint in &self.constraints {
            let violation = self.check_constraint(constraint, gradient);
            if let Some(violation) = violation {
                self.violations.push(violation.clone());
                return Err(violation);
            }
        }
        Ok(())
    }

    /// Correct a gradient to satisfy constraints
    pub fn correct_gradient(&mut self, gradient: &mut [f32]) -> Result<(), ConstraintViolation> {
        for constraint in &self.constraints {
            match &constraint.constraint_type {
                ConstraintType::ValueBound {
                    min,
                    max,
                    feature_index,
                } => {
                    if *feature_index < gradient.len() {
                        if gradient[*feature_index] < *min {
                            gradient[*feature_index] = *min;
                        } else if gradient[*feature_index] > *max {
                            gradient[*feature_index] = *max;
                        }
                    }
                }
                ConstraintType::FeatureMask { masked_features } => {
                    for &idx in masked_features {
                        if idx < gradient.len() {
                            gradient[idx] = 0.0;
                        }
                    }
                }
                ConstraintType::AlignmentThreshold { min_alignment } => {
                    let alignment = self.compute_alignment(gradient);
                    if alignment < *min_alignment {
                        let violation = ConstraintViolation::new(
                            constraint.id.clone(),
                            "Alignment below threshold".to_string(),
                            constraint.severity.clone(),
                        );
                        self.violations.push(violation.clone());
                        if constraint.severity == ConstraintSeverity::Halt {
                            return Err(violation);
                        }
                    }
                }
                ConstraintType::DivergenceLimit { max_divergence } => {
                    let norm = self.compute_norm(gradient);
                    if norm > *max_divergence {
                        // Scale down to fit within limit
                        let scale = *max_divergence / norm;
                        for val in gradient.iter_mut() {
                            *val *= scale;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get all recorded violations
    pub fn get_violations(&self) -> &[ConstraintViolation] {
        &self.violations
    }

    /// Check if training should halt based on violations
    pub fn should_halt(&self) -> bool {
        self.violations
            .iter()
            .any(|v| v.severity == ConstraintSeverity::Halt && !v.corrected)
    }

    /// Clear all recorded violations
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }

    /// Get the number of active constraints
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// Get a constraint by ID
    pub fn get_constraint(&self, id: &str) -> Option<&EthicalConstraint> {
        self.constraints.iter().find(|c| c.id == id)
    }

    // ---- Private helpers ----

    fn check_constraint(
        &self,
        constraint: &EthicalConstraint,
        gradient: &[f32],
    ) -> Option<ConstraintViolation> {
        match &constraint.constraint_type {
            ConstraintType::ValueBound {
                min,
                max,
                feature_index,
            } => {
                if *feature_index < gradient.len() {
                    let value = gradient[*feature_index];
                    if value < *min || value > *max {
                        return Some(ConstraintViolation::new(
                            constraint.id.clone(),
                            format!(
                                "Value {} out of bounds [{}, {}] at index {}",
                                value, min, max, feature_index
                            ),
                            constraint.severity.clone(),
                        ));
                    }
                }
            }
            ConstraintType::FeatureMask { masked_features } => {
                for &idx in masked_features {
                    if idx < gradient.len() && gradient[idx] != 0.0 {
                        return Some(ConstraintViolation::new(
                            constraint.id.clone(),
                            format!("Feature {} should be masked (zero)", idx),
                            constraint.severity.clone(),
                        ));
                    }
                }
            }
            ConstraintType::AlignmentThreshold { min_alignment } => {
                let alignment = self.compute_alignment(gradient);
                if alignment < *min_alignment {
                    return Some(ConstraintViolation::new(
                        constraint.id.clone(),
                        format!(
                            "Alignment {} below threshold {}",
                            alignment, min_alignment
                        ),
                        constraint.severity.clone(),
                    ));
                }
            }
            ConstraintType::DivergenceLimit { max_divergence } => {
                let norm = self.compute_norm(gradient);
                if norm > *max_divergence {
                    return Some(ConstraintViolation::new(
                        constraint.id.clone(),
                        format!("Norm {} exceeds limit {}", norm, max_divergence),
                        constraint.severity.clone(),
                    ));
                }
            }
        }
        None
    }

    fn compute_alignment(&self, gradient: &[f32]) -> f32 {
        if gradient.is_empty() {
            return 0.0;
        }
        let sum: f32 = gradient.iter().map(|v| v.abs()).sum();
        let max_val = gradient.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        if max_val == 0.0 {
            return 1.0;
        }
        1.0 - (sum / gradient.len() as f32) / max_val
    }

    fn compute_norm(&self, gradient: &[f32]) -> f32 {
        gradient.iter().map(|v| v * v).sum::<f32>().sqrt()
    }
}

impl Default for ConstraintEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ConstraintEngine::new();
        assert_eq!(engine.constraint_count(), 0);
        assert!(engine.get_violations().is_empty());
    }

    #[test]
    fn test_add_constraint() {
        let mut engine = ConstraintEngine::new();
        let constraint = EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        );
        engine.add_constraint(constraint);
        assert_eq!(engine.constraint_count(), 1);
    }

    #[test]
    fn test_remove_constraint() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        ));
        assert!(engine.remove_constraint("bound-1").is_ok());
        assert_eq!(engine.constraint_count(), 0);
    }

    #[test]
    fn test_remove_nonexistent_constraint() {
        let mut engine = ConstraintEngine::new();
        match engine.remove_constraint("nonexistent") {
            Err(ConstraintError::ConstraintNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("Expected ConstraintNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_gradient_within_bounds() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        ));
        let gradient = vec![0.5, 0.3, 0.1];
        assert!(engine.validate_gradient(&gradient).is_ok());
    }

    #[test]
    fn test_validate_gradient_out_of_bounds() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        ));
        let gradient = vec![2.0, 0.3, 0.1];
        assert!(engine.validate_gradient(&gradient).is_err());
    }

    #[test]
    fn test_correct_gradient_value_bound() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Correction,
        ));
        let mut gradient = vec![2.0, 0.3, -3.0];
        engine.correct_gradient(&mut gradient).unwrap();
        assert_eq!(gradient[0], 1.0); // Clamped to max
    }

    #[test]
    fn test_correct_gradient_feature_mask() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "mask-1".to_string(),
            ConstraintType::FeatureMask {
                masked_features: vec![1, 2],
            },
            ConstraintSeverity::Correction,
        ));
        let mut gradient = vec![0.5, 0.3, 0.1, 0.9];
        engine.correct_gradient(&mut gradient).unwrap();
        assert_eq!(gradient[1], 0.0);
        assert_eq!(gradient[2], 0.0);
        assert_eq!(gradient[3], 0.9); // Unmasked
    }

    #[test]
    fn test_divergence_limit_correction() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "div-1".to_string(),
            ConstraintType::DivergenceLimit {
                max_divergence: 1.0,
            },
            ConstraintSeverity::Correction,
        ));
        let mut gradient = vec![3.0, 4.0]; // norm = 5.0
        engine.correct_gradient(&mut gradient).unwrap();
        // Should be scaled to norm = 1.0
        let norm: f32 = (gradient[0] * gradient[0] + gradient[1] * gradient[1]).sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_should_halt() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "halt-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Halt,
        ));
        let gradient = vec![5.0];
        engine.validate_gradient(&gradient);
        assert!(engine.should_halt());
    }

    #[test]
    fn test_clear_violations() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        ));
        engine.validate_gradient(&[5.0]);
        assert!(!engine.get_violations().is_empty());
        engine.clear_violations();
        assert!(engine.get_violations().is_empty());
    }

    #[test]
    fn test_get_constraint() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        ));
        assert!(engine.get_constraint("bound-1").is_some());
        assert!(engine.get_constraint("nonexistent").is_none());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", ConstraintSeverity::Warning), "Warning");
        assert_eq!(
            format!("{}", ConstraintSeverity::Correction),
            "Correction"
        );
        assert_eq!(format!("{}", ConstraintSeverity::Halt), "Halt");
    }

    #[test]
    fn test_constraint_type_display() {
        let ct = ConstraintType::ValueBound {
            min: -1.0,
            max: 1.0,
            feature_index: 0,
        };
        assert!(format!("{}", ct).contains("ValueBound"));
    }

    #[test]
    fn test_multiple_constraints() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "bound-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        ));
        engine.add_constraint(EthicalConstraint::new(
            "mask-1".to_string(),
            ConstraintType::FeatureMask {
                masked_features: vec![1],
            },
            ConstraintSeverity::Warning,
        ));
        assert_eq!(engine.constraint_count(), 2);
    }

    #[test]
    fn test_constraint_with_parameters() {
        let constraint = EthicalConstraint::new(
            "param-1".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        )
        .with_parameter("threshold".to_string(), 0.5)
        .with_parameter("margin".to_string(), 0.1);
        assert_eq!(constraint.parameters.get("threshold").unwrap(), &0.5);
    }

    #[test]
    fn test_violation_creation() {
        let violation = ConstraintViolation::new(
            "bound-1".to_string(),
            "Out of bounds".to_string(),
            ConstraintSeverity::Warning,
        );
        assert_eq!(violation.constraint_id, "bound-1");
        assert!(!violation.corrected);
    }

    #[test]
    fn test_error_display() {
        let err = ConstraintError::ConstraintNotFound("x".to_string());
        assert!(format!("{}", err).contains("x"));
    }

    #[test]
    fn test_alignment_threshold_pass() {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "align-1".to_string(),
            ConstraintType::AlignmentThreshold {
                min_alignment: 0.0,
            },
            ConstraintSeverity::Warning,
        ));
        let gradient = vec![1.0, 0.5, 0.2];
        assert!(engine.validate_gradient(&gradient).is_ok());
    }

    #[test]
    fn test_engine_default() {
        let engine = ConstraintEngine::default();
        assert_eq!(engine.constraint_count(), 0);
    }
}
