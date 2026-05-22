//! Discovery Module — KAD + Gossipsub peer discovery for MVP core loop.
//!
//! Feature-gated behind `v2.1-mvp-core`. Provides simplified peer discovery
//! using KAD (Kademlia DHT) and Gossipsub patterns.
//!
//! **Status:** Scaffold — mock data for validation.
//! **License:** Apache 2.0 + Ethical Use Clause

use thiserror::Error;

/// Errors specific to peer discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("KAD query failed: {0}")]
    KadQuery(String),

    #[error("Gossipsub subscription failed: {0}")]
    Gossipsub(String),

    #[error("No peers found")]
    NoPeers,
}

/// Represents a discovered peer in the network.
#[derive(Debug, Clone)]
pub struct Peer {
    /// Unique peer identifier.
    pub id: String,
    /// Peer address (multiaddr format).
    pub address: String,
    /// Peer reputation score (0.0 - 1.0).
    pub score: f64,
}

impl Peer {
    /// Create a new peer entry.
    pub fn new(id: String, address: String, score: f64) -> Self {
        Self { id, address, score }
    }
}

/// MVP Discovery handler using KAD + Gossipsub patterns.
pub struct MvpDiscovery {
    /// Simulated peer list (placeholder for real KAD queries).
    peers: Vec<Peer>,
    /// Discovery status.
    ready: bool,
}

impl MvpDiscovery {
    /// Create a new discovery handler.
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            ready: true,
        }
    }

    /// Check if discovery is ready.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Discover peers via KAD (simplified).
    ///
    /// In production this will:
    /// 1. Query KAD DHT for nearby peers
    /// 2. Filter by reputation score
    /// 3. Return sorted peer list
    pub async fn discover_peers(&mut self) -> Result<Vec<Peer>, DiscoveryError> {
        // Scaffold: return mock peers for validation
        self.peers = vec![
            Peer::new(
                "peer-001".to_string(),
                "/ip4/127.0.0.1/tcp/4001".to_string(),
                0.95,
            ),
            Peer::new(
                "peer-002".to_string(),
                "/ip4/127.0.0.1/tcp/4002".to_string(),
                0.87,
            ),
            Peer::new(
                "peer-003".to_string(),
                "/ip4/127.0.0.1/tcp/4003".to_string(),
                0.92,
            ),
        ];

        if self.peers.is_empty() {
            return Err(DiscoveryError::NoPeers);
        }

        Ok(self.peers.clone())
    }

    /// Get the current peer count.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

impl Default for MvpDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_new() {
        let discovery = MvpDiscovery::new();
        assert!(discovery.is_ready());
        assert_eq!(discovery.peer_count(), 0);
    }

    #[tokio::test]
    async fn test_discover_peers() {
        let mut discovery = MvpDiscovery::new();
        let peers = discovery.discover_peers().await.unwrap();
        assert_eq!(peers.len(), 3);
        assert!(peers.iter().all(|p| p.score >= 0.0 && p.score <= 1.0));
    }

    #[tokio::test]
    async fn test_peer_count_after_discovery() {
        let mut discovery = MvpDiscovery::new();
        discovery.discover_peers().await.unwrap();
        assert_eq!(discovery.peer_count(), 3);
    }

    #[test]
    fn test_peer_new() {
        let peer = Peer::new(
            "test".to_string(),
            "/ip4/127.0.0.1/tcp/4001".to_string(),
            0.9,
        );
        assert_eq!(peer.id, "test");
        assert_eq!(peer.score, 0.9);
    }

    #[test]
    fn test_error_display() {
        let err = DiscoveryError::KadQuery("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_discovery_default() {
        let discovery = MvpDiscovery::default();
        assert!(discovery.is_ready());
    }
}
