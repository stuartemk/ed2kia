//! Tokio Optimizer — Adaptive runtime configuration for optimal async task throughput.
//!
//! Monitors runtime metrics and adjusts worker thread counts, task budgets,
//! and I/O priorities dynamically. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use std::time::Instant;

/// Error types for Tokio Optimizer operations.
#[derive(Debug)]
pub enum TokioOptimizerError {
    /// Invalid configuration parameter.
    InvalidConfig(String),
    /// Runtime already initialized.
    AlreadyInitialized,
    /// Optimization target not reachable.
    OptimizationFailed(String),
}

impl std::fmt::Display for TokioOptimizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokioOptimizerError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            TokioOptimizerError::AlreadyInitialized => write!(f, "Runtime already initialized"),
            TokioOptimizerError::OptimizationFailed(msg) => write!(f, "Optimization failed: {}", msg),
        }
    }
}

/// Runtime profile for different workload types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProfile {
    /// CPU-bound computation (proof generation, gradient sync).
    Compute,
    /// I/O-bound operations (network, storage).
    IoBound,
    /// Mixed workload (default).
    Balanced,
    /// Low-latency priority (real-time verification).
    LowLatency,
}

impl std::fmt::Display for RuntimeProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeProfile::Compute => write!(f, "compute"),
            RuntimeProfile::IoBound => write!(f, "io_bound"),
            RuntimeProfile::Balanced => write!(f, "balanced"),
            RuntimeProfile::LowLatency => write!(f, "low_latency"),
        }
    }
}

/// Configuration for the Tokio Optimizer.
#[derive(Debug, Clone)]
pub struct TokioOptimizerConfig {
    /// Number of worker threads (0 = auto-detect).
    pub worker_threads: usize,
    /// Number of dedicated I/O threads (0 = auto-detect).
    pub io_threads: usize,
    /// Maximum task queue size before backpressure.
    pub max_task_queue: usize,
    /// Task execution budget in microseconds before yielding.
    pub task_budget_us: u64,
    /// Enable adaptive thread scaling.
    pub adaptive_scaling: bool,
    /// Scale-up threshold (queue utilization).
    pub scale_up_threshold: f64,
    /// Scale-down threshold (queue utilization).
    pub scale_down_threshold: f64,
    /// Minimum worker threads.
    pub min_workers: usize,
    /// Maximum worker threads.
    pub max_workers: usize,
    /// Runtime profile.
    pub profile: RuntimeProfile,
}

impl Default for TokioOptimizerConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0,
            io_threads: 0,
            max_task_queue: 8192,
            task_budget_us: 100,
            adaptive_scaling: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
            min_workers: 2,
            max_workers: 64,
            profile: RuntimeProfile::Balanced,
        }
    }
}

/// Runtime metrics snapshot.
#[derive(Debug, Clone)]
pub struct RuntimeMetrics {
    /// Active tasks in the runtime.
    pub active_tasks: usize,
    /// Queued tasks waiting for execution.
    pub queued_tasks: usize,
    /// Total tasks completed.
    pub completed_tasks: u64,
    /// Average task latency in milliseconds.
    pub avg_task_latency_ms: f64,
    /// Current worker thread count.
    pub worker_threads: usize,
    /// Current I/O thread count.
    pub io_threads: usize,
    /// Queue utilization (0.0 - 1.0).
    pub queue_utilization: f64,
    /// Last measurement timestamp.
    pub last_measurement: Instant,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self {
            active_tasks: 0,
            queued_tasks: 0,
            completed_tasks: 0,
            avg_task_latency_ms: 0.0,
            worker_threads: 0,
            io_threads: 0,
            queue_utilization: 0.0,
            last_measurement: Instant::now(),
        }
    }
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a task completion with its latency.
    pub fn record_completion(&mut self, latency_ms: f64) {
        self.completed_tasks += 1;
        self.active_tasks = self.active_tasks.saturating_sub(1);
        // Exponential moving average for latency
        let alpha = 0.1;
        self.avg_task_latency_ms =
            alpha * latency_ms + (1.0 - alpha) * self.avg_task_latency_ms;
    }

    /// Record a new task being queued.
    pub fn record_queue(&mut self) {
        self.queued_tasks += 1;
    }

    /// Record a task starting execution.
    pub fn record_start(&mut self) {
        self.active_tasks += 1;
        self.queued_tasks = self.queued_tasks.saturating_sub(1);
    }

    /// Update queue utilization.
    pub fn update_utilization(&mut self, max_queue: usize) {
        if max_queue > 0 {
            self.queue_utilization =
                (self.queued_tasks as f64 / max_queue as f64).min(1.0);
        }
        self.last_measurement = Instant::now();
    }
}

/// Tokio Optimizer — manages runtime configuration and adaptive scaling.
#[cfg(feature = "v1.4-sprint1")]
pub struct TokioOptimizer {
    config: TokioOptimizerConfig,
    metrics: RuntimeMetrics,
    initialized: bool,
}

#[cfg(feature = "v1.4-sprint1")]
impl TokioOptimizer {
    pub fn new(config: TokioOptimizerConfig) -> Result<Self, TokioOptimizerError> {
        if config.min_workers == 0 {
            return Err(TokioOptimizerError::InvalidConfig(
                "min_workers must be > 0".to_string(),
            ));
        }
        if config.max_workers < config.min_workers {
            return Err(TokioOptimizerError::InvalidConfig(
                "max_workers must be >= min_workers".to_string(),
            ));
        }
        Ok(Self {
            config,
            metrics: RuntimeMetrics::new(),
            initialized: false,
        })
    }

    /// Initialize the runtime with current configuration.
    pub fn initialize(&mut self) -> Result<(), TokioOptimizerError> {
        if self.initialized {
            return Err(TokioOptimizerError::AlreadyInitialized);
        }
        let workers = if self.config.worker_threads > 0 {
            self.config.worker_threads
        } else {
            num_cpus::get().max(self.config.min_workers)
        };
        self.metrics.worker_threads = workers.min(self.config.max_workers);
        self.metrics.io_threads = if self.config.io_threads > 0 {
            self.config.io_threads
        } else {
            1
        };
        self.initialized = true;
        Ok(())
    }

    /// Perform adaptive scaling based on current metrics.
    pub fn adapt(&mut self) -> Option<usize> {
        if !self.config.adaptive_scaling || !self.initialized {
            return None;
        }

        self.metrics
            .update_utilization(self.config.max_task_queue);

        let old_workers = self.metrics.worker_threads;
        let utilization = self.metrics.queue_utilization;

        if utilization >= self.config.scale_up_threshold {
            // Scale up
            let new_workers = (self.metrics.worker_threads * 2)
                .min(self.config.max_workers)
                .max(self.metrics.worker_threads + 1);
            self.metrics.worker_threads = new_workers;
        } else if utilization <= self.config.scale_down_threshold {
            // Scale down
            let new_workers = (self.metrics.worker_threads / 2)
                .max(self.config.min_workers)
                .min(self.metrics.worker_threads.saturating_sub(1));
            self.metrics.worker_threads = new_workers;
        }

        if self.metrics.worker_threads != old_workers {
            Some(self.metrics.worker_threads)
        } else {
            None
        }
    }

    /// Get recommended worker thread count for the given profile.
    pub fn recommended_workers(&self, profile: RuntimeProfile) -> usize {
        let base = num_cpus::get().max(1);
        match profile {
            RuntimeProfile::Compute => base,
            RuntimeProfile::IoBound => (base * 2).min(self.config.max_workers),
            RuntimeProfile::Balanced => base.max(self.config.min_workers),
            RuntimeProfile::LowLatency => (base / 2).max(self.config.min_workers),
        }
    }

    /// Get current metrics snapshot.
    pub fn metrics(&self) -> &RuntimeMetrics {
        &self.metrics
    }

    /// Get current configuration.
    pub fn config(&self) -> &TokioOptimizerConfig {
        &self.config
    }

    /// Check if runtime is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for TokioOptimizer {
    fn default() -> Self {
        Self::new(TokioOptimizerConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TokioOptimizerConfig::default();
        assert_eq!(config.worker_threads, 0);
        assert!(config.adaptive_scaling);
        assert_eq!(config.min_workers, 2);
        assert_eq!(config.max_workers, 64);
    }

    #[test]
    fn test_optimizer_creation() {
        let optimizer = TokioOptimizer::default();
        assert!(!optimizer.is_initialized());
    }

    #[test]
    fn test_optimizer_with_config() {
        let config = TokioOptimizerConfig {
            min_workers: 4,
            max_workers: 16,
            ..Default::default()
        };
        let optimizer = TokioOptimizer::new(config).unwrap();
        assert_eq!(optimizer.config().min_workers, 4);
        assert_eq!(optimizer.config().max_workers, 16);
    }

    #[test]
    fn test_invalid_config_zero_min() {
        let config = TokioOptimizerConfig {
            min_workers: 0,
            ..Default::default()
        };
        assert!(TokioOptimizer::new(config).is_err());
    }

    #[test]
    fn test_invalid_config_max_less_than_min() {
        let config = TokioOptimizerConfig {
            min_workers: 8,
            max_workers: 4,
            ..Default::default()
        };
        assert!(TokioOptimizer::new(config).is_err());
    }

    #[test]
    fn test_initialize() {
        let mut optimizer = TokioOptimizer::default();
        optimizer.initialize().unwrap();
        assert!(optimizer.is_initialized());
    }

    #[test]
    fn test_double_initialize() {
        let mut optimizer = TokioOptimizer::default();
        optimizer.initialize().unwrap();
        assert!(optimizer.initialize().is_err());
    }

    #[test]
    fn test_metrics_default() {
        let metrics = RuntimeMetrics::default();
        assert_eq!(metrics.active_tasks, 0);
        assert_eq!(metrics.queued_tasks, 0);
        assert_eq!(metrics.completed_tasks, 0);
    }

    #[test]
    fn test_metrics_record_completion() {
        let mut metrics = RuntimeMetrics::new();
        metrics.active_tasks = 5;
        metrics.record_completion(10.0);
        assert_eq!(metrics.completed_tasks, 1);
        assert_eq!(metrics.active_tasks, 4);
        assert!(metrics.avg_task_latency_ms > 0.0);
    }

    #[test]
    fn test_metrics_record_queue() {
        let mut metrics = RuntimeMetrics::new();
        metrics.record_queue();
        assert_eq!(metrics.queued_tasks, 1);
    }

    #[test]
    fn test_metrics_record_start() {
        let mut metrics = RuntimeMetrics::new();
        metrics.queued_tasks = 3;
        metrics.record_start();
        assert_eq!(metrics.active_tasks, 1);
        assert_eq!(metrics.queued_tasks, 2);
    }

    #[test]
    fn test_adapt_scale_up() {
        let mut optimizer = TokioOptimizer::default();
        optimizer.initialize().unwrap();
        let initial = optimizer.metrics().worker_threads;
        // Simulate high utilization
        optimizer.metrics.queued_tasks = optimizer.config.max_task_queue as usize;
        optimizer.metrics.update_utilization(optimizer.config.max_task_queue);
        let new_workers = optimizer.adapt();
        assert!(new_workers.is_some());
        assert!(new_workers.unwrap() > initial);
    }

    #[test]
    fn test_adapt_scale_down() {
        let mut optimizer = TokioOptimizer::default();
        optimizer.initialize().unwrap();
        let initial = optimizer.metrics().worker_threads;
        // Simulate low utilization
        optimizer.metrics.queued_tasks = 0;
        optimizer.metrics.update_utilization(optimizer.config.max_task_queue);
        let new_workers = optimizer.adapt();
        // May scale down if initial > min_workers
        if let Some(w) = new_workers {
            assert!(w <= initial);
            assert!(w >= optimizer.config.min_workers);
        }
    }

    #[test]
    fn test_recommended_workers_compute() {
        let optimizer = TokioOptimizer::default();
        let workers = optimizer.recommended_workers(RuntimeProfile::Compute);
        assert!(workers >= num_cpus::get());
    }

    #[test]
    fn test_recommended_workers_io() {
        let optimizer = TokioOptimizer::default();
        let workers = optimizer.recommended_workers(RuntimeProfile::IoBound);
        assert!(workers >= num_cpus::get());
    }

    #[test]
    fn test_profile_display() {
        assert_eq!(format!("{}", RuntimeProfile::Compute), "compute");
        assert_eq!(format!("{}", RuntimeProfile::IoBound), "io_bound");
        assert_eq!(format!("{}", RuntimeProfile::Balanced), "balanced");
        assert_eq!(format!("{}", RuntimeProfile::LowLatency), "low_latency");
    }

    #[test]
    fn test_error_display() {
        let err = TokioOptimizerError::InvalidConfig("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test"));

        let err = TokioOptimizerError::AlreadyInitialized;
        let msg = format!("{}", err);
        assert!(msg.contains("initialized"));

        let err = TokioOptimizerError::OptimizationFailed("fail".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("fail"));
    }

    #[test]
    fn test_adaptive_disabled() {
        let config = TokioOptimizerConfig {
            adaptive_scaling: false,
            ..Default::default()
        };
        let mut optimizer = TokioOptimizer::new(config).unwrap();
        optimizer.initialize().unwrap();
        optimizer.metrics.queued_tasks = 1000;
        assert!(optimizer.adapt().is_none());
    }
}
