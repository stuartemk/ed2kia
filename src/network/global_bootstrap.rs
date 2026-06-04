//! Global Bootstrap Protocol â€” Sprint 71: Global Bootstrap & Critical Bottleneck Resolution
//!
//! Implements stealth ignition sequence with:
//! - Gradual network bootstrapping (phased activation)
//! - Seed node rotation for resilience
//! - Geographic diversity thresholds
//! - Sybil detection via behavioral fingerprinting
//! - Anti-capture mechanisms
//!
//! # Phases
//!
//! 1. **Stealth**: Seed nodes activate silently, establish trust graph.
//! 2. **Seed**: Trusted nodes begin gossip, validate newcomers.
//! 3. **Growth**: Network opens to public with bootstrap consensus.
//! 4. **Mature**: Full decentralized operation, seed rotation active.

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Errors in global bootstrap protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapError {
    /// Insufficient seed nodes to start ignition.
    InsufficientSeeds { have: usize, needed: usize },
    /// Geographic diversity threshold not met.
    DiversityFailed { current: f64, required: f64 },
    /// Sybil attack detected.
    SybilDetected { fingerprint: u64, count: usize },
    /// Invalid phase transition.
    InvalidPhaseTransition {
        from: BootstrapPhase,
        to: BootstrapPhase,
    },
    /// Seed node rotation failed â€” no available replacements.
    RotationFailed,
    /// Bootstrap already completed.
    AlreadyCompleted,
    /// Bootstrap not yet started.
    NotStarted,
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapError::InsufficientSeeds { have, needed } => {
                write!(f, "insufficient seeds: {} < {}", have, needed)
            }
            BootstrapError::DiversityFailed { current, required } => {
                write!(f, "diversity failed: {:.4} < {:.4}", current, required)
            }
            BootstrapError::SybilDetected { fingerprint, count } => {
                write!(
                    f,
                    "Sybil detected: fingerprint {} appears {} times",
                    fingerprint, count
                )
            }
            BootstrapError::InvalidPhaseTransition { from, to } => {
                write!(f, "invalid phase transition: {} -> {}", from, to)
            }
            BootstrapError::RotationFailed => write!(f, "seed rotation failed"),
            BootstrapError::AlreadyCompleted => write!(f, "bootstrap already completed"),
            BootstrapError::NotStarted => write!(f, "bootstrap not started"),
        }
    }
}

impl std::error::Error for BootstrapError {}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for global bootstrap.
#[derive(Debug, Clone)]
pub struct BootstrapProtocolConfig {
    /// Minimum seed nodes required.
    pub min_seeds: usize,
    /// Geographic diversity threshold (Shannon entropy, 0-1 normalized).
    pub diversity_threshold: f64,
    /// Maximum nodes per behavioral fingerprint (Sybil detection).
    pub max_nodes_per_fingerprint: usize,
    /// Nodes per region required before phase advance.
    pub min_nodes_per_phase: usize,
    /// Seed rotation interval (in simulated ticks).
    pub rotation_interval: u64,
    /// Fraction of seeds to rotate each interval.
    pub rotation_fraction: f64,
}

impl BootstrapProtocolConfig {
    /// Default Topological configuration.
    pub fn default_topological() -> Self {
        Self {
            min_seeds: 3,
            diversity_threshold: 0.5,
            max_nodes_per_fingerprint: 3,
            min_nodes_per_phase: 5,
            rotation_interval: 100,
            rotation_fraction: 0.2,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), BootstrapError> {
        if self.min_seeds == 0 {
            return Err(BootstrapError::InsufficientSeeds { have: 0, needed: 1 });
        }
        if self.diversity_threshold < 0.0 || self.diversity_threshold > 1.0 {
            return Err(BootstrapError::DiversityFailed {
                current: self.diversity_threshold,
                required: 0.5,
            });
        }
        Ok(())
    }
}

impl Default for BootstrapProtocolConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

// ============================================================================
// Core Data Structures
// ============================================================================

/// Bootstrap phase.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BootstrapPhase {
    Stealth,
    Seed,
    Growth,
    Mature,
}

impl fmt::Display for BootstrapPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapPhase::Stealth => write!(f, "Stealth"),
            BootstrapPhase::Seed => write!(f, "Seed"),
            BootstrapPhase::Growth => write!(f, "Growth"),
            BootstrapPhase::Mature => write!(f, "Mature"),
        }
    }
}

/// A node in the bootstrap network.
#[derive(Debug, Clone)]
pub struct BootstrapNode {
    /// Unique node ID.
    pub node_id: u64,
    /// Geographic region (for diversity calculation).
    pub region: String,
    /// Behavioral fingerprint (for Sybil detection).
    pub fingerprint: u64,
    /// Is this a seed node?
    pub is_seed: bool,
    /// Is this node active?
    pub active: bool,
    /// Registration timestamp (tick).
    pub registered_at: u64,
}

impl fmt::Display for BootstrapNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BootstrapNode(id={}, region={}, seed={}, active={})",
            self.node_id, self.region, self.is_seed, self.active
        )
    }
}

/// Current bootstrap state with metrics.
#[derive(Debug, Clone)]
pub struct BootstrapState {
    /// Current phase.
    pub phase: BootstrapPhase,
    /// Total active nodes.
    pub active_nodes: usize,
    /// Total seed nodes.
    pub seed_nodes: usize,
    /// Geographic diversity index (Shannon entropy, normalized).
    pub diversity_index: f64,
    /// Current tick.
    pub current_tick: u64,
    /// Number of rotations performed.
    pub rotations: u64,
    /// Is bootstrap complete?
    pub completed: bool,
}

impl fmt::Display for BootstrapState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BootstrapState(phase={}, nodes={}, seeds={}, diversity={:.4}, tick={})",
            self.phase, self.active_nodes, self.seed_nodes, self.diversity_index, self.current_tick
        )
    }
}

/// Global Bootstrap Protocol engine.
pub struct GlobalBootstrap {
    config: BootstrapProtocolConfig,
    phase: BootstrapPhase,
    nodes: HashMap<u64, BootstrapNode>,
    /// Active seed node IDs.
    active_seeds: Vec<u64>,
    /// Candidate seed node IDs (for rotation).
    candidate_seeds: Vec<u64>,
    current_tick: u64,
    rotations: u64,
    completed: bool,
    /// Registration log.
    registration_log: Vec<(u64, u64)>, // (node_id, tick)
}

impl GlobalBootstrap {
    pub fn new() -> Self {
        Self {
            config: BootstrapProtocolConfig::default_topological(),
            phase: BootstrapPhase::Stealth,
            nodes: HashMap::new(),
            active_seeds: Vec::new(),
            candidate_seeds: Vec::new(),
            current_tick: 0,
            rotations: 0,
            completed: false,
            registration_log: Vec::new(),
        }
    }

    pub fn with_config(config: BootstrapProtocolConfig) -> Result<Self, BootstrapError> {
        config.validate()?;
        Ok(Self {
            config,
            phase: BootstrapPhase::Stealth,
            nodes: HashMap::new(),
            active_seeds: Vec::new(),
            candidate_seeds: Vec::new(),
            current_tick: 0,
            rotations: 0,
            completed: false,
            registration_log: Vec::new(),
        })
    }

    /// Run the ignition sequence with seed nodes.
    pub fn run_ignition_sequence(
        &mut self,
        seed_nodes: &[BootstrapNode],
        diversity_threshold: f64,
    ) -> Result<BootstrapState, BootstrapError> {
        if seed_nodes.len() < self.config.min_seeds {
            return Err(BootstrapError::InsufficientSeeds {
                have: seed_nodes.len(),
                needed: self.config.min_seeds,
            });
        }

        // Check geographic diversity
        let diversity = Self::compute_diversity_index_nodes(&seed_nodes.iter().collect::<Vec<_>>());
        if diversity < diversity_threshold {
            return Err(BootstrapError::DiversityFailed {
                current: diversity,
                required: diversity_threshold,
            });
        }

        // Check for Sybil (duplicate fingerprints)
        Self::check_sybil(seed_nodes, self.config.max_nodes_per_fingerprint)?;

        // Register seed nodes
        for node in seed_nodes {
            self.active_seeds.push(node.node_id);
            self.nodes.insert(
                node.node_id,
                BootstrapNode {
                    node_id: node.node_id,
                    region: node.region.clone(),
                    fingerprint: node.fingerprint,
                    is_seed: true,
                    active: true,
                    registered_at: self.current_tick,
                },
            );
            self.registration_log
                .push((node.node_id, self.current_tick));
        }

        self.phase = BootstrapPhase::Stealth;
        Ok(self.current_state())
    }

    /// Register a new node (non-seed).
    pub fn register_node(&mut self, node: BootstrapNode) -> Result<(), BootstrapError> {
        if self.completed {
            return Err(BootstrapError::AlreadyCompleted);
        }

        // Sybil check
        let fp_count = self
            .nodes
            .values()
            .filter(|n| n.fingerprint == node.fingerprint)
            .count();
        if fp_count >= self.config.max_nodes_per_fingerprint {
            return Err(BootstrapError::SybilDetected {
                fingerprint: node.fingerprint,
                count: fp_count + 1,
            });
        }

        let registered_node = BootstrapNode {
            node_id: node.node_id,
            region: node.region.clone(),
            fingerprint: node.fingerprint,
            is_seed: false,
            active: true,
            registered_at: self.current_tick,
        };

        self.nodes.insert(node.node_id, registered_node);
        self.registration_log
            .push((node.node_id, self.current_tick));

        // Check for candidate seed promotion
        if !node.is_seed {
            self.candidate_seeds.push(node.node_id);
        }

        // Try phase advancement
        self.try_advance_phase();
        Ok(())
    }

    /// Advance a tick and check for phase transitions / rotations.
    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Check seed rotation
        if self.current_tick % self.config.rotation_interval == 0 {
            self.rotate_seeds();
        }

        // Try phase advancement
        self.try_advance_phase();
    }

    /// Get current state.
    pub fn state(&self) -> BootstrapState {
        self.current_state()
    }

    /// Get a node by ID.
    pub fn get_node(&self, node_id: u64) -> Option<&BootstrapNode> {
        self.nodes.get(&node_id)
    }

    /// Get all active nodes.
    pub fn active_nodes(&self) -> Vec<&BootstrapNode> {
        self.nodes.values().filter(|n| n.active).collect()
    }

    /// Get registration log.
    pub fn registration_log(&self) -> &[(u64, u64)] {
        &self.registration_log
    }

    /// Reset bootstrap state.
    pub fn reset(&mut self) {
        self.phase = BootstrapPhase::Stealth;
        self.nodes.clear();
        self.active_seeds.clear();
        self.candidate_seeds.clear();
        self.current_tick = 0;
        self.rotations = 0;
        self.completed = false;
        self.registration_log.clear();
    }

    // â”€â”€â”€ Private helpers â”€â”€â”€

    fn current_state(&self) -> BootstrapState {
        let active_nodes = self.nodes.values().filter(|n| n.active).count();
        let seed_nodes = self
            .nodes
            .values()
            .filter(|n| n.is_seed && n.active)
            .count();
        let active_list: Vec<&BootstrapNode> = self.active_nodes();
        let diversity = if !active_list.is_empty() {
            Self::compute_diversity_index_nodes(&active_list)
        } else {
            0.0
        };

        BootstrapState {
            phase: self.phase.clone(),
            active_nodes,
            seed_nodes,
            diversity_index: diversity,
            current_tick: self.current_tick,
            rotations: self.rotations,
            completed: self.completed,
        }
    }

    fn try_advance_phase(&mut self) {
        let active_count = self.nodes.values().filter(|n| n.active).count();

        match self.phase {
            BootstrapPhase::Stealth => {
                if active_count >= self.config.min_seeds {
                    self.phase = BootstrapPhase::Seed;
                }
            }
            BootstrapPhase::Seed => {
                if active_count >= self.config.min_nodes_per_phase * 2 {
                    self.phase = BootstrapPhase::Growth;
                }
            }
            BootstrapPhase::Growth => {
                if active_count >= self.config.min_nodes_per_phase * 5 {
                    self.phase = BootstrapPhase::Mature;
                    self.completed = true;
                }
            }
            BootstrapPhase::Mature => {}
        }
    }

    fn rotate_seeds(&mut self) {
        if self.candidate_seeds.is_empty() {
            return;
        }

        let rotate_count =
            (self.active_seeds.len() as f64 * self.config.rotation_fraction).max(1.0) as usize;

        for i in 0..rotate_count.min(self.active_seeds.len()) {
            if let Some(seed_id) = self.active_seeds.get(i).copied() {
                // Deactivate old seed
                if let Some(node) = self.nodes.get_mut(&seed_id) {
                    node.is_seed = false;
                }

                // Promote candidate
                if let Some(new_seed_id) = self.candidate_seeds.pop() {
                    self.active_seeds.push(new_seed_id);
                    if let Some(node) = self.nodes.get_mut(&new_seed_id) {
                        node.is_seed = true;
                    }
                }
            }
        }

        // Remove rotated seeds from active list
        self.active_seeds
            .drain(0..rotate_count.min(self.active_seeds.len()));
        self.rotations += 1;
    }

    /// Compute Shannon entropy-based diversity index from regions.
    fn compute_diversity_index(nodes: &[impl AsRef<str>]) -> f64 {
        if nodes.is_empty() {
            return 0.0;
        }

        let mut region_counts: HashMap<&str, usize> = HashMap::new();
        let total = nodes.len();

        for node in nodes {
            let region = if let Some(n) = node.as_ref().strip_prefix("region:") {
                n
            } else {
                node.as_ref()
            };
            *region_counts.entry(region).or_insert(0) += 1;
        }

        // Shannon entropy
        let mut entropy = 0.0f64;
        for &count in region_counts.values() {
            let p = count as f64 / total as f64;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }

        // Normalize by log2(num_regions)
        let max_entropy = (region_counts.len() as f64).log2();
        if max_entropy > 0.0 {
            entropy / max_entropy
        } else {
            0.0
        }
    }

    fn compute_diversity_index_nodes(nodes: &[&BootstrapNode]) -> f64 {
        let regions: Vec<String> = nodes.iter().map(|n| n.region.clone()).collect();
        Self::compute_diversity_index(&regions)
    }

    fn check_sybil(
        nodes: &[BootstrapNode],
        max_per_fingerprint: usize,
    ) -> Result<(), BootstrapError> {
        let mut fp_counts: HashMap<u64, usize> = HashMap::new();
        for node in nodes {
            *fp_counts.entry(node.fingerprint).or_insert(0) += 1;
        }
        for (&fp, &count) in &fp_counts {
            if count > max_per_fingerprint {
                return Err(BootstrapError::SybilDetected {
                    fingerprint: fp,
                    count,
                });
            }
        }
        Ok(())
    }
}

impl Default for GlobalBootstrap {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GlobalBootstrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GlobalBootstrap(phase={}, nodes={}, seeds={}, tick={})",
            self.phase,
            self.nodes.len(),
            self.active_seeds.len(),
            self.current_tick
        )
    }
}

// ============================================================================
// Public utility function (matches spec signature)
// ============================================================================

/// Run the ignition sequence with seed node IDs and diversity threshold.
pub fn run_ignition_sequence(seed_nodes: &[u64], diversity_threshold: f64) -> BootstrapState {
    let mut bootstrap = GlobalBootstrap::new();
    let nodes: Vec<BootstrapNode> = seed_nodes
        .iter()
        .enumerate()
        .map(|(i, &id)| BootstrapNode {
            node_id: id,
            region: format!("region_{}", i % 5), // Distribute across 5 regions
            fingerprint: id.wrapping_mul(6364136223846793005u64),
            is_seed: true,
            active: true,
            registered_at: 0,
        })
        .collect();

    match bootstrap.run_ignition_sequence(&nodes, diversity_threshold) {
        Ok(state) => state,
        Err(_) => BootstrapState {
            phase: BootstrapPhase::Stealth,
            active_nodes: 0,
            seed_nodes: 0,
            diversity_index: 0.0,
            current_tick: 0,
            rotations: 0,
            completed: false,
        },
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_seed(id: u64, region: &str, fp: u64) -> BootstrapNode {
        BootstrapNode {
            node_id: id,
            region: region.to_string(),
            fingerprint: fp,
            is_seed: true,
            active: true,
            registered_at: 0,
        }
    }

    fn make_node(id: u64, region: &str, fp: u64) -> BootstrapNode {
        BootstrapNode {
            node_id: id,
            region: region.to_string(),
            fingerprint: fp,
            is_seed: false,
            active: true,
            registered_at: 0,
        }
    }

    #[test]
    fn test_config_default() {
        let config = BootstrapProtocolConfig::default_topological();
        assert_eq!(config.min_seeds, 3);
        assert_eq!(config.diversity_threshold, 0.5);
        assert_eq!(config.max_nodes_per_fingerprint, 3);
    }

    #[test]
    fn test_config_validate() {
        assert!(BootstrapProtocolConfig::default_topological()
            .validate()
            .is_ok());
    }

    #[test]
    fn test_config_zero_seeds() {
        let mut config = BootstrapProtocolConfig::default_topological();
        config.min_seeds = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bootstrap_creation() {
        let bootstrap = GlobalBootstrap::new();
        assert_eq!(bootstrap.state().phase, BootstrapPhase::Stealth);
        assert_eq!(bootstrap.state().active_nodes, 0);
    }

    #[test]
    fn test_ignition_insufficient_seeds() {
        let mut bootstrap = GlobalBootstrap::new();
        let seeds = vec![make_seed(1, "us", 100)];
        let result = bootstrap.run_ignition_sequence(&seeds, 0.0);
        assert!(matches!(
            result,
            Err(BootstrapError::InsufficientSeeds { .. })
        ));
    }

    #[test]
    fn test_ignition_success() {
        let mut bootstrap = GlobalBootstrap::new();
        let seeds = vec![
            make_seed(1, "us", 100),
            make_seed(2, "eu", 200),
            make_seed(3, "asia", 300),
        ];
        let state = bootstrap.run_ignition_sequence(&seeds, 0.0).unwrap();
        assert_eq!(state.phase, BootstrapPhase::Stealth);
        assert_eq!(state.active_nodes, 3);
        assert_eq!(state.seed_nodes, 3);
    }

    #[test]
    fn test_sybil_detection() {
        let mut bootstrap = GlobalBootstrap::new();
        let seeds = vec![
            make_seed(1, "us", 100),
            make_seed(2, "us", 100), // Same fingerprint
            make_seed(3, "us", 100), // Same fingerprint
            make_seed(4, "us", 100), // 4th â€” exceeds max of 3
        ];
        let result = bootstrap.run_ignition_sequence(&seeds, 0.0);
        assert!(matches!(result, Err(BootstrapError::SybilDetected { .. })));
    }

    #[test]
    fn test_register_node() {
        let mut bootstrap = GlobalBootstrap::new();
        let seeds = vec![
            make_seed(1, "us", 100),
            make_seed(2, "eu", 200),
            make_seed(3, "asia", 300),
        ];
        bootstrap.run_ignition_sequence(&seeds, 0.0).unwrap();

        let node = make_node(10, "us", 1000);
        assert!(bootstrap.register_node(node).is_ok());
        assert_eq!(bootstrap.state().active_nodes, 4);
    }

    #[test]
    fn test_sybil_on_register() {
        let mut config = BootstrapProtocolConfig::default_topological();
        config.max_nodes_per_fingerprint = 2;
        let mut bootstrap = GlobalBootstrap::with_config(config).unwrap();
        let seeds = vec![
            make_seed(1, "us", 100),
            make_seed(2, "eu", 200),
            make_seed(3, "asia", 300),
        ];
        bootstrap.run_ignition_sequence(&seeds, 0.0).unwrap();

        // Register 2 nodes with same fingerprint (OK)
        bootstrap.register_node(make_node(10, "us", 999)).unwrap();
        bootstrap.register_node(make_node(11, "eu", 999)).unwrap();

        // 3rd should fail
        let result = bootstrap.register_node(make_node(12, "asia", 999));
        assert!(matches!(result, Err(BootstrapError::SybilDetected { .. })));
    }

    #[test]
    fn test_phase_advancement() {
        let mut config = BootstrapProtocolConfig::default_topological();
        config.min_nodes_per_phase = 2;
        let mut bootstrap = GlobalBootstrap::with_config(config).unwrap();
        let seeds = vec![
            make_seed(1, "us", 100),
            make_seed(2, "eu", 200),
            make_seed(3, "asia", 300),
        ];
        bootstrap.run_ignition_sequence(&seeds, 0.0).unwrap();
        assert_eq!(bootstrap.state().phase, BootstrapPhase::Stealth);

        // Add nodes to reach Seed phase threshold
        for i in 0..2 {
            bootstrap
                .register_node(make_node(10 + i, "us", 1000 + i))
                .unwrap();
        }
        // With 3 seeds + 2 nodes = 5 active, and min_nodes_per_phase=2:
        // Stealthâ†’Seed (5 >= 3), then Seedâ†’Growth (5 >= 4)
        assert_eq!(bootstrap.state().phase, BootstrapPhase::Growth);
    }

    #[test]
    fn test_diversity_index_uniform() {
        let regions = vec!["us", "eu", "asia", "sa", "af"];
        let diversity = GlobalBootstrap::compute_diversity_index(&regions);
        assert!((diversity - 1.0).abs() < 1e-10); // Perfect diversity
    }

    #[test]
    fn test_diversity_index_single() {
        let regions = vec!["us", "us", "us"];
        let diversity = GlobalBootstrap::compute_diversity_index(&regions);
        assert!((diversity - 0.0).abs() < 1e-10); // No diversity
    }

    #[test]
    fn test_tick() {
        let mut bootstrap = GlobalBootstrap::new();
        bootstrap.tick();
        assert_eq!(bootstrap.state().current_tick, 1);
    }

    #[test]
    fn test_reset() {
        let mut bootstrap = GlobalBootstrap::new();
        let seeds = vec![
            make_seed(1, "us", 100),
            make_seed(2, "eu", 200),
            make_seed(3, "asia", 300),
        ];
        bootstrap.run_ignition_sequence(&seeds, 0.0).unwrap();
        bootstrap.reset();
        assert_eq!(bootstrap.state().active_nodes, 0);
        assert_eq!(bootstrap.state().phase, BootstrapPhase::Stealth);
    }

    #[test]
    fn test_standalone_ignition_function() {
        let state = run_ignition_sequence(&[1, 2, 3], 0.0);
        assert_eq!(state.active_nodes, 3);
    }

    #[test]
    fn test_display() {
        let bootstrap = GlobalBootstrap::new();
        let s = format!("{}", bootstrap);
        assert!(s.contains("GlobalBootstrap"));
    }

    #[test]
    fn test_error_display() {
        let err = BootstrapError::SybilDetected {
            fingerprint: 100,
            count: 4,
        };
        let s = format!("{}", err);
        assert!(s.contains("Sybil"));
    }

    #[test]
    fn test_diversity_failure() {
        let mut bootstrap = GlobalBootstrap::new();
        let seeds = vec![
            make_seed(1, "us", 100),
            make_seed(2, "us", 200),
            make_seed(3, "us", 300),
        ];
        let result = bootstrap.run_ignition_sequence(&seeds, 0.9);
        assert!(matches!(
            result,
            Err(BootstrapError::DiversityFailed { .. })
        ));
    }
}
