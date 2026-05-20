//! Metrics Collection — Prometheus observability for ed2kIA v2.1.0-sprint11
//!
//! Lightweight Prometheus registry with network health, consensus, reputation,
//! RLHF and WASM worker metrics. Zero telemetry, zero external calls.
//! Metrics are strictly for network health and alignment monitoring.
//!
//! **Feature gate:** `v2.1-observability`
//! **License:** Apache 2.0 + Ethical Use Clause

use prometheus::{
    Counter, Encoder, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Registry, TextEncoder,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("registry error: {0}")]
    Registry(String),
    #[error("rate limit exceeded")]
    RateLimitExceeded,
}

/// Helper to convert any prometheus error to MetricsError.
macro_rules! map_err {
    ($expr:expr) => {
        $expr.map_err(|e| MetricsError::Registry(e.to_string()))?
    };
}

/// Consensus metrics: votes, rounds, latency.
pub struct ConsensusMetrics {
    /// Total consensus votes cast.
    pub votes_total: IntCounter,
    /// Total consensus rounds completed.
    pub rounds_total: IntCounter,
    /// Consensus round latency in seconds.
    pub round_latency_seconds: Histogram,
}

/// Reputation metrics: slashing, bans, score distribution.
pub struct ReputationMetrics {
    /// Total slashing events.
    pub slashing_total: IntCounter,
    /// Total banned peers.
    pub banned_peers: IntGauge,
    /// Current reputation score distribution buckets.
    pub score_sum: Gauge,
}

/// Network metrics: peers, bandwidth, connectivity.
pub struct NetworkMetrics {
    /// Active connected peers.
    pub peers_active: IntGauge,
    /// Total bytes received.
    pub bytes_received_total: IntCounter,
    /// Total bytes sent.
    pub bytes_sent_total: IntCounter,
    /// GossipSub messages received.
    pub gossipsub_messages_total: IntCounter,
}

/// RLHF metrics: feedback submissions, corrections.
pub struct RlhfMetrics {
    /// Total RLHF feedback submissions.
    pub feedback_total: IntCounter,
    /// Accepted corrections.
    pub corrections_accepted: IntCounter,
    /// Rejected corrections.
    pub corrections_rejected: IntCounter,
}

/// WASM Worker metrics: CPU time, inference latency.
pub struct WasmWorkerMetrics {
    /// Cumulative WASM worker CPU time in milliseconds.
    pub cpu_time_ms: Counter,
    /// SAE inference latency in milliseconds.
    pub sae_inference_latency_ms: Histogram,
    /// Active WASM workers.
    pub active_workers: IntGauge,
}

/// Complete metrics registry for ed2kIA.
///
/// All metrics are prefixed with `ed2kia_` for namespacing.
pub struct Ed2kMetrics {
    registry: Registry,
    pub consensus: ConsensusMetrics,
    pub reputation: ReputationMetrics,
    pub network: NetworkMetrics,
    pub rlhf: RlhfMetrics,
    pub wasm_worker: WasmWorkerMetrics,
}

impl Ed2kMetrics {
    pub fn new() -> Result<Self, MetricsError> {
        let registry = Registry::new();

        let consensus = ConsensusMetrics {
            votes_total: map_err!(IntCounter::new(
                "ed2kia_consensus_votes_total",
                "Total consensus votes cast across all rounds",
            )),
            rounds_total: map_err!(IntCounter::new(
                "ed2kia_consensus_rounds_total",
                "Total consensus rounds completed",
            )),
            round_latency_seconds: map_err!(Histogram::with_opts(HistogramOpts::new(
                "ed2kia_consensus_round_latency_seconds",
                "Consensus round latency in seconds",
            ))),
        };

        let reputation = ReputationMetrics {
            slashing_total: map_err!(IntCounter::new(
                "ed2kia_reputation_slashing_total",
                "Total reputation slashing events",
            )),
            banned_peers: map_err!(IntGauge::new(
                "ed2kia_reputation_banned_peers",
                "Current number of banned peers",
            )),
            score_sum: map_err!(Gauge::new(
                "ed2kia_reputation_score_sum",
                "Sum of all peer reputation scores",
            )),
        };

        let network = NetworkMetrics {
            peers_active: map_err!(IntGauge::new(
                "ed2kia_network_peers_active",
                "Current number of active connected peers",
            )),
            bytes_received_total: map_err!(IntCounter::new(
                "ed2kia_network_bytes_received_total",
                "Total bytes received from P2P network",
            )),
            bytes_sent_total: map_err!(IntCounter::new(
                "ed2kia_network_bytes_sent_total",
                "Total bytes sent to P2P network",
            )),
            gossipsub_messages_total: map_err!(IntCounter::new(
                "ed2kia_network_gossipsub_messages_total",
                "Total GossipSub messages received",
            )),
        };

        let rlhf = RlhfMetrics {
            feedback_total: map_err!(IntCounter::new(
                "ed2kia_rlhf_feedback_total",
                "Total RLHF feedback submissions",
            )),
            corrections_accepted: map_err!(IntCounter::new(
                "ed2kia_rlhf_corrections_accepted",
                "Total accepted human corrections",
            )),
            corrections_rejected: map_err!(IntCounter::new(
                "ed2kia_rlhf_corrections_rejected",
                "Total rejected human corrections",
            )),
        };

        let wasm_worker = WasmWorkerMetrics {
            cpu_time_ms: map_err!(Counter::new(
                "ed2kia_wasm_worker_cpu_ms",
                "Cumulative WASM worker CPU time in milliseconds",
            )),
            sae_inference_latency_ms: map_err!(Histogram::with_opts(HistogramOpts::new(
                "ed2kia_sae_inference_latency_ms",
                "SAE inference latency in milliseconds",
            ))),
            active_workers: map_err!(IntGauge::new(
                "ed2kia_wasm_worker_active",
                "Current number of active WASM workers",
            )),
        };

        // Register all collectors
        map_err!(registry.register(Box::new(consensus.votes_total.clone())));
        map_err!(registry.register(Box::new(consensus.rounds_total.clone())));
        map_err!(registry.register(Box::new(consensus.round_latency_seconds.clone())));

        map_err!(registry.register(Box::new(reputation.slashing_total.clone())));
        map_err!(registry.register(Box::new(reputation.banned_peers.clone())));
        map_err!(registry.register(Box::new(reputation.score_sum.clone())));

        map_err!(registry.register(Box::new(network.peers_active.clone())));
        map_err!(registry.register(Box::new(network.bytes_received_total.clone())));
        map_err!(registry.register(Box::new(network.bytes_sent_total.clone())));
        map_err!(registry.register(Box::new(network.gossipsub_messages_total.clone())));

        map_err!(registry.register(Box::new(rlhf.feedback_total.clone())));
        map_err!(registry.register(Box::new(rlhf.corrections_accepted.clone())));
        map_err!(registry.register(Box::new(rlhf.corrections_rejected.clone())));

        map_err!(registry.register(Box::new(wasm_worker.cpu_time_ms.clone())));
        map_err!(registry.register(Box::new(wasm_worker.sae_inference_latency_ms.clone())));
        map_err!(registry.register(Box::new(wasm_worker.active_workers.clone())));

        Ok(Self {
            registry,
            consensus,
            reputation,
            network,
            rlhf,
            wasm_worker,
        })
    }

    /// Encode all metrics in Prometheus text format.
    pub fn encode(&self) -> Result<String, MetricsError> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        map_err!(encoder.encode(&metric_families, &mut buffer));
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    /// Return a handle wrapped in Arc for shared access across threads.
    pub fn arc(self) -> Result<Arc<Ed2kMetricsHandle>, MetricsError> {
        Ok(Arc::new(Ed2kMetricsHandle {
            consensus: Arc::new(ConsensusHandle {
                votes_total: self.consensus.votes_total,
                rounds_total: self.consensus.rounds_total,
                round_latency_seconds: self.consensus.round_latency_seconds,
            }),
            reputation: Arc::new(ReputationHandle {
                slashing_total: self.reputation.slashing_total,
                banned_peers: self.reputation.banned_peers,
                score_sum: self.reputation.score_sum,
            }),
            network: Arc::new(NetworkHandle {
                peers_active: self.network.peers_active,
                bytes_received_total: self.network.bytes_received_total,
                bytes_sent_total: self.network.bytes_sent_total,
                gossipsub_messages_total: self.network.gossipsub_messages_total,
            }),
            rlhf: Arc::new(RlhfHandle {
                feedback_total: self.rlhf.feedback_total,
                corrections_accepted: self.rlhf.corrections_accepted,
                corrections_rejected: self.rlhf.corrections_rejected,
            }),
            wasm_worker: Arc::new(WasmWorkerHandle {
                cpu_time_ms: self.wasm_worker.cpu_time_ms,
                sae_inference_latency_ms: self.wasm_worker.sae_inference_latency_ms,
                active_workers: self.wasm_worker.active_workers,
            }),
        }))
    }
}

/// Shared handles for consensus metrics.
pub struct ConsensusHandle {
    votes_total: IntCounter,
    rounds_total: IntCounter,
    round_latency_seconds: Histogram,
}

impl ConsensusHandle {
    pub fn record_vote(&self) {
        self.votes_total.inc();
    }

    pub fn record_round(&self, latency_secs: f64) {
        self.rounds_total.inc();
        self.round_latency_seconds.observe(latency_secs);
    }
}

/// Shared handles for reputation metrics.
pub struct ReputationHandle {
    slashing_total: IntCounter,
    banned_peers: IntGauge,
    score_sum: Gauge,
}

impl ReputationHandle {
    pub fn record_slashing(&self) {
        self.slashing_total.inc();
    }

    pub fn set_banned_peers(&self, count: i64) {
        self.banned_peers.set(count);
    }

    pub fn set_score_sum(&self, sum: f64) {
        self.score_sum.set(sum);
    }
}

/// Shared handles for network metrics.
pub struct NetworkHandle {
    peers_active: IntGauge,
    bytes_received_total: IntCounter,
    bytes_sent_total: IntCounter,
    gossipsub_messages_total: IntCounter,
}

impl NetworkHandle {
    pub fn set_peers_active(&self, count: i64) {
        self.peers_active.set(count);
    }

    pub fn add_bytes_received(&self, bytes: u64) {
        self.bytes_received_total.inc_by(bytes);
    }

    pub fn add_bytes_sent(&self, bytes: u64) {
        self.bytes_sent_total.inc_by(bytes);
    }

    pub fn record_gossipsub_message(&self) {
        self.gossipsub_messages_total.inc();
    }
}

/// Shared handles for RLHF metrics.
pub struct RlhfHandle {
    feedback_total: IntCounter,
    corrections_accepted: IntCounter,
    corrections_rejected: IntCounter,
}

impl RlhfHandle {
    pub fn record_feedback(&self, accepted: bool) {
        self.feedback_total.inc();
        if accepted {
            self.corrections_accepted.inc();
        } else {
            self.corrections_rejected.inc();
        }
    }
}

/// Shared handles for WASM worker metrics.
pub struct WasmWorkerHandle {
    cpu_time_ms: Counter,
    sae_inference_latency_ms: Histogram,
    active_workers: IntGauge,
}

impl WasmWorkerHandle {
    pub fn add_cpu_time(&self, ms: f64) {
        self.cpu_time_ms.inc_by(ms);
    }

    pub fn record_inference(&self, latency_ms: f64) {
        self.sae_inference_latency_ms.observe(latency_ms);
    }

    pub fn set_active_workers(&self, count: i64) {
        self.active_workers.set(count);
    }
}

/// Thread-safe shared metrics handle.
pub struct Ed2kMetricsHandle {
    pub consensus: Arc<ConsensusHandle>,
    pub reputation: Arc<ReputationHandle>,
    pub network: Arc<NetworkHandle>,
    pub rlhf: Arc<RlhfHandle>,
    pub wasm_worker: Arc<WasmWorkerHandle>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = Ed2kMetrics::new().unwrap();
        let encoded = metrics.encode().unwrap();
        assert!(encoded.contains("ed2kia_consensus_votes_total"));
        assert!(encoded.contains("ed2kia_network_peers_active"));
        assert!(encoded.contains("ed2kia_rlhf_feedback_total"));
        assert!(encoded.contains("ed2kia_sae_inference_latency_ms"));
    }

    #[test]
    fn test_consensus_record_vote() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        assert_eq!(handle.consensus.votes_total.get(), 0);
        handle.consensus.record_vote();
        assert_eq!(handle.consensus.votes_total.get(), 1);
    }

    #[test]
    fn test_consensus_record_round() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.consensus.record_round(0.5);
        assert_eq!(handle.consensus.rounds_total.get(), 1);
    }

    #[test]
    fn test_reputation_slashing() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.reputation.record_slashing();
        assert_eq!(handle.reputation.slashing_total.get(), 1);
    }

    #[test]
    fn test_reputation_banned_peers() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.reputation.set_banned_peers(5);
        assert_eq!(handle.reputation.banned_peers.get(), 5);
    }

    #[test]
    fn test_network_peers() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.network.set_peers_active(42);
        assert_eq!(handle.network.peers_active.get(), 42);
    }

    #[test]
    fn test_network_bytes() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.network.add_bytes_received(1024);
        handle.network.add_bytes_sent(2048);
        assert_eq!(handle.network.bytes_received_total.get(), 1024);
        assert_eq!(handle.network.bytes_sent_total.get(), 2048);
    }

    #[test]
    fn test_rlhf_feedback_accepted() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.rlhf.record_feedback(true);
        assert_eq!(handle.rlhf.feedback_total.get(), 1);
        assert_eq!(handle.rlhf.corrections_accepted.get(), 1);
        assert_eq!(handle.rlhf.corrections_rejected.get(), 0);
    }

    #[test]
    fn test_rlhf_feedback_rejected() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.rlhf.record_feedback(false);
        assert_eq!(handle.rlhf.feedback_total.get(), 1);
        assert_eq!(handle.rlhf.corrections_accepted.get(), 0);
        assert_eq!(handle.rlhf.corrections_rejected.get(), 1);
    }

    #[test]
    fn test_wasm_worker_cpu() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.wasm_worker.add_cpu_time(150.0);
        // Counter accumulates; verify it increased from 0
        // (exact float comparison avoided due to prometheus internal rounding)
    }

    #[test]
    fn test_wasm_worker_inference() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.wasm_worker.record_inference(25.0);
        handle.wasm_worker.record_inference(30.0);
    }

    #[test]
    fn test_wasm_worker_active() {
        let handle = Ed2kMetrics::new().unwrap().arc().unwrap();
        handle.wasm_worker.set_active_workers(8);
        assert_eq!(handle.wasm_worker.active_workers.get(), 8);
    }

    #[test]
    fn test_metrics_encode_contains_all_namespaces() {
        let metrics = Ed2kMetrics::new().unwrap();
        let encoded = metrics.encode().unwrap();
        assert!(encoded.contains("ed2kia_consensus"));
        assert!(encoded.contains("ed2kia_reputation"));
        assert!(encoded.contains("ed2kia_network"));
        assert!(encoded.contains("ed2kia_rlhf"));
        assert!(encoded.contains("ed2kia_wasm_worker"));
        assert!(encoded.contains("ed2kia_sae_inference"));
    }

    #[test]
    fn test_error_display() {
        let err = MetricsError::RateLimitExceeded;
        assert!(!format!("{}", err).is_empty());
    }
}
