//! Task Router Module — Tensor distribution and result collection for MVP core loop.
//!
//! Feature-gated behind `v2.1-mvp-core`. Provides task routing logic for distributing
//! tensor payloads to discovered peers and collecting inference results.
//!
//! **Status:** Scaffold — mock data for validation.
//! **License:** Apache 2.0 + Ethical Use Clause

use thiserror::Error;

/// Errors specific to task routing.
#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("No available peers for distribution")]
    NoPeers,

    #[error("Tensor distribution failed: {0}")]
    Distribution(String),

    #[error("Result collection failed: {0}")]
    Collection(String),

    #[error("Task expired: {0}")]
    Expired(String),
}

/// Represents a distributed tensor task.
#[derive(Debug, Clone)]
pub struct TensorTask {
    /// Unique task identifier.
    pub task_id: String,
    /// Target peer ID.
    pub peer_id: String,
    /// Tensor payload size in bytes.
    pub payload_size: usize,
    /// Task status.
    pub status: TaskStatus,
}

impl TensorTask {
    /// Create a new tensor task.
    pub fn new(task_id: String, peer_id: String, payload_size: usize) -> Self {
        Self {
            task_id,
            peer_id,
            payload_size,
            status: TaskStatus::Pending,
        }
    }
}

/// Task status states.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Distributed,
    Completed,
    Failed,
}

/// MVP Task Router for distributing tensors and collecting results.
pub struct TaskRouter {
    /// Active tasks in the routing pipeline.
    tasks: Vec<TensorTask>,
    /// Router status.
    ready: bool,
}

impl TaskRouter {
    /// Create a new task router.
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            ready: true,
        }
    }

    /// Check if router is ready.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Distribute tensor payload to peers.
    ///
    /// In production this will:
    /// 1. Partition tensor across available peers
    /// 2. Serialize and send via libp2p request-response
    /// 3. Return list of task IDs for tracking
    pub async fn distribute_tensor(
        &mut self,
        peer_count: usize,
        payload_size: usize,
    ) -> Result<Vec<String>, RoutingError> {
        if peer_count == 0 {
            return Err(RoutingError::NoPeers);
        }

        // Scaffold: create mock tasks for validation
        self.tasks.clear();
        let mut task_ids = Vec::new();

        for i in 0..peer_count {
            let task_id = format!("task-{:03}", i);
            let peer_id = format!("peer-{:03}", i);
            self.tasks
                .push(TensorTask::new(task_id.clone(), peer_id, payload_size));
            task_ids.push(task_id);
        }

        Ok(task_ids)
    }

    /// Collect results from distributed tasks.
    ///
    /// In production this will:
    /// 1. Wait for task completion signals
    /// 2. Aggregate partial results
    /// 3. Return collected inference results
    pub async fn collect_results(
        &mut self,
        task_ids: &[String],
    ) -> Result<Vec<String>, RoutingError> {
        if task_ids.is_empty() {
            return Err(RoutingError::Collection("No task IDs provided".to_string()));
        }

        // Scaffold: simulate result collection
        let mut collected = Vec::new();

        for task_id in task_ids {
            if let Some(task) = self.tasks.iter_mut().find(|t| t.task_id == *task_id) {
                task.status = TaskStatus::Completed;
                collected.push(format!("result-{}", task_id));
            }
        }

        if collected.is_empty() {
            return Err(RoutingError::Collection("No results collected".to_string()));
        }

        Ok(collected)
    }

    /// Get the current task count.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get tasks by status.
    pub fn get_tasks_by_status(&self, status: &TaskStatus) -> Vec<&TensorTask> {
        self.tasks.iter().filter(|t| &t.status == status).collect()
    }
}

impl Default for TaskRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_new() {
        let router = TaskRouter::new();
        assert!(router.is_ready());
        assert_eq!(router.task_count(), 0);
    }

    #[tokio::test]
    async fn test_distribute_tensor() {
        let mut router = TaskRouter::new();
        let task_ids = router.distribute_tensor(3, 1024).await.unwrap();
        assert_eq!(task_ids.len(), 3);
        assert_eq!(router.task_count(), 3);
    }

    #[tokio::test]
    async fn test_distribute_tensor_no_peers() {
        let mut router = TaskRouter::new();
        let result = router.distribute_tensor(0, 1024).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collect_results() {
        let mut router = TaskRouter::new();
        let task_ids = router.distribute_tensor(3, 1024).await.unwrap();
        let results = router.collect_results(&task_ids).await.unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.starts_with("result-")));
    }

    #[tokio::test]
    async fn test_collect_results_empty() {
        let mut router = TaskRouter::new();
        let result = router.collect_results(&[]).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_task_new() {
        let task = TensorTask::new("task-001".to_string(), "peer-001".to_string(), 2048);
        assert_eq!(task.task_id, "task-001");
        assert_eq!(task.payload_size, 2048);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_tasks_by_status() {
        let mut router = TaskRouter::new();
        router.distribute_tensor(3, 1024).await.unwrap();
        let pending = router.get_tasks_by_status(&TaskStatus::Pending);
        assert_eq!(pending.len(), 3);
    }

    #[test]
    fn test_error_display() {
        let err = RoutingError::Distribution("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_router_default() {
        let router = TaskRouter::default();
        assert!(router.is_ready());
    }
}
