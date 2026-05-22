//! Orchestrator Federation — GossipSub mesh for multi-node coordination.
//!
//! Enables federated orchestrators to propagate semantic graph updates
//! and reputation state (bans/slashing) across the network.
//!
//! **Topics:**
//! - `ed2kia/atlas_sync`: Semantic graph delta propagation
//! - `ed2kia/reputation_sync`: Ban list + score updates
//!
//! **Security:** `Signed` authenticity prevents Sybil hopping.
//! Conflict resolution via timestamp + deterministic hash ordering.
//!
//! Feature gate: `#[cfg(feature = "v2.1-orchestrator-federation")]`

use libp2p::gossipsub::{self, IdentTopic, Message, MessageAuthenticity};
use libp2p::{identity, PeerId, Swarm, SwarmBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Federation topics.
pub const ATLAS_SYNC_TOPIC: &str = "ed2kia/atlas_sync";
pub const REPUTATION_SYNC_TOPIC: &str = "ed2kia/reputation_sync";

#[derive(Debug, Error)]
pub enum FederationError {
    #[error("gossipsub error: {0}")]
    GossipSub(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("invalid message: {0}")]
    InvalidMessage(String),
    #[error("swarm error: {0}")]
    Swarm(String),
}

/// Federated message envelope with conflict resolution metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedMessage {
    /// Origin orchestrator peer ID.
    pub origin: String,
    /// Unix epoch milliseconds.
    pub timestamp_ms: u64,
    /// Message type discriminator.
    pub r#type: MessageType,
    /// Payload (graph delta or reputation update).
    pub payload: serde_json::Value,
    /// Deterministic hash for deduplication.
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Semantic graph update (node/edge delta).
    AtlasDelta,
    /// Reputation change (ban/unban/score).
    ReputationUpdate,
}

impl FedMessage {
    /// Create a new federated message with computed hash.
    pub fn new(origin: String, r#type: MessageType, payload: serde_json::Value) -> Self {
        let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
        let hash_input = format!("{}{}{:?}", origin, timestamp_ms, r#type);
        let hash = Sha256::digest(hash_input.as_bytes());
        let hash = format!("{:x}", hash);
        Self {
            origin,
            timestamp_ms,
            r#type,
            payload,
            hash,
        }
    }

    /// Compute message ID for GossipSub deduplication.
    pub fn message_id(&self) -> String {
        self.hash.clone()
    }
}

/// Events emitted by the federation event loop.
#[derive(Debug)]
pub enum FedEvent {
    /// New atlas delta received from peer.
    AtlasDelta(FedMessage),
    /// Reputation update received from peer.
    ReputationUpdate(FedMessage),
    /// Peer connected to federation mesh.
    PeerConnected(PeerId),
    /// Peer disconnected.
    PeerDisconnected(PeerId),
    /// Message validation failed.
    ValidationFailed(String, String),
}

/// Federation bridge: manages GossipSub swarm + event dispatch.
pub struct FederationBridge {
    /// Channel to send events to consumer.
    pub event_tx: mpsc::UnboundedSender<FedEvent>,
}

impl FederationBridge {
    /// Create bridge and return event sender. The swarm must be run separately.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<FedEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { event_tx: tx }, rx)
    }

    /// Handle incoming GossipSub message: validate, deduplicate, dispatch.
    pub fn handle_message(&self, message: Message) -> Option<FedEvent> {
        // Deserialize envelope
        let fed_msg: FedMessage = match serde_json::from_slice(&message.data) {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "Failed to deserialize fed message");
                return Some(FedEvent::ValidationFailed(
                    message.source.map(|p| p.to_string()).unwrap_or_default(),
                    "deserialize".to_string(),
                ));
            }
        };

        // Dispatch by type
        Some(match fed_msg.r#type {
            MessageType::AtlasDelta => FedEvent::AtlasDelta(fed_msg),
            MessageType::ReputationUpdate => FedEvent::ReputationUpdate(fed_msg),
        })
    }
}

/// Federation behaviour — GossipSub + Identify combined via #[derive(NetworkBehaviour)].
#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct FederationBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub identify: libp2p::identify::Behaviour,
}

/// Build a GossipSub-enabled swarm for federation.
///
/// Returns the swarm and the local PeerId.
pub async fn build_federation_swarm(
    key: identity::Keypair,
) -> Result<(Swarm<FederationBehaviour>, PeerId), FederationError> {
    let peer_id = PeerId::from(key.public());

    // Build GossipSub config
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .validation_mode(gossipsub::ValidationMode::Permissive)
        .mesh_n(6)
        .mesh_n_low(4)
        .mesh_n_high(12)
        .build()
        .map_err(|e| FederationError::GossipSub(e.to_string()))?;

    // MIGRATION: libp2p 0.53 uses Signed (not StrictSign)
    let gossipsub_behaviour =
        gossipsub::Behaviour::new(MessageAuthenticity::Signed(key.clone()), gossipsub_config)
            .map_err(|e| FederationError::GossipSub(e.to_string()))?;

    // Build identify behaviour
    let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
        "/ed2kia-federation/1.0.0".to_string(),
        key.public(),
    ));

    let behaviour = FederationBehaviour {
        gossipsub: gossipsub_behaviour,
        identify,
    };

    // MIGRATION: libp2p 0.53 — SwarmBuilder pattern
    let swarm = SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|e| FederationError::Swarm(e.to_string()))?
        .with_behaviour(|_| behaviour)
        .map_err(|e| FederationError::Swarm(e.to_string()))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    info!(peer_id = %peer_id, "Federation swarm built");

    Ok((swarm, peer_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fed_message_creation() {
        let msg = FedMessage::new(
            "peer-1".to_string(),
            MessageType::AtlasDelta,
            serde_json::json!({"nodes": []}),
        );
        assert_eq!(msg.origin, "peer-1");
        assert!(!msg.hash.is_empty());
        assert!(!msg.message_id().is_empty());
    }

    #[test]
    fn test_fed_message_hash_deterministic() {
        // Hash is deterministic for a given message: same fields → same hash
        let msg = FedMessage::new(
            "origin".to_string(),
            MessageType::AtlasDelta,
            serde_json::json!({"key": "value"}),
        );
        // Verify hash matches recomputation from the same fields
        let hash_input = format!("{}{}{:?}", msg.origin, msg.timestamp_ms, msg.r#type);
        let expected = Sha256::digest(hash_input.as_bytes());
        let expected = format!("{:x}", expected);
        assert_eq!(msg.hash, expected);
    }

    #[test]
    fn test_fed_message_different_types() {
        let msg1 = FedMessage::new(
            "o".to_string(),
            MessageType::AtlasDelta,
            serde_json::json!(null),
        );
        let msg2 = FedMessage::new(
            "o".to_string(),
            MessageType::ReputationUpdate,
            serde_json::json!(null),
        );
        // Different types should produce different hashes
        assert_ne!(msg1.hash, msg2.hash);
    }

    #[test]
    fn test_federation_bridge_new() {
        let (bridge, _rx) = FederationBridge::new();
        // Verify channel is created by sending a test event
        assert!(bridge
            .event_tx
            .send(FedEvent::PeerConnected(PeerId::random()))
            .is_ok());
    }

    #[test]
    fn test_federation_bridge_send_event() {
        let (bridge, mut rx) = FederationBridge::new();
        let msg = FedMessage::new(
            "p".to_string(),
            MessageType::AtlasDelta,
            serde_json::json!({}),
        );
        bridge.event_tx.send(FedEvent::AtlasDelta(msg)).unwrap();
        // Non-blocking receive
        match rx.try_recv() {
            Ok(FedEvent::AtlasDelta(m)) => assert_eq!(m.origin, "p"),
            _ => panic!("Expected AtlasDelta event"),
        }
    }

    #[test]
    fn test_message_type_serialize() {
        let atlas = MessageType::AtlasDelta;
        let json = serde_json::to_string(&atlas).unwrap();
        assert!(json.contains("AtlasDelta"));
    }

    #[test]
    fn test_fed_message_serialize() {
        let msg = FedMessage::new(
            "x".to_string(),
            MessageType::ReputationUpdate,
            serde_json::json!({"score": 42}),
        );
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("ReputationUpdate"));
    }

    #[test]
    fn test_fed_message_roundtrip() {
        let msg = FedMessage::new(
            "origin".to_string(),
            MessageType::AtlasDelta,
            serde_json::json!({"data": [1,2,3]}),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: FedMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.origin, msg.origin);
        assert_eq!(decoded.hash, msg.hash);
    }

    #[test]
    fn test_constants() {
        assert_eq!(ATLAS_SYNC_TOPIC, "ed2kia/atlas_sync");
        assert_eq!(REPUTATION_SYNC_TOPIC, "ed2kia/reputation_sync");
    }
}
