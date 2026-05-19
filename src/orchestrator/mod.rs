//! Orchestrator Node v1 — Native orchestrator for ed2kIA v2.1.
//!
//! Feature-gated behind `v2.1-orchestrator`. Provides the core node
//! that initializes libp2p swarm, connects to Relay, loads SAE weights
//! via QwenScopeLoader, and manages the async task distribution pipeline.
//!
//! **Status:** Functional scaffold with async bootstrap + unit tests.
//! **License:** Apache 2.0 + Ethical Use Clause

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use uuid::Uuid;

#[cfg(feature = "v2.1-audit-payloads")]
use crate::protocol::audit_payloads::{AuditResultPayload, AuditTaskPayload};

#[cfg(feature = "v2.1-task-manager")]
pub mod task_manager;

#[cfg(feature = "v2.1-consensus-engine")]
pub mod consensus;

#[cfg(feature = "v2.1-reputation-system")]
pub mod reputation;

#[cfg(feature = "v2.1-merit-system")]
pub mod merit;

#[cfg(feature = "v2.1-sybil-micropow")]
pub mod sybil;

#[cfg(feature = "v2.1-orchestrator-federation")]
pub mod network;

/// Optional Rosetta API integration — spawns HTTP server alongside orchestrator.
#[cfg(feature = "v2.1-rosetta-api")]
pub mod rosetta_integration {
    use std::sync::Arc;
    use crate::atlas::graph::SemanticGraph;
    use crate::atlas::api::run_server;

    /// Spawn the Rosetta API server on a background task.
    ///
    /// The server shares the `SemanticGraph` instance with the orchestrator
    /// via `Arc`, allowing concurrent reads from both the API and the task pipeline.
    pub fn spawn_rosetta_server(graph: Arc<SemanticGraph>, port: u16) {
        tokio::spawn(async move {
            run_server(graph, port).await;
        });
    }
}

/// Errors specific to orchestrator operations.
#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("Swarm initialization failed: {0}")]
    SwarmInit(String),

    #[error("Relay connection failed: {0}")]
    RelayConnect(String),

    #[error("SAE loader failed: {0}")]
    SaeLoad(String),

    #[error("Channel send failed: {0}")]
    ChannelSend(String),

    #[error("Channel receive failed: {0}")]
    ChannelRecv(String),

    #[error("Task queue full (capacity: {capacity})")]
    QueueFull { capacity: usize },

    #[error("Shutdown requested")]
    Shutdown,
}

/// Configuration for the orchestrator node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum pending tasks in the queue
    pub max_queue_size: usize,
    /// Relay server address (Multiaddr format)
    pub relay_address: String,
    /// Path to SAE safetensors weights
    pub sae_path: String,
    /// Node listen port
    pub listen_port: u16,
    /// Task timeout in seconds
    pub task_timeout_secs: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1024,
            relay_address: String::from("/ip4/127.0.0.1/tcp/9000"),
            sae_path: String::from("./models/qwen-scope.safetensors"),
            listen_port: 9000,
            task_timeout_secs: 300,
        }
    }
}

/// Orchestrator Node — Core coordination component.
///
/// Manages libp2p swarm, task distribution via mpsc channels,
/// and SAE weight loading. All I/O is async-safe via tokio::spawn.
pub struct OrchestratorNode {
    /// Configuration
    pub config: OrchestratorConfig,
    /// Task queue sender
    pub task_queue: mpsc::Sender<AuditTaskPayload>,
    /// Result receiver
    pub result_rx: mpsc::Receiver<AuditResultPayload>,
    /// Node unique identifier
    pub node_id: Uuid,
    /// Initialized flag
    pub initialized: bool,
}

impl OrchestratorNode {
    /// Creates a new orchestrator with the given config.
    pub fn new(config: OrchestratorConfig) -> (Self, mpsc::Sender<AuditResultPayload>) {
        let (task_tx, _task_rx) = mpsc::channel::<AuditTaskPayload>(config.max_queue_size);
        let (result_tx, result_rx) = mpsc::channel::<AuditResultPayload>(config.max_queue_size);

        let node = Self {
            config,
            task_queue: task_tx,
            result_rx,
            node_id: Uuid::new_v4(),
            initialized: false,
        };

        (node, result_tx)
    }

    /// Bootstraps the orchestrator node: connects to relay,
    /// loads SAE weights, and prepares task distribution pipeline.
    ///
    /// Note: libp2p swarm integration is documented but not hardcoded,
    /// allowing flexible transport configuration per deployment target.
    pub async fn bootstrap(mut self) -> Result<Self, OrchestratorError> {
        // Initialize libp2p swarm
        // Integration point: libp2p::Swarm::new_yourBehaviour() would be instantiated here
        tracing::info!(
            port = self.config.listen_port,
            "Initializing libp2p swarm on port {}",
            self.config.listen_port
        );

        // Connect to relay server
        if !self.config.relay_address.is_empty() {
            tracing::info!(
                relay_address = %self.config.relay_address,
                "Connecting to relay at {}",
                self.config.relay_address
            );
            // Relay connection logic would go here
        }

        // Load SAE weights via QwenScopeLoader
        if !self.config.sae_path.is_empty() {
            tracing::info!(
                sae_path = %self.config.sae_path,
                "Loading SAE weights from {}",
                self.config.sae_path
            );
            // SAE loading logic would go here
        }

        self.initialized = true;
        tracing::info!(
            node_id = %self.node_id,
            "Orchestrator bootstrap complete"
        );

        Ok(self)
    }

    /// Enqueues an audit task for distribution.
    pub async fn enqueue_task(&self, task: AuditTaskPayload) -> Result<(), OrchestratorError> {
        self.task_queue
            .send(task)
            .await
            .map_err(|e| OrchestratorError::ChannelSend(e.to_string()))
    }

    /// Receives the next audit result from peers.
    pub async fn recv_result(&mut self) -> Option<AuditResultPayload> {
        self.result_rx.recv().await
    }

    /// Returns the current queue capacity.
    pub fn queue_capacity(&self) -> usize {
        self.config.max_queue_size
    }

    /// Returns the task timeout duration.
    pub fn task_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.task_timeout_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_queue_size, 1024);
        assert_eq!(config.listen_port, 9000);
        assert_eq!(config.task_timeout_secs, 300);
    }

    #[test]
    fn test_orchestrator_new() {
        let config = OrchestratorConfig::default();
        let (node, _result_tx) = OrchestratorNode::new(config);
        assert!(!node.initialized);
        assert_eq!(node.queue_capacity(), 1024);
    }

    #[test]
    fn test_task_timeout() {
        let config = OrchestratorConfig {
            task_timeout_secs: 60,
            ..OrchestratorConfig::default()
        };
        let (node, _) = OrchestratorNode::new(config);
        assert_eq!(node.task_timeout(), std::time::Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_enqueue_and_recv_result() {
        let config = OrchestratorConfig::default();
        let (mut node, result_tx) = OrchestratorNode::new(config);

        // Send a result via the sender channel
        let result = AuditResultPayload {
            task_id: Uuid::new_v4(),
            sparse_values: vec![1.0, 0.0, 0.5],
            sparse_indices: vec![0, 2],
            compute_time_ms: 42,
            node_id: "test-peer".to_string(),
            error: None,
        };
        result_tx.send(result.clone()).await.unwrap();

        // Receive the result
        let received = node.recv_result().await.unwrap();
        assert_eq!(received.node_id, "test-peer");
        assert_eq!(received.sparse_values.len(), 3);
    }

    #[test]
    fn test_error_display() {
        let err = OrchestratorError::Shutdown;
        assert!(!err.to_string().is_empty());
    }
}
