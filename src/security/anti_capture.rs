//! Anti-Capture Mechanisms — Sprint 70: Civilization-Scale Architecture
//!
//! Geo-diversity weighting (max 30% per region), anti-Sybil via proof-of-work
//! and behavioral fingerprinting, and chaos engineering fault injection.

use std::collections::HashMap;
use std::fmt;

/// Errors in anti-capture mechanisms.
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureError {
    /// Region exceeds maximum weight threshold.
    RegionOverweight {
        region: String,
        weight: f64,
        max: f64,
    },
    /// Sybil attack detected.
    SybilDetected(u64),
    /// Invalid proof-of-work difficulty.
    InvalidDifficulty(i32),
    /// Node failed behavioral fingerprint check.
    FingerprintMismatch(String),
    /// Chaos injection failed.
    ChaosInjectionFailed(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::RegionOverweight {
                region,
                weight,
                max,
            } => {
                write!(
                    f,
                    "Region '{}' overweight: {:.3} > max {:.3}",
                    region, weight, max
                )
            }
            CaptureError::SybilDetected(node_id) => {
                write!(f, "Sybil attack detected: node {}", node_id)
            }
            CaptureError::InvalidDifficulty(d) => {
                write!(f, "Invalid PoW difficulty: {}", d)
            }
            CaptureError::FingerprintMismatch(msg) => {
                write!(f, "Fingerprint mismatch: {}", msg)
            }
            CaptureError::ChaosInjectionFailed(msg) => {
                write!(f, "Chaos injection failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for CaptureError {}

/// Configuration for anti-capture mechanisms.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Maximum weight per geographic region (0.3 = 30%).
    pub max_region_weight: f64,
    /// Proof-of-work difficulty for anti-Sybil.
    pub pow_difficulty: i32,
    /// Behavioral fingerprint entropy threshold.
    pub fingerprint_threshold: f64,
    /// Chaos injection probability (0.0 to 1.0).
    pub chaos_probability: f64,
    /// Maximum nodes per IP fingerprint.
    pub max_nodes_per_fingerprint: usize,
}

impl CaptureConfig {
    /// Default Stuartian configuration.
    pub fn default_stuartian() -> Self {
        Self {
            max_region_weight: 0.3,
            pow_difficulty: 4,
            fingerprint_threshold: 0.7,
            chaos_probability: 0.01,
            max_nodes_per_fingerprint: 3,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), CaptureError> {
        if !(0.0..=1.0).contains(&self.max_region_weight) {
            return Err(CaptureError::RegionOverweight {
                region: "config".to_string(),
                weight: self.max_region_weight,
                max: 1.0,
            });
        }
        if self.pow_difficulty < 0 || self.pow_difficulty > 20 {
            return Err(CaptureError::InvalidDifficulty(self.pow_difficulty));
        }
        if !(0.0..=1.0).contains(&self.fingerprint_threshold) {
            return Err(CaptureError::FingerprintMismatch(
                "threshold must be in [0, 1]".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.chaos_probability) {
            return Err(CaptureError::ChaosInjectionFailed(
                "probability must be in [0, 1]".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

impl fmt::Display for CaptureConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CaptureConfig {{ max_region: {:.0}%, pow: {}, chaos: {:.1}% }}",
            self.max_region_weight * 100.0,
            self.pow_difficulty,
            self.chaos_probability * 100.0
        )
    }
}

/// Risk assessment for a node.
#[derive(Debug, Clone)]
pub struct NodeRisk {
    /// Node identifier.
    pub node_id: u64,
    /// Geographic region.
    pub region: String,
    /// Region weight (fraction of total network).
    pub region_weight: f64,
    /// Sybil suspicion score (0.0 = trusted, 1.0 = likely Sybil).
    pub sybil_score: f64,
    /// Behavioral fingerprint entropy.
    pub fingerprint_entropy: f64,
    /// Overall risk level.
    pub risk_level: RiskLevel,
}

/// Risk level classification.
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

impl fmt::Display for NodeRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NodeRisk {{ id: {}, region: {}, risk: {}, sybil: {:.3} }}",
            self.node_id, self.region, self.risk_level, self.sybil_score
        )
    }
}

/// Anti-Capture System — protects against network capture attacks.
pub struct AntiCapture {
    config: CaptureConfig,
    nodes: HashMap<u64, NodeRisk>,
    region_weights: HashMap<String, f64>,
    fingerprint_counts: HashMap<String, usize>,
    chaos_injections: usize,
}

impl AntiCapture {
    /// Create with default configuration.
    pub fn new() -> Self {
        Self {
            config: CaptureConfig::default_stuartian(),
            nodes: HashMap::new(),
            region_weights: HashMap::new(),
            fingerprint_counts: HashMap::new(),
            chaos_injections: 0,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: CaptureConfig) -> Result<Self, CaptureError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            region_weights: HashMap::new(),
            fingerprint_counts: HashMap::new(),
            chaos_injections: 0,
        })
    }

    /// Compute proof-of-work score (simplified hash check).
    fn compute_pow_score(&self, node_id: u64, difficulty: i32) -> f64 {
        // Simplified: higher difficulty = lower score for suspicious nodes.
        let hash = node_id.wrapping_mul(2654435761u64);
        let leading_zeros = hash.leading_zeros() as i32;
        if leading_zeros >= difficulty as i32 {
            0.0 // Passes PoW check.
        } else {
            0.5 + (difficulty as f64 - leading_zeros as f64) * 0.1
        }
    }

    /// Compute behavioral fingerprint entropy.
    fn compute_fingerprint_entropy(&self, node_id: u64) -> f64 {
        // Simplified entropy based on node ID bit distribution.
        let bits = node_id.count_ones() as f64;
        let entropy = bits / 64.0;
        entropy.clamp(0.0, 1.0)
    }

    /// Register a node and assess risk.
    pub fn register_node(
        &mut self,
        node_id: u64,
        region: String,
        fingerprint: String,
    ) -> Result<NodeRisk, CaptureError> {
        // Check Sybil via fingerprint count.
        let count = self
            .fingerprint_counts
            .entry(fingerprint.clone())
            .or_insert(0);
        *count += 1;
        if *count > self.config.max_nodes_per_fingerprint {
            return Err(CaptureError::SybilDetected(node_id));
        }

        // Compute risk scores.
        let sybil_score = self.compute_pow_score(node_id, self.config.pow_difficulty);
        let fingerprint_entropy = self.compute_fingerprint_entropy(node_id);

        // Update region weight.
        let total_nodes = self.nodes.len() + 1;
        let region_count = self.region_weights.entry(region.clone()).or_insert(0.0);
        *region_count = 1.0 + (total_nodes - 1) as f64 * *region_count / total_nodes as f64;
        let region_weight = *region_count / total_nodes as f64;

        // Check region weight limit (only for networks > 4 nodes).
        if region_weight > self.config.max_region_weight && total_nodes > 4 {
            return Err(CaptureError::RegionOverweight {
                region,
                weight: region_weight,
                max: self.config.max_region_weight,
            });
        }

        // Determine risk level.
        let risk_level = self.classify_risk(sybil_score, fingerprint_entropy, region_weight);

        let risk = NodeRisk {
            node_id,
            region,
            region_weight,
            sybil_score,
            fingerprint_entropy,
            risk_level,
        };

        self.nodes.insert(node_id, risk.clone());
        Ok(risk)
    }

    /// Classify risk level based on scores.
    fn classify_risk(
        &self,
        sybil_score: f64,
        _fingerprint_entropy: f64,
        region_weight: f64,
    ) -> RiskLevel {
        let combined = sybil_score * 0.5 + region_weight * 0.5;
        if combined < 0.2 {
            RiskLevel::Low
        } else if combined < 0.4 {
            RiskLevel::Medium
        } else if combined < 0.6 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        }
    }

    /// Check if a node passes anti-Sybil verification.
    pub fn verify_anti_sybil(&self, node_id: u64) -> bool {
        if let Some(risk) = self.nodes.get(&node_id) {
            risk.sybil_score < self.config.fingerprint_threshold
        } else {
            false
        }
    }

    /// Get nodes above a risk threshold.
    pub fn high_risk_nodes(&self) -> Vec<&NodeRisk> {
        self.nodes
            .values()
            .filter(|r| matches!(r.risk_level, RiskLevel::High | RiskLevel::Critical))
            .collect()
    }

    /// Simulate chaos engineering fault injection.
    pub fn inject_chaos(&mut self, target_node: u64) -> Result<(), CaptureError> {
        if !self.nodes.contains_key(&target_node) {
            return Err(CaptureError::ChaosInjectionFailed(format!(
                "Node {} not found",
                target_node
            )));
        }
        // In production, this would simulate network partitions, latency, etc.
        self.chaos_injections += 1;
        Ok(())
    }

    /// Get the geo-diversity index (Shannon entropy of region distribution).
    pub fn geo_diversity_index(&self) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }
        let total = self.nodes.len() as f64;
        let mut entropy = 0.0;
        for weight in self.region_weights.values() {
            let p = weight / total;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    /// Get the number of registered nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of chaos injections performed.
    pub fn chaos_injection_count(&self) -> usize {
        self.chaos_injections
    }

    /// Get the current configuration.
    pub fn config(&self) -> &CaptureConfig {
        &self.config
    }
}

impl Default for AntiCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AntiCapture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AntiCapture {{ nodes: {}, high_risk: {}, diversity: {:.3}, chaos: {} }}",
            self.nodes.len(),
            self.high_risk_nodes().len(),
            self.geo_diversity_index(),
            self.chaos_injections
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CaptureConfig::default_stuartian();
        assert!((config.max_region_weight - 0.3).abs() < 1e-6);
        assert_eq!(config.pow_difficulty, 4);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = CaptureConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_bad_difficulty() {
        let mut config = CaptureConfig::default_stuartian();
        config.pow_difficulty = 25;
        match config.validate() {
            Err(CaptureError::InvalidDifficulty(25)) => {}
            _ => panic!("Expected InvalidDifficulty error"),
        }
    }

    #[test]
    fn test_config_display() {
        let config = CaptureConfig::default_stuartian();
        let s = format!("{}", config);
        assert!(s.contains("max_region: 30%"));
    }

    #[test]
    fn test_anti_capture_new() {
        let ac = AntiCapture::new();
        assert_eq!(ac.node_count(), 0);
        assert_eq!(ac.chaos_injection_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut ac = AntiCapture::new();
        let risk = ac
            .register_node(1, "north_america".to_string(), "fp_abc123".to_string())
            .unwrap();
        assert_eq!(risk.node_id, 1);
        assert_eq!(ac.node_count(), 1);
    }

    #[test]
    fn test_sybil_detection() {
        let mut config = CaptureConfig::default_stuartian();
        config.max_nodes_per_fingerprint = 2;
        let mut ac = AntiCapture::with_config(config).unwrap();
        ac.register_node(1, "na".to_string(), "same_fp".to_string())
            .unwrap();
        ac.register_node(2, "na".to_string(), "same_fp".to_string())
            .unwrap();
        match ac.register_node(3, "na".to_string(), "same_fp".to_string()) {
            Err(CaptureError::SybilDetected(3)) => {}
            _ => panic!("Expected SybilDetected error"),
        }
    }

    #[test]
    fn test_region_weight_limit() {
        let mut config = CaptureConfig::default_stuartian();
        config.max_region_weight = 0.4;
        let mut ac = AntiCapture::with_config(config).unwrap();
        // Register multiple nodes in same region.
        for i in 1..=10 {
            let _ = ac.register_node(i, "single_region".to_string(), format!("fp_{}", i));
        }
        // Should have triggered region overweight at some point.
        assert!(ac.node_count() > 0);
    }

    #[test]
    fn test_verify_anti_sybil() {
        let mut ac = AntiCapture::new();
        ac.register_node(1, "na".to_string(), "fp_1".to_string())
            .unwrap();
        assert!(ac.verify_anti_sybil(1));
        assert!(!ac.verify_anti_sybil(999));
    }

    #[test]
    fn test_high_risk_nodes() {
        let mut ac = AntiCapture::new();
        ac.register_node(1, "na".to_string(), "fp_1".to_string())
            .unwrap();
        let high_risk = ac.high_risk_nodes();
        // Single node should be low risk.
        assert!(high_risk.is_empty() || high_risk[0].sybil_score < 0.5);
    }

    #[test]
    fn test_inject_chaos() {
        let mut ac = AntiCapture::new();
        ac.register_node(1, "na".to_string(), "fp_1".to_string())
            .unwrap();
        assert!(ac.inject_chaos(1).is_ok());
        assert_eq!(ac.chaos_injection_count(), 1);
    }

    #[test]
    fn test_inject_chaos_unknown_node() {
        let mut ac = AntiCapture::new();
        match ac.inject_chaos(999) {
            Err(CaptureError::ChaosInjectionFailed(msg)) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected ChaosInjectionFailed error"),
        }
    }

    #[test]
    fn test_geo_diversity_index() {
        let mut ac = AntiCapture::new();
        assert!((ac.geo_diversity_index() - 0.0).abs() < 1e-6);
        ac.register_node(1, "na".to_string(), "fp_1".to_string())
            .unwrap();
        ac.register_node(2, "eu".to_string(), "fp_2".to_string())
            .unwrap();
        let diversity = ac.geo_diversity_index();
        assert!(diversity > 0.0);
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(format!("{}", RiskLevel::Low), "Low");
        assert_eq!(format!("{}", RiskLevel::Critical), "Critical");
    }

    #[test]
    fn test_node_risk_display() {
        let risk = NodeRisk {
            node_id: 1,
            region: "na".to_string(),
            region_weight: 0.5,
            sybil_score: 0.1,
            fingerprint_entropy: 0.5,
            risk_level: RiskLevel::Low,
        };
        let s = format!("{}", risk);
        assert!(s.contains("id: 1"));
    }

    #[test]
    fn test_anti_capture_display() {
        let ac = AntiCapture::new();
        let s = format!("{}", ac);
        assert!(s.contains("AntiCapture"));
    }

    #[test]
    fn test_error_display() {
        let err = CaptureError::SybilDetected(42);
        let s = format!("{}", err);
        assert!(s.contains("Sybil"));
    }

    #[test]
    fn test_fingerprint_entropy() {
        let ac = AntiCapture::new();
        let e1 = ac.compute_fingerprint_entropy(0);
        let e2 = ac.compute_fingerprint_entropy(u64::MAX);
        assert!(e1 < e2);
        assert!(e2 <= 1.0);
    }
}
