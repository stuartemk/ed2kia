//! Temporal Cohesion Engine — Distributed Time Synchronization for P2P Networks.
//!
//! Implements a PTP/NTP-inspired time synchronization protocol adapted for
//! decentralized P2P/GossipSub topologies. Each node maintains a
//! `SymbioticTimestamp` that converges across the network through iterative
//! gossip exchange, enabling precise chronological ordering of events without
//! a central authority.
//!
//! # Design Principles
//!
//! - **Decentralized convergence**: No master clock — all nodes contribute equally.
//! - **PTP/NTP-inspired**: Uses round-trip delay measurement and offset correction.
//! - **GossipSub compatible**: Time samples propagate through existing gossip channels.
//! - **4D integration**: SymbioticTimestamp integrates with StuartianMoralManifold
//!   for precise chronological trajectory evaluation (x, y, z, t).
//! - **WASM compatible**: No blocking syscalls, all time sources abstracted.
//!
//! # Mathematical Foundation
//!
//! For two nodes A and B exchanging timestamps:
//! - t₁ = A sends timestamp (A-local)
//! - t₂ = B receives (B-local)
//! - t₃ = B replies (B-local)
//! - t₄ = A receives reply (A-local)
//!
//! Offset: θ = ((t₂ - t₁) + (t₃ - t₄)) / 2
//! Delay: δ = (t₄ - t₁) - (t₃ - t₂)
//!
//! Corrected time: t_corrected = t_local - θ
//!
//! Convergence criterion: |θₙ - θₙ₋₁| < ε after N gossip rounds.
//!
//! **Feature Gate:** `v3.4-macro-symbiosis`

use std::collections::HashMap;
use std::cmp::Ordering;

/// Unified timestamp for the symbiotic network.
///
/// Combines a millisecond-precision logical clock with a node identifier
/// for deterministic total ordering across the distributed system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbioticTimestamp {
    /// Logical timestamp in milliseconds (monotonically increasing per node).
    pub logical_ms: u64,
    /// Node identifier for tie-breaking (deterministic ordering).
    pub node_id: u64,
}

impl SymbioticTimestamp {
    /// Create a new symbiotic timestamp.
    pub fn new(logical_ms: u64, node_id: u64) -> Self {
        Self { logical_ms, node_id }
    }

    /// Check if this timestamp is strictly before another.
    pub fn is_before(&self, other: &SymbioticTimestamp) -> bool {
        self.logical_ms < other.logical_ms
            || (self.logical_ms == other.logical_ms && self.node_id < other.node_id)
    }

    /// Check if this timestamp is strictly after another.
    pub fn is_after(&self, other: &SymbioticTimestamp) -> bool {
        !self.is_before(other) && self.logical_ms != other.logical_ms
            || (self.logical_ms == other.logical_ms && self.node_id > other.node_id)
    }
}

impl Ord for SymbioticTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.logical_ms
            .cmp(&other.logical_ms)
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for SymbioticTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Time sample received from a peer node during gossip exchange.
#[derive(Debug, Clone, Copy)]
pub struct TimeSample {
    /// Timestamp sent by the peer (peer-local).
    pub t1: u64,
    /// Timestamp when this node received the sample (local).
    pub t2: u64,
    /// Reply timestamp from the peer (peer-local).
    pub t3: u64,
    /// Timestamp when this node received the reply (local).
    pub t4: u64,
    /// Source node identifier.
    pub peer_id: u64,
}

impl TimeSample {
    /// Compute the estimated clock offset from this sample.
    ///
    /// offset = ((t₂ - t₁) + (t₃ - t₄)) / 2
    pub fn offset(&self) -> f64 {
        let t1 = self.t1 as f64;
        let t2 = self.t2 as f64;
        let t3 = self.t3 as f64;
        let t4 = self.t4 as f64;
        ((t2 - t1) + (t3 - t4)) / 2.0
    }

    /// Compute the round-trip delay from this sample.
    ///
    /// delay = (t₄ - t₁) - (t₃ - t₂)
    pub fn delay(&self) -> f64 {
        let t1 = self.t1 as f64;
        let t2 = self.t2 as f64;
        let t3 = self.t3 as f64;
        let t4 = self.t4 as f64;
        (t4 - t1) - (t3 - t2)
    }

    /// Validate that the sample has consistent ordering (t₁ < t₂ < t₃ < t₄).
    pub fn is_valid(&self) -> bool {
        self.t1 <= self.t2 && self.t2 <= self.t3 && self.t3 <= self.t4
    }
}

/// Configuration for the TemporalCohesionEngine.
#[derive(Debug, Clone)]
pub struct TemporalConfig {
    /// Maximum number of time samples to retain per peer.
    pub max_samples_per_peer: usize,
    /// Convergence threshold for offset stability (milliseconds).
    pub convergence_threshold: f64,
    /// Maximum allowed clock drift correction per round (milliseconds).
    pub max_correction_per_round: f64,
    /// Number of gossip rounds before declaring convergence.
    pub convergence_rounds: usize,
    /// Filtering window size for median offset computation.
    pub filter_window: usize,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            max_samples_per_peer: 64,
            convergence_threshold: 50.0, // 50ms target
            max_correction_per_round: 1000.0, // 1 second max per round
            convergence_rounds: 5,
            filter_window: 10,
        }
    }
}

/// Current synchronization state for a single peer.
#[derive(Debug, Clone)]
pub struct PeerSyncState {
    /// Peer node identifier.
    pub peer_id: u64,
    /// Current estimated offset (milliseconds).
    pub estimated_offset: f64,
    /// Current estimated round-trip delay (milliseconds).
    pub estimated_delay: f64,
    /// Number of successful exchanges.
    pub exchange_count: usize,
    /// Last sample timestamp (local ms).
    pub last_sample_ms: u64,
}

/// Overall synchronization status of the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    /// Synchronization has converged within threshold.
    Converged,
    /// Still in progress, currently on given round.
    InProgress { round: usize },
    /// Unable to converge — insufficient peers or invalid samples.
    InsufficientPeers,
}

/// Errors specific to temporal cohesion operations.
#[derive(Debug, Clone)]
pub enum TemporalError {
    /// No peers available for synchronization.
    NoPeersAvailable,
    /// Invalid time sample (out of order timestamps).
    InvalidTimeSample,
    /// Configuration error.
    InvalidConfig(String),
    /// Correction exceeds maximum allowed per round.
    CorrectionExceeded(f64),
}

impl std::fmt::Display for TemporalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemporalError::NoPeersAvailable => write!(f, "No peers available for time synchronization"),
            TemporalError::InvalidTimeSample => write!(f, "Invalid time sample: timestamps out of order"),
            TemporalError::InvalidConfig(msg) => write!(f, "Invalid temporal configuration: {}", msg),
            TemporalError::CorrectionExceeded(value) => {
                write!(f, "Clock correction {:.2}ms exceeds maximum allowed per round", value)
            }
        }
    }
}

/// Temporal Cohesion Engine — Maintains distributed time synchronization.
///
/// Collects time samples from peer nodes via gossip exchange, computes
/// clock offsets using PTP-inspired algorithms, and applies gradual
/// correction to converge the local logical clock toward network consensus.
pub struct TemporalCohesionEngine {
    /// Local node identifier.
    pub node_id: u64,
    /// Current local logical time in milliseconds.
    local_time_ms: u64,
    /// Accumulated clock offset correction.
    accumulated_offset: f64,
    /// Previous offset for convergence detection.
    previous_offset: f64,
    /// Current gossip round counter.
    current_round: usize,
    /// Per-peer time samples (bounded queue).
    peer_samples: HashMap<u64, Vec<TimeSample>>,
    /// Per-peer synchronization state.
    peer_sync: HashMap<u64, PeerSyncState>,
    /// Engine configuration.
    config: TemporalConfig,
    /// Consecutive rounds within convergence threshold.
    stable_rounds: usize,
}

impl TemporalCohesionEngine {
    /// Create a new engine with default configuration.
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            local_time_ms: Self::now_ms(),
            accumulated_offset: 0.0,
            previous_offset: 0.0,
            current_round: 0,
            peer_samples: HashMap::new(),
            peer_sync: HashMap::new(),
            config: TemporalConfig::default(),
            stable_rounds: 0,
        }
    }

    /// Create a new engine with custom configuration.
    pub fn with_config(node_id: u64, config: TemporalConfig) -> Result<Self, TemporalError> {
        if config.max_samples_per_peer == 0 {
            return Err(TemporalError::InvalidConfig(
                "max_samples_per_peer must be > 0".to_string(),
            ));
        }
        if config.filter_window == 0 {
            return Err(TemporalError::InvalidConfig(
                "filter_window must be > 0".to_string(),
            ));
        }
        if config.convergence_rounds == 0 {
            return Err(TemporalError::InvalidConfig(
                "convergence_rounds must be > 0".to_string(),
            ));
        }

        Ok(Self {
            node_id,
            local_time_ms: Self::now_ms(),
            accumulated_offset: 0.0,
            previous_offset: 0.0,
            current_round: 0,
            peer_samples: HashMap::new(),
            peer_sync: HashMap::new(),
            config,
            stable_rounds: 0,
        })
    }

    /// Get current time in milliseconds (abstracted for WASM compatibility).
    #[cfg(not(target_arch = "wasm32"))]
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Get current time in milliseconds (WASM fallback).
    #[cfg(target_arch = "wasm32")]
    fn now_ms() -> u64 {
        0 // WASM environments inject time via host
    }

    /// Advance local logical time by the given duration.
    pub fn advance_time(&mut self, duration_ms: u64) {
        self.local_time_ms += duration_ms;
    }

    /// Record a time sample from a peer gossip exchange.
    ///
    /// The sample represents a complete round-trip exchange:
    /// t₁ = peer sent, t₂ = we received, t₃ = peer replied, t₄ = we received reply.
    pub fn record_sample(&mut self, sample: TimeSample) -> Result<(), TemporalError> {
        if !sample.is_valid() {
            return Err(TemporalError::InvalidTimeSample);
        }

        let peer_id = sample.peer_id;

        // Add sample to bounded queue.
        let samples = self.peer_samples.entry(peer_id).or_default();
        samples.push(sample);

        // Enforce max samples per peer (FIFO eviction).
        if samples.len() > self.config.max_samples_per_peer {
            samples.remove(0);
        }

        // Update peer sync state.
        let offset = sample.offset();
        let delay = sample.delay();

        let state = self.peer_sync.entry(peer_id).or_insert(PeerSyncState {
            peer_id,
            estimated_offset: 0.0,
            estimated_delay: 0.0,
            exchange_count: 0,
            last_sample_ms: 0,
        });

        // Exponential moving average for offset and delay.
        let alpha = 0.3;
        state.estimated_offset = alpha * offset + (1.0 - alpha) * state.estimated_offset;
        state.estimated_delay = alpha * delay + (1.0 - alpha) * state.estimated_delay;
        state.exchange_count += 1;
        state.last_sample_ms = self.local_time_ms;

        Ok(())
    }

    /// Execute a synchronization round: compute median offset from all peers
    /// and apply gradual correction.
    pub fn sync_round(&mut self) -> Result<f64, TemporalError> {
        if self.peer_sync.is_empty() {
            return Err(TemporalError::NoPeersAvailable);
        }

        // Collect offsets from all peers.
        let mut offsets: Vec<f64> = self.peer_sync.values()
            .map(|s| s.estimated_offset)
            .collect();

        if offsets.is_empty() {
            return Err(TemporalError::NoPeersAvailable);
        }

        // Compute median offset (robust to outliers).
        offsets.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let median_offset = self.median(&offsets);

        // Apply gradual correction (clamped to max per round).
        let correction = median_offset - self.accumulated_offset;
        let clamped_correction = correction.clamp(
            -self.config.max_correction_per_round,
            self.config.max_correction_per_round,
        );

        if correction.abs() > self.config.max_correction_per_round {
            // Log warning but continue with clamped correction.
        }

        self.previous_offset = self.accumulated_offset;
        self.accumulated_offset += clamped_correction;

        // Check convergence.
        let drift = (self.accumulated_offset - self.previous_offset).abs();
        if drift < self.config.convergence_threshold {
            self.stable_rounds += 1;
        } else {
            self.stable_rounds = 0;
        }

        self.current_round += 1;

        Ok(self.accumulated_offset)
    }

    /// Get the current synchronization status.
    pub fn get_sync_status(&self) -> SyncStatus {
        if self.peer_sync.len() < 2 {
            return SyncStatus::InsufficientPeers;
        }

        if self.stable_rounds >= self.config.convergence_rounds {
            SyncStatus::Converged
        } else {
            SyncStatus::InProgress {
                round: self.current_round,
            }
        }
    }

    /// Generate a SymbioticTimestamp for the current corrected local time.
    pub fn generate_timestamp(&self) -> SymbioticTimestamp {
        let corrected_time = (self.local_time_ms as f64 - self.accumulated_offset) as u64;
        SymbioticTimestamp::new(corrected_time, self.node_id)
    }

    /// Get the accumulated clock offset.
    pub fn accumulated_offset(&self) -> f64 {
        self.accumulated_offset
    }

    /// Get the number of connected peers.
    pub fn peer_count(&self) -> usize {
        self.peer_sync.len()
    }

    /// Get synchronization state for a specific peer.
    pub fn get_peer_state(&self, peer_id: u64) -> Option<&PeerSyncState> {
        self.peer_sync.get(&peer_id)
    }

    /// Get all peer synchronization states.
    pub fn get_all_peer_states(&self) -> Vec<&PeerSyncState> {
        self.peer_sync.values().collect()
    }

    /// Compute the timestamp variance across all peers (convergence metric).
    ///
    /// Returns the standard deviation of peer offsets in milliseconds.
    pub fn timestamp_variance(&self) -> f64 {
        let offsets: Vec<f64> = self.peer_sync.values()
            .map(|s| s.estimated_offset)
            .collect();

        if offsets.len() < 2 {
            return 0.0;
        }

        let mean: f64 = offsets.iter().sum::<f64>() / offsets.len() as f64;
        let variance: f64 = offsets
            .iter()
            .map(|o| (o - mean).powi(2))
            .sum::<f64>()
            / offsets.len() as f64;

        variance.sqrt()
    }

    /// Reset the engine state (for testing or re-initialization).
    pub fn reset(&mut self) {
        self.accumulated_offset = 0.0;
        self.previous_offset = 0.0;
        self.current_round = 0;
        self.stable_rounds = 0;
        self.peer_samples.clear();
        self.peer_sync.clear();
    }

    /// Compute median of a sorted slice.
    fn median(&self, sorted: &[f64]) -> f64 {
        let len = sorted.len();
        if len == 0 {
            return 0.0;
        }
        if len % 2 == 1 {
            sorted[len / 2]
        } else {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_sample(peer_id: u64, base: u64) -> TimeSample {
        TimeSample {
            t1: base,
            t2: base + 10,
            t3: base + 20,
            t4: base + 30,
            peer_id,
        }
    }

    #[test]
    fn test_timestamp_creation() {
        let ts = SymbioticTimestamp::new(1000, 1);
        assert_eq!(ts.logical_ms, 1000);
        assert_eq!(ts.node_id, 1);
    }

    #[test]
    fn test_timestamp_ordering() {
        let ts1 = SymbioticTimestamp::new(1000, 1);
        let ts2 = SymbioticTimestamp::new(1001, 1);
        assert!(ts1.is_before(&ts2));
        assert!(ts2.is_after(&ts1));
    }

    #[test]
    fn test_timestamp_tiebreak() {
        let ts1 = SymbioticTimestamp::new(1000, 1);
        let ts2 = SymbioticTimestamp::new(1000, 2);
        assert!(ts1.is_before(&ts2));
    }

    #[test]
    fn test_timestamp_equality() {
        let ts1 = SymbioticTimestamp::new(1000, 1);
        let ts2 = SymbioticTimestamp::new(1000, 1);
        assert_eq!(ts1, ts2);
    }

    #[test]
    fn test_sample_offset() {
        // Symmetric delay: offset should be 0.
        let sample = TimeSample {
            t1: 100, t2: 110, t3: 120, t4: 130,
            peer_id: 1,
        };
        let offset = sample.offset();
        assert!((offset - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_sample_delay() {
        let sample = TimeSample {
            t1: 100, t2: 110, t3: 120, t4: 130,
            peer_id: 1,
        };
        let delay = sample.delay();
        assert!((delay - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_sample_valid() {
        let valid = TimeSample {
            t1: 100, t2: 110, t3: 120, t4: 130,
            peer_id: 1,
        };
        assert!(valid.is_valid());

        let invalid = TimeSample {
            t1: 130, t2: 110, t3: 120, t4: 100,
            peer_id: 1,
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_engine_creation() {
        let engine = TemporalCohesionEngine::new(1);
        assert_eq!(engine.node_id, 1);
        assert_eq!(engine.peer_count(), 0);
    }

    #[test]
    fn test_engine_custom_config() {
        let config = TemporalConfig {
            max_samples_per_peer: 32,
            convergence_threshold: 25.0,
            ..Default::default()
        };
        let engine = TemporalCohesionEngine::with_config(1, config).unwrap();
        assert_eq!(engine.node_id, 1);
    }

    #[test]
    fn test_invalid_config_zero_samples() {
        let config = TemporalConfig {
            max_samples_per_peer: 0,
            ..Default::default()
        };
        let result = TemporalCohesionEngine::with_config(1, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_zero_filter() {
        let config = TemporalConfig {
            filter_window: 0,
            ..Default::default()
        };
        let result = TemporalCohesionEngine::with_config(1, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_record_sample() {
        let mut engine = TemporalCohesionEngine::new(1);
        let sample = valid_sample(2, 1000);
        engine.record_sample(sample).unwrap();
        assert_eq!(engine.peer_count(), 1);
    }

    #[test]
    fn test_record_invalid_sample() {
        let mut engine = TemporalCohesionEngine::new(1);
        let sample = TimeSample {
            t1: 200, t2: 100, t3: 150, t4: 120,
            peer_id: 2,
        };
        let result = engine.record_sample(sample);
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_round_no_peers() {
        let mut engine = TemporalCohesionEngine::new(1);
        let result = engine.sync_round();
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_round_convergence() {
        let mut engine = TemporalCohesionEngine::new(1);

        // Add samples from 3 peers with small offsets.
        for peer_id in 2..=4 {
            for i in 0..5 {
                let sample = valid_sample(peer_id, 1000 + i * 10);
                engine.record_sample(sample).unwrap();
            }
        }

        // Run multiple sync rounds.
        for _ in 0..10 {
            engine.sync_round().unwrap();
            engine.advance_time(10);
        }

        let status = engine.get_sync_status();
        assert!(matches!(status, SyncStatus::Converged));
    }

    #[test]
    fn test_generate_timestamp() {
        let engine = TemporalCohesionEngine::new(5);
        let ts = engine.generate_timestamp();
        assert_eq!(ts.node_id, 5);
    }

    #[test]
    fn test_timestamp_variance_single_peer() {
        let mut engine = TemporalCohesionEngine::new(1);
        let sample = valid_sample(2, 1000);
        engine.record_sample(sample).unwrap();
        let variance = engine.timestamp_variance();
        assert_eq!(variance, 0.0);
    }

    #[test]
    fn test_timestamp_variance_multiple_peers() {
        let mut engine = TemporalCohesionEngine::new(1);

        // Peer 2: zero offset (symmetric)
        for i in 0..3 {
            engine.record_sample(valid_sample(2, 1000 + i * 10)).unwrap();
        }
        // Peer 3: positive offset (asymmetric delays -> non-zero offset)
        // θ = ((t2-t1) + (t3-t4)) / 2 = ((30) + (-20)) / 2 = 5ms
        // Ordering: t1=1000 < t2=1030 < t3=1040 < t4=1060 ✓
        for i in 0..3 {
            let sample = TimeSample {
                t1: 1000 + i * 10,
                t2: 1030 + i * 10,
                t3: 1040 + i * 10,
                t4: 1060 + i * 10,
                peer_id: 3,
            };
            engine.record_sample(sample).unwrap();
        }

        engine.sync_round().unwrap();
        let variance = engine.timestamp_variance();
        assert!(variance > 0.0);
    }

    #[test]
    fn test_reset() {
        let mut engine = TemporalCohesionEngine::new(1);
        let sample = valid_sample(2, 1000);
        engine.record_sample(sample).unwrap();
        engine.sync_round().unwrap();

        engine.reset();
        assert_eq!(engine.peer_count(), 0);
        assert_eq!(engine.accumulated_offset(), 0.0);
    }

    #[test]
    fn test_advance_time() {
        let mut engine = TemporalCohesionEngine::new(1);
        let initial = engine.local_time_ms;
        engine.advance_time(100);
        assert_eq!(engine.local_time_ms, initial + 100);
    }

    #[test]
    fn test_insufficient_peers_status() {
        let mut engine = TemporalCohesionEngine::new(1);
        let sample = valid_sample(2, 1000);
        engine.record_sample(sample).unwrap();

        let status = engine.get_sync_status();
        assert!(matches!(status, SyncStatus::InsufficientPeers));
    }

    #[test]
    fn test_peer_sync_state() {
        let mut engine = TemporalCohesionEngine::new(1);
        let sample = valid_sample(2, 1000);
        engine.record_sample(sample).unwrap();

        let state = engine.get_peer_state(2);
        assert!(state.is_some());
        assert_eq!(state.unwrap().exchange_count, 1);
    }

    #[test]
    fn test_all_peer_states() {
        let mut engine = TemporalCohesionEngine::new(1);
        for peer_id in 2..=5 {
            let sample = valid_sample(peer_id, 1000);
            engine.record_sample(sample).unwrap();
        }

        let states = engine.get_all_peer_states();
        assert_eq!(states.len(), 4);
    }

    #[test]
    fn test_bounded_samples() {
        let config = TemporalConfig {
            max_samples_per_peer: 5,
            ..Default::default()
        };
        let mut engine = TemporalCohesionEngine::with_config(1, config).unwrap();

        for i in 0..10 {
            let sample = valid_sample(2, 1000 + i);
            engine.record_sample(sample).unwrap();
        }

        let samples = engine.peer_samples.get(&2).unwrap();
        assert_eq!(samples.len(), 5);
    }

    #[test]
    fn test_sync_status_in_progress() {
        let config = TemporalConfig {
            convergence_rounds: 100,
            ..Default::default()
        };
        let mut engine = TemporalCohesionEngine::with_config(1, config).unwrap();

        for peer_id in 2..=4 {
            let sample = valid_sample(peer_id, 1000);
            engine.record_sample(sample).unwrap();
        }

        engine.sync_round().unwrap();
        let status = engine.get_sync_status();
        assert!(matches!(status, SyncStatus::InProgress { .. }));
    }
}
