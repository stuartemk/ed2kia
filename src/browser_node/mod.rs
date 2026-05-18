//! Browser Node Scaffold v1 — WASM-based P2P node for browser environments.
//!
//! Feature-gated behind `v2.1-wasm-browser`. Compiles to `wasm32-unknown-unknown`.
//! Provides:
//! - `BrowserNode`: Placeholder struct for browser-based P2P participation
//! - WebRTC/WebSocket transport configuration (libp2p-webrtc-websys + libp2p-websocket-websys)
//! - Web Worker bridge for background compute tasks
//!
//! **Status:** Scaffold only — zero functional logic until RFC approval.
//! **License:** Apache 2.0 + Ethical Use Clause

#![cfg(feature = "v2.1-wasm-browser")]

use thiserror::Error;

// ============================================================================
// Sub-modules — Feature-gated
// ============================================================================

#[cfg(feature = "v2.1-wasm-workers")]
pub mod worker;

#[cfg(feature = "v2.1-webrtc-relay")]
pub mod webrtc_transport;

// ============================================================================
// Error Types
// ============================================================================

/// Errors specific to browser node operations.
#[derive(Debug, Error)]
pub enum BrowserNodeError {
    #[error("Web Worker initialization failed: {0}")]
    WorkerInit(String),

    #[error("Transport configuration error: {0}")]
    TransportConfig(String),

    #[error("Message serialization error: {0}")]
    Serialization(String),

    #[error("Browser API not available: {0}")]
    ApiUnavailable(String),
}

// ============================================================================
// Browser Node Configuration
// ============================================================================

/// Configuration for the browser-based P2P node.
pub struct BrowserNodeConfig {
    /// Enable WebRTC transport for direct peer connections.
    pub webrtc_enabled: bool,
    /// Enable WebSocket transport for relay-based connections.
    pub websocket_enabled: bool,
    /// Maximum concurrent Web Worker threads.
    pub max_workers: usize,
    /// Signal server URL for WebRTC signaling (optional).
    pub signal_server: Option<String>,
}

impl Default for BrowserNodeConfig {
    fn default() -> Self {
        Self {
            webrtc_enabled: true,
            websocket_enabled: true,
            max_workers: 2,
            signal_server: None,
        }
    }
}

// ============================================================================
// Browser Node
// ============================================================================

/// Placeholder browser node struct.
///
/// When activated, this will coordinate:
/// 1. libp2p swarm with WebRTC/WebSocket transports
/// 2. Web Worker pool for background SAE inference
/// 3. Message passing between main thread and workers
pub struct BrowserNode {
    config: BrowserNodeConfig,
    initialized: bool,
}

impl BrowserNode {
    /// Create a new browser node with default configuration.
    pub fn new() -> Self {
        Self {
            config: BrowserNodeConfig::default(),
            initialized: false,
        }
    }

    /// Create a new browser node with custom configuration.
    pub fn with_config(config: BrowserNodeConfig) -> Self {
        Self {
            config,
            initialized: false,
        }
    }

    /// Initialize the browser node (placeholder).
    ///
    /// In production this will:
    /// 1. Create libp2p swarm with browser transports
    /// 2. Spawn Web Workers for compute
    /// 3. Establish message channels
    pub fn init(&mut self) -> Result<(), BrowserNodeError> {
        // Scaffold: validate configuration
        if self.config.max_workers == 0 {
            return Err(BrowserNodeError::TransportConfig(
                "max_workers must be > 0".to_string(),
            ));
        }

        if !self.config.webrtc_enabled && !self.config.websocket_enabled {
            return Err(BrowserNodeError::TransportConfig(
                "At least one transport (WebRTC or WebSocket) must be enabled".to_string(),
            ));
        }

        self.initialized = true;
        Ok(())
    }

    /// Check if the node is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the current configuration.
    pub fn config(&self) -> &BrowserNodeConfig {
        &self.config
    }
}

impl Default for BrowserNode {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Web Worker Bridge
// ============================================================================

/// Handle to a Web Worker for background compute.
pub struct WorkerHandle {
    /// Unique identifier for this worker.
    pub id: usize,
    /// Current status of the worker.
    pub status: WorkerStatus,
}

/// Status of a Web Worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Error,
}

/// Initialize the Web Worker bridge (placeholder).
///
/// Returns a handle to the created worker. In production this will:
/// 1. Create a new Web Worker from the compiled WASM module
/// 2. Establish message passing channels
/// 3. Register task handlers
pub fn init_worker_bridge(worker_id: usize) -> Result<WorkerHandle, BrowserNodeError> {
    Ok(WorkerHandle {
        id: worker_id,
        status: WorkerStatus::Idle,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_node_default() {
        let node = BrowserNode::new();
        assert!(!node.is_initialized());
        assert!(node.config().webrtc_enabled);
        assert!(node.config().websocket_enabled);
        assert_eq!(node.config().max_workers, 2);
    }

    #[test]
    fn test_browser_node_with_config() {
        let config = BrowserNodeConfig {
            webrtc_enabled: false,
            websocket_enabled: true,
            max_workers: 4,
            signal_server: Some("wss://signal.example.com".to_string()),
        };
        let node = BrowserNode::with_config(config);
        assert!(!node.config().webrtc_enabled);
        assert!(node.config().websocket_enabled);
        assert_eq!(node.config().max_workers, 4);
    }

    #[test]
    fn test_browser_node_init_success() {
        let mut node = BrowserNode::new();
        assert!(node.init().is_ok());
        assert!(node.is_initialized());
    }

    #[test]
    fn test_browser_node_init_zero_workers() {
        let config = BrowserNodeConfig {
            max_workers: 0,
            ..Default::default()
        };
        let mut node = BrowserNode::with_config(config);
        let err = node.init().unwrap_err();
        assert!(matches!(err, BrowserNodeError::TransportConfig(_)));
    }

    #[test]
    fn test_browser_node_init_no_transport() {
        let config = BrowserNodeConfig {
            webrtc_enabled: false,
            websocket_enabled: false,
            ..Default::default()
        };
        let mut node = BrowserNode::with_config(config);
        let err = node.init().unwrap_err();
        assert!(matches!(err, BrowserNodeError::TransportConfig(_)));
    }

    #[test]
    fn test_init_worker_bridge() {
        let handle = init_worker_bridge(1).unwrap();
        assert_eq!(handle.id, 1);
        assert_eq!(handle.status, WorkerStatus::Idle);
    }

    #[test]
    fn test_browser_node_default_impl() {
        let node = BrowserNode::default();
        assert!(!node.is_initialized());
    }

    #[test]
    fn test_error_display() {
        let err = BrowserNodeError::WorkerInit("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }
}
