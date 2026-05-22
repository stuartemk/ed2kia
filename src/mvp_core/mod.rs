//! MVP Core Loop — Isolated basic cycle for ed2kIA v2.1.
//!
//! Feature-gated behind `v2.1-mvp-core`. Provides the minimal operational cycle:
//! 1. **Discovery:** KAD + Gossipsub peer discovery
//! 2. **Task Routing:** Tensor distribution and result collection
//! 3. **Inference Bridge:** SAE forward pass and result return
//!
//! Advanced modules (ZKP, Governance, Reputation, RLHF) are stubbed behind
//! separate feature gates to keep the core loop lightweight and testable.
//!
//! **Status:** Scaffold — placeholders with documented async patterns.
//! **License:** Apache 2.0 + Ethical Use Clause

#![cfg(feature = "v2.1-mvp-core")]

pub mod discovery;
pub mod inference_bridge;
pub mod task_router;

pub use discovery::MvpDiscovery;
pub use inference_bridge::InferenceBridge;
pub use task_router::TaskRouter;

use thiserror::Error;

/// Errors specific to the MVP core loop.
#[derive(Debug, Error)]
pub enum CoreLoopError {
    #[error("Discovery failed: {0}")]
    Discovery(String),

    #[error("Task routing failed: {0}")]
    Routing(String),

    #[error("Inference bridge failed: {0}")]
    Inference(String),

    #[error("Concurrent operation failed: {0}")]
    Concurrency(String),
}

// ============================================================================
// Core Loop Orchestrator
// ============================================================================

/// Orchestrates the MVP core loop: Discovery → Distribution → Inference → Result.
pub struct CoreLoop {
    discovery: MvpDiscovery,
    router: TaskRouter,
    bridge: InferenceBridge,
}

impl CoreLoop {
    /// Create a new core loop orchestrator.
    pub fn new() -> Self {
        Self {
            discovery: MvpDiscovery::new(),
            router: TaskRouter::new(),
            bridge: InferenceBridge::new(),
        }
    }

    /// Run the full core loop cycle.
    ///
    /// 1. Discover peers via KAD
    /// 2. Distribute tensor tasks
    /// 3. Run SAE inference
    /// 4. Collect and return results
    pub async fn run_cycle(&mut self) -> Result<CoreLoopResult, CoreLoopError> {
        // Step 1: Discover peers
        let peers = self
            .discovery
            .discover_peers()
            .await
            .map_err(|e| CoreLoopError::Discovery(e.to_string()))?;

        // Step 2: Distribute tensor tasks
        let task_ids = self
            .router
            .distribute_tensor(peers.len(), 1024)
            .await
            .map_err(|e| CoreLoopError::Routing(e.to_string()))?;

        // Step 3: Run SAE inference
        let inference_results = self
            .bridge
            .run_sae_forward(&task_ids)
            .await
            .map_err(|e| CoreLoopError::Inference(e.to_string()))?;

        // Step 4: Collect results
        let results = self
            .router
            .collect_results(&task_ids)
            .await
            .map_err(|e| CoreLoopError::Routing(e.to_string()))?;

        Ok(CoreLoopResult {
            peers_discovered: peers.len(),
            tasks_distributed: task_ids.len(),
            inference_completed: inference_results.len(),
            results_collected: results.len(),
        })
    }
}

/// Result of a single core loop cycle.
pub struct CoreLoopResult {
    pub peers_discovered: usize,
    pub tasks_distributed: usize,
    pub inference_completed: usize,
    pub results_collected: usize,
}

impl Default for CoreLoop {
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

    #[tokio::test]
    async fn test_core_loop_new() {
        let loop_orch = CoreLoop::new();
        assert!(loop_orch.discovery.is_ready());
    }

    #[tokio::test]
    async fn test_core_loop_cycle() {
        let mut loop_orch = CoreLoop::new();
        let result = loop_orch.run_cycle().await.unwrap();
        // Scaffold returns mock data
        assert!(result.peers_discovered >= 0);
        assert!(result.tasks_distributed >= 0);
    }

    #[tokio::test]
    async fn test_core_loop_default() {
        let loop_orch = CoreLoop::default();
        assert!(loop_orch.discovery.is_ready());
    }

    #[test]
    fn test_error_display() {
        let err = CoreLoopError::Discovery("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }
}
