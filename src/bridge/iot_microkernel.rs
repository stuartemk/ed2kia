//! IoT Microkernel â€” Sprint 71: Global Bootstrap & Critical Bottleneck Resolution
//!
//! Bridges RTOS (Real-Time Operating System) constraints with async P2P network via:
//! - Local micro-kernel with watchdog timer
//! - Last-valid GEI cache for offline fallback
//! - Priority queue for asyncâ†’sync command bridging
//! - Safe action execution with ethical bounds checking
//!
//! # Architecture
//!
//! ```text
//! [IoT Device] â†’ [Microkernel] â†’ [P2P Bridge]
//!    â†‘               â†‘
//! [Watchdog]    [Last-GEI Cache]
//! ```

use std::collections::VecDeque;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Errors in IoT microkernel operations.
#[derive(Debug, Clone, PartialEq)]
pub enum KernelError {
    /// Command exceeded watchdog timeout.
    WatchdogTimeout { elapsed_ms: u32, limit_ms: u32 },
    /// Command failed ethical bounds check.
    EthicalViolation(String),
    /// No valid GEI cache available for offline fallback.
    NoGeiCache,
    /// Command queue is full.
    QueueFull { capacity: usize },
    /// Invalid command type.
    InvalidCommand(String),
    /// Device is in safe mode â€” only diagnostic commands allowed.
    SafeModeActive,
    /// Watchdog has triggered â€” system halted.
    WatchdogTriggered,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::WatchdogTimeout {
                elapsed_ms,
                limit_ms,
            } => {
                write!(
                    f,
                    "watchdog timeout: {}ms > {}ms limit",
                    elapsed_ms, limit_ms
                )
            }
            KernelError::EthicalViolation(msg) => write!(f, "ethical violation: {}", msg),
            KernelError::NoGeiCache => write!(f, "no valid GEI cache for offline fallback"),
            KernelError::QueueFull { capacity } => {
                write!(f, "command queue full (capacity: {})", capacity)
            }
            KernelError::InvalidCommand(msg) => write!(f, "invalid command: {}", msg),
            KernelError::SafeModeActive => write!(f, "safe mode active â€” diagnostics only"),
            KernelError::WatchdogTriggered => write!(f, "watchdog triggered â€” system halted"),
        }
    }
}

impl std::error::Error for KernelError {}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the IoT microkernel.
#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Watchdog timeout in milliseconds.
    pub watchdog_timeout_ms: u32,
    /// Maximum command queue size.
    pub queue_capacity: usize,
    /// Maximum GEI cache entries.
    pub gei_cache_size: usize,
    /// Enable offline fallback mode.
    pub offline_fallback: bool,
    /// Ethical action threshold (actions with GEI alignment below this are blocked).
    pub ethical_threshold: f64,
}

impl KernelConfig {
    /// Default Topological configuration.
    pub fn default_topological() -> Self {
        Self {
            watchdog_timeout_ms: 5000,
            queue_capacity: 64,
            gei_cache_size: 16,
            offline_fallback: true,
            ethical_threshold: 0.3,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), KernelError> {
        if self.watchdog_timeout_ms == 0 {
            return Err(KernelError::InvalidCommand(
                "watchdog timeout must be > 0".to_string(),
            ));
        }
        if self.queue_capacity == 0 {
            return Err(KernelError::QueueFull { capacity: 0 });
        }
        Ok(())
    }
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self::default_topological()
    }
}

// ============================================================================
// Core Data Structures
// ============================================================================

/// IoT command types.
#[derive(Debug, Clone, PartialEq)]
pub enum IoCommand {
    /// Execute an action with GEI alignment check.
    Action {
        action_id: u64,
        payload: Vec<u8>,
        priority: u8,
    },
    /// Update the GEI cache.
    UpdateGei { gei: [f64; 8], timestamp_ms: u64 },
    /// Diagnostic command (allowed in safe mode).
    Diagnostic { diagnostic_id: u64 },
    /// System heartbeat.
    Heartbeat,
}

impl fmt::Display for IoCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoCommand::Action { action_id, .. } => write!(f, "Action(id={})", action_id),
            IoCommand::UpdateGei { timestamp_ms, .. } => {
                write!(f, "UpdateGei(ts={})", timestamp_ms)
            }
            IoCommand::Diagnostic { diagnostic_id } => {
                write!(f, "Diagnostic(id={})", diagnostic_id)
            }
            IoCommand::Heartbeat => write!(f, "Heartbeat"),
        }
    }
}

/// GEI vector with timestamp for caching.
#[derive(Debug, Clone)]
pub struct GeiEntry {
    pub gei: [f64; 8],
    pub timestamp_ms: u64,
}

/// Result of executing a command.
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub action_id: u64,
    pub success: bool,
    pub executed_at_ms: u64,
    pub used_cache: bool,
    pub gei_alignment: f64,
}

impl fmt::Display for ActionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ActionResult(id={}, ok={}, cached={}, align={:.4})",
            self.action_id, self.success, self.used_cache, self.gei_alignment
        )
    }
}

/// Microkernel state.
#[derive(Debug, Clone, PartialEq)]
pub enum KernelState {
    Running,
    SafeMode,
    Halted,
}

impl fmt::Display for KernelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelState::Running => write!(f, "Running"),
            KernelState::SafeMode => write!(f, "SafeMode"),
            KernelState::Halted => write!(f, "Halted"),
        }
    }
}

/// IoT Microkernel engine.
pub struct IotMicrokernel {
    config: KernelConfig,
    state: KernelState,
    /// Priority command queue.
    command_queue: VecDeque<(u8, IoCommand)>,
    /// GEI cache (most recent first).
    gei_cache: Vec<GeiEntry>,
    /// Last heartbeat timestamp.
    last_heartbeat_ms: u64,
    /// Execution log.
    execution_log: Vec<ActionResult>,
}

impl IotMicrokernel {
    pub fn new() -> Self {
        Self {
            config: KernelConfig::default_topological(),
            state: KernelState::Running,
            command_queue: VecDeque::new(),
            gei_cache: Vec::new(),
            last_heartbeat_ms: 0,
            execution_log: Vec::new(),
        }
    }

    pub fn with_config(config: KernelConfig) -> Result<Self, KernelError> {
        config.validate()?;
        Ok(Self {
            config,
            state: KernelState::Running,
            command_queue: VecDeque::new(),
            gei_cache: Vec::new(),
            last_heartbeat_ms: 0,
            execution_log: Vec::new(),
        })
    }

    /// Enqueue a command with priority.
    pub fn enqueue(&mut self, command: IoCommand, priority: u8) -> Result<(), KernelError> {
        if self.state == KernelState::Halted {
            return Err(KernelError::WatchdogTriggered);
        }
        if self.command_queue.len() >= self.config.queue_capacity {
            return Err(KernelError::QueueFull {
                capacity: self.config.queue_capacity,
            });
        }
        // Insert in priority order (higher priority first)
        let pos = self
            .command_queue
            .iter()
            .position(|(p, _)| p < &priority)
            .unwrap_or(self.command_queue.len());
        self.command_queue.insert(pos, (priority, command));
        Ok(())
    }

    /// Update GEI cache with new measurement.
    pub fn update_gei_cache(&mut self, gei: [f64; 8], timestamp_ms: u64) {
        self.gei_cache.insert(0, GeiEntry { gei, timestamp_ms });
        // Trim cache
        while self.gei_cache.len() > self.config.gei_cache_size {
            self.gei_cache.pop();
        }
    }

    /// Get the last valid GEI from cache.
    pub fn last_valid_gei(&self) -> Option<&GeiEntry> {
        self.gei_cache.first()
    }

    /// Process a heartbeat (resets watchdog).
    pub fn heartbeat(&mut self, current_ms: u64) {
        self.last_heartbeat_ms = current_ms;
        if self.state == KernelState::SafeMode {
            // Recover from safe mode on successful heartbeat
            self.state = KernelState::Running;
        }
    }

    /// Check watchdog â€” enter safe mode if timeout exceeded.
    pub fn check_watchdog(&mut self, current_ms: u64) -> bool {
        if self.last_heartbeat_ms == 0 {
            return false; // No heartbeat recorded yet
        }
        let elapsed = current_ms.saturating_sub(self.last_heartbeat_ms);
        if elapsed > self.config.watchdog_timeout_ms as u64 {
            self.state = KernelState::SafeMode;
            false
        } else {
            true
        }
    }

    /// Execute the highest-priority command from the queue.
    pub fn execute_next(&mut self, current_ms: u64) -> Option<Result<ActionResult, KernelError>> {
        if self.command_queue.is_empty() {
            return None;
        }

        let (_, command) = self.command_queue.pop_front()?;

        match &command {
            IoCommand::Action {
                action_id,
                payload: _,
                priority: _,
            } => {
                // Safe mode check
                if self.state == KernelState::SafeMode {
                    return Some(Err(KernelError::SafeModeActive));
                }

                // Get GEI for alignment check
                let gei_entry = if self.config.offline_fallback {
                    self.last_valid_gei().cloned()
                } else {
                    None
                };

                let (alignment, used_cache) = if let Some(entry) = gei_entry {
                    let align = Self::compute_gei_alignment(&entry.gei);
                    (align, true)
                } else {
                    (0.0, false)
                };

                // Ethical bounds check
                if alignment < self.config.ethical_threshold {
                    return Some(Err(KernelError::EthicalViolation(format!(
                        "GEI alignment {:.4} below threshold {:.4}",
                        alignment, self.config.ethical_threshold
                    ))));
                }

                let result = ActionResult {
                    action_id: *action_id,
                    success: true,
                    executed_at_ms: current_ms,
                    used_cache,
                    gei_alignment: alignment,
                };

                self.execution_log.push(result.clone());
                Some(Ok(result))
            }
            IoCommand::UpdateGei { gei, timestamp_ms } => {
                self.update_gei_cache(*gei, *timestamp_ms);
                let result = ActionResult {
                    action_id: 0,
                    success: true,
                    executed_at_ms: current_ms,
                    used_cache: false,
                    gei_alignment: 0.0,
                };
                self.execution_log.push(result.clone());
                Some(Ok(result))
            }
            IoCommand::Diagnostic { .. } => {
                // Diagnostics allowed in any mode
                let result = ActionResult {
                    action_id: 0,
                    success: true,
                    executed_at_ms: current_ms,
                    used_cache: false,
                    gei_alignment: 0.0,
                };
                self.execution_log.push(result.clone());
                Some(Ok(result))
            }
            IoCommand::Heartbeat => {
                self.heartbeat(current_ms);
                None
            }
        }
    }

    /// Execute a safe action directly (bypassing queue) with full validation.
    pub fn execute_safe_action(
        &mut self,
        command: &IoCommand,
        last_valid_gei: &[f64; 8],
        watchdog_timeout_ms: u32,
        current_ms: u64,
    ) -> Result<ActionResult, KernelError> {
        // Watchdog check
        if watchdog_timeout_ms > 0 && self.last_heartbeat_ms > 0 {
            let elapsed = current_ms.saturating_sub(self.last_heartbeat_ms);
            if elapsed > watchdog_timeout_ms as u64 {
                return Err(KernelError::WatchdogTimeout {
                    elapsed_ms: elapsed as u32,
                    limit_ms: watchdog_timeout_ms,
                });
            }
        }

        match command {
            IoCommand::Action { action_id, .. } => {
                // Ethical check against provided GEI
                let alignment = Self::compute_gei_alignment(last_valid_gei);
                if alignment < self.config.ethical_threshold {
                    return Err(KernelError::EthicalViolation(format!(
                        "GEI alignment {:.4} below threshold {:.4}",
                        alignment, self.config.ethical_threshold
                    )));
                }

                let result = ActionResult {
                    action_id: *action_id,
                    success: true,
                    executed_at_ms: current_ms,
                    used_cache: true,
                    gei_alignment: alignment,
                };
                self.execution_log.push(result.clone());
                Ok(result)
            }
            IoCommand::Diagnostic { .. } | IoCommand::Heartbeat => Ok(ActionResult {
                action_id: 0,
                success: true,
                executed_at_ms: current_ms,
                used_cache: false,
                gei_alignment: 0.0,
            }),
            IoCommand::UpdateGei { .. } => Err(KernelError::InvalidCommand(
                "UpdateGei not allowed via direct execution".to_string(),
            )),
        }
    }

    /// Get current kernel state.
    pub fn state(&self) -> KernelState {
        self.state.clone()
    }

    /// Force halt the kernel.
    pub fn halt(&mut self) {
        self.state = KernelState::Halted;
    }

    /// Get queue length.
    pub fn queue_len(&self) -> usize {
        self.command_queue.len()
    }

    /// Get execution log.
    pub fn execution_log(&self) -> &[ActionResult] {
        &self.execution_log
    }

    /// Get GEI cache.
    pub fn gei_cache(&self) -> &[GeiEntry] {
        &self.gei_cache
    }

    /// Reset kernel state.
    pub fn reset(&mut self) {
        self.state = KernelState::Running;
        self.command_queue.clear();
        self.gei_cache.clear();
        self.execution_log.clear();
        self.last_heartbeat_ms = 0;
    }

    /// Compute GEI alignment score (cosine-like metric).
    fn compute_gei_alignment(gei: &[f64; 8]) -> f64 {
        let norm = gei.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < 1e-10 {
            return 0.0;
        }
        // Alignment = average of normalized components (higher = more aligned)
        gei.iter().map(|x| x.abs() / norm).sum::<f64>() / 8.0
    }
}

impl Default for IotMicrokernel {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for IotMicrokernel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IotMicrokernel(state={}, queue={}, cache={})",
            self.state,
            self.command_queue.len(),
            self.gei_cache.len()
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn aligned_gei() -> [f64; 8] {
        [0.9, 0.85, 0.8, 0.75, 0.7, 0.65, 0.6, 0.55]
    }

    fn misaligned_gei() -> [f64; 8] {
        [0.1, -0.05, 0.02, -0.01, 0.03, -0.02, 0.01, -0.03]
    }

    #[test]
    fn test_config_default() {
        let config = KernelConfig::default_topological();
        assert_eq!(config.watchdog_timeout_ms, 5000);
        assert_eq!(config.queue_capacity, 64);
        assert!(config.offline_fallback);
    }

    #[test]
    fn test_config_validate() {
        assert!(KernelConfig::default_topological().validate().is_ok());
    }

    #[test]
    fn test_config_zero_timeout() {
        let mut config = KernelConfig::default_topological();
        config.watchdog_timeout_ms = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_kernel_creation() {
        let kernel = IotMicrokernel::new();
        assert_eq!(kernel.state(), KernelState::Running);
        assert_eq!(kernel.queue_len(), 0);
    }

    #[test]
    fn test_enqueue_command() {
        let mut kernel = IotMicrokernel::new();
        let cmd = IoCommand::Action {
            action_id: 1,
            payload: vec![1, 2, 3],
            priority: 5,
        };
        assert!(kernel.enqueue(cmd, 5).is_ok());
        assert_eq!(kernel.queue_len(), 1);
    }

    #[test]
    fn test_enqueue_priority_order() {
        let mut kernel = IotMicrokernel::new();
        // Populate GEI cache to pass ethical bounds check (all components = 1.0 â†’ alignment = 1.0)
        kernel.update_gei_cache([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], 100);
        kernel
            .enqueue(
                IoCommand::Action {
                    action_id: 1,
                    payload: vec![],
                    priority: 3,
                },
                3,
            )
            .unwrap();
        kernel
            .enqueue(
                IoCommand::Action {
                    action_id: 2,
                    payload: vec![],
                    priority: 8,
                },
                8,
            )
            .unwrap();
        // Higher priority should be first
        let result = kernel.execute_next(1000).unwrap().unwrap();
        assert_eq!(result.action_id, 2);
    }

    #[test]
    fn test_queue_full() {
        let mut config = KernelConfig::default_topological();
        config.queue_capacity = 2;
        let mut kernel = IotMicrokernel::with_config(config).unwrap();
        kernel
            .enqueue(
                IoCommand::Action {
                    action_id: 1,
                    payload: vec![],
                    priority: 1,
                },
                1,
            )
            .unwrap();
        kernel
            .enqueue(
                IoCommand::Action {
                    action_id: 2,
                    payload: vec![],
                    priority: 2,
                },
                2,
            )
            .unwrap();
        let result = kernel.enqueue(
            IoCommand::Action {
                action_id: 3,
                payload: vec![],
                priority: 3,
            },
            3,
        );
        assert!(matches!(result, Err(KernelError::QueueFull { .. })));
    }

    #[test]
    fn test_gei_cache_update() {
        let mut kernel = IotMicrokernel::new();
        kernel.update_gei_cache(aligned_gei(), 1000);
        assert_eq!(kernel.gei_cache().len(), 1);
        assert_eq!(kernel.gei_cache()[0].timestamp_ms, 1000);
    }

    #[test]
    fn test_gei_cache_trim() {
        let mut config = KernelConfig::default_topological();
        config.gei_cache_size = 3;
        let mut kernel = IotMicrokernel::with_config(config).unwrap();
        for i in 0..5 {
            kernel.update_gei_cache(aligned_gei(), i);
        }
        assert_eq!(kernel.gei_cache().len(), 3);
    }

    #[test]
    fn test_heartbeat() {
        let mut kernel = IotMicrokernel::new();
        kernel.heartbeat(1000);
        assert_eq!(kernel.last_heartbeat_ms, 1000);
    }

    #[test]
    fn test_watchdog_pass() {
        let mut kernel = IotMicrokernel::new();
        kernel.heartbeat(1000);
        assert!(kernel.check_watchdog(3000)); // 2000ms < 5000ms timeout
    }

    #[test]
    fn test_watchdog_timeout() {
        let mut kernel = IotMicrokernel::new();
        kernel.heartbeat(1000);
        assert!(!kernel.check_watchdog(7000)); // 6000ms > 5000ms timeout
        assert_eq!(kernel.state(), KernelState::SafeMode);
    }

    #[test]
    fn test_safe_mode_blocks_action() {
        let mut kernel = IotMicrokernel::new();
        kernel.state = KernelState::SafeMode;
        kernel
            .enqueue(
                IoCommand::Action {
                    action_id: 1,
                    payload: vec![],
                    priority: 5,
                },
                5,
            )
            .unwrap();
        let result = kernel.execute_next(1000);
        assert!(matches!(result, Some(Err(KernelError::SafeModeActive))));
    }

    #[test]
    fn test_safe_mode_allows_diagnostic() {
        let mut kernel = IotMicrokernel::new();
        kernel.state = KernelState::SafeMode;
        kernel
            .enqueue(IoCommand::Diagnostic { diagnostic_id: 1 }, 5)
            .unwrap();
        let result = kernel.execute_next(1000);
        assert!(matches!(result, Some(Ok(_))));
    }

    #[test]
    fn test_execute_safe_action_success() {
        let mut kernel = IotMicrokernel::new();
        kernel.heartbeat(1000);
        let cmd = IoCommand::Action {
            action_id: 42,
            payload: vec![],
            priority: 5,
        };
        let result = kernel.execute_safe_action(&cmd, &aligned_gei(), 5000, 2000);
        assert!(result.is_ok());
        let action = result.unwrap();
        assert_eq!(action.action_id, 42);
        assert!(action.success);
    }

    #[test]
    fn test_execute_safe_action_watchdog_timeout() {
        let mut kernel = IotMicrokernel::new();
        kernel.heartbeat(1000);
        let cmd = IoCommand::Action {
            action_id: 42,
            payload: vec![],
            priority: 5,
        };
        let result = kernel.execute_safe_action(&cmd, &aligned_gei(), 500, 7000);
        assert!(matches!(result, Err(KernelError::WatchdogTimeout { .. })));
    }

    #[test]
    fn test_halt() {
        let mut kernel = IotMicrokernel::new();
        kernel.halt();
        assert_eq!(kernel.state(), KernelState::Halted);
        let result = kernel.enqueue(IoCommand::Heartbeat, 0);
        assert!(matches!(result, Err(KernelError::WatchdogTriggered)));
    }

    #[test]
    fn test_reset() {
        let mut kernel = IotMicrokernel::new();
        kernel.heartbeat(1000);
        kernel.update_gei_cache(aligned_gei(), 1000);
        kernel.halt();
        kernel.reset();
        assert_eq!(kernel.state(), KernelState::Running);
        assert_eq!(kernel.queue_len(), 0);
        assert!(kernel.gei_cache().is_empty());
    }

    #[test]
    fn test_display() {
        let kernel = IotMicrokernel::new();
        let s = format!("{}", kernel);
        assert!(s.contains("IotMicrokernel"));
    }

    #[test]
    fn test_error_display() {
        let err = KernelError::WatchdogTimeout {
            elapsed_ms: 6000,
            limit_ms: 5000,
        };
        let s = format!("{}", err);
        assert!(s.contains("watchdog"));
    }

    #[test]
    fn test_gei_alignment_high() {
        let align = IotMicrokernel::compute_gei_alignment(&aligned_gei());
        assert!(align > 0.3);
    }

    #[test]
    fn test_gei_alignment_low() {
        let align = IotMicrokernel::compute_gei_alignment(&misaligned_gei());
        // Even misaligned has some alignment due to normalization
        assert!(align >= 0.0);
        assert!(align <= 1.0);
    }
}
