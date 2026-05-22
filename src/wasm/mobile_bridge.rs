//! WASM Mobile Bridge v1 — Lightweight P2P sync adapter for mobile/WASM targets.
//!
//! Provides memory-limited WASM compilation target (wasm32-unknown-unknown),
//! syscall boundaries, background task scheduling, and adaptive P2P sync mocking.

mod internal {
    use std::collections::VecDeque;
    use std::fmt;

    /// Error types for WASM mobile bridge operations.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MobileBridgeError {
        /// Memory limit exceeded.
        MemoryLimitExceeded,
        /// Task queue is full.
        QueueFull,
        /// Task not found.
        TaskNotFound,
        /// Sync operation timed out.
        SyncTimeout,
        /// Invalid WASM target configuration.
        InvalidConfig,
    }

    impl fmt::Display for MobileBridgeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MobileBridgeError::MemoryLimitExceeded => write!(f, "WASM memory limit exceeded"),
                MobileBridgeError::QueueFull => write!(f, "Background task queue is full"),
                MobileBridgeError::TaskNotFound => write!(f, "Task not found in scheduler"),
                MobileBridgeError::SyncTimeout => write!(f, "P2P sync operation timed out"),
                MobileBridgeError::InvalidConfig => write!(f, "Invalid WASM target configuration"),
            }
        }
    }

    impl std::error::Error for MobileBridgeError {}

    /// Configuration for the WASM mobile bridge.
    #[derive(Debug, Clone)]
    pub struct MobileBridgeConfig {
        /// Maximum memory allocation in bytes (default: 64MB for mobile).
        pub max_memory_bytes: usize,
        /// Maximum number of background tasks.
        pub max_background_tasks: usize,
        /// Sync timeout in milliseconds.
        pub sync_timeout_ms: u64,
        /// Adaptive sync: reduce frequency when battery is low.
        pub adaptive_sync_enabled: bool,
        /// Maximum P2P message size in bytes.
        pub max_message_size_bytes: usize,
    }

    impl Default for MobileBridgeConfig {
        fn default() -> Self {
            Self {
                max_memory_bytes: 64 * 1024 * 1024, // 64MB
                max_background_tasks: 8,
                sync_timeout_ms: 30_000, // 30 seconds
                adaptive_sync_enabled: true,
                max_message_size_bytes: 1024 * 1024, // 1MB
            }
        }
    }

    /// Background task entry for the scheduler.
    #[derive(Debug, Clone)]
    pub struct BackgroundTask {
        /// Unique task identifier.
        pub task_id: String,
        /// Task priority (higher = executed first).
        pub priority: u32,
        /// Estimated memory usage in bytes.
        pub estimated_memory: usize,
        /// Task status.
        pub status: TaskStatus,
        /// Created timestamp (ms since epoch).
        pub created_at_ms: u64,
        /// Last execution timestamp (ms since epoch).
        pub last_executed_ms: u64,
    }

    impl BackgroundTask {
        /// Create a new background task.
        pub fn new(
            task_id: String,
            priority: u32,
            estimated_memory: usize,
            created_at_ms: u64,
        ) -> Self {
            Self {
                task_id,
                priority,
                estimated_memory,
                status: TaskStatus::Pending,
                created_at_ms,
                last_executed_ms: 0,
            }
        }

        /// Mark task as executing.
        pub fn start(&mut self) {
            self.status = TaskStatus::Executing;
        }

        /// Mark task as completed.
        pub fn complete(&mut self, current_ms: u64) {
            self.status = TaskStatus::Completed;
            self.last_executed_ms = current_ms;
        }

        /// Mark task as failed.
        pub fn fail(&mut self, _reason: &str) {
            self.status = TaskStatus::Failed;
        }
    }

    /// Task execution status.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TaskStatus {
        /// Task is waiting to be executed.
        Pending,
        /// Task is currently executing.
        Executing,
        /// Task completed successfully.
        Completed,
        /// Task failed.
        Failed,
    }

    impl fmt::Display for TaskStatus {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TaskStatus::Pending => write!(f, "Pending"),
                TaskStatus::Executing => write!(f, "Executing"),
                TaskStatus::Completed => write!(f, "Completed"),
                TaskStatus::Failed => write!(f, "Failed"),
            }
        }
    }

    /// P2P sync state for mobile/WASM targets.
    #[derive(Debug, Clone)]
    pub struct SyncState {
        /// Current sync progress (0.0 to 1.0).
        pub progress: f64,
        /// Bytes synced so far.
        pub bytes_synced: u64,
        /// Total bytes to sync.
        pub bytes_total: u64,
        /// Last sync timestamp (ms since epoch).
        pub last_sync_ms: u64,
        /// Sync status.
        pub status: SyncStatus,
    }

    impl SyncState {
        /// Create initial sync state.
        pub fn new(bytes_total: u64) -> Self {
            Self {
                progress: 0.0,
                bytes_synced: 0,
                bytes_total,
                last_sync_ms: 0,
                status: SyncStatus::Idle,
            }
        }

        /// Update sync progress.
        pub fn update(&mut self, bytes_synced: u64, current_ms: u64) {
            self.bytes_synced = bytes_synced;
            self.progress = if self.bytes_total > 0 {
                bytes_synced as f64 / self.bytes_total as f64
            } else {
                0.0
            };
            self.last_sync_ms = current_ms;
            if self.progress >= 1.0 {
                self.status = SyncStatus::Complete;
            } else {
                self.status = SyncStatus::InProgress;
            }
        }
    }

    /// Sync operation status.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SyncStatus {
        /// No sync in progress.
        Idle,
        /// Sync is currently running.
        InProgress,
        /// Sync completed successfully.
        Complete,
        /// Sync failed.
        Failed,
    }

    impl fmt::Display for SyncStatus {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SyncStatus::Idle => write!(f, "Idle"),
                SyncStatus::InProgress => write!(f, "InProgress"),
                SyncStatus::Complete => write!(f, "Complete"),
                SyncStatus::Failed => write!(f, "Failed"),
            }
        }
    }

    /// Memory tracker for WASM sandbox limits.
    #[derive(Debug, Clone)]
    pub struct MemoryTracker {
        /// Maximum allowed memory in bytes.
        max_memory: usize,
        /// Currently allocated memory in bytes.
        allocated: usize,
    }

    impl MemoryTracker {
        /// Create a new memory tracker.
        pub fn new(max_memory: usize) -> Self {
            Self {
                max_memory,
                allocated: 0,
            }
        }

        /// Try to allocate memory. Returns Ok if within limits.
        pub fn try_allocate(&mut self, size: usize) -> Result<(), MobileBridgeError> {
            if self.allocated + size > self.max_memory {
                return Err(MobileBridgeError::MemoryLimitExceeded);
            }
            self.allocated += size;
            Ok(())
        }

        /// Free previously allocated memory.
        pub fn free(&mut self, size: usize) {
            self.allocated = self.allocated.saturating_sub(size);
        }

        /// Get current utilization ratio (0.0 to 1.0).
        pub fn utilization(&self) -> f64 {
            if self.max_memory == 0 {
                return 0.0;
            }
            self.allocated as f64 / self.max_memory as f64
        }

        /// Get remaining memory in bytes.
        pub fn remaining(&self) -> usize {
            self.max_memory.saturating_sub(self.allocated)
        }
    }

    /// WASM Mobile Bridge — manages background tasks, memory limits, and adaptive P2P sync.
    pub struct MobileBridge {
        /// Configuration.
        config: MobileBridgeConfig,
        /// Memory tracker.
        memory: MemoryTracker,
        /// Background task queue (priority-ordered).
        tasks: VecDeque<BackgroundTask>,
        /// Current sync state.
        sync_state: SyncState,
        /// Total memory allocated to tasks.
        task_memory: usize,
    }

    impl MobileBridge {
        /// Create a new mobile bridge with default configuration.
        pub fn new() -> Self {
            let config = MobileBridgeConfig::default();
            Self {
                memory: MemoryTracker::new(config.max_memory_bytes),
                tasks: VecDeque::new(),
                sync_state: SyncState::new(0),
                task_memory: 0,
                config,
            }
        }

        /// Create a new mobile bridge with custom configuration.
        pub fn with_config(config: MobileBridgeConfig) -> Result<Self, MobileBridgeError> {
            if config.max_memory_bytes == 0 {
                return Err(MobileBridgeError::InvalidConfig);
            }
            if config.max_background_tasks == 0 {
                return Err(MobileBridgeError::InvalidConfig);
            }
            Ok(Self {
                memory: MemoryTracker::new(config.max_memory_bytes),
                tasks: VecDeque::new(),
                sync_state: SyncState::new(0),
                task_memory: 0,
                config,
            })
        }

        /// Submit a background task for execution.
        pub fn submit_task(
            &mut self,
            task_id: String,
            priority: u32,
            estimated_memory: usize,
            created_at_ms: u64,
        ) -> Result<BackgroundTask, MobileBridgeError> {
            // Check queue capacity
            if self.tasks.len() >= self.config.max_background_tasks {
                return Err(MobileBridgeError::QueueFull);
            }

            // Check memory availability
            let mut temp_memory = self.memory.clone();
            temp_memory
                .try_allocate(estimated_memory)
                .map_err(|_| MobileBridgeError::MemoryLimitExceeded)?;

            let task = BackgroundTask::new(task_id, priority, estimated_memory, created_at_ms);
            self.memory.try_allocate(estimated_memory)?;
            self.task_memory += estimated_memory;
            self.tasks.push_back(task.clone());
            Ok(task)
        }

        /// Get the next task to execute (highest priority).
        pub fn next_task(&mut self) -> Option<BackgroundTask> {
            if self.tasks.is_empty() {
                return None;
            }

            // Find highest priority task
            let mut best_idx = 0;
            let mut best_priority = 0;
            for (idx, task) in self.tasks.iter().enumerate() {
                if task.status == TaskStatus::Pending && task.priority > best_priority {
                    best_idx = idx;
                    best_priority = task.priority;
                }
            }

            if best_priority == 0 {
                return None; // No pending tasks
            }

            let task = self.tasks.remove(best_idx).unwrap();
            Some(task)
        }

        /// Complete a task and free its memory.
        pub fn complete_task(
            &mut self,
            task_id: &str,
            current_ms: u64,
        ) -> Result<(), MobileBridgeError> {
            for task in self.tasks.iter_mut() {
                if task.task_id == task_id {
                    let freed = task.estimated_memory;
                    task.complete(current_ms);
                    self.memory.free(freed);
                    self.task_memory = self.task_memory.saturating_sub(freed);
                    return Ok(());
                }
            }
            Err(MobileBridgeError::TaskNotFound)
        }

        /// Start a P2P sync operation.
        pub fn start_sync(&mut self, bytes_total: u64, current_ms: u64) {
            self.sync_state = SyncState::new(bytes_total);
            self.sync_state.last_sync_ms = current_ms;
            self.sync_state.status = SyncStatus::InProgress;
        }

        /// Update sync progress.
        pub fn update_sync(&mut self, bytes_synced: u64, current_ms: u64) {
            self.sync_state.update(bytes_synced, current_ms);
        }

        /// Check if sync should be throttled based on adaptive settings.
        pub fn should_throttle_sync(&self, battery_level: f64, is_charging: bool) -> bool {
            if !self.config.adaptive_sync_enabled {
                return false;
            }
            // Throttle if battery is low (< 20%) and not charging
            battery_level < 0.2 && !is_charging
        }

        /// Get current sync state.
        pub fn get_sync_state(&self) -> &SyncState {
            &self.sync_state
        }

        /// Get memory utilization.
        pub fn memory_utilization(&self) -> f64 {
            self.memory.utilization()
        }

        /// Get remaining memory.
        pub fn remaining_memory(&self) -> usize {
            self.memory.remaining()
        }

        /// Get the number of pending tasks.
        pub fn pending_task_count(&self) -> usize {
            self.tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Pending)
                .count()
        }

        /// Cancel all pending tasks and free their memory.
        pub fn cancel_all_tasks(&mut self) -> usize {
            let count = self.tasks.len();
            for task in self.tasks.iter() {
                self.memory.free(task.estimated_memory);
            }
            self.task_memory = 0;
            self.tasks.clear();
            count
        }

        /// Check if sync has timed out.
        pub fn is_sync_timed_out(&self, current_ms: u64) -> bool {
            if self.sync_state.status != SyncStatus::InProgress {
                return false;
            }
            current_ms.saturating_sub(self.sync_state.last_sync_ms) > self.config.sync_timeout_ms
        }
    }

    impl Default for MobileBridge {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_bridge_creation() {
            let bridge = MobileBridge::new();
            assert_eq!(bridge.pending_task_count(), 0);
            assert!(bridge.memory_utilization() < 1.0);
        }

        #[test]
        fn test_bridge_with_config() {
            let config = MobileBridgeConfig {
                max_memory_bytes: 32 * 1024 * 1024,
                max_background_tasks: 4,
                ..Default::default()
            };
            let bridge = MobileBridge::with_config(config).unwrap();
            assert_eq!(bridge.pending_task_count(), 0);
        }

        #[test]
        fn test_bridge_invalid_config_zero_memory() {
            let config = MobileBridgeConfig {
                max_memory_bytes: 0,
                ..Default::default()
            };
            assert!(MobileBridge::with_config(config).is_err());
        }

        #[test]
        fn test_bridge_invalid_config_zero_tasks() {
            let config = MobileBridgeConfig {
                max_background_tasks: 0,
                ..Default::default()
            };
            assert!(MobileBridge::with_config(config).is_err());
        }

        #[test]
        fn test_submit_task() {
            let mut bridge = MobileBridge::new();
            let task = bridge
                .submit_task("task1".to_string(), 1, 1024, 1000)
                .unwrap();
            assert_eq!(task.task_id, "task1");
            assert_eq!(bridge.pending_task_count(), 1);
        }

        #[test]
        fn test_submit_task_memory_exceeded() {
            let config = MobileBridgeConfig {
                max_memory_bytes: 100,
                ..Default::default()
            };
            let mut bridge = MobileBridge::with_config(config).unwrap();
            let result = bridge.submit_task("big".to_string(), 1, 200, 1000);
            assert_eq!(result.unwrap_err(), MobileBridgeError::MemoryLimitExceeded);
        }

        #[test]
        fn test_submit_task_queue_full() {
            let config = MobileBridgeConfig {
                max_memory_bytes: 1024 * 1024,
                max_background_tasks: 2,
                ..Default::default()
            };
            let mut bridge = MobileBridge::with_config(config).unwrap();
            assert!(bridge.submit_task("t1".to_string(), 1, 100, 1000).is_ok());
            assert!(bridge.submit_task("t2".to_string(), 1, 100, 1000).is_ok());
            assert_eq!(
                bridge
                    .submit_task("t3".to_string(), 1, 100, 1000)
                    .unwrap_err(),
                MobileBridgeError::QueueFull
            );
        }

        #[test]
        fn test_next_task_priority() {
            let mut bridge = MobileBridge::new();
            bridge.submit_task("low".to_string(), 1, 100, 1000).unwrap();
            bridge
                .submit_task("high".to_string(), 10, 100, 1000)
                .unwrap();

            let task = bridge.next_task().unwrap();
            assert_eq!(task.task_id, "high");
        }

        #[test]
        fn test_complete_task() {
            let mut bridge = MobileBridge::new();
            bridge
                .submit_task("task1".to_string(), 1, 1024, 1000)
                .unwrap();
            bridge.next_task(); // Remove from queue for execution
            assert!(bridge.complete_task("task1", 2000).is_err()); // Already removed

            // Submit and complete without removing
            bridge
                .submit_task("task2".to_string(), 1, 512, 1000)
                .unwrap();
            assert!(bridge.complete_task("task2", 2000).is_ok());
        }

        #[test]
        fn test_complete_task_not_found() {
            let mut bridge = MobileBridge::new();
            assert_eq!(
                bridge.complete_task("unknown", 1000).unwrap_err(),
                MobileBridgeError::TaskNotFound
            );
        }

        #[test]
        fn test_sync_state() {
            let mut bridge = MobileBridge::new();
            bridge.start_sync(1000, 1000);
            assert_eq!(bridge.get_sync_state().status, SyncStatus::InProgress);

            bridge.update_sync(500, 2000);
            assert!((bridge.get_sync_state().progress - 0.5).abs() < 0.01);

            bridge.update_sync(1000, 3000);
            assert_eq!(bridge.get_sync_state().status, SyncStatus::Complete);
        }

        #[test]
        fn test_adaptive_sync_throttle() {
            let bridge = MobileBridge::new();
            assert!(bridge.should_throttle_sync(0.1, false)); // Low battery, not charging
            assert!(!bridge.should_throttle_sync(0.1, true)); // Low battery, charging
            assert!(!bridge.should_throttle_sync(0.8, false)); // High battery
        }

        #[test]
        fn test_adaptive_sync_disabled() {
            let config = MobileBridgeConfig {
                adaptive_sync_enabled: false,
                ..Default::default()
            };
            let bridge = MobileBridge::with_config(config).unwrap();
            assert!(!bridge.should_throttle_sync(0.1, false));
        }

        #[test]
        fn test_memory_tracker() {
            let mut tracker = MemoryTracker::new(1000);
            assert!(tracker.try_allocate(500).is_ok());
            assert!((tracker.utilization() - 0.5).abs() < 0.01);
            assert_eq!(tracker.remaining(), 500);

            assert!(tracker.try_allocate(600).is_err());
            tracker.free(300);
            assert!(tracker.try_allocate(600).is_ok());
        }

        #[test]
        fn test_cancel_all_tasks() {
            let mut bridge = MobileBridge::new();
            bridge.submit_task("t1".to_string(), 1, 100, 1000).unwrap();
            bridge.submit_task("t2".to_string(), 1, 100, 1000).unwrap();

            let cancelled = bridge.cancel_all_tasks();
            assert_eq!(cancelled, 2);
            assert_eq!(bridge.pending_task_count(), 0);
        }

        #[test]
        fn test_sync_timeout() {
            let mut bridge = MobileBridge::new();
            bridge.start_sync(1000, 1000);
            assert!(!bridge.is_sync_timed_out(1000));
            assert!(bridge.is_sync_timed_out(40_000)); // Past 30s timeout
        }

        #[test]
        fn test_config_default() {
            let config = MobileBridgeConfig::default();
            assert_eq!(config.max_memory_bytes, 64 * 1024 * 1024);
            assert_eq!(config.max_background_tasks, 8);
            assert_eq!(config.sync_timeout_ms, 30_000);
            assert!(config.adaptive_sync_enabled);
        }

        #[test]
        fn test_error_display() {
            assert_eq!(
                format!("{}", MobileBridgeError::MemoryLimitExceeded),
                "WASM memory limit exceeded"
            );
            assert_eq!(
                format!("{}", MobileBridgeError::QueueFull),
                "Background task queue is full"
            );
        }

        #[test]
        fn test_task_status_display() {
            assert_eq!(format!("{}", TaskStatus::Pending), "Pending");
            assert_eq!(format!("{}", TaskStatus::Executing), "Executing");
            assert_eq!(format!("{}", TaskStatus::Completed), "Completed");
            assert_eq!(format!("{}", TaskStatus::Failed), "Failed");
        }

        #[test]
        fn test_sync_status_display() {
            assert_eq!(format!("{}", SyncStatus::Idle), "Idle");
            assert_eq!(format!("{}", SyncStatus::InProgress), "InProgress");
            assert_eq!(format!("{}", SyncStatus::Complete), "Complete");
            assert_eq!(format!("{}", SyncStatus::Failed), "Failed");
        }

        #[test]
        fn test_bridge_default() {
            let bridge = MobileBridge::default();
            assert_eq!(bridge.pending_task_count(), 0);
        }
    }
}

pub use internal::{
    BackgroundTask, MemoryTracker, MobileBridge, MobileBridgeConfig, MobileBridgeError, SyncState,
    SyncStatus, TaskStatus,
};
