//! WASM Mobile Hardening v1 — Memory limits, adaptive scheduler, thermal fallback & isolation tests.
//!
//! Provides hardening layers for WASM mobile bridge operations:
//! - Memory/syscall limits enforcement
//! - Adaptive scheduler with thermal awareness
//! - Fallback mechanisms for constrained environments
//! - Isolation testing utilities
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Hardening Layers                           │
//! │  ┌──────────────────┐  ┌──────────────────────────────┐    │
//! │  │  MemoryLimiter   │  │  SyscallFilter               │    │
//! │  │  - max_bytes     │  │  - allowed_syscalls          │    │
//! │  │  - current_usage │  │  - deny_list                 │    │
//! │  └──────────────────┘  └──────────────────────────────┘    │
//! ├─────────────────────────────────────────────────────────────┤
//! │              Adaptive Scheduler                              │
//! │  ┌──────────────────┐  ┌──────────────────────────────┐    │
//! │  │  ThermalMonitor  │  │  PriorityScheduler           │    │
//! │  │  - temp_level    │  │  - task_queue                │    │
//! │  │  - fallback_act  │  │  - priority_levels           │    │
//! │  └──────────────────┘  └──────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! Feature-gated behind `cfg(feature = "v2.0-sprint2")`.

mod internal {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;
    use std::fmt;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Hardening operation errors.
    #[derive(Debug, Clone, PartialEq)]
    pub enum HardeningError {
        /// Memory limit exceeded.
        MemoryLimitExceeded(usize),
        /// Syscall not allowed.
        SyscallNotAllowed(String),
        /// Thermal threshold exceeded.
        ThermalThresholdExceeded(f32),
        /// Scheduler queue full.
        SchedulerQueueFull,
        /// Task priority invalid.
        InvalidPriority(u32),
        /// Isolation test failed.
        IsolationTestFailed(String),
    }

    impl fmt::Display for HardeningError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                HardeningError::MemoryLimitExceeded(bytes) => {
                    write!(f, "Memory limit exceeded: {} bytes", bytes)
                }
                HardeningError::SyscallNotAllowed(syscall) => {
                    write!(f, "Syscall not allowed: {}", syscall)
                }
                HardeningError::ThermalThresholdExceeded(temp) => {
                    write!(f, "Thermal threshold exceeded: {}°C", temp)
                }
                HardeningError::SchedulerQueueFull => {
                    write!(f, "Scheduler queue is full")
                }
                HardeningError::InvalidPriority(p) => {
                    write!(f, "Invalid task priority: {}", p)
                }
                HardeningError::IsolationTestFailed(msg) => {
                    write!(f, "Isolation test failed: {}", msg)
                }
            }
        }
    }

    impl std::error::Error for HardeningError {}

    // ============================================================================
    // Memory Limiter
    // ============================================================================

    /// Memory usage level indicator.
    #[derive(Debug, Clone, PartialEq)]
    pub enum MemoryLevel {
        /// Normal usage (< 70%).
        Normal,
        /// Warning usage (70-90%).
        Warning,
        /// Critical usage (> 90%).
        Critical,
    }

    impl fmt::Display for MemoryLevel {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MemoryLevel::Normal => write!(f, "normal"),
                MemoryLevel::Warning => write!(f, "warning"),
                MemoryLevel::Critical => write!(f, "critical"),
            }
        }
    }

    /// Enforces memory allocation limits for WASM modules.
    pub struct MemoryLimiter {
        /// Maximum allowed memory in bytes.
        max_bytes: usize,
        /// Current memory usage in bytes.
        current_usage: usize,
        /// High water mark (peak usage).
        high_water_mark: usize,
        /// Number of allocations rejected.
        rejected_count: usize,
    }

    impl MemoryLimiter {
        /// Create a new memory limiter.
        pub fn new(max_bytes: usize) -> Self {
            Self {
                max_bytes,
                current_usage: 0,
                high_water_mark: 0,
                rejected_count: 0,
            }
        }

        /// Try to allocate memory. Returns Ok if within limits.
        pub fn try_allocate(&mut self, bytes: usize) -> Result<(), HardeningError> {
            if self.current_usage + bytes > self.max_bytes {
                self.rejected_count += 1;
                return Err(HardeningError::MemoryLimitExceeded(
                    self.current_usage + bytes,
                ));
            }
            self.current_usage += bytes;
            if self.current_usage > self.high_water_mark {
                self.high_water_mark = self.current_usage;
            }
            Ok(())
        }

        /// Release allocated memory.
        pub fn release(&mut self, bytes: usize) {
            if bytes <= self.current_usage {
                self.current_usage -= bytes;
            } else {
                self.current_usage = 0;
            }
        }

        /// Get current memory level.
        pub fn level(&self) -> MemoryLevel {
            if self.max_bytes == 0 {
                return MemoryLevel::Critical;
            }
            let ratio = self.current_usage as f64 / self.max_bytes as f64;
            if ratio > 0.9 {
                MemoryLevel::Critical
            } else if ratio > 0.7 {
                MemoryLevel::Warning
            } else {
                MemoryLevel::Normal
            }
        }

        /// Get current usage.
        pub fn current_usage(&self) -> usize {
            self.current_usage
        }

        /// Get maximum allowed bytes.
        pub fn max_bytes(&self) -> usize {
            self.max_bytes
        }

        /// Get high water mark.
        pub fn high_water_mark(&self) -> usize {
            self.high_water_mark
        }

        /// Get number of rejected allocations.
        pub fn rejected_count(&self) -> usize {
            self.rejected_count
        }

        /// Get utilization ratio.
        pub fn utilization(&self) -> f64 {
            if self.max_bytes == 0 {
                return 1.0;
            }
            self.current_usage as f64 / self.max_bytes as f64
        }

        /// Reset usage counters.
        pub fn reset(&mut self) {
            self.current_usage = 0;
            self.high_water_mark = 0;
            self.rejected_count = 0;
        }
    }

    // ============================================================================
    // Syscall Filter
    // ============================================================================

    /// Filters syscalls allowed in WASM sandbox.
    pub struct SyscallFilter {
        /// Allowed syscall names.
        allowed: Vec<String>,
        /// Denied syscall names (explicit blocklist).
        denied: Vec<String>,
        /// Number of blocked syscalls.
        blocked_count: usize,
    }

    impl SyscallFilter {
        /// Create a new syscall filter with default allowed list.
        pub fn new() -> Self {
            Self {
                allowed: vec![
                    "memory_alloc".to_string(),
                    "memory_free".to_string(),
                    "wasm_memory_grow".to_string(),
                    "wasm_table_grow".to_string(),
                    "wasm_unreachable".to_string(),
                ],
                denied: vec![
                    "exit".to_string(),
                    "fork".to_string(),
                    "exec".to_string(),
                    "open".to_string(),
                    "read".to_string(),
                    "write".to_string(),
                    "connect".to_string(),
                    "bind".to_string(),
                ],
                blocked_count: 0,
            }
        }

        /// Check if a syscall is allowed.
        pub fn is_allowed(&mut self, syscall: &str) -> bool {
            // Explicit deny takes precedence
            if self.denied.iter().any(|s| s == syscall) {
                self.blocked_count += 1;
                return false;
            }
            // Check allowed list
            self.allowed.iter().any(|s| s == syscall)
        }

        /// Add a syscall to the allowed list.
        pub fn allow(&mut self, syscall: String) {
            if !self.allowed.contains(&syscall) {
                self.allowed.push(syscall.clone());
            }
            // Remove from denied if present
            self.denied.retain(|s| s != &syscall);
        }

        /// Add a syscall to the denied list.
        pub fn deny(&mut self, syscall: String) {
            if !self.denied.contains(&syscall) {
                self.denied.push(syscall.clone());
            }
            // Remove from allowed if present
            self.allowed.retain(|s| s != &syscall);
        }

        /// Get number of blocked syscalls.
        pub fn blocked_count(&self) -> usize {
            self.blocked_count
        }

        /// Get allowed syscalls.
        pub fn allowed(&self) -> &[String] {
            &self.allowed
        }

        /// Get denied syscalls.
        pub fn denied(&self) -> &[String] {
            &self.denied
        }
    }

    impl Default for SyscallFilter {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Thermal Monitor
    // ============================================================================

    /// Thermal level for adaptive scheduling.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ThermalLevel {
        /// Normal temperature.
        Normal,
        /// Moderate heat — reduce non-critical tasks.
        Moderate,
        /// High heat — aggressive throttling.
        High,
        /// Critical — emergency fallback.
        Critical,
    }

    impl fmt::Display for ThermalLevel {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ThermalLevel::Normal => write!(f, "normal"),
                ThermalLevel::Moderate => write!(f, "moderate"),
                ThermalLevel::High => write!(f, "high"),
                ThermalLevel::Critical => write!(f, "critical"),
            }
        }
    }

    /// Monitors thermal state and triggers fallback actions.
    pub struct ThermalMonitor {
        /// Current temperature in Celsius.
        current_temp: f32,
        /// Moderate threshold.
        moderate_threshold: f32,
        /// High threshold.
        high_threshold: f32,
        /// Critical threshold.
        critical_threshold: f32,
        /// Current thermal level.
        level: ThermalLevel,
        /// Fallback activated.
        fallback_active: bool,
    }

    impl ThermalMonitor {
        /// Create a new thermal monitor.
        pub fn new() -> Self {
            Self {
                current_temp: 25.0,
                moderate_threshold: 60.0,
                high_threshold: 75.0,
                critical_threshold: 85.0,
                level: ThermalLevel::Normal,
                fallback_active: false,
            }
        }

        /// Update temperature reading.
        pub fn update_temperature(&mut self, temp: f32) {
            self.current_temp = temp;
            self.level = self.compute_level(temp);
            self.fallback_active = self.level == ThermalLevel::Critical;
        }

        /// Compute thermal level from temperature.
        fn compute_level(&self, temp: f32) -> ThermalLevel {
            if temp >= self.critical_threshold {
                ThermalLevel::Critical
            } else if temp >= self.high_threshold {
                ThermalLevel::High
            } else if temp >= self.moderate_threshold {
                ThermalLevel::Moderate
            } else {
                ThermalLevel::Normal
            }
        }

        /// Get current thermal level.
        pub fn level(&self) -> &ThermalLevel {
            &self.level
        }

        /// Get current temperature.
        pub fn current_temp(&self) -> f32 {
            self.current_temp
        }

        /// Check if fallback is active.
        pub fn is_fallback_active(&self) -> bool {
            self.fallback_active
        }

        /// Get recommended task reduction factor (0.0-1.0).
        pub fn task_reduction_factor(&self) -> f32 {
            match self.level {
                ThermalLevel::Normal => 1.0,
                ThermalLevel::Moderate => 0.7,
                ThermalLevel::High => 0.4,
                ThermalLevel::Critical => 0.1,
            }
        }

        /// Reset to normal state.
        pub fn reset(&mut self) {
            self.current_temp = 25.0;
            self.level = ThermalLevel::Normal;
            self.fallback_active = false;
        }
    }

    impl Default for ThermalMonitor {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Priority Scheduler
    // ============================================================================

    /// Task priority levels.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TaskPriority {
        /// Critical system task.
        Critical = 0,
        /// High priority task.
        High = 1,
        /// Normal priority task.
        Normal = 2,
        /// Low priority (background) task.
        Low = 3,
    }

    impl fmt::Display for TaskPriority {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TaskPriority::Critical => write!(f, "critical"),
                TaskPriority::High => write!(f, "high"),
                TaskPriority::Normal => write!(f, "normal"),
                TaskPriority::Low => write!(f, "low"),
            }
        }
    }

    impl PartialOrd for TaskPriority {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for TaskPriority {
        fn cmp(&self, other: &Self) -> Ordering {
            // Lower numeric value = higher priority
            (*other as u8).cmp(&(*self as u8))
        }
    }

    /// A schedulable task.
    #[derive(Debug, Clone)]
    pub struct SchedulableTask {
        /// Task identifier.
        pub task_id: String,
        /// Task priority.
        pub priority: TaskPriority,
        /// Estimated memory usage.
        pub estimated_memory: usize,
        /// Task is ready to execute.
        pub ready: bool,
    }

    impl SchedulableTask {
        /// Create a new schedulable task.
        pub fn new(task_id: String, priority: TaskPriority, estimated_memory: usize) -> Self {
            Self {
                task_id,
                priority,
                estimated_memory,
                ready: true,
            }
        }
    }

    impl Eq for SchedulableTask {}

    impl PartialEq for SchedulableTask {
        fn eq(&self, other: &Self) -> bool {
            self.task_id == other.task_id
        }
    }

    impl Ord for SchedulableTask {
        fn cmp(&self, other: &Self) -> Ordering {
            // Higher priority first, then by task_id for stability
            self.priority
                .cmp(&other.priority)
                .then_with(|| self.task_id.cmp(&other.task_id))
        }
    }

    impl PartialOrd for SchedulableTask {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    /// Adaptive priority scheduler with thermal awareness.
    pub struct PriorityScheduler {
        /// Task queue (priority heap).
        queue: BinaryHeap<SchedulableTask>,
        /// Maximum queue size.
        max_queue_size: usize,
        /// Thermal monitor.
        thermal: ThermalMonitor,
        /// Memory limiter.
        memory: MemoryLimiter,
        /// Tasks executed count.
        executed_count: usize,
        /// Tasks skipped count (due to thermal/memory).
        skipped_count: usize,
    }

    impl PriorityScheduler {
        /// Create a new priority scheduler.
        pub fn new(max_queue_size: usize, max_memory: usize) -> Self {
            Self {
                queue: BinaryHeap::new(),
                max_queue_size,
                thermal: ThermalMonitor::new(),
                memory: MemoryLimiter::new(max_memory),
                executed_count: 0,
                skipped_count: 0,
            }
        }

        /// Add a task to the scheduler.
        pub fn add_task(&mut self, task: SchedulableTask) -> Result<(), HardeningError> {
            if self.queue.len() >= self.max_queue_size {
                return Err(HardeningError::SchedulerQueueFull);
            }
            self.queue.push(task);
            Ok(())
        }

        /// Get the next task to execute (considering thermal/memory constraints).
        pub fn next_task(&mut self) -> Option<SchedulableTask> {
            // Check thermal constraints
            let reduction = self.thermal.task_reduction_factor();
            if reduction <= 0.1 {
                // Critical thermal — skip non-critical tasks
                if let Some(task) = self.queue.peek().cloned() {
                    if task.priority == TaskPriority::Critical {
                        self.queue.pop()
                    } else {
                        self.skipped_count += 1;
                        None
                    }
                } else {
                    None
                }
            } else {
                self.queue.pop()
            }
        }

        /// Execute the next task (simulate).
        pub fn execute_next(&mut self) -> Result<Option<SchedulableTask>, HardeningError> {
            if let Some(task) = self.next_task() {
                // Check memory
                match self.memory.try_allocate(task.estimated_memory) {
                    Ok(()) => {
                        self.executed_count += 1;
                        Ok(Some(task))
                    }
                    Err(_) => {
                        self.skipped_count += 1;
                        Ok(None)
                    }
                }
            } else {
                Ok(None)
            }
        }

        /// Update thermal state.
        pub fn update_thermal(&mut self, temp: f32) {
            self.thermal.update_temperature(temp);
        }

        /// Get queue length.
        pub fn queue_len(&self) -> usize {
            self.queue.len()
        }

        /// Check if queue is empty.
        pub fn is_empty(&self) -> bool {
            self.queue.is_empty()
        }

        /// Get thermal monitor reference.
        pub fn thermal(&self) -> &ThermalMonitor {
            &self.thermal
        }

        /// Get memory limiter reference.
        pub fn memory(&self) -> &MemoryLimiter {
            &self.memory
        }

        /// Get executed count.
        pub fn executed_count(&self) -> usize {
            self.executed_count
        }

        /// Get skipped count.
        pub fn skipped_count(&self) -> usize {
            self.skipped_count
        }

        /// Release memory for a completed task.
        pub fn release_task_memory(&mut self, bytes: usize) {
            self.memory.release(bytes);
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    mod tests {
        use super::*;

        #[test]
        fn test_memory_limiter_new() {
            let limiter = MemoryLimiter::new(1024);
            assert_eq!(limiter.current_usage(), 0);
            assert_eq!(limiter.max_bytes(), 1024);
            assert_eq!(limiter.level(), MemoryLevel::Normal);
        }

        #[test]
        fn test_memory_limiter_allocate() {
            let mut limiter = MemoryLimiter::new(1024);
            limiter.try_allocate(512).unwrap();
            assert_eq!(limiter.current_usage(), 512);
            assert_eq!(limiter.high_water_mark(), 512);
        }

        #[test]
        fn test_memory_limiter_exceed() {
            let mut limiter = MemoryLimiter::new(1024);
            limiter.try_allocate(1025).unwrap_err();
            assert_eq!(limiter.rejected_count(), 1);
        }

        #[test]
        fn test_memory_limiter_release() {
            let mut limiter = MemoryLimiter::new(1024);
            limiter.try_allocate(512).unwrap();
            limiter.release(256);
            assert_eq!(limiter.current_usage(), 256);
        }

        #[test]
        fn test_memory_limiter_level_warning() {
            let mut limiter = MemoryLimiter::new(1000);
            limiter.try_allocate(750).unwrap();
            assert_eq!(limiter.level(), MemoryLevel::Warning);
        }

        #[test]
        fn test_memory_limiter_level_critical() {
            let mut limiter = MemoryLimiter::new(1000);
            limiter.try_allocate(950).unwrap();
            assert_eq!(limiter.level(), MemoryLevel::Critical);
        }

        #[test]
        fn test_memory_limiter_utilization() {
            let mut limiter = MemoryLimiter::new(1000);
            limiter.try_allocate(500).unwrap();
            assert!((limiter.utilization() - 0.5).abs() < 0.001);
        }

        #[test]
        fn test_memory_limiter_reset() {
            let mut limiter = MemoryLimiter::new(1024);
            limiter.try_allocate(512).unwrap();
            limiter.reset();
            assert_eq!(limiter.current_usage(), 0);
            assert_eq!(limiter.rejected_count(), 0);
        }

        #[test]
        fn test_memory_level_display() {
            assert_eq!(format!("{}", MemoryLevel::Normal), "normal");
            assert_eq!(format!("{}", MemoryLevel::Warning), "warning");
            assert_eq!(format!("{}", MemoryLevel::Critical), "critical");
        }

        #[test]
        fn test_syscall_filter_default() {
            let mut filter = SyscallFilter::new();
            assert!(filter.is_allowed("memory_alloc"));
            assert!(!filter.is_allowed("exit"));
        }

        #[test]
        fn test_syscall_filter_allow() {
            let mut filter = SyscallFilter::new();
            filter.allow("custom_syscall".to_string());
            assert!(filter.is_allowed("custom_syscall"));
        }

        #[test]
        fn test_syscall_filter_deny() {
            let mut filter = SyscallFilter::new();
            filter.deny("memory_alloc".to_string());
            assert!(!filter.is_allowed("memory_alloc"));
        }

        #[test]
        fn test_syscall_filter_blocked_count() {
            let mut filter = SyscallFilter::new();
            assert!(!filter.is_allowed("fork"));
            assert!(!filter.is_allowed("exec"));
            assert_eq!(filter.blocked_count(), 2);
        }

        #[test]
        fn test_thermal_monitor_new() {
            let monitor = ThermalMonitor::new();
            assert_eq!(monitor.current_temp(), 25.0);
            assert_eq!(*monitor.level(), ThermalLevel::Normal);
            assert!(!monitor.is_fallback_active());
        }

        #[test]
        fn test_thermal_monitor_moderate() {
            let mut monitor = ThermalMonitor::new();
            monitor.update_temperature(65.0);
            assert_eq!(*monitor.level(), ThermalLevel::Moderate);
            assert!(!monitor.is_fallback_active());
        }

        #[test]
        fn test_thermal_monitor_high() {
            let mut monitor = ThermalMonitor::new();
            monitor.update_temperature(80.0);
            assert_eq!(*monitor.level(), ThermalLevel::High);
            assert!(!monitor.is_fallback_active());
        }

        #[test]
        fn test_thermal_monitor_critical() {
            let mut monitor = ThermalMonitor::new();
            monitor.update_temperature(90.0);
            assert_eq!(*monitor.level(), ThermalLevel::Critical);
            assert!(monitor.is_fallback_active());
        }

        #[test]
        fn test_thermal_monitor_reduction_factor() {
            let monitor = ThermalMonitor::new();
            assert_eq!(monitor.task_reduction_factor(), 1.0);
        }

        #[test]
        fn test_thermal_monitor_reset() {
            let mut monitor = ThermalMonitor::new();
            monitor.update_temperature(90.0);
            monitor.reset();
            assert_eq!(monitor.current_temp(), 25.0);
            assert_eq!(*monitor.level(), ThermalLevel::Normal);
        }

        #[test]
        fn test_thermal_level_display() {
            assert_eq!(format!("{}", ThermalLevel::Normal), "normal");
            assert_eq!(format!("{}", ThermalLevel::Moderate), "moderate");
            assert_eq!(format!("{}", ThermalLevel::High), "high");
            assert_eq!(format!("{}", ThermalLevel::Critical), "critical");
        }

        #[test]
        fn test_task_priority_ordering() {
            assert!(TaskPriority::Critical > TaskPriority::High);
            assert!(TaskPriority::High > TaskPriority::Normal);
            assert!(TaskPriority::Normal > TaskPriority::Low);
        }

        #[test]
        fn test_task_priority_display() {
            assert_eq!(format!("{}", TaskPriority::Critical), "critical");
            assert_eq!(format!("{}", TaskPriority::High), "high");
            assert_eq!(format!("{}", TaskPriority::Normal), "normal");
            assert_eq!(format!("{}", TaskPriority::Low), "low");
        }

        #[test]
        fn test_scheduler_new() {
            let scheduler = PriorityScheduler::new(10, 1024);
            assert!(scheduler.is_empty());
            assert_eq!(scheduler.queue_len(), 0);
        }

        #[test]
        fn test_scheduler_add_task() {
            let mut scheduler = PriorityScheduler::new(10, 1024);
            let task = SchedulableTask::new("task-1".to_string(), TaskPriority::Normal, 100);
            scheduler.add_task(task).unwrap();
            assert_eq!(scheduler.queue_len(), 1);
        }

        #[test]
        fn test_scheduler_queue_full() {
            let mut scheduler = PriorityScheduler::new(1, 1024);
            scheduler
                .add_task(SchedulableTask::new(
                    "t1".to_string(),
                    TaskPriority::Normal,
                    100,
                ))
                .unwrap();
            assert_eq!(
                scheduler.add_task(SchedulableTask::new(
                    "t2".to_string(),
                    TaskPriority::Normal,
                    100
                )),
                Err(HardeningError::SchedulerQueueFull)
            );
        }

        #[test]
        fn test_scheduler_priority_order() {
            let mut scheduler = PriorityScheduler::new(10, 1024);
            scheduler
                .add_task(SchedulableTask::new(
                    "low".to_string(),
                    TaskPriority::Low,
                    100,
                ))
                .unwrap();
            scheduler
                .add_task(SchedulableTask::new(
                    "critical".to_string(),
                    TaskPriority::Critical,
                    100,
                ))
                .unwrap();
            scheduler
                .add_task(SchedulableTask::new(
                    "normal".to_string(),
                    TaskPriority::Normal,
                    100,
                ))
                .unwrap();

            let task = scheduler.next_task().unwrap();
            assert_eq!(task.task_id, "critical");
        }

        #[test]
        fn test_scheduler_thermal_throttle() {
            let mut scheduler = PriorityScheduler::new(10, 1024);
            scheduler
                .add_task(SchedulableTask::new(
                    "normal".to_string(),
                    TaskPriority::Normal,
                    100,
                ))
                .unwrap();
            scheduler.update_thermal(90.0); // Critical

            let result = scheduler.execute_next();
            // Normal task should be skipped in critical thermal
            assert!(result.unwrap().is_none());
            assert_eq!(scheduler.skipped_count(), 1);
        }

        #[test]
        fn test_scheduler_memory_limit() {
            let mut scheduler = PriorityScheduler::new(10, 100); // Small memory
            scheduler
                .add_task(SchedulableTask::new(
                    "big".to_string(),
                    TaskPriority::Critical,
                    200,
                ))
                .unwrap();

            let result = scheduler.execute_next();
            // Task exceeds memory limit
            assert!(result.unwrap().is_none());
            assert_eq!(scheduler.skipped_count(), 1);
        }

        #[test]
        fn test_scheduler_execute_success() {
            let mut scheduler = PriorityScheduler::new(10, 1024);
            scheduler
                .add_task(SchedulableTask::new(
                    "task-1".to_string(),
                    TaskPriority::Critical,
                    100,
                ))
                .unwrap();

            let task = scheduler.execute_next().unwrap().unwrap();
            assert_eq!(task.task_id, "task-1");
            assert_eq!(scheduler.executed_count(), 1);
        }

        #[test]
        fn test_error_display() {
            let err = HardeningError::MemoryLimitExceeded(2048);
            let msg = format!("{}", err);
            assert!(msg.contains("2048"));
        }

        #[test]
        fn test_full_hardening_lifecycle() {
            let mut scheduler = PriorityScheduler::new(20, 4096);

            // Add tasks with different priorities
            scheduler
                .add_task(SchedulableTask::new(
                    "bg-sync".to_string(),
                    TaskPriority::Low,
                    512,
                ))
                .unwrap();
            scheduler
                .add_task(SchedulableTask::new(
                    "zkp-proof".to_string(),
                    TaskPriority::High,
                    1024,
                ))
                .unwrap();
            scheduler
                .add_task(SchedulableTask::new(
                    "heartbeat".to_string(),
                    TaskPriority::Critical,
                    128,
                ))
                .unwrap();

            assert_eq!(scheduler.queue_len(), 3);

            // Execute in priority order
            let task1 = scheduler.execute_next().unwrap().unwrap();
            assert_eq!(task1.task_id, "heartbeat");

            let task2 = scheduler.execute_next().unwrap().unwrap();
            assert_eq!(task2.task_id, "zkp-proof");

            // Simulate thermal event
            scheduler.update_thermal(80.0); // High
            assert_eq!(*scheduler.thermal().level(), ThermalLevel::High);

            // Release memory
            scheduler.release_task_memory(128);
            scheduler.release_task_memory(1024);
        }
    }
}

pub use internal::{
    HardeningError, MemoryLevel, MemoryLimiter, PriorityScheduler, SchedulableTask, SyscallFilter,
    TaskPriority, ThermalLevel, ThermalMonitor,
};
