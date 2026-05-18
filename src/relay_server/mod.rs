//! Relay Server Module — WebRTC/Circuit Relay v2 signaling scaffold for ed2kIA v2.1.
//!
//! Feature-gated behind `v2.1-relay-server`. Provides the "Faro" (Lighthouse) relay
//! node that enables browser-based WASM nodes to discover and connect to the P2P network
//! through WebRTC and Circuit Relay v2 transports.
//!
//! **Status:** Scaffold — placeholder for production signaling logic.
//! **License:** Apache 2.0 + Ethical Use Clause

use std::collections::HashMap;
use thiserror::Error;

/// Errors specific to relay server operations.
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("Relay bootstrap failed: {0}")]
    Bootstrap(String),

    #[error("Signaling error: {0}")]
    Signaling(String),

    #[error("Circuit negotiation failed: {0}")]
    CircuitNegotiation(String),

    #[error("Invalid peer configuration: {0}")]
    InvalidConfig(String),
}

/// Relay node configuration.
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Listening port for WebSocket signaling.
    pub signaling_port: u16,
    /// Maximum concurrent relay circuits.
    pub max_circuits: usize,
    /// Circuit lease duration in seconds.
    pub lease_duration_secs: u64,
    /// Enable WebRTC transport.
    pub enable_webrtc: bool,
    /// Enable Circuit Relay v2.
    pub enable_relay_v2: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            signaling_port: 9090,
            max_circuits: 1000,
            lease_duration_secs: 300,
            enable_webrtc: true,
            enable_relay_v2: true,
        }
    }
}

/// Active relay circuit tracking.
#[derive(Debug, Clone)]
pub struct RelayCircuit {
    /// Unique circuit identifier.
    pub circuit_id: String,
    /// Source peer ID (browser node).
    pub source_peer_id: String,
    /// Destination peer ID (full node).
    pub destination_peer_id: String,
    /// Transport type (WebRTC or Relay v2).
    pub transport: RelayTransport,
    /// Circuit creation timestamp (ms).
    pub created_at_ms: u64,
    /// Lease expiry timestamp (ms).
    pub expires_at_ms: u64,
}

/// Transport type for relay circuits.
#[derive(Debug, Clone, PartialEq)]
pub enum RelayTransport {
    WebRTC,
    CircuitRelayV2,
}

impl std::fmt::Display for RelayTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelayTransport::WebRTC => write!(f, "WebRTC"),
            RelayTransport::CircuitRelayV2 => write!(f, "CircuitRelayV2"),
        }
    }
}

/// Relay Node — "El Faro" scaffold for browser node connectivity.
pub struct RelayNode {
    /// Relay configuration.
    config: RelayConfig,
    /// Active circuits map.
    circuits: HashMap<String, RelayCircuit>,
    /// Is relay running.
    running: bool,
}

impl RelayNode {
    /// Create a new relay node with default configuration.
    pub fn new() -> Self {
        Self {
            config: RelayConfig::default(),
            circuits: HashMap::new(),
            running: false,
        }
    }

    /// Create a new relay node with custom configuration.
    pub fn with_config(config: RelayConfig) -> Result<Self, RelayError> {
        if config.max_circuits == 0 {
            return Err(RelayError::InvalidConfig(
                "max_circuits must be > 0".to_string(),
            ));
        }
        if config.lease_duration_secs == 0 {
            return Err(RelayError::InvalidConfig(
                "lease_duration_secs must be > 0".to_string(),
            ));
        }
        Ok(Self {
            config,
            circuits: HashMap::new(),
            running: false,
        })
    }

    /// Bootstrap the relay node.
    ///
    /// In production this will:
    /// 1. Initialize libp2p Swarm with WebRTC + Relay v2 transports
    /// 2. Start WebSocket signaling server
    /// 3. Register with bootstrap nodes
    pub async fn bootstrap(&mut self) -> Result<(), RelayError> {
        // Scaffold: simulate bootstrap
        self.running = true;
        Ok(())
    }

    /// Create a new relay circuit between source and destination.
    pub fn create_circuit(
        &mut self,
        source_peer_id: String,
        destination_peer_id: String,
        transport: RelayTransport,
        current_ms: u64,
    ) -> Result<RelayCircuit, RelayError> {
        if self.circuits.len() >= self.config.max_circuits {
            return Err(RelayError::CircuitNegotiation(
                "Max circuits reached".to_string(),
            ));
        }

        let slice_len = std::cmp::min(8, source_peer_id.len());
        let circuit_id = format!("circuit-{}-{}", &source_peer_id[..slice_len], current_ms);
        let circuit = RelayCircuit {
            circuit_id: circuit_id.clone(),
            source_peer_id,
            destination_peer_id,
            transport,
            created_at_ms: current_ms,
            expires_at_ms: current_ms + (self.config.lease_duration_secs * 1000),
        };

        self.circuits.insert(circuit_id, circuit.clone());
        Ok(circuit)
    }

    /// Check if a circuit is still valid (not expired).
    pub fn is_circuit_valid(&self, circuit_id: &str, current_ms: u64) -> bool {
        match self.circuits.get(circuit_id) {
            Some(circuit) => current_ms < circuit.expires_at_ms,
            None => false,
        }
    }

    /// Clean up expired circuits.
    pub fn cleanup_expired(&mut self, current_ms: u64) -> usize {
        let before = self.circuits.len();
        self.circuits.retain(|_, circuit| current_ms < circuit.expires_at_ms);
        before - self.circuits.len()
    }

    /// Get active circuit count.
    pub fn active_circuit_count(&self) -> usize {
        self.circuits.len()
    }

    /// Check if relay is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the relay node.
    pub fn stop(&mut self) {
        self.running = false;
        self.circuits.clear();
    }
}

impl Default for RelayNode {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_config_default() {
        let config = RelayConfig::default();
        assert_eq!(config.signaling_port, 9090);
        assert_eq!(config.max_circuits, 1000);
        assert!(config.enable_webrtc);
        assert!(config.enable_relay_v2);
    }

    #[test]
    fn test_relay_node_new() {
        let node = RelayNode::new();
        assert!(!node.is_running());
        assert_eq!(node.active_circuit_count(), 0);
    }

    #[test]
    fn test_relay_node_with_config() {
        let config = RelayConfig {
            max_circuits: 500,
            ..RelayConfig::default()
        };
        let node = RelayNode::with_config(config).unwrap();
        assert_eq!(node.active_circuit_count(), 0);
    }

    #[test]
    fn test_relay_node_invalid_config_zero_circuits() {
        let config = RelayConfig {
            max_circuits: 0,
            ..RelayConfig::default()
        };
        assert!(RelayNode::with_config(config).is_err());
    }

    #[test]
    fn test_relay_node_invalid_config_zero_lease() {
        let config = RelayConfig {
            lease_duration_secs: 0,
            ..RelayConfig::default()
        };
        assert!(RelayNode::with_config(config).is_err());
    }

    #[tokio::test]
    async fn test_relay_bootstrap() {
        let mut node = RelayNode::new();
        node.bootstrap().await.unwrap();
        assert!(node.is_running());
    }

    #[test]
    fn test_create_circuit() {
        let mut node = RelayNode::new();
        let circuit = node.create_circuit(
            "peer-abc123".to_string(),
            "peer-xyz789".to_string(),
            RelayTransport::WebRTC,
            1000000,
        ).unwrap();
        assert_eq!(node.active_circuit_count(), 1);
        assert_eq!(circuit.source_peer_id, "peer-abc123");
        assert_eq!(circuit.transport, RelayTransport::WebRTC);
    }

    #[test]
    fn test_circuit_validity() {
        let mut node = RelayNode::new();
        let circuit = node.create_circuit(
            "peer-a".to_string(),
            "peer-b".to_string(),
            RelayTransport::CircuitRelayV2,
            1000000,
        ).unwrap();
        assert!(node.is_circuit_valid(&circuit.circuit_id, 1000000 + 1000));
        assert!(!node.is_circuit_valid(&circuit.circuit_id, 1000000 + (301 * 1000)));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut node = RelayNode::new();
        node.create_circuit("peer-a".to_string(), "peer-b".to_string(), RelayTransport::WebRTC, 1000000).unwrap();
        node.create_circuit("peer-c".to_string(), "peer-d".to_string(), RelayTransport::WebRTC, 2000000).unwrap();
        assert_eq!(node.active_circuit_count(), 2);
        let cleaned = node.cleanup_expired(1000000 + (301 * 1000));
        assert_eq!(cleaned, 1);
        assert_eq!(node.active_circuit_count(), 1);
    }

    #[test]
    fn test_max_circuits() {
        let config = RelayConfig {
            max_circuits: 1,
            ..RelayConfig::default()
        };
        let mut node = RelayNode::with_config(config).unwrap();
        node.create_circuit("peer-a".to_string(), "peer-b".to_string(), RelayTransport::WebRTC, 1000000).unwrap();
        assert!(node.create_circuit("peer-c".to_string(), "peer-d".to_string(), RelayTransport::WebRTC, 1000001).is_err());
    }

    #[test]
    fn test_relay_stop() {
        let mut node = RelayNode::new();
        node.create_circuit("peer-a".to_string(), "peer-b".to_string(), RelayTransport::WebRTC, 1000000).unwrap();
        node.stop();
        assert!(!node.is_running());
        assert_eq!(node.active_circuit_count(), 0);
    }

    #[test]
    fn test_transport_display() {
        assert_eq!(format!("{}", RelayTransport::WebRTC), "WebRTC");
        assert_eq!(format!("{}", RelayTransport::CircuitRelayV2), "CircuitRelayV2");
    }

    #[test]
    fn test_error_display() {
        let err = RelayError::Bootstrap("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_relay_default() {
        let node = RelayNode::default();
        assert!(!node.is_running());
    }
}
