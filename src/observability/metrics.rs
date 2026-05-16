//! Metrics Collection — Placeholder structs for Prometheus/Grafana integration
//!
//! **STATUS:** SCAFFOLD ONLY — Zero functional logic.
//! **APPROVAL REQUIRED:** RFC-002 approval required before implementation.
//! **LICENSE:** Apache 2.0 + Cláusula de Uso Ética

/// Trait for observable components that can expose metrics.
///
/// Implementation pending RFC-002 approval.
pub trait Observable {
    /// Collects metrics as key-value pairs.
    ///
    /// Returns a vector of (metric_name, value) tuples.
    /// Format compatible with Prometheus exposition.
    fn collect(&self) -> Vec<(&str, f64)>;
}

/// Placeholder: Node-level metrics collection.
///
/// Target: CPU, memory, tensor throughput, P2P bandwidth.
/// Implementation pending RFC-002 approval.
pub struct NodeMetrics;

/// Placeholder: Lease management metrics.
///
/// Target: Lease creation rate, expiration rate, renewal latency.
/// Implementation pending RFC-002 approval.
pub struct LeaseMetrics;

/// Placeholder: Federation consensus metrics.
///
/// Target: Round latency, participation rate, vote distribution.
/// Implementation pending RFC-002 approval.
pub struct FederationMetrics;

/// Placeholder: ZKP pipeline metrics.
///
/// Target: Proof generation time, verification time, circuit complexity.
/// Implementation pending RFC-002 approval.
pub struct ZkpMetrics;
