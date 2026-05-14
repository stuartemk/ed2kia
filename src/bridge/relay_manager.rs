//! Relay Manager — Quorum-based relay coordination for cross-chain message delivery.
//!
//! Manages relay nodes, tracks signatures, enforces quorum thresholds, and coordinates
//! message delivery with fallback strategies. Designed for sub-100ms relay latency.
//!
//! **Design:** Signature collection with quorum enforcement and adaptive relay routing.
//! Zero financial logic — operates on compute credits and technical state only.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.6-sprint1")]
mod internal {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for relay manager operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum RelayError {
        /// Relay node not found.
        NodeNotFound(String),
        /// Quorum not reached.
        QuorumNotReached { current: u32, required: u32 },
        /// Duplicate signature.
        DuplicateSignature(String),
        /// Relay capacity exceeded.
        RelayFull,
        /// Message not found.
        MessageNotFound(String),
        /// Relay timeout exceeded.
        TimeoutExceeded,
        /// Node reputation too low.
        ReputationTooLow { current: f64, min: f64 },
    }

    impl std::fmt::Display for RelayError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                RelayError::NodeNotFound(id) => write!(f, "Relay node {} not found", id),
                RelayError::QuorumNotReached { current, required } => {
                    write!(
                        f,
                        "Quorum not reached: {} signatures, {} required",
                        current, required
                    )
                }
                RelayError::DuplicateSignature(id) => {
                    write!(f, "Duplicate signature from node {}", id)
                }
                RelayError::RelayFull => write!(f, "Relay capacity exceeded"),
                RelayError::MessageNotFound(id) => write!(f, "Message {} not found", id),
                RelayError::TimeoutExceeded => write!(f, "Relay timeout exceeded"),
                RelayError::ReputationTooLow { current, min } => {
                    write!(
                        f,
                        "Node reputation {:.4} below minimum {:.4}",
                        current, min
                    )
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for the relay manager.
    #[derive(Debug, Clone)]
    pub struct RelayConfig {
        /// Minimum number of signatures for quorum.
        pub min_quorum_signatures: u32,
        /// Maximum relay nodes.
        pub max_relay_nodes: usize,
        /// Relay timeout in milliseconds.
        pub relay_timeout_ms: u64,
        /// Minimum node reputation for relay participation.
        pub min_node_reputation: f64,
        /// Maximum pending relays.
        pub max_pending_relays: usize,
        /// Enable adaptive routing.
        pub enable_adaptive_routing: bool,
    }

    impl Default for RelayConfig {
        fn default() -> Self {
            Self {
                min_quorum_signatures: 3,
                max_relay_nodes: 32,
                relay_timeout_ms: 100,
                min_node_reputation: 0.5,
                max_pending_relays: 256,
                enable_adaptive_routing: true,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Relay Node
    // ---------------------------------------------------------------------------

    /// Relay node entry with reputation and latency tracking.
    #[derive(Debug, Clone)]
    pub struct RelayNode {
        pub node_id: String,
        pub reputation: f64,
        pub avg_latency_ms: f64,
        pub signatures_provided: u64,
        pub signatures_failed: u64,
        pub active: bool,
    }

    impl RelayNode {
        pub fn new(node_id: String, reputation: f64) -> Self {
            Self {
                node_id,
                reputation,
                avg_latency_ms: 0.0,
                signatures_provided: 0,
                signatures_failed: 0,
                active: true,
            }
        }

        pub fn update_latency(&mut self, latency_ms: f64, alpha: f64) {
            self.avg_latency_ms = self.avg_latency_ms * (1.0 - alpha) + latency_ms * alpha;
        }

        pub fn record_signature(&mut self, success: bool, alpha: f64) {
            if success {
                self.signatures_provided += 1;
            } else {
                self.signatures_failed += 1;
            }
            let signal = if success { 1.0 } else { 0.0 };
            self.reputation = self.reputation * (1.0 - alpha) + signal * alpha;
            self.reputation = self.reputation.clamp(0.0, 1.0);
        }

        pub fn success_rate(&self) -> f64 {
            let total = self.signatures_provided + self.signatures_failed;
            if total == 0 {
                return 1.0;
            }
            self.signatures_provided as f64 / total as f64
        }

        pub fn relay_score(&self) -> f64 {
            // Higher reputation and lower latency = better score
            self.reputation * 100.0 / (1.0 + self.avg_latency_ms)
        }
    }

    // ---------------------------------------------------------------------------
    // Relay Session
    // ---------------------------------------------------------------------------

    /// Relay session tracking for a single message.
    #[derive(Debug, Clone)]
    pub struct RelaySession {
        pub message_id: String,
        pub signatures: Vec<String>,
        pub created_at: Instant,
        pub timeout_ms: u64,
        pub completed: bool,
        pub relay_time_ms: u64,
    }

    impl RelaySession {
        pub fn new(message_id: String, timeout_ms: u64) -> Self {
            Self {
                message_id,
                signatures: Vec::new(),
                created_at: Instant::now(),
                timeout_ms,
                completed: false,
                relay_time_ms: 0,
            }
        }

        pub fn is_expired(&self) -> bool {
            self.created_at.elapsed() > Duration::from_millis(self.timeout_ms)
        }

        pub fn quorum_reached(&self, min_signatures: u32) -> bool {
            self.signatures.len() as u32 >= min_signatures
        }

        pub fn add_signature(&mut self, node_id: &str) -> Result<(), RelayError> {
            if self.signatures.contains(&node_id.to_string()) {
                return Err(RelayError::DuplicateSignature(node_id.to_string()));
            }
            self.signatures.push(node_id.to_string());
            Ok(())
        }
    }

    // ---------------------------------------------------------------------------
    // Relay Stats
    // ---------------------------------------------------------------------------

    /// Statistics for relay operations.
    #[derive(Debug, Clone)]
    pub struct RelayStats {
        pub relays_initiated: u64,
        pub relays_completed: u64,
        pub relays_failed: u64,
        pub relays_timed_out: u64,
        pub avg_relay_time_ms: f64,
        pub total_signatures: u64,
    }

    impl Default for RelayStats {
        fn default() -> Self {
            Self {
                relays_initiated: 0,
                relays_completed: 0,
                relays_failed: 0,
                relays_timed_out: 0,
                avg_relay_time_ms: 0.0,
                total_signatures: 0,
            }
        }
    }

    impl RelayStats {
        pub fn record_completion(&mut self, time_ms: u64) {
            self.relays_completed += 1;
            self.avg_relay_time_ms = self.avg_relay_time_ms * 0.9 + time_ms as f64 * 0.1;
        }

        pub fn record_failure(&mut self) {
            self.relays_failed += 1;
        }

        pub fn record_timeout(&mut self) {
            self.relays_timed_out += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Relay Manager
    // ---------------------------------------------------------------------------

    /// Relay manager with quorum-based coordination.
    #[derive(Debug)]
    pub struct RelayManager {
        config: RelayConfig,
        nodes: HashMap<String, RelayNode>,
        sessions: HashMap<String, RelaySession>,
        stats: RelayStats,
    }

    impl RelayManager {
        /// Create a new relay manager with the given configuration.
        pub fn new(config: RelayConfig) -> Self {
            Self {
                config,
                nodes: HashMap::new(),
                sessions: HashMap::new(),
                stats: RelayStats::default(),
            }
        }

        /// Register a relay node with initial reputation.
        pub fn register_node(
            &mut self,
            node_id: String,
            reputation: f64,
        ) -> Result<(), RelayError> {
            if self.nodes.len() >= self.config.max_relay_nodes {
                return Err(RelayError::RelayFull);
            }
            if self.nodes.contains_key(&node_id) {
                return Err(RelayError::NodeNotFound(node_id.clone()));
            }
            self.nodes
                .insert(node_id.clone(), RelayNode::new(node_id, reputation));
            Ok(())
        }

        /// Create a new relay session for a message.
        pub fn create_session(&mut self, message_id: String) -> Result<(), RelayError> {
            if self.sessions.len() >= self.config.max_pending_relays {
                return Err(RelayError::RelayFull);
            }
            let session = RelaySession::new(message_id.clone(), self.config.relay_timeout_ms);
            self.sessions.insert(message_id, session);
            self.stats.relays_initiated += 1;
            Ok(())
        }

        /// Submit a signature from a relay node.
        pub fn submit_signature(
            &mut self,
            message_id: &str,
            node_id: &str,
        ) -> Result<(), RelayError> {
            // Check node exists and meets reputation
            let node = self
                .nodes
                .get(node_id)
                .ok_or_else(|| RelayError::NodeNotFound(node_id.to_string()))?;

            if node.reputation < self.config.min_node_reputation {
                return Err(RelayError::ReputationTooLow {
                    current: node.reputation,
                    min: self.config.min_node_reputation,
                });
            }

            // Check session exists
            let session = self
                .sessions
                .get_mut(message_id)
                .ok_or_else(|| RelayError::MessageNotFound(message_id.to_string()))?;

            if session.is_expired() {
                return Err(RelayError::TimeoutExceeded);
            }

            // Add signature
            session.add_signature(node_id)?;

            // Update node stats
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.record_signature(true, 0.1);
            }

            self.stats.total_signatures += 1;
            Ok(())
        }

        /// Complete a relay session if quorum is reached.
        pub fn complete_relay(&mut self, message_id: &str) -> Result<(), RelayError> {
            let session = self
                .sessions
                .get_mut(message_id)
                .ok_or_else(|| RelayError::MessageNotFound(message_id.to_string()))?;

            if !session.quorum_reached(self.config.min_quorum_signatures) {
                return Err(RelayError::QuorumNotReached {
                    current: session.signatures.len() as u32,
                    required: self.config.min_quorum_signatures,
                });
            }

            let relay_time = session.created_at.elapsed().as_millis() as u64;
            session.relay_time_ms = relay_time;
            session.completed = true;

            self.stats.record_completion(relay_time);
            Ok(())
        }

        /// Get the best relay nodes for a message (sorted by relay score).
        pub fn get_best_nodes(&self, count: usize) -> Vec<&RelayNode> {
            let mut nodes: Vec<&RelayNode> = self.nodes.values().filter(|n| n.active).collect();
            nodes.sort_by(|a, b| {
                b.relay_score()
                    .partial_cmp(&a.relay_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            nodes.into_iter().take(count).collect()
        }

        /// Clean up expired sessions.
        pub fn cleanup_expired(&mut self) -> usize {
            let before = self.sessions.len();
            self.sessions.retain(|_, v| !v.is_expired() || v.completed);
            before - self.sessions.len()
        }

        /// Get relay statistics.
        pub fn get_stats(&self) -> &RelayStats {
            &self.stats
        }

        /// Reset relay statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Get registered node count.
        pub fn node_count(&self) -> usize {
            self.nodes.len()
        }

        /// Get active session count.
        pub fn active_session_count(&self) -> usize {
            self.sessions.values().filter(|s| !s.completed).count()
        }
    }

    impl Default for RelayManager {
        fn default() -> Self {
            Self::new(RelayConfig::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_relay_creation() {
            let relay = RelayManager::default();
            assert_eq!(relay.node_count(), 0);
            assert_eq!(relay.active_session_count(), 0);
        }

        #[test]
        fn test_relay_with_config() {
            let config = RelayConfig {
                min_quorum_signatures: 5,
                ..Default::default()
            };
            let relay = RelayManager::new(config);
            assert_eq!(relay.config.min_quorum_signatures, 5);
        }

        #[test]
        fn test_register_node() {
            let mut relay = RelayManager::default();
            assert!(relay.register_node("node_1".to_string(), 0.9).is_ok());
            assert_eq!(relay.node_count(), 1);
        }

        #[test]
        fn test_register_node_duplicate() {
            let mut relay = RelayManager::default();
            relay.register_node("node_1".to_string(), 0.9).unwrap();
            match relay.register_node("node_1".to_string(), 0.8).unwrap_err() {
                RelayError::NodeNotFound(id) => assert_eq!(id, "node_1"),
                e => panic!("Expected NodeNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_create_session() {
            let mut relay = RelayManager::default();
            assert!(relay.create_session("msg_1".to_string()).is_ok());
            assert_eq!(relay.active_session_count(), 1);
        }

        #[test]
        fn test_submit_signature() {
            let mut relay = RelayManager::default();
            relay.register_node("n1".to_string(), 0.9).unwrap();
            relay.create_session("msg_1".to_string()).unwrap();
            assert!(relay.submit_signature("msg_1", "n1").is_ok());
        }

        #[test]
        fn test_submit_signature_node_not_found() {
            let mut relay = RelayManager::default();
            relay.create_session("msg_1".to_string()).unwrap();
            match relay.submit_signature("msg_1", "unknown").unwrap_err() {
                RelayError::NodeNotFound(id) => assert_eq!(id, "unknown"),
                e => panic!("Expected NodeNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_submit_signature_reputation_too_low() {
            let mut relay = RelayManager::default();
            relay.register_node("low_rep".to_string(), 0.1).unwrap();
            relay.create_session("msg_1".to_string()).unwrap();
            match relay.submit_signature("msg_1", "low_rep").unwrap_err() {
                RelayError::ReputationTooLow { .. } => {}
                e => panic!("Expected ReputationTooLow, got {:?}", e),
            }
        }

        #[test]
        fn test_duplicate_signature() {
            let mut relay = RelayManager::default();
            relay.register_node("n1".to_string(), 0.9).unwrap();
            relay.create_session("msg_1".to_string()).unwrap();
            relay.submit_signature("msg_1", "n1").unwrap();
            match relay.submit_signature("msg_1", "n1").unwrap_err() {
                RelayError::DuplicateSignature(id) => assert_eq!(id, "n1"),
                e => panic!("Expected DuplicateSignature, got {:?}", e),
            }
        }

        #[test]
        fn test_complete_relay_quorum_met() {
            let mut relay = RelayManager::default();
            relay.register_node("n1".to_string(), 0.9).unwrap();
            relay.register_node("n2".to_string(), 0.8).unwrap();
            relay.register_node("n3".to_string(), 0.7).unwrap();
            relay.create_session("msg_1".to_string()).unwrap();
            relay.submit_signature("msg_1", "n1").unwrap();
            relay.submit_signature("msg_1", "n2").unwrap();
            relay.submit_signature("msg_1", "n3").unwrap();
            assert!(relay.complete_relay("msg_1").is_ok());
        }

        #[test]
        fn test_complete_relay_quorum_not_met() {
            let mut relay = RelayManager::default();
            relay.register_node("n1".to_string(), 0.9).unwrap();
            relay.create_session("msg_1".to_string()).unwrap();
            relay.submit_signature("msg_1", "n1").unwrap();
            match relay.complete_relay("msg_1").unwrap_err() {
                RelayError::QuorumNotReached { current, required } => {
                    assert_eq!(current, 1);
                    assert_eq!(required, 3);
                }
                e => panic!("Expected QuorumNotReached, got {:?}", e),
            }
        }

        #[test]
        fn test_get_best_nodes() {
            let mut relay = RelayManager::default();
            relay.register_node("high_rep".to_string(), 0.95).unwrap();
            relay.register_node("med_rep".to_string(), 0.7).unwrap();
            relay.register_node("low_rep".to_string(), 0.5).unwrap();
            let best = relay.get_best_nodes(2);
            assert_eq!(best.len(), 2);
            assert_eq!(best[0].node_id, "high_rep");
        }

        #[test]
        fn test_node_relay_score() {
            let mut node = RelayNode::new("test".to_string(), 0.9);
            node.avg_latency_ms = 10.0;
            let score = node.relay_score();
            assert!((score - 0.9 * 100.0 / 11.0).abs() < 0.01);
        }

        #[test]
        fn test_node_success_rate() {
            let mut node = RelayNode::new("test".to_string(), 0.9);
            node.signatures_provided = 90;
            node.signatures_failed = 10;
            assert!((node.success_rate() - 0.9).abs() < 0.01);
        }

        #[test]
        fn test_stats_recording() {
            let mut relay = RelayManager::default();
            relay.register_node("n1".to_string(), 0.9).unwrap();
            relay.register_node("n2".to_string(), 0.8).unwrap();
            relay.register_node("n3".to_string(), 0.7).unwrap();
            relay.create_session("msg_1".to_string()).unwrap();
            relay.submit_signature("msg_1", "n1").unwrap();
            relay.submit_signature("msg_1", "n2").unwrap();
            relay.submit_signature("msg_1", "n3").unwrap();
            relay.complete_relay("msg_1").unwrap();
            let stats = relay.get_stats();
            assert_eq!(stats.relays_initiated, 1);
            assert_eq!(stats.relays_completed, 1);
            assert_eq!(stats.total_signatures, 3);
        }

        #[test]
        fn test_reset_stats() {
            let mut relay = RelayManager::default();
            relay.create_session("msg_1".to_string()).unwrap();
            relay.reset_stats();
            let stats = relay.get_stats();
            assert_eq!(stats.relays_initiated, 0);
        }

        #[test]
        fn test_error_display() {
            let err = RelayError::NodeNotFound("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_config_default() {
            let config = RelayConfig::default();
            assert_eq!(config.min_quorum_signatures, 3);
            assert!(config.enable_adaptive_routing);
        }
    }
}

#[cfg(feature = "v1.6-sprint1")]
pub use internal::*;
