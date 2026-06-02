//! Symbiotic Diversity Loss — Sprint 76: Ontological Debugging & Thermodynamic Pivots
//!
//! Resuelve el bug ontológico: `Love = Zero Conflict` → muerte térmica / mode collapse.
//!
//! La optimización de Pareto diferencia entre fricción generativa (constructiva)
//! y conflicto destructivo (aniquilación). La pérdida se formula como:
//!
//! ```text
//! L = max(Diversidad) - λ · Conflicto_Destructivo
//! ```
//!
//! La fricción constructiva NO es penalizada, permitiendo evolución sin colapso.
//!
//! # Garantías
//!
//! - Complejidad: O(n²) para diversidad de Shannon, O(1) para pérdida Pareto
//! - Memoria: O(n) para distribución de nichos
//! - La pérdida puede ser negativa (equilibrio cooperativo)

use std::fmt;

/// Error types for Symbiotic Diversity Loss
#[derive(Debug, Clone, PartialEq)]
pub enum DiversityError {
    /// Empty distribution provided
    EmptyDistribution,
    /// Invalid lambda weight (must be >= 0)
    InvalidLambda(f64),
    /// Invalid diversity metric (must be >= 0)
    InvalidDiversity(f64),
    /// Distribution contains negative values
    NegativeProbability,
    /// Distribution does not sum to 1
    InvalidNormalization(f64),
}

impl fmt::Display for DiversityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiversityError::EmptyDistribution => write!(f, "Empty distribution"),
            DiversityError::InvalidLambda(l) => write!(f, "Invalid lambda weight: {}", l),
            DiversityError::InvalidDiversity(d) => write!(f, "Invalid diversity metric: {}", d),
            DiversityError::NegativeProbability => {
                write!(f, "Distribution contains negative values")
            }
            DiversityError::InvalidNormalization(s) => {
                write!(f, "Distribution sum {} != 1.0", s)
            }
        }
    }
}

impl std::error::Error for DiversityError {}

/// Configuration for Pareto loss computation.
#[derive(Debug, Clone)]
pub struct ParetoConfig {
    /// Weight for destructive conflict penalty (λ).
    pub lambda: f64,
    /// Minimum acceptable diversity (Shannon entropy).
    pub min_diversity: f64,
    /// Tolerance for constructive friction (non-penalized).
    pub friction_tolerance: f64,
    /// Epsilon to avoid log(0).
    pub epsilon: f64,
}

impl ParetoConfig {
    /// Default Stuartian configuration.
    pub fn default_stuartian() -> Self {
        Self {
            lambda: 0.5,
            min_diversity: 0.1,
            friction_tolerance: 0.3,
            epsilon: 1e-9,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), DiversityError> {
        if self.lambda < 0.0 {
            return Err(DiversityError::InvalidLambda(self.lambda));
        }
        if self.min_diversity < 0.0 {
            return Err(DiversityError::InvalidDiversity(self.min_diversity));
        }
        Ok(())
    }
}

impl Default for ParetoConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// Record of a Pareto loss computation.
#[derive(Debug, Clone)]
pub struct LossRecord {
    /// Computed diversity metric (Shannon entropy).
    pub diversity: f64,
    /// Destructive conflict component.
    pub destructive_conflict: f64,
    /// Constructive friction component (non-penalized).
    pub constructive_friction: f64,
    /// Final Pareto loss value.
    pub loss: f64,
    /// Whether the system is in cooperative equilibrium.
    pub cooperative: bool,
}

impl fmt::Display for LossRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LossRecord {{ diversity={:.4}, destructive={:.4}, friction={:.4}, loss={:.4}, cooperative={} }}",
            self.diversity, self.destructive_conflict, self.constructive_friction, self.loss, self.cooperative
        )
    }
}

/// Stateful engine for tracking diversity loss over time.
#[derive(Debug, Clone)]
pub struct SymbioticDiversityLoss {
    config: ParetoConfig,
    records: Vec<LossRecord>,
}

impl SymbioticDiversityLoss {
    /// Create a new engine with default Stuartian configuration.
    pub fn new() -> Self {
        Self {
            config: ParetoConfig::default_stuartian(),
            records: Vec::new(),
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(config: ParetoConfig) -> Result<Self, DiversityError> {
        config.validate()?;
        Ok(Self {
            config,
            records: Vec::new(),
        })
    }

    /// Compute the Pareto loss and record the result.
    ///
    /// # Arguments
    /// * `niche_distribution` — Probability distribution across niches (diversity proxy).
    /// * `destructive_conflict` — Measured destructive conflict (penalized).
    /// * `constructive_friction` — Measured constructive friction (non-penalized).
    pub fn compute(
        &mut self,
        niche_distribution: &[f64],
        destructive_conflict: f64,
        constructive_friction: f64,
    ) -> Result<LossRecord, DiversityError> {
        let diversity = shannon_entropy(niche_distribution, self.config.epsilon)?;
        let loss = compute_pareto_loss(
            diversity,
            destructive_conflict,
            constructive_friction,
            self.config.lambda,
        );
        let cooperative = loss <= 0.0 && diversity >= self.config.min_diversity;
        let record = LossRecord {
            diversity,
            destructive_conflict,
            constructive_friction,
            loss,
            cooperative,
        };
        self.records.push(record.clone());
        Ok(record)
    }

    /// Compute without recording (stateless).
    pub fn compute_instant(
        &self,
        niche_distribution: &[f64],
        destructive_conflict: f64,
        constructive_friction: f64,
    ) -> Result<LossRecord, DiversityError> {
        let diversity = shannon_entropy(niche_distribution, self.config.epsilon)?;
        let loss = compute_pareto_loss(
            diversity,
            destructive_conflict,
            constructive_friction,
            self.config.lambda,
        );
        let cooperative = loss <= 0.0 && diversity >= self.config.min_diversity;
        Ok(LossRecord {
            diversity,
            destructive_conflict,
            constructive_friction,
            loss,
            cooperative,
        })
    }

    /// Average loss across all recorded computations.
    pub fn average_loss(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let sum: f64 = self.records.iter().map(|r| r.loss).sum();
        Some(sum / self.records.len() as f64)
    }

    /// Average diversity across all recorded computations.
    pub fn average_diversity(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let sum: f64 = self.records.iter().map(|r| r.diversity).sum();
        Some(sum / self.records.len() as f64)
    }

    /// Rate of cooperative equilibrium across records.
    pub fn cooperative_rate(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let count = self.records.iter().filter(|r| r.cooperative).count();
        Some(count as f64 / self.records.len() as f64)
    }

    /// Latest loss record.
    pub fn latest(&self) -> Option<&LossRecord> {
        self.records.last()
    }

    /// Number of recorded computations.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Reset all records.
    pub fn reset(&mut self) {
        self.records.clear();
    }
}

impl Default for SymbioticDiversityLoss {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SymbioticDiversityLoss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SymbioticDiversityLoss {{ records={}, avg_loss={:?}, avg_diversity={:?}, cooperative_rate={:?} }}",
            self.record_count(),
            self.average_loss(),
            self.average_diversity(),
            self.cooperative_rate()
        )
    }
}

// ─── Public Standalone Functions ───────────────────────────────────────────────

/// Compute the Pareto loss: L = max(Diversidad) - λ · Conflicto_Destructivo.
///
/// La fricción constructiva NO es penalizada, permitiendo evolución sin colapso.
///
/// # Arguments
/// * `diversity_metric` — Shannon entropy or other diversity measure (>= 0).
/// * `destructive_conflict` — Destructive conflict component (penalized).
/// * `constructive_friction` — Constructive friction (non-penalized, informational).
/// * `lambda` — Weight for destructive conflict (>= 0).
///
/// # Returns
/// Loss value. Negative indicates cooperative equilibrium.
pub fn compute_pareto_loss(
    diversity_metric: f64,
    destructive_conflict: f64,
    _constructive_friction: f64,
    lambda: f64,
) -> f64 {
    // L = -diversity + λ · destructive_conflict
    // Negamos diversidad porque queremos maximizarla (minimizar pérdida).
    // Fricción constructiva no penalizada.
    -diversity_metric + lambda * destructive_conflict
}

/// Compute Shannon entropy of a probability distribution.
///
/// H = -Σ p_i · log(p_i)
///
/// # Arguments
/// * `distribution` — Probability distribution (must sum to ~1.0, no negatives).
/// * `epsilon` — Small value to avoid log(0).
pub fn shannon_entropy(distribution: &[f64], epsilon: f64) -> Result<f64, DiversityError> {
    if distribution.is_empty() {
        return Err(DiversityError::EmptyDistribution);
    }
    for &p in distribution {
        if p < -epsilon {
            return Err(DiversityError::NegativeProbability);
        }
    }
    let mut entropy = 0.0;
    for &p in distribution {
        let p_clamped = p.max(epsilon);
        entropy -= p_clamped * p_clamped.log2();
    }
    Ok(entropy)
}

/// Compute normalized Shannon entropy (0 to 1).
pub fn normalized_shannon_entropy(
    distribution: &[f64],
    epsilon: f64,
) -> Result<f64, DiversityError> {
    let entropy = shannon_entropy(distribution, epsilon)?;
    let max_entropy = (distribution.len() as f64).log2().max(epsilon);
    Ok(entropy / max_entropy)
}

/// Check if the system is in cooperative equilibrium.
///
/// Cooperative when loss <= 0 AND diversity >= min_diversity.
pub fn is_cooperative_equilibrium(loss: f64, diversity: f64, min_diversity: f64) -> bool {
    loss <= 0.0 && diversity >= min_diversity
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_distribution() -> Vec<f64> {
        vec![0.25, 0.25, 0.25, 0.25]
    }

    fn skewed_distribution() -> Vec<f64> {
        vec![0.7, 0.15, 0.1, 0.05]
    }

    #[test]
    fn test_config_default() {
        let config = ParetoConfig::default_stuartian();
        assert!(config.validate().is_ok());
        assert_eq!(config.lambda, 0.5);
    }

    #[test]
    fn test_config_negative_lambda() {
        let config = ParetoConfig {
            lambda: -1.0,
            ..ParetoConfig::default_stuartian()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_shannon_entropy_uniform() {
        let dist = valid_distribution();
        let entropy = shannon_entropy(&dist, 1e-9).unwrap();
        // Uniform distribution of 4 elements: H = log2(4) = 2.0
        assert!((entropy - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_shannon_entropy_skewed() {
        let dist = skewed_distribution();
        let entropy = shannon_entropy(&dist, 1e-9).unwrap();
        // Skewed distribution has lower entropy than uniform
        assert!(entropy < 2.0);
        assert!(entropy > 0.0);
    }

    #[test]
    fn test_shannon_entropy_empty() {
        let dist: Vec<f64> = vec![];
        assert!(shannon_entropy(&dist, 1e-9).is_err());
    }

    #[test]
    fn test_shannon_entropy_negative() {
        let dist = vec![-0.1, 0.5, 0.5];
        assert!(shannon_entropy(&dist, 1e-9).is_err());
    }

    #[test]
    fn test_normalized_entropy_uniform() {
        let dist = valid_distribution();
        let norm = normalized_shannon_entropy(&dist, 1e-9).unwrap();
        // Uniform distribution should have normalized entropy ~1.0
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalized_entropy_skewed() {
        let dist = skewed_distribution();
        let norm = normalized_shannon_entropy(&dist, 1e-9).unwrap();
        assert!(norm < 1.0);
        assert!(norm > 0.0);
    }

    #[test]
    fn test_pareto_loss_cooperative() {
        // High diversity, low destructive conflict → negative loss (cooperative)
        let loss = compute_pareto_loss(2.0, 0.1, 0.5, 0.5);
        assert!(loss < 0.0);
    }

    #[test]
    fn test_pareto_loss_destructive() {
        // Low diversity, high destructive conflict → positive loss
        let loss = compute_pareto_loss(0.5, 2.0, 0.1, 0.5);
        assert!(loss > 0.0);
    }

    #[test]
    fn test_pareto_loss_ignores_constructive_friction() {
        // Constructive friction should not affect loss
        let loss1 = compute_pareto_loss(1.0, 0.5, 0.0, 0.5);
        let loss2 = compute_pareto_loss(1.0, 0.5, 1.0, 0.5);
        assert!((loss1 - loss2).abs() < 1e-9);
    }

    #[test]
    fn test_is_cooperative_equilibrium_true() {
        assert!(is_cooperative_equilibrium(-0.5, 1.5, 0.1));
    }

    #[test]
    fn test_is_cooperative_equilibrium_low_diversity() {
        assert!(!is_cooperative_equilibrium(-0.5, 0.05, 0.1));
    }

    #[test]
    fn test_is_cooperative_equilibrium_positive_loss() {
        assert!(!is_cooperative_equilibrium(0.5, 1.5, 0.1));
    }

    #[test]
    fn test_engine_creation() {
        let engine = SymbioticDiversityLoss::new();
        assert_eq!(engine.record_count(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = ParetoConfig::default_stuartian();
        let engine = SymbioticDiversityLoss::with_config(config).unwrap();
        assert_eq!(engine.record_count(), 0);
    }

    #[test]
    fn test_engine_compute() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();
        let record = engine.compute(&dist, 0.1, 0.3).unwrap();
        assert!(record.diversity > 0.0);
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn test_engine_compute_cooperative() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();
        let record = engine.compute(&dist, 0.01, 0.5).unwrap();
        assert!(record.cooperative);
    }

    #[test]
    fn test_engine_compute_non_cooperative() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = vec![0.97, 0.01, 0.01, 0.01];
        let record = engine.compute(&dist, 5.0, 0.0).unwrap();
        assert!(!record.cooperative);
    }

    #[test]
    fn test_engine_average_loss() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();
        engine.compute(&dist, 0.1, 0.3).unwrap();
        engine.compute(&dist, 0.2, 0.3).unwrap();
        let avg = engine.average_loss().unwrap();
        assert!(avg.is_finite());
    }

    #[test]
    fn test_engine_average_diversity() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();
        engine.compute(&dist, 0.1, 0.3).unwrap();
        let avg = engine.average_diversity().unwrap();
        assert!((avg - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_engine_cooperative_rate() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();
        engine.compute(&dist, 0.01, 0.5).unwrap(); // cooperative
        engine.compute(&dist, 5.0, 0.0).unwrap(); // not cooperative
        let rate = engine.cooperative_rate().unwrap();
        assert!((rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_engine_latest() {
        let mut engine = SymbioticDiversityLoss::new();
        assert!(engine.latest().is_none());
        let dist = valid_distribution();
        engine.compute(&dist, 0.1, 0.3).unwrap();
        assert!(engine.latest().is_some());
    }

    #[test]
    fn test_engine_reset() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();
        engine.compute(&dist, 0.1, 0.3).unwrap();
        engine.reset();
        assert_eq!(engine.record_count(), 0);
        assert!(engine.average_loss().is_none());
    }

    #[test]
    fn test_engine_display() {
        let engine = SymbioticDiversityLoss::new();
        let s = format!("{}", engine);
        assert!(s.contains("SymbioticDiversityLoss"));
    }

    #[test]
    fn test_record_display() {
        let record = LossRecord {
            diversity: 1.5,
            destructive_conflict: 0.3,
            constructive_friction: 0.4,
            loss: -1.0,
            cooperative: true,
        };
        let s = format!("{}", record);
        assert!(s.contains("LossRecord"));
        assert!(s.contains("cooperative=true"));
    }

    #[test]
    fn test_error_display() {
        let err = DiversityError::EmptyDistribution;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_compute_instant() {
        let engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();
        let record = engine.compute_instant(&dist, 0.1, 0.3).unwrap();
        assert!(record.diversity > 0.0);
        assert_eq!(engine.record_count(), 0); // not recorded
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = SymbioticDiversityLoss::new();
        let dist = valid_distribution();

        // Simulate evolution: start with high conflict, reduce over time
        for i in 0..10 {
            let conflict = 1.0 - (i as f64) * 0.1;
            let record = engine.compute(&dist, conflict, 0.3).unwrap();
            if i >= 5 {
                assert!(record.cooperative || record.loss < 0.0);
            }
        }

        assert_eq!(engine.record_count(), 10);
        let avg_loss = engine.average_loss().unwrap();
        let avg_div = engine.average_diversity().unwrap();
        assert!(avg_loss.is_finite());
        assert!((avg_div - 2.0).abs() < 1e-6);
    }
}
