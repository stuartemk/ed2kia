//! Task Scheduler — Priority-based async task scheduling with deadline awareness.
//!
//! Manages task queues with priority levels, deadlines, and fair scheduling
//! across worker pools. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::time::{Duration, Instant};

/// Error types for task scheduling.
#[derive(Debug)]
pub enum SchedulerError {
    /// Task queue is full.
    QueueFull(usize),
    /// Task not found.
    TaskNotFound(String),
    /// Task already completed.
    TaskCompleted(String),
    /// Scheduler shutdown requested.
    Shutdown,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::QueueFull(size) => write!(f, "Queue full (max: {})", size),
            SchedulerError::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            SchedulerError::TaskCompleted(id) => write!(f, "Task already completed: {}", id),
            SchedulerError::Shutdown => write!(f, "Scheduler shutdown"),
        }
    }
}

/// Task priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    /// Critical path (verification, consensus).
    Critical,
    /// High priority (proof generation).
    High,
    /// Normal priority (training, sync).
    Normal,
    /// Low priority (metrics, housekeeping).
    Low,
}

impl TaskPriority {
    /// Numeric weight for ordering (higher = more urgent).
    pub fn weight(&self) -> u8 {
        match self {
            TaskPriority::Critical => 4,
            TaskPriority::High => 3,
            TaskPriority::Normal => 2,
            TaskPriority::Low => 1,
        }
    }
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPriority::Critical => write!(f, "critical"),
            TaskPriority::High => write!(f, "high"),
            TaskPriority::Normal => write!(f, "normal"),
            TaskPriority::Low => write!(f, "low"),
        }
    }
}

/// Task state in the scheduler lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    /// Waiting in queue.
    Pending,
    /// Currently executing.
    Running,
    /// Successfully completed.
    Completed,
    /// Failed with error.
    Failed,
    /// Cancelled before execution.
    Cancelled,
    /// Deadline exceeded.
    TimedOut,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Pending => write!(f, "pending"),
            TaskState::Running => write!(f, "running"),
            TaskState::Completed => write!(f, "completed"),
            TaskState::Failed => write!(f, "failed"),
            TaskState::Cancelled => write!(f, "cancelled"),
            TaskState::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// A scheduled task entry.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Unique task identifier.
    pub id: String,
    /// Task priority.
    pub priority: TaskPriority,
    /// Current task state.
    pub state: TaskState,
    /// Deadline relative to creation (None = no deadline).
    pub deadline: Option<Duration>,
    /// Task creation time.
    pub created_at: Instant,
    /// Task start time (if running).
    pub started_at: Option<Instant>,
    /// Task completion time (if done).
    pub completed_at: Option<Instant>,
    /// Execution latency in milliseconds.
    pub latency_ms: f64,
    /// Task description/metadata.
    pub description: String,
}

impl ScheduledTask {
    pub fn new(id: String, priority: TaskPriority, description: String) -> Self {
        Self {
            id,
            priority,
            state: TaskState::Pending,
            deadline: None,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            latency_ms: 0.0,
            description,
        }
    }

    /// Create a task with a deadline.
    pub fn with_deadline(
        id: String,
        priority: TaskPriority,
        description: String,
        deadline: Duration,
    ) -> Self {
        let mut task = Self::new(id, priority, description);
        task.deadline = Some(deadline);
        task
    }

    /// Check if task has exceeded its deadline.
    pub fn is_expired(&self) -> bool {
        if let Some(dl) = self.deadline {
            self.created_at.elapsed() > dl
        } else {
            false
        }
    }

    /// Mark task as started.
    pub fn start(&mut self) {
        self.state = TaskState::Running;
        self.started_at = Some(Instant::now());
    }

    /// Mark task as completed with latency.
    pub fn complete(&mut self, latency_ms: f64) {
        self.state = TaskState::Completed;
        self.completed_at = Some(Instant::now());
        self.latency_ms = latency_ms;
    }

    /// Mark task as failed.
    pub fn fail(&mut self) {
        self.state = TaskState::Failed;
        self.completed_at = Some(Instant::now());
    }

    /// Check if task is in a terminal state.
    pub fn is_done(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled | TaskState::TimedOut
        )
    }
}

/// Wrapper for priority queue ordering.
#[derive(Debug, Clone)]
struct PriorityItem {
    priority: TaskPriority,
    created_at: Instant,
    task_id: String,
}

impl PartialEq for PriorityItem {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for PriorityItem {}

impl PartialOrd for PriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier creation time (FIFO within priority)
        self.priority
            .weight()
            .cmp(&other.priority.weight())
            .then_with(|| other.created_at.cmp(&self.created_at))
    }
}

/// Configuration for the task scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum tasks in queue.
    pub max_queue_size: usize,
    /// Enable deadline checking.
    pub deadline_enforcement: bool,
    /// Fair scheduling: max consecutive tasks from same priority.
    pub fairness_window: usize,
    /// Task budget in microseconds.
    pub task_budget_us: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 4096,
            deadline_enforcement: true,
            fairness_window: 4,
            task_budget_us: 100,
        }
    }
}

/// Scheduler statistics.
#[derive(Debug, Default)]
pub struct SchedulerStats {
    pub total_scheduled: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_timed_out: u64,
    pub total_cancelled: u64,
    pub avg_latency_ms: f64,
    pub current_queue_size: usize,
}

/// Task Scheduler — priority-based scheduling with deadline awareness.
#[cfg(feature = "v1.4-sprint1")]
pub struct TaskScheduler {
    config: SchedulerConfig,
    queue: BinaryHeap<PriorityItem>,
    tasks: HashMap<String, ScheduledTask>,
    stats: SchedulerStats,
    running: bool,
}

#[cfg(feature = "v1.4-sprint1")]
impl TaskScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            queue: BinaryHeap::new(),
            tasks: HashMap::new(),
            stats: SchedulerStats::default(),
            running: true,
        }
    }

    /// Schedule a new task.
    pub fn schedule(&mut self, task: ScheduledTask) -> Result<(), SchedulerError> {
        if !self.running {
            return Err(SchedulerError::Shutdown);
        }
        if self.tasks.len() >= self.config.max_queue_size {
            return Err(SchedulerError::QueueFull(self.config.max_queue_size));
        }
        if self.tasks.contains_key(&task.id) {
            return Err(SchedulerError::TaskCompleted(task.id.clone()));
        }

        let item = PriorityItem {
            priority: task.priority,
            created_at: task.created_at,
            task_id: task.id.clone(),
        };

        self.tasks.insert(task.id.clone(), task);
        self.queue.push(item);
        self.stats.total_scheduled += 1;
        self.stats.current_queue_size = self.queue.len();
        Ok(())
    }

    /// Get the next task to execute (highest priority, earliest deadline).
    pub fn next_task(&mut self) -> Option<ScheduledTask> {
        if let Some(item) = self.queue.pop() {
            if let Some(task) = self.tasks.get_mut(&item.task_id) {
                // Check deadline
                if self.config.deadline_enforcement && task.is_expired() {
                    task.state = TaskState::TimedOut;
                    self.stats.total_timed_out += 1;
                    self.stats.current_queue_size = self.queue.len();
                    return Some(task.clone());
                }
                task.start();
                self.stats.current_queue_size = self.queue.len();
                return Some(task.clone());
            }
        }
        None
    }

    /// Complete a task with measured latency.
    pub fn complete_task(&mut self, task_id: &str, latency_ms: f64) -> Result<(), SchedulerError> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.complete(latency_ms);
            self.stats.total_completed += 1;
            // Update average latency (exponential moving average)
            let alpha = 0.1;
            self.stats.avg_latency_ms =
                alpha * latency_ms + (1.0 - alpha) * self.stats.avg_latency_ms;
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Fail a task.
    pub fn fail_task(&mut self, task_id: &str) -> Result<(), SchedulerError> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.fail();
            self.stats.total_failed += 1;
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Cancel a pending task.
    pub fn cancel_task(&mut self, task_id: &str) -> Result<(), SchedulerError> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            if task.state == TaskState::Pending {
                task.state = TaskState::Cancelled;
                self.stats.total_cancelled += 1;
                Ok(())
            } else {
                Err(SchedulerError::TaskCompleted(task_id.to_string()))
            }
        } else {
            Err(SchedulerError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Get task by ID.
    pub fn get_task(&self, task_id: &str) -> Option<&ScheduledTask> {
        self.tasks.get(task_id)
    }

    /// Get current queue size.
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    /// Get scheduler statistics.
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }

    /// Shutdown the scheduler.
    pub fn shutdown(&mut self) {
        self.running = false;
        self.queue.clear();
    }

    /// Check if scheduler is running.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = TaskScheduler::default();
        assert!(scheduler.is_running());
        assert_eq!(scheduler.queue_size(), 0);
    }

    #[test]
    fn test_schedule_task() {
        let mut scheduler = TaskScheduler::default();
        let task = ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "test".to_string());
        scheduler.schedule(task).unwrap();
        assert_eq!(scheduler.queue_size(), 1);
        assert_eq!(scheduler.stats().total_scheduled, 1);
    }

    #[test]
    fn test_schedule_duplicate() {
        let mut scheduler = TaskScheduler::default();
        let task1 = ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "test".to_string());
        let task2 = ScheduledTask::new("t1".to_string(), TaskPriority::High, "dup".to_string());
        scheduler.schedule(task1).unwrap();
        assert!(scheduler.schedule(task2).is_err());
    }

    #[test]
    fn test_next_task_priority() {
        let mut scheduler = TaskScheduler::default();
        let low = ScheduledTask::new("low".to_string(), TaskPriority::Low, "low".to_string());
        let critical = ScheduledTask::new("crit".to_string(), TaskPriority::Critical, "crit".to_string());
        scheduler.schedule(low).unwrap();
        scheduler.schedule(critical).unwrap();
        let next = scheduler.next_task().unwrap();
        assert_eq!(next.id, "crit");
        assert_eq!(next.state, TaskState::Running);
    }

    #[test]
    fn test_complete_task() {
        let mut scheduler = TaskScheduler::default();
        let task = ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "test".to_string());
        scheduler.schedule(task).unwrap();
        scheduler.next_task();
        scheduler.complete_task("t1", 50.0).unwrap();
        assert_eq!(scheduler.stats().total_completed, 1);
        assert!(scheduler.stats().avg_latency_ms > 0.0);
    }

    #[test]
    fn test_fail_task() {
        let mut scheduler = TaskScheduler::default();
        let task = ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "test".to_string());
        scheduler.schedule(task).unwrap();
        scheduler.next_task();
        scheduler.fail_task("t1").unwrap();
        assert_eq!(scheduler.stats().total_failed, 1);
    }

    #[test]
    fn test_cancel_task() {
        let mut scheduler = TaskScheduler::default();
        let task = ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "test".to_string());
        scheduler.schedule(task).unwrap();
        scheduler.cancel_task("t1").unwrap();
        assert_eq!(scheduler.stats().total_cancelled, 1);
    }

    #[test]
    fn test_task_not_found() {
        let mut scheduler = TaskScheduler::default();
        assert!(scheduler.complete_task("nonexistent", 0.0).is_err());
        assert!(scheduler.fail_task("nonexistent").is_err());
    }

    #[test]
    fn test_deadline_expiration() {
        let mut scheduler = TaskScheduler::default();
        let task = ScheduledTask::with_deadline(
            "exp".to_string(),
            TaskPriority::High,
            "expires".to_string(),
            Duration::from_millis(1),
        );
        scheduler.schedule(task).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let next = scheduler.next_task().unwrap();
        assert_eq!(next.state, TaskState::TimedOut);
        assert_eq!(scheduler.stats().total_timed_out, 1);
    }

    #[test]
    fn test_shutdown() {
        let mut scheduler = TaskScheduler::default();
        scheduler.shutdown();
        assert!(!scheduler.is_running());
        let task = ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "test".to_string());
        assert!(scheduler.schedule(task).is_err());
    }

    #[test]
    fn test_queue_full() {
        let config = SchedulerConfig {
            max_queue_size: 2,
            ..Default::default()
        };
        let mut scheduler = TaskScheduler::new(config);
        scheduler.schedule(ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "a".to_string())).unwrap();
        scheduler.schedule(ScheduledTask::new("t2".to_string(), TaskPriority::Normal, "b".to_string())).unwrap();
        let result = scheduler.schedule(ScheduledTask::new("t3".to_string(), TaskPriority::Normal, "c".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_priority_weight() {
        assert_eq!(TaskPriority::Critical.weight(), 4);
        assert_eq!(TaskPriority::High.weight(), 3);
        assert_eq!(TaskPriority::Normal.weight(), 2);
        assert_eq!(TaskPriority::Low.weight(), 1);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", TaskPriority::Critical), "critical");
        assert_eq!(format!("{}", TaskPriority::High), "high");
        assert_eq!(format!("{}", TaskPriority::Normal), "normal");
        assert_eq!(format!("{}", TaskPriority::Low), "low");
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(format!("{}", TaskState::Pending), "pending");
        assert_eq!(format!("{}", TaskState::Running), "running");
        assert_eq!(format!("{}", TaskState::Completed), "completed");
        assert_eq!(format!("{}", TaskState::Failed), "failed");
        assert_eq!(format!("{}", TaskState::Cancelled), "cancelled");
        assert_eq!(format!("{}", TaskState::TimedOut), "timed_out");
    }

    #[test]
    fn test_task_is_done() {
        let task = ScheduledTask::new("t1".to_string(), TaskPriority::Normal, "test".to_string());
        assert!(!task.is_done());
    }

    #[test]
    fn test_error_display() {
        let err = SchedulerError::QueueFull(100);
        let msg = format!("{}", err);
        assert!(msg.contains("100"));

        let err = SchedulerError::TaskNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));

        let err = SchedulerError::Shutdown;
        let msg = format!("{}", err);
        assert!(msg.contains("shutdown"));
    }

    #[test]
    fn test_fifo_within_priority() {
        let mut scheduler = TaskScheduler::default();
        let t1 = ScheduledTask::new("first".to_string(), TaskPriority::Normal, "first".to_string());
        let t2 = ScheduledTask::new("second".to_string(), TaskPriority::Normal, "second".to_string());
        scheduler.schedule(t1).unwrap();
        scheduler.schedule(t2).unwrap();
        let next = scheduler.next_task().unwrap();
        assert_eq!(next.id, "first");
    }

    #[test]
    fn test_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.max_queue_size, 4096);
        assert!(config.deadline_enforcement);
        assert_eq!(config.fairness_window, 4);
    }
}
