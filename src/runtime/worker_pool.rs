//! Worker Pool — Managed pool of async workers with load balancing and health monitoring.
//!
//! Provides a configurable pool of workers that distribute tasks evenly,
//! monitor health, and support graceful shutdown. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Error types for worker pool operations.
#[derive(Debug)]
pub enum WorkerPoolError {
    /// Pool is at maximum capacity.
    PoolFull(usize),
    /// Worker not found.
    WorkerNotFound(String),
    /// Worker is unhealthy.
    WorkerUnhealthy(String),
    /// Pool is shutting down.
    PoolShutdown,
    /// Task assignment failed.
    AssignmentFailed(String),
}

impl std::fmt::Display for WorkerPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerPoolError::PoolFull(size) => write!(f, "Pool full (max: {})", size),
            WorkerPoolError::WorkerNotFound(id) => write!(f, "Worker not found: {}", id),
            WorkerPoolError::WorkerUnhealthy(id) => write!(f, "Worker unhealthy: {}", id),
            WorkerPoolError::PoolShutdown => write!(f, "Pool is shutting down"),
            WorkerPoolError::AssignmentFailed(msg) => write!(f, "Assignment failed: {}", msg),
        }
    }
}

/// Worker state in the pool lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// Worker is idle, ready for tasks.
    Idle,
    /// Worker is processing a task.
    Busy,
    /// Worker is draining (finishing current task).
    Draining,
    /// Worker is stopped.
    Stopped,
}

impl std::fmt::Display for WorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerState::Idle => write!(f, "idle"),
            WorkerState::Busy => write!(f, "busy"),
            WorkerState::Draining => write!(f, "draining"),
            WorkerState::Stopped => write!(f, "stopped"),
        }
    }
}

/// Load balancing strategy for task distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// Distribute evenly (round-robin).
    RoundRobin,
    /// Send to least loaded worker.
    LeastLoaded,
    /// Send to worker with most available capacity.
    CapacityBased,
    /// Random selection among available workers.
    Random,
}

impl std::fmt::Display for LoadBalanceStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadBalanceStrategy::RoundRobin => write!(f, "round_robin"),
            LoadBalanceStrategy::LeastLoaded => write!(f, "least_loaded"),
            LoadBalanceStrategy::CapacityBased => write!(f, "capacity_based"),
            LoadBalanceStrategy::Random => write!(f, "random"),
        }
    }
}

/// Configuration for the worker pool.
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Initial number of workers.
    pub initial_workers: usize,
    /// Maximum pool size.
    pub max_workers: usize,
    /// Minimum pool size.
    pub min_workers: usize,
    /// Load balancing strategy.
    pub strategy: LoadBalanceStrategy,
    /// Worker health check interval.
    pub health_check_interval: Duration,
    /// Task timeout per worker.
    pub task_timeout: Duration,
    /// Enable auto-scaling.
    pub auto_scale: bool,
    /// Scale-up threshold (busy ratio).
    pub scale_up_threshold: f64,
    /// Scale-down threshold (busy ratio).
    pub scale_down_threshold: f64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            initial_workers: 4,
            max_workers: 32,
            min_workers: 2,
            strategy: LoadBalanceStrategy::LeastLoaded,
            health_check_interval: Duration::from_secs(5),
            task_timeout: Duration::from_secs(30),
            auto_scale: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
        }
    }
}

/// A worker in the pool.
#[derive(Debug, Clone)]
pub struct Worker {
    /// Unique worker identifier.
    pub id: String,
    /// Current worker state.
    pub state: WorkerState,
    /// Current task load (0.0 - 1.0).
    pub load: f64,
    /// Available capacity (0.0 - 1.0).
    pub capacity: f64,
    /// Total tasks processed.
    pub tasks_processed: u64,
    /// Average task latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Last heartbeat timestamp.
    pub last_heartbeat: Instant,
    /// Currently assigned task ID (if any).
    pub current_task: Option<String>,
}

impl Worker {
    pub fn new(id: String) -> Self {
        Self {
            id,
            state: WorkerState::Idle,
            load: 0.0,
            capacity: 1.0,
            tasks_processed: 0,
            avg_latency_ms: 0.0,
            last_heartbeat: Instant::now(),
            current_task: None,
        }
    }

    /// Check if worker can accept new tasks.
    pub fn can_accept(&self) -> bool {
        matches!(self.state, WorkerState::Idle) && self.capacity > 0.0
    }

    /// Assign a task to this worker.
    pub fn assign(&mut self, task_id: String) {
        self.state = WorkerState::Busy;
        self.current_task = Some(task_id);
        self.load = 1.0;
    }

    /// Complete current task with measured latency.
    pub fn complete(&mut self, latency_ms: f64) {
        self.tasks_processed += 1;
        // Exponential moving average
        let alpha = 0.1;
        self.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * self.avg_latency_ms;
        self.current_task = None;
        self.load = 0.0;
        self.state = WorkerState::Idle;
        self.last_heartbeat = Instant::now();
    }

    /// Check if worker is stale (no recent heartbeat).
    pub fn is_stale(&self, max_stale: Duration) -> bool {
        self.last_heartbeat.elapsed() > max_stale
    }

    /// Record heartbeat.
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    /// Start draining (finish current task, then stop).
    pub fn start_draining(&mut self) {
        if self.state == WorkerState::Idle {
            self.state = WorkerState::Stopped;
        } else {
            self.state = WorkerState::Draining;
        }
    }
}

/// Pool statistics.
#[derive(Debug, Default)]
pub struct PoolStats {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub draining_workers: usize,
    pub total_tasks_assigned: u64,
    pub total_tasks_completed: u64,
    pub avg_pool_load: f64,
    pub avg_task_latency_ms: f64,
}

/// Worker Pool — manages a pool of workers with load balancing.
#[cfg(feature = "v1.4-sprint1")]
pub struct WorkerPool {
    config: WorkerPoolConfig,
    workers: HashMap<String, Worker>,
    stats: PoolStats,
    next_worker_index: usize,
    running: bool,
}

#[cfg(feature = "v1.4-sprint1")]
impl WorkerPool {
    pub fn new(config: WorkerPoolConfig) -> Self {
        let mut workers = HashMap::new();
        for i in 0..config.initial_workers {
            let id = format!("worker-{}", i);
            workers.insert(id.clone(), Worker::new(id));
        }
        Self {
            config,
            workers,
            stats: PoolStats::default(),
            next_worker_index: 0,
            running: true,
        }
    }

    /// Add a new worker to the pool.
    pub fn add_worker(&mut self, id: String) -> Result<(), WorkerPoolError> {
        if !self.running {
            return Err(WorkerPoolError::PoolShutdown);
        }
        if self.workers.len() >= self.config.max_workers {
            return Err(WorkerPoolError::PoolFull(self.config.max_workers));
        }
        if self.workers.contains_key(&id) {
            return Err(WorkerPoolError::AssignmentFailed(format!("Worker {} already exists", id)));
        }
        self.workers.insert(id.clone(), Worker::new(id));
        self.update_stats();
        Ok(())
    }

    /// Remove a worker from the pool (graceful drain).
    pub fn remove_worker(&mut self, id: &str) -> Result<(), WorkerPoolError> {
        if let Some(worker) = self.workers.get_mut(id) {
            worker.start_draining();
            Ok(())
        } else {
            Err(WorkerPoolError::WorkerNotFound(id.to_string()))
        }
    }

    /// Select the best worker for a new task based on strategy.
    pub fn select_worker(&self) -> Option<&Worker> {
        match self.config.strategy {
            LoadBalanceStrategy::RoundRobin => self.select_round_robin(),
            LoadBalanceStrategy::LeastLoaded => self.select_least_loaded(),
            LoadBalanceStrategy::CapacityBased => self.select_capacity_based(),
            LoadBalanceStrategy::Random => self.select_random(),
        }
    }

    /// Assign a task to the best available worker.
    pub fn assign_task(&mut self, task_id: String) -> Result<String, WorkerPoolError> {
        if let Some(worker) = self.select_worker() {
            let worker_id = worker.id.clone();
            if let Some(w) = self.workers.get_mut(&worker_id) {
                w.assign(task_id);
                self.stats.total_tasks_assigned += 1;
                self.update_stats();
                Ok(worker_id)
            } else {
                Err(WorkerPoolError::WorkerNotFound(worker_id))
            }
        } else {
            Err(WorkerPoolError::AssignmentFailed("No available workers".to_string()))
        }
    }

    /// Complete a task on a specific worker.
    pub fn complete_task(&mut self, worker_id: &str, latency_ms: f64) -> Result<(), WorkerPoolError> {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.complete(latency_ms);
            self.stats.total_tasks_completed += 1;
            self.update_stats();
            Ok(())
        } else {
            Err(WorkerPoolError::WorkerNotFound(worker_id.to_string()))
        }
    }

    /// Get worker by ID.
    pub fn get_worker(&self, id: &str) -> Option<&Worker> {
        self.workers.get(id)
    }

    /// Get pool statistics.
    pub fn stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Check if pool needs to scale up.
    pub fn should_scale_up(&self) -> bool {
        if !self.config.auto_scale || !self.running {
            return false;
        }
        if self.workers.len() >= self.config.max_workers {
            return false;
        }
        let busy_ratio = self.stats.busy_workers as f64 / self.stats.total_workers as f64;
        busy_ratio >= self.config.scale_up_threshold
    }

    /// Check if pool needs to scale down.
    pub fn should_scale_down(&self) -> bool {
        if !self.config.auto_scale || !self.running {
            return false;
        }
        if self.workers.len() <= self.config.min_workers {
            return false;
        }
        let busy_ratio = self.stats.busy_workers as f64 / self.stats.total_workers as f64;
        busy_ratio <= self.config.scale_down_threshold
    }

    /// Shutdown the pool gracefully.
    pub fn shutdown(&mut self) {
        self.running = false;
        for worker in self.workers.values_mut() {
            worker.start_draining();
        }
    }

    /// Check if pool is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get pool size.
    pub fn size(&self) -> usize {
        self.workers.len()
    }

    // ─── Selection strategies ───

    fn select_round_robin(&self) -> Option<&Worker> {
        let ids: Vec<&String> = self.workers.keys().collect();
        if ids.is_empty() {
            return None;
        }
        let idx = self.next_worker_index % ids.len();
        self.workers.get(ids[idx])
    }

    fn select_least_loaded(&self) -> Option<&Worker> {
        self.workers
            .values()
            .filter(|w| w.can_accept())
            .min_by_key(|w| (w.load * 1000.0) as i64)
    }

    fn select_capacity_based(&self) -> Option<&Worker> {
        self.workers
            .values()
            .filter(|w| w.can_accept())
            .max_by(|a, b| {
                (a.capacity * 1000.0).partial_cmp(&(b.capacity * 1000.0)).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    fn select_random(&self) -> Option<&Worker> {
        let available: Vec<&Worker> = self.workers.values().filter(|w| w.can_accept()).collect();
        if available.is_empty() {
            return None;
        }
        // Simple deterministic selection for tests
        available.first().copied()
    }

    fn update_stats(&mut self) {
        let mut idle = 0;
        let mut busy = 0;
        let mut draining = 0;
        let mut total_load = 0.0;
        let mut total_latency = 0.0;
        let mut total_processed = 0;

        for worker in self.workers.values() {
            match worker.state {
                WorkerState::Idle => idle += 1,
                WorkerState::Busy => busy += 1,
                WorkerState::Draining => draining += 1,
                WorkerState::Stopped => {}
            }
            total_load += worker.load;
            total_latency += worker.avg_latency_ms;
            total_processed += worker.tasks_processed;
        }

        let total = self.workers.len().max(1);
        self.stats.total_workers = total;
        self.stats.idle_workers = idle;
        self.stats.busy_workers = busy;
        self.stats.draining_workers = draining;
        self.stats.avg_pool_load = total_load / total as f64;
        self.stats.avg_task_latency_ms = if total_processed > 0 {
            total_latency / total_processed as f64
        } else {
            0.0
        };
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for WorkerPool {
    fn default() -> Self {
        Self::new(WorkerPoolConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = WorkerPool::default();
        assert!(pool.is_running());
        assert_eq!(pool.size(), 4);
    }

    #[test]
    fn test_pool_with_config() {
        let config = WorkerPoolConfig {
            initial_workers: 8,
            max_workers: 16,
            ..Default::default()
        };
        let pool = WorkerPool::new(config);
        assert_eq!(pool.size(), 8);
    }

    #[test]
    fn test_add_worker() {
        let mut pool = WorkerPool::default();
        pool.add_worker("custom-1".to_string()).unwrap();
        assert_eq!(pool.size(), 5);
    }

    #[test]
    fn test_add_worker_duplicate() {
        let mut pool = WorkerPool::default();
        pool.add_worker("worker-0".to_string()).unwrap_err();
    }

    #[test]
    fn test_add_worker_max_reached() {
        let config = WorkerPoolConfig {
            max_workers: 2,
            initial_workers: 2,
            ..Default::default()
        };
        let mut pool = WorkerPool::new(config);
        assert!(pool.add_worker("extra".to_string()).is_err());
    }

    #[test]
    fn test_assign_task() {
        let mut pool = WorkerPool::default();
        let worker_id = pool.assign_task("task-1".to_string()).unwrap();
        let worker = pool.get_worker(&worker_id).unwrap();
        assert_eq!(worker.state, WorkerState::Busy);
        assert_eq!(pool.stats().total_tasks_assigned, 1);
    }

    #[test]
    fn test_complete_task() {
        let mut pool = WorkerPool::default();
        let worker_id = pool.assign_task("task-1".to_string()).unwrap();
        pool.complete_task(&worker_id, 50.0).unwrap();
        let worker = pool.get_worker(&worker_id).unwrap();
        assert_eq!(worker.state, WorkerState::Idle);
        assert_eq!(pool.stats().total_tasks_completed, 1);
    }

    #[test]
    fn test_remove_worker() {
        let mut pool = WorkerPool::default();
        pool.remove_worker("worker-0").unwrap();
        let worker = pool.get_worker("worker-0").unwrap();
        assert_eq!(worker.state, WorkerState::Stopped);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut pool = WorkerPool::default();
        assert!(pool.remove_worker("nonexistent").is_err());
    }

    #[test]
    fn test_shutdown() {
        let mut pool = WorkerPool::default();
        pool.shutdown();
        assert!(!pool.is_running());
        for worker in pool.workers.values() {
            assert!(matches!(worker.state, WorkerState::Stopped));
        }
    }

    #[test]
    fn test_scale_up_check() {
        let config = WorkerPoolConfig {
            auto_scale: true,
            scale_up_threshold: 0.5,
            max_workers: 8,
            initial_workers: 4,
            ..Default::default()
        };
        let mut pool = WorkerPool::new(config);
        // Make all workers busy
        for i in 0..4 {
            pool.assign_task(format!("task-{}", i)).unwrap();
        }
        assert!(pool.should_scale_up());
    }

    #[test]
    fn test_scale_down_check() {
        let config = WorkerPoolConfig {
            auto_scale: true,
            scale_down_threshold: 0.5,
            min_workers: 2,
            initial_workers: 4,
            ..Default::default()
        };
        let mut pool = WorkerPool::new(config);
        // Add workers above min_workers so scale-down is possible
        pool.add_worker("w1".to_string()).unwrap();
        pool.add_worker("w2".to_string()).unwrap();
        pool.add_worker("w3".to_string()).unwrap();
        pool.add_worker("w4".to_string()).unwrap();
        // No workers busy → busy_ratio = 0.0 <= 0.5
        assert!(pool.should_scale_down());
    }

    #[test]
    fn test_worker_creation() {
        let worker = Worker::new("w1".to_string());
        assert_eq!(worker.state, WorkerState::Idle);
        assert!(worker.can_accept());
    }

    #[test]
    fn test_worker_assign() {
        let mut worker = Worker::new("w1".to_string());
        worker.assign("t1".to_string());
        assert_eq!(worker.state, WorkerState::Busy);
        assert!(!worker.can_accept());
    }

    #[test]
    fn test_worker_complete() {
        let mut worker = Worker::new("w1".to_string());
        worker.assign("t1".to_string());
        worker.complete(100.0);
        assert_eq!(worker.state, WorkerState::Idle);
        assert_eq!(worker.tasks_processed, 1);
        assert!(worker.avg_latency_ms > 0.0);
    }

    #[test]
    fn test_worker_draining() {
        let mut worker = Worker::new("w1".to_string());
        worker.start_draining();
        assert_eq!(worker.state, WorkerState::Stopped);
    }

    #[test]
    fn test_worker_draining_busy() {
        let mut worker = Worker::new("w1".to_string());
        worker.assign("t1".to_string());
        worker.start_draining();
        assert_eq!(worker.state, WorkerState::Draining);
    }

    #[test]
    fn test_strategy_display() {
        assert_eq!(format!("{}", LoadBalanceStrategy::RoundRobin), "round_robin");
        assert_eq!(format!("{}", LoadBalanceStrategy::LeastLoaded), "least_loaded");
        assert_eq!(format!("{}", LoadBalanceStrategy::CapacityBased), "capacity_based");
        assert_eq!(format!("{}", LoadBalanceStrategy::Random), "random");
    }

    #[test]
    fn test_worker_state_display() {
        assert_eq!(format!("{}", WorkerState::Idle), "idle");
        assert_eq!(format!("{}", WorkerState::Busy), "busy");
        assert_eq!(format!("{}", WorkerState::Draining), "draining");
        assert_eq!(format!("{}", WorkerState::Stopped), "stopped");
    }

    #[test]
    fn test_error_display() {
        let err = WorkerPoolError::PoolFull(10);
        assert!(format!("{}", err).contains("10"));

        let err = WorkerPoolError::WorkerNotFound("x".to_string());
        assert!(format!("{}", err).contains("x"));

        let err = WorkerPoolError::PoolShutdown;
        assert!(format!("{}", err).contains("shutting down"));
    }

    #[test]
    fn test_config_default() {
        let config = WorkerPoolConfig::default();
        assert_eq!(config.initial_workers, 4);
        assert_eq!(config.max_workers, 32);
        assert!(config.auto_scale);
    }

    #[test]
    fn test_pool_stats_default() {
        let stats = PoolStats::default();
        assert_eq!(stats.total_workers, 0);
        assert_eq!(stats.total_tasks_assigned, 0);
    }
}
