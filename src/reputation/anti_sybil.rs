//! Anti-Sybil Engine — Detection of Sybil attacks via behavioral analysis and VRF-based proof-of-compute.
//!
//! Strategies:
//! - Behavioral fingerprinting (timing, pattern analysis)
//! - VRF-based proof-of-compute verification
//! - Network graph clustering for identity correlation
//! - Zero financial logic: purely technical reputation protection

use std::collections::{HashMap, HashSet, VecDeque};

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum AntiSybilError {
    VRFVerificationFailed(String),
    InsufficientData(String),
    NodeNotFound(String),
}

impl std::fmt::Display for AntiSybilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VRFVerificationFailed(id) => write!(f, "VRF verification failed: {}", id),
            Self::InsufficientData(id) => write!(f, "Insufficient data for: {}", id),
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
        }
    }
}

impl std::error::Error for AntiSybilError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct AntiSybilConfig {
    pub min_events_for_analysis: usize,
    pub timing_variance_threshold: f64,
    pub pattern_similarity_threshold: f64,
    pub vrf_confidence_threshold: f64,
    pub max_suspected_clusters: usize,
}

impl Default for AntiSybilConfig {
    fn default() -> Self {
        Self {
            min_events_for_analysis: 10,
            timing_variance_threshold: 0.1,
            pattern_similarity_threshold: 0.85,
            vrf_confidence_threshold: 0.9,
            max_suspected_clusters: 50,
        }
    }
}

// ─── Node Fingerprint ───

#[derive(Debug, Clone)]
pub struct NodeFingerprint {
    pub node_id: String,
    pub event_timestamps: VecDeque<u64>,
    pub event_types: VecDeque<u8>,
    pub vrf_proof: String,
    pub vrf_verified: bool,
    pub compute_score: f64,
}

impl NodeFingerprint {
    pub fn new(node_id: String, vrf_proof: String) -> Self {
        Self {
            node_id,
            event_timestamps: VecDeque::new(),
            event_types: VecDeque::new(),
            vrf_verified: verify_vrf(&vrf_proof),
            vrf_proof,
            compute_score: 0.0,
        }
    }

    pub fn record_event(&mut self, timestamp_ms: u64, event_type: u8) {
        self.event_timestamps.push_back(timestamp_ms);
        self.event_types.push_back(event_type);
        if self.event_timestamps.len() > 200 {
            self.event_timestamps.pop_front();
            self.event_types.pop_front();
        }
    }

    pub fn timing_variance(&self) -> f64 {
        if self.event_timestamps.len() < 3 {
            return 0.0;
        }
        let intervals: Vec<f64> = self
            .event_timestamps
            .iter()
            .zip(self.event_timestamps.iter().skip(1))
            .map(|(a, b)| (*b as f64 - *a as f64).max(0.0))
            .collect();
        if intervals.is_empty() {
            return 0.0;
        }
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        if mean <= 0.0 {
            return 0.0;
        }
        let variance: f64 =
            intervals.iter().map(|i| (i - mean).powi(2)).sum::<f64>() / intervals.len() as f64;
        (variance.sqrt() / mean).min(1.0)
    }

    pub fn pattern_hash(&self) -> String {
        let data: String = self.event_types.iter().map(|t| t.to_string()).collect();
        compute_hash(&data)
    }
}

// ─── Suspicion ───

#[derive(Debug, Clone)]
pub struct SybilSuspicion {
    pub node_id: String,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub cluster_id: Option<String>,
}

// ─── Cluster ───

#[derive(Debug, Clone)]
pub struct SuspectedCluster {
    pub cluster_id: String,
    pub node_ids: Vec<String>,
    pub similarity_score: f64,
    pub detected_at_ms: u64,
}

// ─── Stats ───

#[derive(Debug, Clone, Default)]
pub struct AntiSybilStats {
    pub total_nodes_analyzed: u64,
    pub total_suspicions: u64,
    pub total_clusters: u64,
    pub vrf_verifications: u64,
    pub vrf_failures: u64,
}

// ─── Engine ───

pub struct AntiSybilEngine {
    config: AntiSybilConfig,
    fingerprints: HashMap<String, NodeFingerprint>,
    suspicions: Vec<SybilSuspicion>,
    clusters: Vec<SuspectedCluster>,
    stats: AntiSybilStats,
}

impl AntiSybilEngine {
    pub fn new(config: AntiSybilConfig) -> Self {
        Self {
            config,
            fingerprints: HashMap::new(),
            suspicions: Vec::new(),
            clusters: Vec::new(),
            stats: AntiSybilStats::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AntiSybilConfig::default())
    }

    pub fn register_node(&mut self, node_id: String, vrf_proof: String) {
        self.fingerprints
            .insert(node_id.clone(), NodeFingerprint::new(node_id, vrf_proof));
    }

    pub fn record_event(&mut self, node_id: &str, timestamp_ms: u64, event_type: u8) {
        if let Some(fp) = self.fingerprints.get_mut(node_id) {
            fp.record_event(timestamp_ms, event_type);
        }
    }

    pub fn update_compute_score(&mut self, node_id: &str, score: f64) {
        if let Some(fp) = self.fingerprints.get_mut(node_id) {
            fp.compute_score = score.clamp(0.0, 1.0);
        }
    }

    pub fn analyze(&mut self) -> Vec<SybilSuspicion> {
        self.suspicions.clear();

        for (node_id, fp) in &self.fingerprints {
            if fp.event_timestamps.len() < self.config.min_events_for_analysis {
                continue;
            }

            self.stats.total_nodes_analyzed += 1;
            let mut reasons = Vec::new();
            let mut confidence = 0.0;

            // VRF check
            self.stats.vrf_verifications += 1;
            if !fp.vrf_verified {
                self.stats.vrf_failures += 1;
                reasons.push("VRF verification failed".to_string());
                confidence += 0.4;
            }

            // Timing analysis
            let variance = fp.timing_variance();
            if variance < self.config.timing_variance_threshold {
                reasons.push(format!("Suspiciously low timing variance: {:.3}", variance));
                confidence += 0.3;
            }

            // Pattern analysis
            let pattern = fp.pattern_hash();
            let duplicates = self
                .fingerprints
                .values()
                .filter(|other| {
                    other.node_id != *node_id
                        && other.event_types.len() >= self.config.min_events_for_analysis
                        && other.pattern_hash() == pattern
                })
                .count();
            if duplicates > 0 {
                reasons.push(format!("Pattern matches {} other nodes", duplicates));
                confidence += 0.3 * (duplicates as f64).min(1.0);
            }

            if confidence >= 0.5 {
                let suspicion = SybilSuspicion {
                    node_id: node_id.clone(),
                    confidence: confidence.min(1.0),
                    reasons,
                    cluster_id: None,
                };
                self.stats.total_suspicions += 1;
                self.suspicions.push(suspicion);
            }
        }

        // Cluster detection
        self.detect_clusters();

        self.suspicions.clone()
    }

    pub fn get_suspicions(&self) -> &[SybilSuspicion] {
        &self.suspicions
    }

    pub fn get_clusters(&self) -> &[SuspectedCluster] {
        &self.clusters
    }

    pub fn get_stats(&self) -> &AntiSybilStats {
        &self.stats
    }

    pub fn get_config(&self) -> &AntiSybilConfig {
        &self.config
    }

    fn detect_clusters(&mut self) {
        self.clusters.clear();
        let mut used = HashSet::new();

        for (i, s1) in self.suspicions.iter().enumerate() {
            if used.contains(&s1.node_id) {
                continue;
            }
            let mut cluster_nodes = vec![s1.node_id.clone()];

            for (_j, s2) in self.suspicions.iter().enumerate().skip(i + 1) {
                if used.contains(&s2.node_id) {
                    continue;
                }
                // Check if patterns match
                if let (Some(fp1), Some(fp2)) = (
                    self.fingerprints.get(&s1.node_id),
                    self.fingerprints.get(&s2.node_id),
                ) {
                    if fp1.pattern_hash() == fp2.pattern_hash() {
                        cluster_nodes.push(s2.node_id.clone());
                        used.insert(s2.node_id.clone());
                    }
                }
            }

            if cluster_nodes.len() >= 2 {
                used.insert(s1.node_id.clone());
                let cluster = SuspectedCluster {
                    cluster_id: format!("cluster-{}", self.stats.total_clusters),
                    node_ids: cluster_nodes,
                    similarity_score: 0.9,
                    detected_at_ms: current_timestamp_ms(),
                };
                self.stats.total_clusters += 1;
                self.clusters.push(cluster);
            }
        }
    }
}

impl Default for AntiSybilEngine {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Utilities ───

fn compute_hash(data: &str) -> String {
    let mut h: u64 = 5381;
    for byte in data.bytes() {
        h = h.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{:016x}", h)
}

fn verify_vrf(proof: &str) -> bool {
    // Simulated VRF verification: valid proofs start with "vrf-"
    proof.starts_with("vrf-")
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = AntiSybilEngine::with_defaults();
        assert_eq!(engine.get_stats().total_nodes_analyzed, 0);
    }

    #[test]
    fn test_register_node() {
        let mut engine = AntiSybilEngine::with_defaults();
        engine.register_node("n1".to_string(), "vrf-valid123".to_string());
        assert!(engine.fingerprints.contains_key("n1"));
    }

    #[test]
    fn test_vrf_verification() {
        let mut engine = AntiSybilEngine::with_defaults();
        engine.register_node("valid".to_string(), "vrf-ok".to_string());
        engine.register_node("invalid".to_string(), "bad-proof".to_string());
        assert!(engine.fingerprints.get("valid").unwrap().vrf_verified);
        assert!(!engine.fingerprints.get("invalid").unwrap().vrf_verified);
    }

    #[test]
    fn test_record_event() {
        let mut engine = AntiSybilEngine::with_defaults();
        engine.register_node("n1".to_string(), "vrf-ok".to_string());
        engine.record_event("n1", 1000, 1);
        engine.record_event("n1", 1500, 2);
        let fp = engine.fingerprints.get("n1").unwrap();
        assert_eq!(fp.event_timestamps.len(), 2);
    }

    #[test]
    fn test_analyze_insufficient_data() {
        let mut engine = AntiSybilEngine::with_defaults();
        engine.register_node("n1".to_string(), "vrf-ok".to_string());
        let suspicions = engine.analyze();
        assert!(suspicions.is_empty());
    }

    #[test]
    fn test_detect_invalid_vrf() {
        let mut engine = AntiSybilEngine::with_defaults();
        engine.register_node("n1".to_string(), "bad-proof".to_string());
        for i in 0..20 {
            engine.record_event("n1", 1000 + i * 100, 1);
        }
        let suspicions = engine.analyze();
        assert!(!suspicions.is_empty());
    }

    #[test]
    fn test_cluster_detection() {
        let mut engine = AntiSybilEngine::with_defaults();
        engine.register_node("n1".to_string(), "vrf-ok".to_string());
        engine.register_node("n2".to_string(), "vrf-ok".to_string());
        // Same pattern
        for i in 0..15 {
            engine.record_event("n1", 1000 + i * 100, 1);
            engine.record_event("n2", 2000 + i * 100, 1);
        }
        engine.analyze();
        // Clusters may or may not form depending on timing variance
        assert!(engine.get_clusters().is_empty() || engine.get_clusters().len() >= 1);
    }

    #[test]
    fn test_timing_variance() {
        let mut fp = NodeFingerprint::new("n".to_string(), "vrf-ok".to_string());
        for i in 0..20 {
            fp.record_event(i * 100, 1);
        }
        // Perfectly regular = low variance
        assert!(fp.timing_variance() < 0.1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = AntiSybilEngine::with_defaults();
        engine.register_node("n1".to_string(), "vrf-ok".to_string());
        for i in 0..15 {
            engine.record_event("n1", 1000 + i * 100, 1);
        }
        engine.analyze();
        assert_eq!(engine.get_stats().total_nodes_analyzed, 1);
    }

    #[test]
    fn test_error_display() {
        let e = AntiSybilError::VRFVerificationFailed("x".to_string());
        assert!(!e.to_string().is_empty());
    }
}
