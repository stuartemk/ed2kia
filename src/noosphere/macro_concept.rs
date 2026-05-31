//! MacroConceptBirth — Emergent higher-order concept detection.
//!
//! A MacroConcept is born when three criteria are simultaneously met:
//! 1. **PH₂ persistence** > threshold (topological void structure)
//! 2. **Lyapunov exponent** < 0 (convergence / attractor behavior)
//! 3. **Human correlation** via Steering Bridge > 0.75 (steward validation)
//!
//! Feature gate: `v3.9-noosphere-engine`

use std::collections::HashMap;

/// Default PH₂ persistence threshold for macro-concept emergence.
const DEFAULT_PH2_THRESHOLD: f64 = 0.3;

/// Default Lyapunov threshold (must be below this for convergence).
const DEFAULT_LYAPUNOV_THRESHOLD: f64 = 0.0;

/// Default human correlation threshold for steward validation.
const DEFAULT_HUMAN_THRESHOLD: f64 = 0.75;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum MacroConceptError {
    /// Concept ID already exists.
    DuplicateConcept(u128),
    /// Concept ID not found.
    ConceptNotFound(u128),
    /// PH₂ persistence below threshold.
    InsufficientPersistence { actual: f64, threshold: f64 },
    /// Lyapunov exponent above threshold (diverging).
    DivergingLyapunov { actual: f64, threshold: f64 },
    /// Human correlation below threshold.
    InsufficientHumanCorrelation { actual: f64, threshold: f64 },
    /// Invalid persistence value (must be >= 0).
    InvalidPersistence(f64),
    /// Invalid correlation value (must be in [0, 1]).
    InvalidCorrelation(f64),
}

impl std::fmt::Display for MacroConceptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroConceptError::DuplicateConcept(id) => write!(f, "Duplicate concept {}", id),
            MacroConceptError::ConceptNotFound(id) => write!(f, "Concept {} not found", id),
            MacroConceptError::InsufficientPersistence { actual, threshold } => {
                write!(
                    f,
                    "PH₂ persistence {:.4} < threshold {:.4}",
                    actual, threshold
                )
            }
            MacroConceptError::DivergingLyapunov { actual, threshold } => {
                write!(
                    f,
                    "Lyapunov {:.4} >= threshold {:.4} (diverging)",
                    actual, threshold
                )
            }
            MacroConceptError::InsufficientHumanCorrelation { actual, threshold } => {
                write!(
                    f,
                    "Human correlation {:.4} < threshold {:.4}",
                    actual, threshold
                )
            }
            MacroConceptError::InvalidPersistence(v) => {
                write!(f, "Invalid persistence: {} (must be >= 0)", v)
            }
            MacroConceptError::InvalidCorrelation(v) => {
                write!(f, "Invalid correlation: {} (must be [0,1])", v)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Emergence criteria
// ---------------------------------------------------------------------------

/// The three criteria for MacroConceptBirth.
#[derive(Debug, Clone)]
pub struct EmergenceCriteria {
    /// PH₂ persistence value from HOPH analysis.
    pub ph2_persistence: f64,
    /// Lyapunov exponent (negative = converging attractor).
    pub lyapunov_exponent: f64,
    /// Human steward correlation via Steering Bridge.
    pub human_correlation: f64,
}

impl EmergenceCriteria {
    /// Check if all three criteria are met for birth.
    pub fn meets_birth_requirements(&self, config: &BirthConfig) -> Result<(), MacroConceptError> {
        if self.ph2_persistence < config.ph2_threshold {
            return Err(MacroConceptError::InsufficientPersistence {
                actual: self.ph2_persistence,
                threshold: config.ph2_threshold,
            });
        }
        if self.lyapunov_exponent >= config.lyapunov_threshold {
            return Err(MacroConceptError::DivergingLyapunov {
                actual: self.lyapunov_exponent,
                threshold: config.lyapunov_threshold,
            });
        }
        if self.human_correlation < config.human_threshold {
            return Err(MacroConceptError::InsufficientHumanCorrelation {
                actual: self.human_correlation,
                threshold: config.human_threshold,
            });
        }
        Ok(())
    }

    /// Calculate an overall emergence score in [0, 1].
    /// Higher = stronger case for birth.
    pub fn emergence_score(&self, config: &BirthConfig) -> f64 {
        let ph2_score = (self.ph2_persistence / config.ph2_threshold).min(1.0);
        // Lyapunov: more negative = better. Map [-inf, threshold] → [0, 1].
        let lyap_score = if self.lyapunov_exponent < config.lyapunov_threshold {
            (1.0 - (self.lyapunov_exponent / config.lyapunov_threshold.abs()).tanh()).min(1.0)
        } else {
            0.0
        };
        let human_score = self.human_correlation / config.human_threshold;
        // Weighted average: topology 40%, dynamics 30%, human 30%.
        ph2_score * 0.4 + lyap_score * 0.3 + human_score * 0.3
    }
}

// ---------------------------------------------------------------------------
// MacroConcept state
// ---------------------------------------------------------------------------

/// Lifecycle phase of a MacroConcept.
#[derive(Debug, Clone, PartialEq)]
pub enum ConceptPhase {
    /// Candidate — criteria being evaluated.
    Candidate,
    /// Born — all criteria met, concept is active.
    Born,
    /// Mature — sustained for multiple cycles.
    Mature,
    /// Dissolved — criteria no longer met.
    Dissolved,
}

/// A born MacroConcept in the noosphere.
#[derive(Debug, Clone)]
pub struct MacroConcept {
    /// Unique concept identifier.
    pub id: u128,
    /// Human-readable label.
    pub label: String,
    /// Current lifecycle phase.
    pub phase: ConceptPhase,
    /// Number of noosphere cycles this concept has been sustained.
    pub cycles_sustained: u32,
    /// Last emergence criteria that produced this concept.
    pub criteria: EmergenceCriteria,
    /// Overall emergence score at birth.
    pub birth_score: f64,
}

impl MacroConcept {
    pub fn new_candidate(id: u128, label: String, criteria: EmergenceCriteria) -> Self {
        MacroConcept {
            id,
            label,
            phase: ConceptPhase::Candidate,
            cycles_sustained: 0,
            criteria,
            birth_score: 0.0,
        }
    }

    /// Promote candidate → Born if criteria are met.
    pub fn promote(&mut self, config: &BirthConfig) -> Result<(), MacroConceptError> {
        self.criteria.meets_birth_requirements(config)?;
        self.birth_score = self.criteria.emergence_score(config);
        self.phase = ConceptPhase::Born;
        self.cycles_sustained = 1;
        Ok(())
    }

    /// Advance one noosphere cycle.
    pub fn advance_cycle(&mut self) {
        self.cycles_sustained += 1;
        if self.phase == ConceptPhase::Born && self.cycles_sustained >= 3 {
            self.phase = ConceptPhase::Mature;
        }
    }

    /// Dissolve the concept (criteria no longer met).
    pub fn dissolve(&mut self) {
        self.phase = ConceptPhase::Dissolved;
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for MacroConceptBirth thresholds.
#[derive(Debug, Clone)]
pub struct BirthConfig {
    /// Minimum PH₂ persistence for emergence.
    pub ph2_threshold: f64,
    /// Maximum Lyapunov exponent for convergence.
    pub lyapunov_threshold: f64,
    /// Minimum human correlation for steward validation.
    pub human_threshold: f64,
}

impl Default for BirthConfig {
    fn default() -> Self {
        BirthConfig {
            ph2_threshold: DEFAULT_PH2_THRESHOLD,
            lyapunov_threshold: DEFAULT_LYAPUNOV_THRESHOLD,
            human_threshold: DEFAULT_HUMAN_THRESHOLD,
        }
    }
}

// ---------------------------------------------------------------------------
// MacroConceptBirth engine
// ---------------------------------------------------------------------------

/// Engine that evaluates and manages MacroConcept emergence.
#[derive(Debug, Clone)]
pub struct MacroConceptBirth {
    config: BirthConfig,
    concepts: HashMap<u128, MacroConcept>,
    /// Monotonically increasing counter for concept IDs.
    next_id: u128,
}

impl MacroConceptBirth {
    /// Create with default thresholds.
    pub fn new() -> Self {
        Self::with_config(BirthConfig::default())
    }

    /// Create with explicit thresholds.
    pub fn with_config(config: BirthConfig) -> Self {
        MacroConceptBirth {
            config,
            concepts: HashMap::new(),
            next_id: 1,
        }
    }

    /// Submit a new candidate concept for evaluation.
    pub fn submit_candidate(
        &mut self,
        label: String,
        criteria: EmergenceCriteria,
    ) -> Result<u128, MacroConceptError> {
        let id = self.next_id;
        self.next_id += 1;

        let concept = MacroConcept::new_candidate(id, label, criteria);
        self.concepts.insert(id, concept);
        Ok(id)
    }

    /// Evaluate all candidates and promote those that meet criteria.
    /// Returns the IDs of newly born concepts.
    pub fn evaluate_candidates(&mut self) -> Vec<u128> {
        let candidates: Vec<u128> = self
            .concepts
            .iter()
            .filter(|(_, c)| c.phase == ConceptPhase::Candidate)
            .map(|(&id, _)| id)
            .collect();

        let mut born = Vec::new();
        for id in candidates {
            if let Some(concept) = self.concepts.get_mut(&id) {
                match concept.promote(&self.config) {
                    Ok(()) => born.push(id),
                    Err(_) => {
                        // Failed criteria — dissolve.
                        concept.dissolve();
                    }
                }
            }
        }
        born
    }

    /// Advance all active concepts by one cycle.
    pub fn advance_cycles(&mut self) {
        for concept in self.concepts.values_mut() {
            if concept.phase == ConceptPhase::Born || concept.phase == ConceptPhase::Mature {
                concept.advance_cycle();
            }
        }
    }

    /// Get a concept by ID.
    pub fn get(&self, id: u128) -> Option<&MacroConcept> {
        self.concepts.get(&id)
    }

    /// Remove and return a concept.
    pub fn remove(&mut self, id: u128) -> Option<MacroConcept> {
        self.concepts.remove(&id)
    }

    /// Count active (Born or Mature) concepts.
    pub fn active_count(&self) -> usize {
        self.concepts
            .values()
            .filter(|c| c.phase == ConceptPhase::Born || c.phase == ConceptPhase::Mature)
            .count()
    }

    /// Count dissolved concepts.
    pub fn dissolved_count(&self) -> usize {
        self.concepts
            .values()
            .filter(|c| c.phase == ConceptPhase::Dissolved)
            .count()
    }

    /// Iterate over all concepts.
    pub fn iter(&self) -> impl Iterator<Item = (&u128, &MacroConcept)> {
        self.concepts.iter()
    }

    /// Reset the engine.
    pub fn reset(&mut self) {
        self.concepts.clear();
        self.next_id = 1;
    }

    /// Update configuration thresholds.
    pub fn update_config(&mut self, config: BirthConfig) {
        self.config = config;
    }
}

impl Default for MacroConceptBirth {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_criteria() -> EmergenceCriteria {
        EmergenceCriteria {
            ph2_persistence: 0.5,
            lyapunov_exponent: -0.2,
            human_correlation: 0.9,
        }
    }

    fn failing_criteria() -> EmergenceCriteria {
        EmergenceCriteria {
            ph2_persistence: 0.1,
            lyapunov_exponent: 0.5,
            human_correlation: 0.3,
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = MacroConceptBirth::new();
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn test_engine_custom_config() {
        let config = BirthConfig {
            ph2_threshold: 0.5,
            lyapunov_threshold: -0.1,
            human_threshold: 0.8,
        };
        let engine = MacroConceptBirth::with_config(config);
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn test_submit_candidate() {
        let mut engine = MacroConceptBirth::new();
        let id = engine
            .submit_candidate("test concept".into(), valid_criteria())
            .unwrap();
        assert_eq!(id, 1);
        let concept = engine.get(id).unwrap();
        assert_eq!(concept.phase, ConceptPhase::Candidate);
    }

    #[test]
    fn test_promote_valid_candidate() {
        let mut engine = MacroConceptBirth::new();
        let id = engine
            .submit_candidate("strong concept".into(), valid_criteria())
            .unwrap();
        let born = engine.evaluate_candidates();
        assert_eq!(born, vec![id]);
        assert_eq!(engine.get(id).unwrap().phase, ConceptPhase::Born);
    }

    #[test]
    fn test_dissolve_failing_candidate() {
        let mut engine = MacroConceptBirth::new();
        let id = engine
            .submit_candidate("weak concept".into(), failing_criteria())
            .unwrap();
        let born = engine.evaluate_candidates();
        assert!(born.is_empty());
        assert_eq!(engine.get(id).unwrap().phase, ConceptPhase::Dissolved);
    }

    #[test]
    fn test_mature_after_three_cycles() {
        let mut engine = MacroConceptBirth::new();
        let id = engine
            .submit_candidate("enduring".into(), valid_criteria())
            .unwrap();
        engine.evaluate_candidates();
        assert_eq!(engine.get(id).unwrap().phase, ConceptPhase::Born);
        engine.advance_cycles(); // cycle 2
        engine.advance_cycles(); // cycle 3
        engine.advance_cycles(); // cycle 4 → mature
        assert_eq!(engine.get(id).unwrap().phase, ConceptPhase::Mature);
    }

    #[test]
    fn test_emergence_score_valid() {
        let criteria = valid_criteria();
        let config = BirthConfig::default();
        let score = criteria.emergence_score(&config);
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_meets_birth_requirements_pass() {
        let criteria = valid_criteria();
        let config = BirthConfig::default();
        assert!(criteria.meets_birth_requirements(&config).is_ok());
    }

    #[test]
    fn test_insufficient_persistence() {
        let criteria = EmergenceCriteria {
            ph2_persistence: 0.05,
            lyapunov_exponent: -0.5,
            human_correlation: 0.9,
        };
        let config = BirthConfig::default();
        match criteria.meets_birth_requirements(&config) {
            Err(MacroConceptError::InsufficientPersistence { .. }) => {}
            other => panic!("Expected InsufficientPersistence, got {:?}", other),
        }
    }

    #[test]
    fn test_diverging_lyapunov() {
        let criteria = EmergenceCriteria {
            ph2_persistence: 0.5,
            lyapunov_exponent: 0.3,
            human_correlation: 0.9,
        };
        let config = BirthConfig::default();
        match criteria.meets_birth_requirements(&config) {
            Err(MacroConceptError::DivergingLyapunov { .. }) => {}
            other => panic!("Expected DivergingLyapunov, got {:?}", other),
        }
    }

    #[test]
    fn test_insufficient_human_correlation() {
        let criteria = EmergenceCriteria {
            ph2_persistence: 0.5,
            lyapunov_exponent: -0.3,
            human_correlation: 0.4,
        };
        let config = BirthConfig::default();
        match criteria.meets_birth_requirements(&config) {
            Err(MacroConceptError::InsufficientHumanCorrelation { .. }) => {}
            other => panic!("Expected InsufficientHumanCorrelation, got {:?}", other),
        }
    }

    #[test]
    fn test_active_count() {
        let mut engine = MacroConceptBirth::new();
        engine
            .submit_candidate("a".into(), valid_criteria())
            .unwrap();
        engine
            .submit_candidate("b".into(), valid_criteria())
            .unwrap();
        engine.evaluate_candidates();
        assert_eq!(engine.active_count(), 2);
    }

    #[test]
    fn test_dissolved_count() {
        let mut engine = MacroConceptBirth::new();
        engine
            .submit_candidate("weak".into(), failing_criteria())
            .unwrap();
        engine.evaluate_candidates();
        assert_eq!(engine.dissolved_count(), 1);
    }

    #[test]
    fn test_remove_concept() {
        let mut engine = MacroConceptBirth::new();
        let id = engine
            .submit_candidate("temp".into(), valid_criteria())
            .unwrap();
        let removed = engine.remove(id).unwrap();
        assert_eq!(removed.id, id);
        assert!(engine.get(id).is_none());
    }

    #[test]
    fn test_reset() {
        let mut engine = MacroConceptBirth::new();
        engine
            .submit_candidate("x".into(), valid_criteria())
            .unwrap();
        engine.reset();
        assert_eq!(engine.active_count(), 0);
        assert_eq!(engine.dissolved_count(), 0);
    }

    #[test]
    fn test_concept_default() {
        let engine = MacroConceptBirth::default();
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = MacroConceptError::InsufficientPersistence {
            actual: 0.1,
            threshold: 0.3,
        };
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_iter() {
        let mut engine = MacroConceptBirth::new();
        engine
            .submit_candidate("a".into(), valid_criteria())
            .unwrap();
        engine
            .submit_candidate("b".into(), valid_criteria())
            .unwrap();
        assert_eq!(engine.iter().count(), 2);
    }

    #[test]
    fn test_update_config() {
        let mut engine = MacroConceptBirth::new();
        let new_config = BirthConfig {
            ph2_threshold: 0.1,
            lyapunov_threshold: 0.0,
            human_threshold: 0.5,
        };
        engine.update_config(new_config.clone());
        // Now previously failing criteria should pass.
        let criteria = EmergenceCriteria {
            ph2_persistence: 0.15,
            lyapunov_exponent: -0.1,
            human_correlation: 0.6,
        };
        assert!(criteria.meets_birth_requirements(&new_config).is_ok());
    }

    #[test]
    fn test_birth_score_recorded() {
        let mut engine = MacroConceptBirth::new();
        let id = engine
            .submit_candidate("scored".into(), valid_criteria())
            .unwrap();
        engine.evaluate_candidates();
        let concept = engine.get(id).unwrap();
        assert!(concept.birth_score > 0.0);
    }
}
