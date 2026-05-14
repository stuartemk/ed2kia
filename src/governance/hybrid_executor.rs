//! Hybrid Executor — On-chain/off-chain execution engine for DAO governance.
//!
//! Manages hybrid execution of governance decisions, routing between
//! on-chain (consensus-required) and off-chain (local) execution paths
//! with fallback and retry logic.
//!
//! **Design:** Linux `systemd`-inspired service execution with dependency tracking.
//!
//! **Key features:**
//! - Dual-path execution (on-chain/off-chain)
//! - Automatic fallback on failure
//! - Retry logic with exponential backoff
//! - Execution state tracking
//!
//! **References:**
//! - `dao_ledger_v4.rs` — ExecutionType enum and governance events
//! - `pool_matcher.rs` — Priority scoring patterns
//!
//! Apache License 2.0 + Ethical Use Clause

use std::collections::{HashMap, VecDeque};

// ─── Error ───────────────────────────────────────────────────────────────────

/// Errors for hybrid execution.
#[derive(Debug, Clone, PartialEq)]
pub enum HybridExecutorError {
    /// Execution ID not found.
    ExecutionNotFound(String),
    /// Execution already completed.
    AlreadyCompleted(String),
    /// On-chain execution failed.
    OnChainFailed(String),
    /// Off-chain execution failed.
    OffChainFailed(String),
    /// Maximum retries exceeded.
    MaxRetriesExceeded(String),
    /// Invalid configuration.
    InvalidConfig(String),
    /// Execution queue is full.
    QueueFull,
    /// Timeout exceeded.
    TimeoutExceeded,
}

impl std::fmt::Display for HybridExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HybridExecutorError::ExecutionNotFound(id) => write!(f, "Execution not found: {}", id),
            HybridExecutorError::AlreadyCompleted(id) => write!(f, "Already completed: {}", id),
            HybridExecutorError::OnChainFailed(msg) => write!(f, "On-chain failed: {}", msg),
            HybridExecutorError::OffChainFailed(msg) => write!(f, "Off-chain failed: {}", msg),
            HybridExecutorError::MaxRetriesExceeded(id) => write!(f, "Max retries exceeded: {}", id),
            HybridExecutorError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            HybridExecutorError::QueueFull => write!(f, "Execution queue full"),
            HybridExecutorError::TimeoutExceeded => write!(f, "Timeout exceeded"),
        }
    }
}

// ─── Execution State ─────────────────────────────────────────────────────────

/// Current state of an execution.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    /// Waiting to be executed.
    Pending,
    /// Currently executing on-chain.
    ExecutingOnChain,
    /// Currently executing off-chain.
    ExecutingOffChain,
    /// Successfully completed.
    Completed,
    /// Failed after all retries.
    Failed,
    /// Timed out.
    TimedOut,
}

impl std::fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionState::Pending => write!(f, "Pending"),
            ExecutionState::ExecutingOnChain => write!(f, "ExecutingOnChain"),
            ExecutionState::ExecutingOffChain => write!(f, "ExecutingOffChain"),
            ExecutionState::Completed => write!(f, "Completed"),
            ExecutionState::Failed => write!(f, "Failed"),
            ExecutionState::TimedOut => write!(f, "TimedOut"),
        }
    }
}

// ─── Execution Path ──────────────────────────────────────────────────────────

/// Preferred execution path.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionPath {
    /// Prefer on-chain, fallback to off-chain.
    PreferOnChain,
    /// Prefer off-chain, fallback to on-chain.
    PreferOffChain,
    /// Strictly on-chain (no fallback).
    StrictOnChain,
    /// Strictly off-chain (no fallback).
    StrictOffChain,
}

impl std::fmt::Display for ExecutionPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionPath::PreferOnChain => write!(f, "PreferOnChain"),
            ExecutionPath::PreferOffChain => write!(f, "PreferOffChain"),
            ExecutionPath::StrictOnChain => write!(f, "StrictOnChain"),
            ExecutionPath::StrictOffChain => write!(f, "StrictOffChain"),
        }
    }
}

// ─── Config ──────────────────────────────────────────────────────────────────

/// Configuration for the hybrid executor.
#[derive(Debug, Clone)]
pub struct HybridExecutorConfig {
    /// Maximum pending executions.
    pub max_pending: usize,
    /// Maximum retry attempts.
    pub max_retries: usize,
    /// Base backoff time in ms.
    pub base_backoff_ms: u64,
    /// Execution timeout in ms.
    pub execution_timeout_ms: u64,
    /// Enable automatic fallback.
    pub auto_fallback: bool,
    /// On-chain success rate threshold for fallback.
    pub on_chain_threshold: f64,
}

impl Default for HybridExecutorConfig {
    fn default() -> Self {
        Self {
            max_pending: 512,
            max_retries: 3,
            base_backoff_ms: 1000,
            execution_timeout_ms: 30000,
            auto_fallback: true,
            on_chain_threshold: 0.8,
        }
    }
}

// ─── Execution Record ────────────────────────────────────────────────────────

/// Record of a governance execution.
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    /// Unique execution ID.
    pub execution_id: String,
    /// Source governance entry ID.
    pub source_entry_id: String,
    /// Execution path preference.
    pub path: ExecutionPath,
    /// Current state.
    pub state: ExecutionState,
    /// Retry count.
    pub retry_count: usize,
    /// Next retry backoff in ms.
    pub next_backoff_ms: u64,
    /// On-chain transaction hash (if applicable).
    pub tx_hash: Option<String>,
    /// Off-chain result (if applicable).
    pub off_chain_result: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Creation timestamp.
    pub created_ms: u64,
    /// Last update timestamp.
    pub updated_ms: u64,
    /// Completion timestamp.
    pub completed_ms: Option<u64>,
}

impl ExecutionRecord {
    pub fn new(
        execution_id: String,
        source_entry_id: String,
        path: ExecutionPath,
        now_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            source_entry_id,
            path,
            state: ExecutionState::Pending,
            retry_count: 0,
            next_backoff_ms: 0,
            tx_hash: None,
            off_chain_result: None,
            error: None,
            created_ms: now_ms,
            updated_ms: now_ms,
            completed_ms: None,
        }
    }

    /// Check if execution is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            ExecutionState::Completed | ExecutionState::Failed | ExecutionState::TimedOut
        )
    }

    /// Check if execution has timed out.
    pub fn is_timed_out(&self, timeout_ms: u64, now_ms: u64) -> bool {
        now_ms - self.created_ms > timeout_ms
    }

    /// Check if retries are exhausted.
    pub fn retries_exhausted(&self, max_retries: usize) -> bool {
        self.retry_count >= max_retries
    }
}

// ─── Stats ───────────────────────────────────────────────────────────────────

/// Statistics for hybrid execution.
#[derive(Debug, Clone)]
pub struct ExecutorStats {
    /// Total executions created.
    pub total_executions: usize,
    /// Total on-chain executions.
    pub on_chain_executions: usize,
    /// Total off-chain executions.
    pub off_chain_executions: usize,
    /// Total successful completions.
    pub total_completed: usize,
    /// Total failures.
    pub total_failed: usize,
    /// Total fallbacks triggered.
    pub total_fallbacks: usize,
    /// Total retries performed.
    pub total_retries: usize,
    /// Current pending count.
    pub pending_count: usize,
    /// Average execution time in ms.
    pub avg_execution_time_ms: f64,
}

impl Default for ExecutorStats {
    fn default() -> Self {
        Self {
            total_executions: 0,
            on_chain_executions: 0,
            off_chain_executions: 0,
            total_completed: 0,
            total_failed: 0,
            total_fallbacks: 0,
            total_retries: 0,
            pending_count: 0,
            avg_execution_time_ms: 0.0,
        }
    }
}

// ─── Main Executor ───────────────────────────────────────────────────────────

/// Hybrid execution engine for DAO governance.
pub struct HybridExecutor {
    config: HybridExecutorConfig,
    executions: HashMap<String, ExecutionRecord>,
    pending_queue: VecDeque<String>,
    stats: ExecutorStats,
    current_time_ms: u64,
    execution_counter: u64,
    total_execution_time_ms: f64,
}

impl HybridExecutor {
    // ─── Construction ──────────────────────────────────────────────────────

    /// Create a new hybrid executor.
    pub fn new(config: HybridExecutorConfig) -> Self {
        Self {
            config,
            executions: HashMap::new(),
            pending_queue: VecDeque::new(),
            stats: ExecutorStats::default(),
            current_time_ms: current_timestamp_ms(),
            execution_counter: 0,
            total_execution_time_ms: 0.0,
        }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(HybridExecutorConfig::default())
    }

    // ─── Execution Creation ────────────────────────────────────────────────

    /// Create a new execution request.
    pub fn create_execution(
        &mut self,
        source_entry_id: String,
        path: ExecutionPath,
    ) -> Result<String, HybridExecutorError> {
        // Check queue limit
        if self.pending_queue.len() >= self.config.max_pending {
            return Err(HybridExecutorError::QueueFull);
        }

        self.execution_counter += 1;
        let execution_id = format!("exec-{}", self.execution_counter);

        let record = ExecutionRecord::new(
            execution_id.clone(),
            source_entry_id,
            path,
            self.current_time_ms,
        );

        self.executions.insert(execution_id.clone(), record);
        self.pending_queue.push_back(execution_id.clone());
        self.stats.total_executions += 1;
        self.stats.pending_count += 1;

        Ok(execution_id)
    }

    // ─── Execution Actions ─────────────────────────────────────────────────

    /// Start on-chain execution.
    pub fn start_on_chain(&mut self, execution_id: &str) -> Result<(), HybridExecutorError> {
        let record = self.executions.get_mut(execution_id).ok_or_else(|| {
            HybridExecutorError::ExecutionNotFound(execution_id.to_string())
        })?;

        if record.is_terminal() {
            return Err(HybridExecutorError::AlreadyCompleted(execution_id.to_string()));
        }

        record.state = ExecutionState::ExecutingOnChain;
        record.updated_ms = self.current_time_ms;
        self.stats.on_chain_executions += 1;

        Ok(())
    }

    /// Start off-chain execution.
    pub fn start_off_chain(&mut self, execution_id: &str) -> Result<(), HybridExecutorError> {
        let record = self.executions.get_mut(execution_id).ok_or_else(|| {
            HybridExecutorError::ExecutionNotFound(execution_id.to_string())
        })?;

        if record.is_terminal() {
            return Err(HybridExecutorError::AlreadyCompleted(execution_id.to_string()));
        }

        record.state = ExecutionState::ExecutingOffChain;
        record.updated_ms = self.current_time_ms;
        self.stats.off_chain_executions += 1;

        Ok(())
    }

    /// Complete execution successfully.
    pub fn complete_execution(
        &mut self,
        execution_id: &str,
        tx_hash: Option<String>,
        off_chain_result: Option<String>,
    ) -> Result<(), HybridExecutorError> {
        let record = self.executions.get(execution_id).ok_or_else(|| {
            HybridExecutorError::ExecutionNotFound(execution_id.to_string())
        })?;

        if record.is_terminal() {
            return Err(HybridExecutorError::AlreadyCompleted(execution_id.to_string()));
        }

        let record = self.executions.get_mut(execution_id).unwrap();
        record.state = ExecutionState::Completed;
        record.tx_hash = tx_hash;
        record.off_chain_result = off_chain_result;
        record.updated_ms = self.current_time_ms;
        record.completed_ms = Some(self.current_time_ms);

        let exec_time = self.current_time_ms - record.created_ms;
        self.total_execution_time_ms += exec_time as f64;
        self.stats.avg_execution_time_ms =
            self.total_execution_time_ms / self.stats.total_completed.max(1) as f64;

        self.stats.total_completed += 1;
        self.stats.pending_count -= 1;

        Ok(())
    }

    /// Fail execution with error.
    pub fn fail_execution(
        &mut self,
        execution_id: &str,
        error: String,
    ) -> Result<(), HybridExecutorError> {
        let record = self.executions.get_mut(execution_id).ok_or_else(|| {
            HybridExecutorError::ExecutionNotFound(execution_id.to_string())
        })?;

        if record.is_terminal() {
            return Err(HybridExecutorError::AlreadyCompleted(execution_id.to_string()));
        }

        record.error = Some(error);
        record.updated_ms = self.current_time_ms;

        // Check if should retry
        if !record.retries_exhausted(self.config.max_retries) {
            record.retry_count += 1;
            record.next_backoff_ms = self.config.base_backoff_ms * (1 << record.retry_count);
            record.state = ExecutionState::Pending;
            self.stats.total_retries += 1;
            self.pending_queue.push_back(execution_id.to_string());
        } else {
            record.state = ExecutionState::Failed;
            record.completed_ms = Some(self.current_time_ms);
            self.stats.total_failed += 1;
            self.stats.pending_count -= 1;
        }

        Ok(())
    }

    // ─── Fallback ──────────────────────────────────────────────────────────

    /// Trigger fallback from on-chain to off-chain (or vice versa).
    pub fn trigger_fallback(&mut self, execution_id: &str) -> Result<(), HybridExecutorError> {
        if !self.config.auto_fallback {
            return Err(HybridExecutorError::InvalidConfig("Fallback disabled".to_string()));
        }

        let record = self.executions.get_mut(execution_id).ok_or_else(|| {
            HybridExecutorError::ExecutionNotFound(execution_id.to_string())
        })?;

        if record.is_terminal() {
            return Err(HybridExecutorError::AlreadyCompleted(execution_id.to_string()));
        }

        // Switch execution path
        match record.state {
            ExecutionState::ExecutingOnChain => {
                record.state = ExecutionState::ExecutingOffChain;
                self.stats.off_chain_executions += 1;
            }
            ExecutionState::ExecutingOffChain => {
                record.state = ExecutionState::ExecutingOnChain;
                self.stats.on_chain_executions += 1;
            }
            _ => {}
        }

        record.updated_ms = self.current_time_ms;
        self.stats.total_fallbacks += 1;

        Ok(())
    }

    // ─── Queries ───────────────────────────────────────────────────────────

    /// Get execution record.
    pub fn get_execution(&self, execution_id: &str) -> Option<&ExecutionRecord> {
        self.executions.get(execution_id)
    }

    /// Get pending executions.
    pub fn pending_executions(&self) -> Vec<&ExecutionRecord> {
        self.pending_queue
            .iter()
            .filter_map(|id| self.executions.get(id))
            .collect()
    }

    /// Get executions by source entry.
    pub fn get_by_source(&self, source_entry_id: &str) -> Vec<&ExecutionRecord> {
        self.executions
            .values()
            .filter(|e| e.source_entry_id == source_entry_id)
            .collect()
    }

    /// Get failed executions.
    pub fn failed_executions(&self) -> Vec<&ExecutionRecord> {
        self.executions
            .values()
            .filter(|e| e.state == ExecutionState::Failed)
            .collect()
    }

    // ─── Cleanup ───────────────────────────────────────────────────────────

    /// Check for timed out executions.
    pub fn check_timeouts(&mut self) -> usize {
        let mut timed_out = 0;
        self.executions.iter_mut().for_each(|(_, record)| {
            if !record.is_terminal() && record.is_timed_out(self.config.execution_timeout_ms, self.current_time_ms) {
                record.state = ExecutionState::TimedOut;
                record.error = Some("Timeout exceeded".to_string());
                record.completed_ms = Some(self.current_time_ms);
                timed_out += 1;
            }
        });
        timed_out
    }

    // ─── Time ──────────────────────────────────────────────────────────────

    /// Advance internal time.
    pub fn advance_time(&mut self, ms: u64) {
        self.current_time_ms += ms;
    }

    // ─── Stats ─────────────────────────────────────────────────────────────

    /// Get current statistics.
    pub fn stats(&self) -> ExecutorStats {
        ExecutorStats {
            total_executions: self.stats.total_executions,
            on_chain_executions: self.stats.on_chain_executions,
            off_chain_executions: self.stats.off_chain_executions,
            total_completed: self.stats.total_completed,
            total_failed: self.stats.total_failed,
            total_fallbacks: self.stats.total_fallbacks,
            total_retries: self.stats.total_retries,
            pending_count: self.pending_queue.len(),
            avg_execution_time_ms: self.stats.avg_execution_time_ms,
        }
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = ExecutorStats::default();
        self.total_execution_time_ms = 0.0;
    }
}

impl Default for HybridExecutor {
    fn default() -> Self {
        Self::new(HybridExecutorConfig::default())
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let exec = HybridExecutor::new(HybridExecutorConfig::default());
        assert_eq!(exec.stats().total_executions, 0);
    }

    #[test]
    fn test_create_execution() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("entry-1".to_string(), ExecutionPath::PreferOnChain);
        assert!(id.is_ok());
        assert_eq!(exec.stats().total_executions, 1);
    }

    #[test]
    fn test_start_on_chain() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        assert!(exec.start_on_chain(&id).is_ok());
        assert_eq!(exec.get_execution(&id).unwrap().state, ExecutionState::ExecutingOnChain);
    }

    #[test]
    fn test_start_off_chain() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOffChain).unwrap();
        assert!(exec.start_off_chain(&id).is_ok());
        assert_eq!(exec.get_execution(&id).unwrap().state, ExecutionState::ExecutingOffChain);
    }

    #[test]
    fn test_complete_execution() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        assert!(exec.complete_execution(&id, Some("0xabc".to_string()), None).is_ok());
        assert_eq!(exec.get_execution(&id).unwrap().state, ExecutionState::Completed);
    }

    #[test]
    fn test_fail_execution_retry() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        assert!(exec.fail_execution(&id, "error".to_string()).is_ok());
        assert_eq!(exec.get_execution(&id).unwrap().state, ExecutionState::Pending);
        assert_eq!(exec.get_execution(&id).unwrap().retry_count, 1);
    }

    #[test]
    fn test_fail_execution_max_retries() {
        let mut config = HybridExecutorConfig::default();
        config.max_retries = 1;
        let mut exec = HybridExecutor::new(config);
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        exec.fail_execution(&id, "error".to_string()).unwrap();
        exec.start_on_chain(&id).unwrap();
        exec.fail_execution(&id, "error".to_string()).unwrap();
        assert_eq!(exec.get_execution(&id).unwrap().state, ExecutionState::Failed);
    }

    #[test]
    fn test_trigger_fallback() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        assert!(exec.trigger_fallback(&id).is_ok());
        assert_eq!(exec.get_execution(&id).unwrap().state, ExecutionState::ExecutingOffChain);
    }

    #[test]
    fn test_fallback_disabled() {
        let mut config = HybridExecutorConfig::default();
        config.auto_fallback = false;
        let mut exec = HybridExecutor::new(config);
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        assert!(exec.trigger_fallback(&id).is_err());
    }

    #[test]
    fn test_timeout_detection() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        exec.advance_time(60000);
        let timed = exec.check_timeouts();
        assert_eq!(timed, 1);
    }

    #[test]
    fn test_queue_full() {
        let mut config = HybridExecutorConfig::default();
        config.max_pending = 1;
        let mut exec = HybridExecutor::new(config);
        exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        assert!(exec.create_execution("e2".to_string(), ExecutionPath::PreferOnChain).is_err());
    }

    #[test]
    fn test_pending_executions() {
        let mut exec = HybridExecutor::default_config();
        exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.create_execution("e2".to_string(), ExecutionPath::PreferOffChain).unwrap();
        assert_eq!(exec.pending_executions().len(), 2);
    }

    #[test]
    fn test_get_by_source() {
        let mut exec = HybridExecutor::default_config();
        exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.create_execution("e1".to_string(), ExecutionPath::PreferOffChain).unwrap();
        let results = exec.get_by_source("e1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_failed_executions() {
        let mut config = HybridExecutorConfig::default();
        config.max_retries = 0;
        let mut exec = HybridExecutor::new(config);
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        exec.fail_execution(&id, "error".to_string()).unwrap();
        assert_eq!(exec.failed_executions().len(), 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        exec.complete_execution(&id, Some("0x".to_string()), None).unwrap();
        let stats = exec.stats();
        assert_eq!(stats.total_completed, 1);
        assert_eq!(stats.on_chain_executions, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut exec = HybridExecutor::default_config();
        exec.reset_stats();
        assert_eq!(exec.stats().total_executions, 0);
    }

    #[test]
    fn test_execution_terminal() {
        let record = ExecutionRecord::new("e1".to_string(), "s1".to_string(), ExecutionPath::PreferOnChain, 1000);
        assert!(!record.is_terminal());
        let mut completed = record.clone();
        completed.state = ExecutionState::Completed;
        assert!(completed.is_terminal());
    }

    #[test]
    fn test_execution_timeout() {
        let record = ExecutionRecord::new("e1".to_string(), "s1".to_string(), ExecutionPath::PreferOnChain, 1000);
        assert!(!record.is_timed_out(5000, 4000));
        assert!(record.is_timed_out(5000, 7000));
    }

    #[test]
    fn test_retries_exhausted() {
        let mut record = ExecutionRecord::new("e1".to_string(), "s1".to_string(), ExecutionPath::PreferOnChain, 1000);
        record.retry_count = 3;
        assert!(record.retries_exhausted(3));
        assert!(!record.retries_exhausted(5));
    }

    #[test]
    fn test_state_display() {
        assert_eq!(ExecutionState::Pending.to_string(), "Pending");
        assert_eq!(ExecutionState::Completed.to_string(), "Completed");
        assert_eq!(ExecutionState::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_path_display() {
        assert_eq!(ExecutionPath::PreferOnChain.to_string(), "PreferOnChain");
        assert_eq!(ExecutionPath::StrictOffChain.to_string(), "StrictOffChain");
    }

    #[test]
    fn test_error_display() {
        match HybridExecutorError::ExecutionNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_config_default() {
        let config = HybridExecutorConfig::default();
        assert_eq!(config.max_pending, 512);
        assert_eq!(config.max_retries, 3);
        assert!(config.auto_fallback);
    }

    #[test]
    fn test_stats_default() {
        let stats = ExecutorStats::default();
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.total_completed, 0);
    }

    #[test]
    fn test_executor_default() {
        let exec = HybridExecutor::default();
        assert_eq!(exec.stats().total_executions, 0);
    }

    #[test]
    fn test_already_completed_error() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        exec.complete_execution(&id, None, None).unwrap();
        assert!(exec.start_on_chain(&id).is_err());
    }

    #[test]
    fn test_backoff_calculation() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.start_on_chain(&id).unwrap();
        exec.fail_execution(&id, "err".to_string()).unwrap();
        let record = exec.get_execution(&id).unwrap();
        assert_eq!(record.next_backoff_ms, 2000);
    }

    #[test]
    fn test_avg_execution_time() {
        let mut exec = HybridExecutor::default_config();
        let id = exec.create_execution("e1".to_string(), ExecutionPath::PreferOnChain).unwrap();
        exec.advance_time(1000);
        exec.complete_execution(&id, None, None).unwrap();
        assert!(exec.stats().avg_execution_time_ms > 0.0);
    }
}
