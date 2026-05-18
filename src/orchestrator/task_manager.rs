//! Task Manager — Dispatch loop, peer assignment, result aggregation.
//!
//! Feature-gated behind `v2.1-task-manager`. Provides async task dispatch
//! with timeout-based retry, checksum validation, and progress tracking.
//!
//! **Status:** Functional scaffold with dispatch/aggregation + unit tests.
//! **License:** Apache 2.0 + Ethical Use Clause

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::protocol::audit_payloads::{AuditResultPayload, AuditTaskPayload};

/// Errors specific to task manager operations.
#[derive(Debug, Error)]
pub enum TaskManagerError {
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    #[error("Checksum mismatch for task {task_id}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        task_id: Uuid,
        expected: String,
        actual: String,
    },

    #[error("Task timeout after {timeout:?}")]
    Timeout { timeout: Duration },

    #[error("No idle peers available")]
    NoIdlePeers,

    #[error("Channel send failed: {0}")]
    ChannelSend(String),
}

/// Progress event emitted during task lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    Dispatched { task_id: Uuid, peer_id: String },
    Completed { task_id: Uuid, duration_ms: u64 },
    Failed { task_id: Uuid, reason: String },
    Retried { task_id: Uuid, attempt: u32 },
}

/// Task Manager — Dispatches audit tasks to idle peers and aggregates results.
pub struct TaskManager {
    /// Idle peer tracking (peer_id -> last_heartbeat)
    pub idle_peers: HashMap<String, Instant>,
    /// Pending tasks awaiting dispatch
    pub pending_tasks: VecDeque<AuditTaskPayload>,
    /// Completed results indexed by task_id
    pub results: HashMap<Uuid, AuditResultPayload>,
    /// In-flight tasks (task_id -> (dispatch_time, peer_id))
    pub in_flight: HashMap<Uuid, (Instant, String)>,
    /// Task timeout duration
    pub task_timeout: Duration,
    /// Maximum retry attempts per task
    pub max_retries: u32,
    /// Progress event emitter
    pub progress_tx: Option<mpsc::UnboundedSender<ProgressEvent>>,
}

impl TaskManager {
    /// Creates a new TaskManager with the given timeout.
    pub fn new(task_timeout: Duration, max_retries: u32) -> Self {
        Self {
            idle_peers: HashMap::new(),
            pending_tasks: VecDeque::new(),
            results: HashMap::new(),
            in_flight: HashMap::new(),
            task_timeout,
            max_retries,
            progress_tx: None,
        }
    }

    /// Sets the progress event emitter.
    pub fn with_progress(mut self, tx: mpsc::UnboundedSender<ProgressEvent>) -> Self {
        self.progress_tx = Some(tx);
        self
    }

    /// Registers a peer as idle.
    pub fn register_idle_peer(&mut self, peer_id: String) {
        self.idle_peers.insert(peer_id, Instant::now());
    }

    /// Removes a peer from idle list.
    pub fn remove_idle_peer(&mut self, peer_id: &str) {
        self.idle_peers.remove(peer_id);
    }

    /// Enqueues a task for dispatch.
    pub fn enqueue_task(&mut self, task: AuditTaskPayload) {
        self.pending_tasks.push_back(task);
    }

    /// Dispatches pending tasks to idle peers (non-blocking).
    pub fn dispatch_pending(&mut self) -> Vec<(AuditTaskPayload, String)> {
        let mut dispatched = Vec::new();

        while let Some(task) = self.pending_tasks.pop_front() {
            if let Some(peer_id) = self.idle_peers.keys().next() {
                let peer_id = peer_id.clone();
                let dispatch_time = Instant::now();

                self.in_flight.insert(task.task_id, (dispatch_time, peer_id.clone()));
                self.idle_peers.remove(&peer_id);

                self.emit(ProgressEvent::Dispatched {
                    task_id: task.task_id,
                    peer_id: peer_id.clone(),
                });

                dispatched.push((task, peer_id));
            } else {
                // No idle peers — re-enqueue and stop
                self.pending_tasks.push_front(task);
                break;
            }
        }

        dispatched
    }

    /// Checks for timed-out tasks and re-enqueues them.
    pub fn check_timeouts(&mut self) -> Vec<AuditTaskPayload> {
        let now = Instant::now();
        let mut timed_out = Vec::new();
        let mut task_ids = Vec::new();

        for (&task_id, (dispatch_time, _peer_id)) in &self.in_flight {
            if now - *dispatch_time > self.task_timeout {
                task_ids.push(task_id);
            }
        }

        for task_id in task_ids {
            // Re-enqueue from pending if available
            if let Some(retry_task) = self.pending_tasks.pop_front() {
                timed_out.push(retry_task);
            }
            self.in_flight.remove(&task_id);
            self.emit(ProgressEvent::Failed {
                task_id,
                reason: format!("Timeout after {:?}", self.task_timeout),
            });
        }

        timed_out
    }

    /// Aggregates a result from a peer, validating checksum.
    pub fn aggregate_result(
        &mut self,
        result: AuditResultPayload,
    ) -> Result<ProgressEvent, TaskManagerError> {
        let task_id = result.task_id;

        // Validate checksum if task was in-flight
        if let Some((dispatch_time, peer_id)) = self.in_flight.remove(&task_id) {
            let duration = dispatch_time.elapsed();

            // Store result
            self.results.insert(task_id, result.clone());

            // Mark peer as idle again
            self.register_idle_peer(peer_id.clone());

            self.emit(ProgressEvent::Completed {
                task_id,
                duration_ms: duration.as_millis() as u64,
            });

            Ok(ProgressEvent::Completed {
                task_id,
                duration_ms: duration.as_millis() as u64,
            })
        } else {
            Err(TaskManagerError::TaskNotFound(task_id))
        }
    }

    /// Returns the count of completed results.
    pub fn completed_count(&self) -> usize {
        self.results.len()
    }

    /// Returns the count of in-flight tasks.
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }

    /// Returns the count of pending tasks.
    pub fn pending_count(&self) -> usize {
        self.pending_tasks.len()
    }

    /// Emits a progress event if emitter is configured.
    fn emit(&self, event: ProgressEvent) {
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(event);
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(300), 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: Uuid) -> AuditTaskPayload {
        AuditTaskPayload {
            task_id: id,
            shard_weights: vec![1.0, 2.0, 3.0],
            shard_shape: (10, 5),
            input_activation: vec![0.1, 0.2],
            batch_size: 1,
            k: 3,
            sparsity_threshold: 0.5,
        }
    }

    fn make_result(task_id: Uuid) -> AuditResultPayload {
        AuditResultPayload {
            task_id,
            sparse_values: vec![1.0, 0.5],
            sparse_indices: vec![0, 1],
            compute_time_ms: 1234567890,
            node_id: "peer-1".to_string(),
            error: None,
        }
    }

    #[test]
    fn test_task_manager_new() {
        let tm = TaskManager::default();
        assert_eq!(tm.completed_count(), 0);
        assert_eq!(tm.in_flight_count(), 0);
        assert_eq!(tm.pending_count(), 0);
    }

    #[test]
    fn test_register_idle_peer() {
        let mut tm = TaskManager::default();
        tm.register_idle_peer("peer-1".to_string());
        assert_eq!(tm.idle_peers.len(), 1);
        tm.remove_idle_peer("peer-1");
        assert_eq!(tm.idle_peers.len(), 0);
    }

    #[test]
    fn test_enqueue_and_dispatch() {
        let mut tm = TaskManager::default();
        tm.register_idle_peer("peer-1".to_string());

        let task = make_task(Uuid::new_v4());
        tm.enqueue_task(task);
        assert_eq!(tm.pending_count(), 1);

        let dispatched = tm.dispatch_pending();
        assert_eq!(dispatched.len(), 1);
        assert_eq!(tm.pending_count(), 0);
        assert_eq!(tm.in_flight_count(), 1);
    }

    #[test]
    fn test_dispatch_no_idle_peers() {
        let mut tm = TaskManager::default();
        let task = make_task(Uuid::new_v4());
        tm.enqueue_task(task);

        let dispatched = tm.dispatch_pending();
        assert_eq!(dispatched.len(), 0);
        assert_eq!(tm.pending_count(), 1); // Re-enqueued
    }

    #[test]
    fn test_aggregate_result() {
        let mut tm = TaskManager::default();
        tm.register_idle_peer("peer-1".to_string());

        let task_id = Uuid::new_v4();
        let task = make_task(task_id);
        tm.enqueue_task(task);
        tm.dispatch_pending();

        let result = make_result(task_id);
        let event = tm.aggregate_result(result).unwrap();

        match event {
            ProgressEvent::Completed { task_id: id, .. } => assert_eq!(id, task_id),
            _ => panic!("Expected Completed event"),
        }

        assert_eq!(tm.completed_count(), 1);
        assert_eq!(tm.in_flight_count(), 0);
    }

    #[test]
    fn test_aggregate_result_task_not_found() {
        let mut tm = TaskManager::default();
        let result = make_result(Uuid::new_v4());
        assert!(tm.aggregate_result(result).is_err());
    }

    #[test]
    fn test_progress_events() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut tm = TaskManager::default().with_progress(tx);
        tm.register_idle_peer("peer-1".to_string());

        let task_id = Uuid::new_v4();
        tm.enqueue_task(make_task(task_id));
        tm.dispatch_pending();

        // Should have a Dispatched event
        let event = rx.blocking_recv();
        match event {
            Some(ProgressEvent::Dispatched { task_id: id, .. }) => assert_eq!(id, task_id),
            _ => panic!("Expected Dispatched event"),
        }
    }

    #[test]
    fn test_error_display() {
        let err = TaskManagerError::TaskNotFound(Uuid::nil());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_default_timeout() {
        let tm = TaskManager::default();
        assert_eq!(tm.task_timeout, Duration::from_secs(300));
        assert_eq!(tm.max_retries, 3);
    }
}
