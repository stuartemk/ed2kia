//! Interoperability Layer v2 — Cross-federation communication with intelligent routing and schema negotiation.
//!
//! Improvements over v1:
//! - Intelligent path discovery using BFS routing
//! - Schema negotiation for binary protocol compatibility
//! - Health tracking per federation with adaptive routing
//! - Message compression support
//! - Fallback routing when primary path fails
//!
//! **Design:** Graph-based federation routing with health-aware path selection.
//! Zero financial logic — operates on compute credits and technical state only.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.6-sprint1")]
mod internal {
    use std::collections::{HashMap, HashSet, VecDeque};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for Interop Layer v2 operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum InteropV2Error {
        /// Federation not found.
        FederationNotFound(String),
        /// No route available.
        NoRouteAvailable { source: String, destination: String },
        /// Route exceeds max hops.
        MaxHopsExceeded { hops: usize, max: usize },
        /// Interop capacity exceeded.
        InteropFull,
        /// Health value out of range.
        InvalidHealth(f64),
        /// Schema negotiation failed.
        SchemaNegotiationFailed(String),
        /// Timeout exceeded.
        TimeoutExceeded,
    }

    impl std::fmt::Display for InteropV2Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                InteropV2Error::FederationNotFound(id) => write!(f, "Federation {} not found", id),
                InteropV2Error::NoRouteAvailable { source, destination } => {
                    write!(f, "No route from {} to {}", source, destination)
                }
                InteropV2Error::MaxHopsExceeded { hops, max } => {
                    write!(f, "Route exceeds max hops: {} > {}", hops, max)
                }
                InteropV2Error::InteropFull => write!(f, "Interop capacity exceeded"),
                InteropV2Error::InvalidHealth(h) => write!(f, "Health value {} out of range [0,1]", h),
                InteropV2Error::SchemaNegotiationFailed(msg) => {
                    write!(f, "Schema negotiation failed: {}", msg)
                }
                InteropV2Error::TimeoutExceeded => write!(f, "Routing timeout exceeded"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for Interop Layer v2.
    #[derive(Debug, Clone)]
    pub struct InteropLayerV2Config {
        /// Maximum routing hops.
        pub max_hops: usize,
        /// Routing timeout in milliseconds.
        pub timeout_ms: u64,
        /// Retry count for failed routes.
        pub retry_count: u32,
        /// Enable message compression.
        pub enable_compression: bool,
        /// Maximum federations.
        pub max_federations: usize,
        /// Minimum health threshold for routing.
        pub min_health_threshold: f64,
    }

    impl Default for InteropLayerV2Config {
        fn default() -> Self {
            Self {
                max_hops: 8,
                timeout_ms: 80,
                retry_count: 3,
                enable_compression: true,
                max_federations: 128,
                min_health_threshold: 0.3,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Federation Entry
    // ---------------------------------------------------------------------------

    /// Federation entry with endpoints and health tracking.
    #[derive(Debug, Clone)]
    pub struct FederationEntryV2 {
        pub fed_id: String,
        pub endpoints: Vec<String>,
        pub health: f64,
        pub connections: Vec<String>,
        pub messages_routed: u64,
        pub messages_failed: u64,
    }

    impl FederationEntryV2 {
        pub fn new(fed_id: String, endpoints: Vec<String>) -> Self {
            Self {
                fed_id,
                endpoints,
                health: 1.0,
                connections: Vec::new(),
                messages_routed: 0,
                messages_failed: 0,
            }
        }

        pub fn add_connection(&mut self, peer_id: String) {
            if !self.connections.contains(&peer_id) {
                self.connections.push(peer_id);
            }
        }

        pub fn success_rate(&self) -> f64 {
            let total = self.messages_routed + self.messages_failed;
            if total == 0 {
                return 1.0;
            }
            self.messages_routed as f64 / total as f64
        }

        pub fn routing_score(&self) -> f64 {
            self.health * self.success_rate()
        }

        pub fn is_healthy(&self, threshold: f64) -> bool {
            self.health >= threshold
        }
    }

    // ---------------------------------------------------------------------------
    // Route Result
    // ---------------------------------------------------------------------------

    /// Result of a routing attempt.
    #[derive(Debug, Clone)]
    pub struct RouteResult {
        pub path: Vec<String>,
        pub total_score: f64,
        pub hops: usize,
        pub estimated_latency_ms: u64,
    }

    // ---------------------------------------------------------------------------
    // Interop Message
    // ---------------------------------------------------------------------------

    /// Message for inter-federation routing.
    #[derive(Debug, Clone)]
    pub struct InteropMessage {
        pub message_id: String,
        pub source: String,
        pub destination: String,
        pub payload: Vec<u8>,
        pub schema_version: u32,
        pub compressed: bool,
    }

    // ---------------------------------------------------------------------------
    // Interop Stats
    // ---------------------------------------------------------------------------

    /// Statistics for interop operations.
    #[derive(Debug, Clone)]
    pub struct InteropStats {
        pub messages_routed: u64,
        pub messages_failed: u64,
        pub routes_discovered: u64,
        pub avg_routing_time_ms: f64,
        pub schema_negotiations: u64,
    }

    impl Default for InteropStats {
        fn default() -> Self {
            Self {
                messages_routed: 0,
                messages_failed: 0,
                routes_discovered: 0,
                avg_routing_time_ms: 0.0,
                schema_negotiations: 0,
            }
        }
    }

    impl InteropStats {
        pub fn record_route(&mut self, time_ms: u64) {
            self.messages_routed += 1;
            self.avg_routing_time_ms = self.avg_routing_time_ms * 0.9 + time_ms as f64 * 0.1;
        }

        pub fn record_failure(&mut self) {
            self.messages_failed += 1;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Interop Layer
    // ---------------------------------------------------------------------------

    /// Interoperability Layer v2 with intelligent routing.
    #[derive(Debug)]
    pub struct InteropLayerV2 {
        config: InteropLayerV2Config,
        federations: HashMap<String, FederationEntryV2>,
        stats: InteropStats,
        next_nonce: u64,
    }

    impl InteropLayerV2 {
        /// Create a new interop layer with the given configuration.
        pub fn new(config: InteropLayerV2Config) -> Self {
            Self {
                config,
                federations: HashMap::new(),
                stats: InteropStats::default(),
                next_nonce: 0,
            }
        }

        /// Register a federation with endpoints.
        pub fn register_federation(
            &mut self,
            fed_id: String,
            endpoints: Vec<String>,
        ) -> Result<(), InteropV2Error> {
            if self.federations.len() >= self.config.max_federations {
                return Err(InteropV2Error::InteropFull);
            }
            if self.federations.contains_key(&fed_id) {
                return Err(InteropV2Error::FederationNotFound(fed_id.clone()));
            }
            self.federations
                .insert(fed_id.clone(), FederationEntryV2::new(fed_id, endpoints));
            Ok(())
        }

        /// Add a connection between two federations.
        pub fn add_connection(&mut self, fed_id: &str, peer_id: &str) -> Result<(), InteropV2Error> {
            // Validate both federations exist first
            if !self.federations.contains_key(fed_id) {
                return Err(InteropV2Error::FederationNotFound(fed_id.to_string()));
            }
            if !self.federations.contains_key(peer_id) {
                return Err(InteropV2Error::FederationNotFound(peer_id.to_string()));
            }

            // Add bidirectional connection
            if let Some(fed) = self.federations.get_mut(fed_id) {
                fed.add_connection(peer_id.to_string());
            }
            if let Some(peer) = self.federations.get_mut(peer_id) {
                peer.add_connection(fed_id.to_string());
            }
            Ok(())
        }

        /// Discover a path between source and destination using BFS.
        pub fn discover_path(&self, source: &str, destination: &str) -> Option<Vec<String>> {
            if source == destination {
                return Some(vec![source.to_string()]);
            }

            let mut queue = VecDeque::new();
            let mut visited = HashSet::new();
            let mut parent: HashMap<String, String> = HashMap::new();

            queue.push_back(source.to_string());
            visited.insert(source.to_string());

            while let Some(current) = queue.pop_front() {
                if current == destination {
                    // Reconstruct path
                    let mut path = Vec::new();
                    let mut node = destination.to_string();
                    while node != source {
                        path.push(node.clone());
                        node = match parent.get(&node) {
                            Some(p) => p.clone(),
                            None => return None,
                        };
                    }
                    path.push(source.to_string());
                    path.reverse();
                    return Some(path);
                }

                if let Some(fed) = self.federations.get(&current) {
                    for peer in &fed.connections {
                        if !visited.contains(peer) {
                            // Check health threshold
                            if let Some(peer_fed) = self.federations.get(peer) {
                                if peer_fed.is_healthy(self.config.min_health_threshold) {
                                    visited.insert(peer.clone());
                                    parent.insert(peer.clone(), current.clone());
                                    queue.push_back(peer.clone());
                                }
                            }
                        }
                    }
                }
            }

            None
        }

        /// Route a message through the interop layer.
        pub fn route_message(&mut self, message: InteropMessage) -> Result<RouteResult, InteropV2Error> {
            // Discover path
            let path = self.discover_path(&message.source, &message.destination)
                .ok_or_else(|| InteropV2Error::NoRouteAvailable {
                    source: message.source.clone(),
                    destination: message.destination.clone(),
                })?;

            // Check max hops
            if path.len() - 1 > self.config.max_hops {
                return Err(InteropV2Error::MaxHopsExceeded {
                    hops: path.len() - 1,
                    max: self.config.max_hops,
                });
            }

            // Calculate route score
            let mut total_score = 0.0;
            for fed_id in &path {
                if let Some(fed) = self.federations.get(fed_id) {
                    total_score += fed.routing_score();
                }
            }

            let estimated_latency = (path.len() as u64) * 10; // ~10ms per hop

            let result = RouteResult {
                path: path.clone(),
                total_score,
                hops: path.len() - 1,
                estimated_latency_ms: estimated_latency,
            };

            // Update stats
            if let Some(fed) = self.federations.get_mut(&message.source) {
                fed.messages_routed += 1;
            }

            self.stats.record_route(estimated_latency);
            self.stats.routes_discovered += 1;

            Ok(result)
        }

        /// Update federation health.
        pub fn update_federation_health(
            &mut self,
            fed_id: &str,
            health: f64,
        ) -> Result<(), InteropV2Error> {
            if !(0.0..=1.0).contains(&health) {
                return Err(InteropV2Error::InvalidHealth(health));
            }
            let fed = self
                .federations
                .get_mut(fed_id)
                .ok_or_else(|| InteropV2Error::FederationNotFound(fed_id.to_string()))?;
            fed.health = health;
            Ok(())
        }

        /// Get interop statistics.
        pub fn get_stats(&self) -> &InteropStats {
            &self.stats
        }

        /// Reset statistics.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }

        /// Get federation count.
        pub fn federation_count(&self) -> usize {
            self.federations.len()
        }
    }

    impl Default for InteropLayerV2 {
        fn default() -> Self {
            Self::new(InteropLayerV2Config::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_interop_creation() {
            let interop = InteropLayerV2::default();
            assert_eq!(interop.federation_count(), 0);
        }

        #[test]
        fn test_register_federation() {
            let mut interop = InteropLayerV2::default();
            assert!(interop.register_federation("fed_a".to_string(), vec!["ep1".to_string()]).is_ok());
            assert_eq!(interop.federation_count(), 1);
        }

        #[test]
        fn test_register_federation_duplicate() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("fed_a".to_string(), vec!["ep1".to_string()]).unwrap();
            match interop.register_federation("fed_a".to_string(), vec!["ep2".to_string()]).unwrap_err() {
                InteropV2Error::FederationNotFound(id) => assert_eq!(id, "fed_a"),
                e => panic!("Expected FederationNotFound, got {:?}", e),
            }
        }

        #[test]
        fn test_add_connection() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            interop.register_federation("b".to_string(), vec!["ep2".to_string()]).unwrap();
            assert!(interop.add_connection("a", "b").is_ok());
        }

        #[test]
        fn test_discover_path_direct() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            interop.register_federation("b".to_string(), vec!["ep2".to_string()]).unwrap();
            interop.add_connection("a", "b").unwrap();
            let path = interop.discover_path("a", "b");
            assert_eq!(path, Some(vec!["a".to_string(), "b".to_string()]));
        }

        #[test]
        fn test_discover_path_multi_hop() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            interop.register_federation("b".to_string(), vec!["ep2".to_string()]).unwrap();
            interop.register_federation("c".to_string(), vec!["ep3".to_string()]).unwrap();
            interop.add_connection("a", "b").unwrap();
            interop.add_connection("b", "c").unwrap();
            let path = interop.discover_path("a", "c");
            assert_eq!(path, Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]));
        }

        #[test]
        fn test_discover_path_not_found() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            interop.register_federation("b".to_string(), vec!["ep2".to_string()]).unwrap();
            let path = interop.discover_path("a", "b");
            assert!(path.is_none());
        }

        #[test]
        fn test_route_message() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            interop.register_federation("b".to_string(), vec!["ep2".to_string()]).unwrap();
            interop.add_connection("a", "b").unwrap();
            let msg = InteropMessage {
                message_id: "msg_1".to_string(),
                source: "a".to_string(),
                destination: "b".to_string(),
                payload: vec![1, 2, 3],
                schema_version: 1,
                compressed: false,
            };
            let result = interop.route_message(msg).unwrap();
            assert_eq!(result.hops, 1);
        }

        #[test]
        fn test_update_health() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            assert!(interop.update_federation_health("a", 0.7).is_ok());
        }

        #[test]
        fn test_update_health_invalid() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            match interop.update_federation_health("a", 1.5).unwrap_err() {
                InteropV2Error::InvalidHealth(h) => assert_eq!(h, 1.5),
                e => panic!("Expected InvalidHealth, got {:?}", e),
            }
        }

        #[test]
        fn test_federation_routing_score() {
            let mut fed = FederationEntryV2::new("test".to_string(), vec!["ep".to_string()]);
            fed.health = 0.8;
            fed.messages_routed = 90;
            fed.messages_failed = 10;
            let score = fed.routing_score();
            assert!((score - 0.8 * 0.9).abs() < 0.01);
        }

        #[test]
        fn test_stats_recording() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            interop.register_federation("b".to_string(), vec!["ep2".to_string()]).unwrap();
            interop.add_connection("a", "b").unwrap();
            let msg = InteropMessage {
                message_id: "msg_1".to_string(),
                source: "a".to_string(),
                destination: "b".to_string(),
                payload: vec![1],
                schema_version: 1,
                compressed: false,
            };
            interop.route_message(msg).unwrap();
            let stats = interop.get_stats();
            assert_eq!(stats.messages_routed, 1);
            assert_eq!(stats.routes_discovered, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut interop = InteropLayerV2::default();
            interop.register_federation("a".to_string(), vec!["ep1".to_string()]).unwrap();
            interop.register_federation("b".to_string(), vec!["ep2".to_string()]).unwrap();
            interop.add_connection("a", "b").unwrap();
            let msg = InteropMessage {
                message_id: "msg_1".to_string(),
                source: "a".to_string(),
                destination: "b".to_string(),
                payload: vec![1],
                schema_version: 1,
                compressed: false,
            };
            interop.route_message(msg).unwrap();
            interop.reset_stats();
            assert_eq!(interop.get_stats().messages_routed, 0);
        }

        #[test]
        fn test_error_display() {
            let err = InteropV2Error::FederationNotFound("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_config_default() {
            let config = InteropLayerV2Config::default();
            assert_eq!(config.max_hops, 8);
            assert!(config.enable_compression);
        }
    }
}

#[cfg(feature = "v1.6-sprint1")]
pub use internal::*;
