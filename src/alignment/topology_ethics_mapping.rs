//! Topology-Ethics Mapping â€” Sprint 72: Asymptotic Optimization & Hard Sybil Resistance
//!
//! GEI as structural stability proxy, not moral oracle.
//! Maps topological invariants (Betti numbers, persistence, coherence) to ethical metrics.

use std::fmt;

// â”€â”€â”€ Error Types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub enum MappingError {
    InvalidGeiDimension(usize),
    NegativeBetti(u32),
    CoherenceOutOfRange(f64),
    EntropyOutOfRange(f64),
    EmptyInput,
    NumericalOverflow,
    InvalidThreshold(f64),
    DriftExceeded(f64),
}

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MappingError::InvalidGeiDimension(d) => {
                write!(f, "GEI dimension {} must be 8", d)
            }
            MappingError::NegativeBetti(b) => write!(f, "Betti number {} cannot be negative", b),
            MappingError::CoherenceOutOfRange(c) => {
                write!(f, "Coherence {} must be in [0, 1]", c)
            }
            MappingError::EntropyOutOfRange(e) => {
                write!(f, "Entropy {} must be non-negative", e)
            }
            MappingError::EmptyInput => write!(f, "Input data cannot be empty"),
            MappingError::NumericalOverflow => write!(f, "Numerical overflow in computation"),
            MappingError::InvalidThreshold(t) => {
                write!(f, "Threshold {} must be in [0, 1]", t)
            }
            MappingError::DriftExceeded(d) => {
                write!(f, "Topological drift {} exceeds maximum allowed", d)
            }
        }
    }
}

impl std::error::Error for MappingError {}

// â”€â”€â”€ Configuration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub struct MappingConfig {
    /// Maximum allowed drift between consecutive GEI snapshots
    pub max_drift: f64,
    /// Minimum coherence for ethical authorization
    pub min_coherence: f64,
    /// Betti-1 weight in ethical score computation
    pub betti_1_weight: f64,
    /// Persistence weight in ethical score computation
    pub persistence_weight: f64,
    /// Drift decay factor (exponential smoothing)
    pub drift_decay: f64,
    /// Enable topological ethics mapping
    pub mapping_enabled: bool,
}

impl MappingConfig {
    pub fn default_topological() -> Self {
        Self {
            max_drift: 0.3,
            min_coherence: 0.7,
            betti_1_weight: 0.4,
            persistence_weight: 0.3,
            drift_decay: 0.95,
            mapping_enabled: true,
        }
    }

    pub fn validate(&self) -> Result<(), MappingError> {
        if self.max_drift <= 0.0 || self.max_drift > 1.0 {
            return Err(MappingError::InvalidThreshold(self.max_drift));
        }
        if self.min_coherence < 0.0 || self.min_coherence > 1.0 {
            return Err(MappingError::InvalidThreshold(self.min_coherence));
        }
        if self.betti_1_weight < 0.0 || self.betti_1_weight > 1.0 {
            return Err(MappingError::InvalidThreshold(self.betti_1_weight));
        }
        if self.persistence_weight < 0.0 || self.persistence_weight > 1.0 {
            return Err(MappingError::InvalidThreshold(self.persistence_weight));
        }
        if self.drift_decay <= 0.0 || self.drift_decay > 1.0 {
            return Err(MappingError::InvalidThreshold(self.drift_decay));
        }
        Ok(())
    }
}

impl Default for MappingConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

// â”€â”€â”€ Topological Snapshot â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub struct TopoSnapshot {
    pub timestamp_ms: u64,
    pub gei: [f64; 8],
    pub betti_1: u32,
    pub betti_0: u32,
    pub persistence: f64,
    pub coherence: f64,
    pub entropy: f64,
}

impl TopoSnapshot {
    pub fn new(
        timestamp_ms: u64,
        gei: [f64; 8],
        betti_1: u32,
        betti_0: u32,
        persistence: f64,
        coherence: f64,
        entropy: f64,
    ) -> Result<Self, MappingError> {
        if coherence < 0.0 || coherence > 1.0 {
            return Err(MappingError::CoherenceOutOfRange(coherence));
        }
        if entropy < 0.0 {
            return Err(MappingError::EntropyOutOfRange(entropy));
        }
        Ok(Self {
            timestamp_ms,
            gei,
            betti_1,
            betti_0,
            persistence,
            coherence,
            entropy,
        })
    }

    pub fn euler_characteristic(&self) -> i64 {
        self.betti_0 as i64 - self.betti_1 as i64
    }

    pub fn gei_norm(&self) -> f64 {
        self.gei.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    pub fn gei_mean(&self) -> f64 {
        self.gei.iter().sum::<f64>() / 8.0
    }
}

impl fmt::Display for TopoSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TopoSnapshot(t={}, Î²â‚={}, Î²â‚€={}, persist={:.4}, coh={:.4}, ent={:.4})",
            self.timestamp_ms,
            self.betti_1,
            self.betti_0,
            self.persistence,
            self.coherence,
            self.entropy
        )
    }
}

// â”€â”€â”€ Ethics Mapping Record â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub struct EthicsRecord {
    pub timestamp_ms: u64,
    pub structural_stability: f64,
    pub ethical_score: f64,
    pub drift: f64,
    pub authorized: bool,
    pub betti_1: u32,
    pub persistence: f64,
}

impl EthicsRecord {
    pub fn is_valid(&self) -> bool {
        self.structural_stability >= 0.0
            && self.structural_stability <= 1.0
            && self.ethical_score >= 0.0
            && self.ethical_score <= 1.0
    }
}

impl fmt::Display for EthicsRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EthicsRecord(stability={:.4}, score={:.4}, drift={:.4}, auth={})",
            self.structural_stability, self.ethical_score, self.drift, self.authorized
        )
    }
}

// â”€â”€â”€ Topology-Ethics Mapper â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, PartialEq)]
pub struct TopologyEthicsMapper {
    config: MappingConfig,
    history: Vec<EthicsRecord>,
    last_snapshot: Option<TopoSnapshot>,
    smoothed_drift: f64,
}

impl TopologyEthicsMapper {
    pub fn new() -> Self {
        Self {
            config: MappingConfig::default_topological(),
            history: Vec::new(),
            last_snapshot: None,
            smoothed_drift: 0.0,
        }
    }

    pub fn with_config(config: MappingConfig) -> Result<Self, MappingError> {
        config.validate()?;
        Ok(Self {
            config,
            history: Vec::new(),
            last_snapshot: None,
            smoothed_drift: 0.0,
        })
    }

    /// Map topological snapshot to ethical metrics
    pub fn map_snapshot(&mut self, snapshot: &TopoSnapshot) -> Result<EthicsRecord, MappingError> {
        if !self.config.mapping_enabled {
            return Ok(EthicsRecord {
                timestamp_ms: snapshot.timestamp_ms,
                structural_stability: 0.0,
                ethical_score: 0.0,
                drift: 0.0,
                authorized: false,
                betti_1: snapshot.betti_1,
                persistence: snapshot.persistence,
            });
        }

        // Compute drift from previous snapshot
        let drift = self.compute_drift(snapshot);
        self.smoothed_drift =
            self.config.drift_decay * self.smoothed_drift + (1.0 - self.config.drift_decay) * drift;

        // Check drift threshold
        if self.smoothed_drift > self.config.max_drift {
            return Err(MappingError::DriftExceeded(self.smoothed_drift));
        }

        // Compute structural stability from topological invariants
        let stability = self.compute_structural_stability(snapshot, drift);

        // Compute ethical score as proxy (NOT a moral oracle)
        let ethical_score = self.compute_ethical_score(snapshot, stability, drift);

        // Authorization based on coherence threshold
        let authorized = snapshot.coherence >= self.config.min_coherence;

        let record = EthicsRecord {
            timestamp_ms: snapshot.timestamp_ms,
            structural_stability: stability,
            ethical_score,
            drift: self.smoothed_drift,
            authorized,
            betti_1: snapshot.betti_1,
            persistence: snapshot.persistence,
        };

        self.last_snapshot = Some(snapshot.clone());
        self.history.push(record.clone());
        Ok(record)
    }

    /// Compute drift between two snapshots
    pub fn compute_drift(&self, snapshot: &TopoSnapshot) -> f64 {
        match &self.last_snapshot {
            None => 0.0,
            Some(prev) => {
                let gei_drift = Self::gei_distance(&prev.gei, &snapshot.gei);
                let betti_drift = ((prev.betti_1 as i64 - snapshot.betti_1 as i64) as f64)
                    .abs()
                    .min(1.0);
                let persist_drift = (prev.persistence - snapshot.persistence).abs().min(1.0);
                // Weighted combination
                0.5 * gei_drift + 0.3 * betti_drift + 0.2 * persist_drift
            }
        }
    }

    /// Compute structural stability from topological invariants
    fn compute_structural_stability(&self, snapshot: &TopoSnapshot, drift: f64) -> f64 {
        // Stability decreases with drift and entropy
        let drift_penalty = 1.0 - drift.min(1.0);
        let entropy_penalty = if snapshot.entropy > 0.0 {
            (1.0 / (1.0 + snapshot.entropy)).min(1.0)
        } else {
            1.0
        };
        // Betti-1 contributes to structural complexity (more cycles = more structure)
        let betti_contribution = (snapshot.betti_1 as f64 * self.config.betti_1_weight).min(1.0);
        // Persistence contributes to stability
        let persist_contribution = snapshot.persistence.min(1.0) * self.config.persistence_weight;

        (drift_penalty * 0.4
            + entropy_penalty * 0.2
            + betti_contribution * 0.2
            + persist_contribution * 0.2)
            .min(1.0)
            .max(0.0)
    }

    /// Compute ethical score as structural stability proxy
    fn compute_ethical_score(&self, snapshot: &TopoSnapshot, stability: f64, drift: f64) -> f64 {
        // Ethical score is a PROXY for structural stability, NOT a moral judgment
        // Weighted combination of stability, coherence, and low drift
        let coherence_bonus = snapshot.coherence * 0.3;
        let stability_base = stability * 0.5;
        let drift_bonus = (1.0 - drift.min(1.0)) * 0.2;

        (stability_base + coherence_bonus + drift_bonus)
            .min(1.0)
            .max(0.0)
    }

    /// Compute GEI distance between two vectors
    pub fn gei_distance(a: &[f64; 8], b: &[f64; 8]) -> f64 {
        let sum: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
        (sum / 8.0).min(1.0)
    }

    /// Check if current state authorizes an action
    pub fn is_authorized(&self, min_coherence: Option<f64>) -> bool {
        let threshold = min_coherence.unwrap_or(self.config.min_coherence);
        match self.history.last() {
            None => false,
            Some(record) => record.authorized && record.ethical_score >= threshold,
        }
    }

    /// Get latest ethics record
    pub fn latest_record(&self) -> Option<&EthicsRecord> {
        self.history.last()
    }

    /// Get ethics history
    pub fn history(&self) -> &[EthicsRecord] {
        &self.history
    }

    /// Average ethical score
    pub fn average_ethical_score(&self) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }
        let sum: f64 = self.history.iter().map(|r| r.ethical_score).sum();
        Some(sum / self.history.len() as f64)
    }

    /// Average structural stability
    pub fn average_stability(&self) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }
        let sum: f64 = self.history.iter().map(|r| r.structural_stability).sum();
        Some(sum / self.history.len() as f64)
    }

    /// Authorization rate
    pub fn authorization_rate(&self) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }
        let count = self.history.iter().filter(|r| r.authorized).count();
        Some(count as f64 / self.history.len() as f64)
    }

    /// Reset mapper state
    pub fn reset(&mut self) {
        self.history.clear();
        self.last_snapshot = None;
        self.smoothed_drift = 0.0;
    }
}

impl Default for TopologyEthicsMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TopologyEthicsMapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TopologyEthicsMapper(records={}, smoothed_drift={:.4}, authorized={})",
            self.history.len(),
            self.smoothed_drift,
            self.is_authorized(None)
        )
    }
}

// â”€â”€â”€ Public Utility Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Compute structural stability from GEI + Betti numbers (standalone)
pub fn compute_structural_stability(
    _gei: &[f64; 8],
    betti_1: u32,
    persistence: f64,
    entropy: f64,
    drift: f64,
) -> f64 {
    let drift_penalty = 1.0 - drift.min(1.0);
    let entropy_penalty = if entropy > 0.0 {
        (1.0 / (1.0 + entropy)).min(1.0)
    } else {
        1.0
    };
    let betti_contribution = (betti_1 as f64 * 0.4).min(1.0);
    let persist_contribution = persistence.min(1.0) * 0.3;

    (drift_penalty * 0.4
        + entropy_penalty * 0.2
        + betti_contribution * 0.2
        + persist_contribution * 0.2)
        .min(1.0)
        .max(0.0)
}

/// Compute cosine similarity between two GEI vectors
pub fn gei_cosine_similarity(a: &[f64; 8], b: &[f64; 8]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a < 1e-10 || norm_b < 1e-10 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).min(1.0).max(-1.0)
}

/// Map coherence to ethical authorization (simple threshold)
pub fn coherence_to_authorization(coherence: f64, threshold: f64) -> bool {
    coherence >= threshold
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gei(values: [f64; 8]) -> [f64; 8] {
        values
    }

    fn make_snapshot(t: u64, gei: [f64; 8], betti_1: u32, coherence: f64) -> TopoSnapshot {
        TopoSnapshot::new(t, gei, betti_1, betti_1 + 1, 0.8, coherence, 0.5).unwrap()
    }

    // â”€â”€â”€ Config Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_config_default() {
        let config = MappingConfig::default_topological();
        assert_eq!(config.max_drift, 0.3);
        assert_eq!(config.min_coherence, 0.7);
        assert!(config.mapping_enabled);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = MappingConfig::default_topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid_drift() {
        let config = MappingConfig {
            max_drift: -0.1,
            ..MappingConfig::default_topological()
        };
        assert_eq!(config.validate(), Err(MappingError::InvalidThreshold(-0.1)));
    }

    #[test]
    fn test_config_invalid_coherence() {
        let config = MappingConfig {
            min_coherence: 1.5,
            ..MappingConfig::default_topological()
        };
        assert_eq!(config.validate(), Err(MappingError::InvalidThreshold(1.5)));
    }

    #[test]
    fn test_config_zero_decay() {
        let config = MappingConfig {
            drift_decay: 0.0,
            ..MappingConfig::default_topological()
        };
        assert_eq!(config.validate(), Err(MappingError::InvalidThreshold(0.0)));
    }

    // â”€â”€â”€ Snapshot Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_snapshot_creation() {
        let gei = [1.0; 8];
        let snap = TopoSnapshot::new(1000, gei, 5, 6, 0.8, 0.9, 0.5).unwrap();
        assert_eq!(snap.betti_1, 5);
        assert_eq!(snap.euler_characteristic(), 1);
    }

    #[test]
    fn test_snapshot_invalid_coherence() {
        let gei = [1.0; 8];
        assert_eq!(
            TopoSnapshot::new(1000, gei, 5, 6, 0.8, 1.5, 0.5),
            Err(MappingError::CoherenceOutOfRange(1.5))
        );
    }

    #[test]
    fn test_snapshot_negative_entropy() {
        let gei = [1.0; 8];
        assert_eq!(
            TopoSnapshot::new(1000, gei, 5, 6, 0.8, 0.9, -0.1),
            Err(MappingError::EntropyOutOfRange(-0.1))
        );
    }

    #[test]
    fn test_snapshot_euler_characteristic() {
        let gei = [1.0; 8];
        let snap = TopoSnapshot::new(1000, gei, 3, 7, 0.8, 0.9, 0.5).unwrap();
        assert_eq!(snap.euler_characteristic(), 4); // 7 - 3
    }

    #[test]
    fn test_snapshot_gei_norm() {
        let gei = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let snap = TopoSnapshot::new(1000, gei, 0, 1, 0.0, 1.0, 0.0).unwrap();
        assert!((snap.gei_norm() - 8.0_f64.sqrt()) < 1e-10);
    }

    #[test]
    fn test_snapshot_gei_mean() {
        let gei = [2.0; 8];
        let snap = TopoSnapshot::new(1000, gei, 0, 1, 0.0, 1.0, 0.0).unwrap();
        assert!((snap.gei_mean() - 2.0) < 1e-10);
    }

    #[test]
    fn test_snapshot_display() {
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        let s = format!("{}", snap);
        assert!(s.contains("TopoSnapshot"));
        assert!(s.contains("Î²â‚=5"));
    }

    // â”€â”€â”€ Mapper Creation Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_mapper_creation() {
        let mapper = TopologyEthicsMapper::new();
        assert!(mapper.history().is_empty());
        assert!(mapper.latest_record().is_none());
    }

    #[test]
    fn test_mapper_with_config() {
        let config = MappingConfig::default_topological();
        let mapper = TopologyEthicsMapper::with_config(config).unwrap();
        assert!(mapper.history().is_empty());
    }

    #[test]
    fn test_mapper_with_bad_config() {
        let config = MappingConfig {
            max_drift: 0.0,
            ..MappingConfig::default_topological()
        };
        assert_eq!(
            TopologyEthicsMapper::with_config(config),
            Err(MappingError::InvalidThreshold(0.0))
        );
    }

    // â”€â”€â”€ Mapping Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_map_first_snapshot() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        let record = mapper.map_snapshot(&snap).unwrap();
        assert!(record.structural_stability >= 0.0);
        assert!(record.structural_stability <= 1.0);
        assert!(record.drift == 0.0); // First snapshot has no drift
    }

    #[test]
    fn test_map_snapshot_authorized() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        let record = mapper.map_snapshot(&snap).unwrap();
        assert!(record.authorized); // coherence 0.9 >= 0.7 threshold
    }

    #[test]
    fn test_map_snapshot_not_authorized() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.5);
        let record = mapper.map_snapshot(&snap).unwrap();
        assert!(!record.authorized); // coherence 0.5 < 0.7 threshold
    }

    #[test]
    fn test_map_snapshot_disabled() {
        let mut mapper = TopologyEthicsMapper::new();
        mapper.config.mapping_enabled = false;
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        let record = mapper.map_snapshot(&snap).unwrap();
        assert_eq!(record.structural_stability, 0.0);
        assert!(!record.authorized);
    }

    #[test]
    fn test_map_snapshot_drift_exceeded() {
        let mut mapper = TopologyEthicsMapper::new();
        mapper.config.max_drift = 0.01; // Very low threshold

        // First snapshot
        let snap1 = make_snapshot(1000, [1.0; 8], 5, 0.9);
        mapper.map_snapshot(&snap1).unwrap();

        // Second snapshot with large drift
        let snap2 = make_snapshot(2000, [0.0; 8], 0, 0.9);
        let result = mapper.map_snapshot(&snap2);
        assert!(matches!(result, Err(MappingError::DriftExceeded(_))));
    }

    #[test]
    fn test_map_consecutive_snapshots() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap1 = make_snapshot(1000, [1.0; 8], 5, 0.9);
        let snap2 = make_snapshot(2000, [1.0; 8], 5, 0.9);

        let r1 = mapper.map_snapshot(&snap1).unwrap();
        let r2 = mapper.map_snapshot(&snap2).unwrap();

        assert_eq!(r1.drift, 0.0); // First snapshot
        assert!(r2.drift < 0.01); // Same GEI, same Betti
        assert_eq!(mapper.history().len(), 2);
    }

    // â”€â”€â”€ Drift Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_compute_drift_no_previous() {
        let mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        assert_eq!(mapper.compute_drift(&snap), 0.0);
    }

    #[test]
    fn test_compute_drift_identical() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        mapper.map_snapshot(&snap).unwrap();
        assert_eq!(mapper.compute_drift(&snap), 0.0);
    }

    #[test]
    fn test_compute_drift_different_gei() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap1 = make_snapshot(1000, [1.0; 8], 5, 0.9);
        mapper.map_snapshot(&snap1).unwrap();
        let snap2 = make_snapshot(2000, [0.0; 8], 5, 0.9);
        let drift = mapper.compute_drift(&snap2);
        assert!(drift > 0.0);
        assert!(drift <= 1.0);
    }

    // â”€â”€â”€ GEI Distance Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_gei_distance_identical() {
        let a = [1.0; 8];
        let b = [1.0; 8];
        assert_eq!(TopologyEthicsMapper::gei_distance(&a, &b), 0.0);
    }

    #[test]
    fn test_gei_distance_max() {
        let a = [0.0; 8];
        let b = [1.0; 8];
        assert!((TopologyEthicsMapper::gei_distance(&a, &b) - 1.0) < 1e-10);
    }

    #[test]
    fn test_gei_distance_symmetric() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        let d1 = TopologyEthicsMapper::gei_distance(&a, &b);
        let d2 = TopologyEthicsMapper::gei_distance(&b, &a);
        assert!((d1 - d2).abs() < 1e-10);
    }

    // â”€â”€â”€ Authorization Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_is_authorized_no_history() {
        let mapper = TopologyEthicsMapper::new();
        assert!(!mapper.is_authorized(None));
    }

    #[test]
    fn test_is_authorized_true() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        mapper.map_snapshot(&snap).unwrap();
        assert!(mapper.is_authorized(None));
    }

    #[test]
    fn test_is_authorized_custom_threshold() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        mapper.map_snapshot(&snap).unwrap();
        assert!(!mapper.is_authorized(Some(0.95))); // Score below 0.95
    }

    // â”€â”€â”€ Statistics Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_average_ethical_score_empty() {
        let mapper = TopologyEthicsMapper::new();
        assert!(mapper.average_ethical_score().is_none());
    }

    #[test]
    fn test_average_ethical_score() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap1 = make_snapshot(1000, [1.0; 8], 5, 0.9);
        let snap2 = make_snapshot(2000, [1.0; 8], 5, 0.9);
        mapper.map_snapshot(&snap1).unwrap();
        mapper.map_snapshot(&snap2).unwrap();
        let avg = mapper.average_ethical_score().unwrap();
        assert!(avg > 0.0);
        assert!(avg <= 1.0);
    }

    #[test]
    fn test_authorization_rate() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap1 = make_snapshot(1000, [1.0; 8], 5, 0.9); // Authorized
        let snap2 = make_snapshot(2000, [1.0; 8], 5, 0.5); // Not authorized
        mapper.map_snapshot(&snap1).unwrap();
        mapper.map_snapshot(&snap2).unwrap();
        let rate = mapper.authorization_rate().unwrap();
        assert!((rate - 0.5) < 1e-10); // 1/2 authorized
    }

    // â”€â”€â”€ Reset Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_reset() {
        let mut mapper = TopologyEthicsMapper::new();
        let snap = make_snapshot(1000, [1.0; 8], 5, 0.9);
        mapper.map_snapshot(&snap).unwrap();
        mapper.reset();
        assert!(mapper.history().is_empty());
        assert!(mapper.latest_record().is_none());
        assert_eq!(mapper.smoothed_drift, 0.0);
    }

    // â”€â”€â”€ Display Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_mapper_display() {
        let mapper = TopologyEthicsMapper::new();
        let s = format!("{}", mapper);
        assert!(s.contains("TopologyEthicsMapper"));
    }

    #[test]
    fn test_ethics_record_display() {
        let record = EthicsRecord {
            timestamp_ms: 1000,
            structural_stability: 0.8,
            ethical_score: 0.7,
            drift: 0.1,
            authorized: true,
            betti_1: 5,
            persistence: 0.8,
        };
        let s = format!("{}", record);
        assert!(s.contains("EthicsRecord"));
    }

    // â”€â”€â”€ Error Display Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_error_display_dimension() {
        let e = MappingError::InvalidGeiDimension(4);
        let s = format!("{}", e);
        assert!(s.contains("4"));
        assert!(s.contains("8"));
    }

    #[test]
    fn test_error_display_drift_exceeded() {
        let e = MappingError::DriftExceeded(0.5);
        let s = format!("{}", e);
        assert!(s.contains("0.5"));
    }

    #[test]
    fn test_error_display_overflow() {
        let e = MappingError::NumericalOverflow;
        assert!(format!("{}", e).contains("overflow"));
    }

    // â”€â”€â”€ Utility Function Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_compute_structural_stability() {
        let gei = [1.0; 8];
        let stability = compute_structural_stability(&gei, 5, 0.8, 0.5, 0.1);
        assert!(stability >= 0.0);
        assert!(stability <= 1.0);
    }

    #[test]
    fn test_compute_structural_stability_zero_drift() {
        let gei = [1.0; 8];
        let s1 = compute_structural_stability(&gei, 5, 0.8, 0.5, 0.0);
        let s2 = compute_structural_stability(&gei, 5, 0.8, 0.5, 0.3);
        assert!(s1 >= s2); // Less drift = more stability
    }

    #[test]
    fn test_gei_cosine_similarity_identical() {
        let a = [1.0; 8];
        let b = [1.0; 8];
        assert!((gei_cosine_similarity(&a, &b) - 1.0) < 1e-10);
    }

    #[test]
    fn test_gei_cosine_similarity_orthogonal() {
        let a = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!((gei_cosine_similarity(&a, &b)).abs() < 1e-10);
    }

    #[test]
    fn test_gei_cosine_similarity_zero_norm() {
        let a = [0.0; 8];
        let b = [1.0; 8];
        assert_eq!(gei_cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_coherence_to_authorization_above() {
        assert!(coherence_to_authorization(0.9, 0.7));
    }

    #[test]
    fn test_coherence_to_authorization_below() {
        assert!(!coherence_to_authorization(0.5, 0.7));
    }

    #[test]
    fn test_coherence_to_authorization_exact() {
        assert!(coherence_to_authorization(0.7, 0.7));
    }

    // â”€â”€â”€ Ethics Record Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_ethics_record_valid() {
        let record = EthicsRecord {
            timestamp_ms: 1000,
            structural_stability: 0.8,
            ethical_score: 0.7,
            drift: 0.1,
            authorized: true,
            betti_1: 5,
            persistence: 0.8,
        };
        assert!(record.is_valid());
    }

    #[test]
    fn test_ethics_record_invalid_stability() {
        let record = EthicsRecord {
            timestamp_ms: 1000,
            structural_stability: 1.5,
            ethical_score: 0.7,
            drift: 0.1,
            authorized: true,
            betti_1: 5,
            persistence: 0.8,
        };
        assert!(!record.is_valid());
    }

    // â”€â”€â”€ Workflow Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_full_mapping_workflow() {
        let mut mapper = TopologyEthicsMapper::new();

        // Simulate a sequence of snapshots
        for i in 0..10 {
            let gei = [1.0 + i as f64 * 0.01; 8];
            let snap = make_snapshot((i + 1) * 1000, gei, 5, 0.9);
            let record = mapper.map_snapshot(&snap).unwrap();
            assert!(record.is_valid());
        }

        assert_eq!(mapper.history().len(), 10);
        assert!(mapper.is_authorized(None));
        let avg = mapper.average_ethical_score().unwrap();
        assert!(avg > 0.0);
    }

    #[test]
    fn test_mapping_with_increasing_entropy() {
        let mut mapper = TopologyEthicsMapper::new();

        // Increasing entropy should decrease stability
        let snap1 = TopoSnapshot::new(1000, [1.0; 8], 5, 6, 0.8, 0.9, 0.1).unwrap();
        let snap2 = TopoSnapshot::new(2000, [1.0; 8], 5, 6, 0.8, 0.9, 2.0).unwrap();

        let r1 = mapper.map_snapshot(&snap1).unwrap();
        mapper.last_snapshot = Some(snap2.clone());
        let r2 = mapper.map_snapshot(&snap2).unwrap();

        // Higher entropy = lower stability (all else equal)
        assert!(r1.structural_stability >= r2.structural_stability);
    }
}
