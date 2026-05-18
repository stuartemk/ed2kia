//! Fault Tolerance — Tolerancia a fallos para entrenamiento distribuido
//!
//! Implementa detección de fallos, recuperación automática, circuit breakers
//! y mecanismos de reintentos con backoff exponencial.

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum FaultError {
    NodeAlreadyRegistered(String),
    NodeNotRegistered(String),
    CircuitBreakerOpen(String),
    MaxRetriesExceeded(String),
    InvalidConfig(String),
    RecoveryFailed(String),
}

impl fmt::Display for FaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FaultError::NodeAlreadyRegistered(id) => {
                write!(f, "Node already registered: {}", id)
            }
            FaultError::NodeNotRegistered(id) => {
                write!(f, "Node not registered: {}", id)
            }
            FaultError::CircuitBreakerOpen(id) => {
                write!(f, "Circuit breaker open for: {}", id)
            }
            FaultError::MaxRetriesExceeded(id) => {
                write!(f, "Max retries exceeded for: {}", id)
            }
            FaultError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            FaultError::RecoveryFailed(msg) => write!(f, "Recovery failed: {}", msg),
        }
    }
}

impl std::error::Error for FaultError {}

// ============================================================================
// Circuit Breaker State
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open(Instant),
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open(_since) => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

// ============================================================================
// Failure Type
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum FailureType {
    Timeout,
    GradientCorruption,
    NetworkPartition,
    ComputeFailure,
    CheckpointFailure,
    Unknown,
}

impl fmt::Display for FailureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FailureType::Timeout => write!(f, "Timeout"),
            FailureType::GradientCorruption => write!(f, "GradientCorruption"),
            FailureType::NetworkPartition => write!(f, "NetworkPartition"),
            FailureType::ComputeFailure => write!(f, "ComputeFailure"),
            FailureType::CheckpointFailure => write!(f, "CheckpointFailure"),
            FailureType::Unknown => write!(f, "Unknown"),
        }
    }
}

// ============================================================================
// Node Health Status
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum NodeHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Recovering,
}

impl fmt::Display for NodeHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeHealth::Healthy => write!(f, "Healthy"),
            NodeHealth::Degraded => write!(f, "Degraded"),
            NodeHealth::Unhealthy => write!(f, "Unhealthy"),
            NodeHealth::Recovering => write!(f, "Recovering"),
        }
    }
}

// ============================================================================
// Circuit Breaker
// ============================================================================

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub reset_timeout: Duration,
    pub last_failure: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold: 3,
            reset_timeout,
            last_failure: None,
        }
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.failure_count = 0;

        if let CircuitState::HalfOpen = self.state {
            if self.success_count >= self.success_threshold {
                self.state = CircuitState::Closed;
                self.success_count = 0;
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.success_count = 0;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open(Instant::now());
        }
    }

    pub fn allow_request(&mut self) -> bool {
        match &self.state {
            CircuitState::Closed => true,
            CircuitState::Open(since) => {
                if since.elapsed() > self.reset_timeout {
                    self.state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn is_open(&self) -> bool {
        matches!(self.state, CircuitState::Open(_))
    }
}

// ============================================================================
// Retry Policy
// ============================================================================

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
    pub jitter_enabled: bool,
}

impl RetryPolicy {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_enabled: true,
        }
    }

    pub fn get_backoff(&self, attempt: u32) -> Duration {
        let mut backoff = self.initial_backoff.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);

        if self.jitter_enabled {
            // Simple jitter: add random variation (simulated)
            backoff *= 0.8 + (attempt as f64 % 10.0) / 10.0 * 0.4;
        }

        let backoff_ms = backoff as u64;
        let capped = backoff_ms.min(self.max_backoff.as_millis() as u64);
        Duration::from_millis(capped)
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3)
    }
}

// ============================================================================
// Node Fault Record
// ============================================================================

#[derive(Debug, Clone)]
pub struct NodeFaultRecord {
    pub node_id: String,
    pub health: NodeHealth,
    pub circuit_breaker: CircuitBreaker,
    pub failure_history: Vec<FailureEntry>,
    pub consecutive_failures: u32,
    pub last_heartbeat: Instant,
    pub recovery_attempts: u32,
    pub is_isolated: bool,
}

impl NodeFaultRecord {
    pub fn new(node_id: String, failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            node_id,
            health: NodeHealth::Healthy,
            circuit_breaker: CircuitBreaker::new(failure_threshold, reset_timeout),
            failure_history: Vec::new(),
            consecutive_failures: 0,
            last_heartbeat: Instant::now(),
            recovery_attempts: 0,
            is_isolated: false,
        }
    }

    pub fn record_failure(&mut self, failure_type: FailureType) {
        self.circuit_breaker.record_failure();
        self.consecutive_failures += 1;

        self.failure_history.push(FailureEntry {
            failure_type,
            timestamp: Instant::now(),
            attempt: self.consecutive_failures,
        });

        self.update_health();
    }

    pub fn record_success(&mut self) {
        self.circuit_breaker.record_success();
        self.consecutive_failures = 0;
        self.update_health();
    }

    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_heartbeat.elapsed() > timeout
    }

    pub fn attempt_recovery(&mut self) -> bool {
        if self.is_isolated {
            return false;
        }

        self.recovery_attempts += 1;
        self.health = NodeHealth::Recovering;
        true
    }

    fn update_health(&mut self) {
        if self.circuit_breaker.is_open() {
            self.health = NodeHealth::Unhealthy;
            if self.consecutive_failures >= 5 {
                self.is_isolated = true;
            }
        } else if self.consecutive_failures > 0 {
            self.health = NodeHealth::Degraded;
        } else {
            self.health = NodeHealth::Healthy;
        }
    }
}

// ============================================================================
// Failure Entry
// ============================================================================

#[derive(Debug, Clone)]
pub struct FailureEntry {
    pub failure_type: FailureType,
    pub timestamp: Instant,
    pub attempt: u32,
}

// ============================================================================
// Recovery Strategy
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    Restart,
    RejoinFromCheckpoint,
    ResyncGradients,
    IsolateAndReplace,
}

impl fmt::Display for RecoveryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryStrategy::Restart => write!(f, "Restart"),
            RecoveryStrategy::RejoinFromCheckpoint => write!(f, "RejoinFromCheckpoint"),
            RecoveryStrategy::ResyncGradients => write!(f, "ResyncGradients"),
            RecoveryStrategy::IsolateAndReplace => write!(f, "IsolateAndReplace"),
        }
    }
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct FaultToleranceConfig {
    pub failure_threshold: u32,
    pub circuit_reset_timeout: Duration,
    pub heartbeat_timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub auto_recovery_enabled: bool,
    pub max_recovery_attempts: u32,
    pub isolation_threshold: u32,
}

impl FaultToleranceConfig {
    pub fn new(failure_threshold: u32) -> Self {
        Self {
            failure_threshold,
            circuit_reset_timeout: Duration::from_secs(60),
            heartbeat_timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            auto_recovery_enabled: true,
            max_recovery_attempts: 3,
            isolation_threshold: 5,
        }
    }

    pub fn validate(&self) -> Result<(), FaultError> {
        if self.failure_threshold == 0 {
            return Err(FaultError::InvalidConfig(
                "Failure threshold must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self::new(3)
    }
}

// ============================================================================
// Fault Tolerance Manager
// ============================================================================

pub struct FaultToleranceManager {
    config: FaultToleranceConfig,
    nodes: HashMap<String, NodeFaultRecord>,
    recovery_log: Vec<RecoveryEntry>,
}

impl FaultToleranceManager {
    pub fn new(config: FaultToleranceConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            recovery_log: Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // Node Registration
    // ------------------------------------------------------------------

    pub fn register_node(&mut self, node_id: String) -> Result<(), FaultError> {
        if self.nodes.contains_key(&node_id) {
            return Err(FaultError::NodeAlreadyRegistered(node_id));
        }
        self.nodes.insert(
            node_id.clone(),
            NodeFaultRecord::new(
                node_id,
                self.config.failure_threshold,
                self.config.circuit_reset_timeout,
            ),
        );
        Ok(())
    }

    pub fn unregister_node(&mut self, node_id: &str) -> Result<(), FaultError> {
        if !self.nodes.contains_key(node_id) {
            return Err(FaultError::NodeNotRegistered(node_id.to_string()));
        }
        self.nodes.remove(node_id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Failure Tracking
    // ------------------------------------------------------------------

    pub fn record_node_failure(
        &mut self,
        node_id: &str,
        failure_type: FailureType,
    ) -> Result<(), FaultError> {
        let record = self
            .nodes
            .get_mut(node_id)
            .ok_or(FaultError::NodeNotRegistered(node_id.to_string()))?;

        record.record_failure(failure_type);
        Ok(())
    }

    pub fn record_node_success(&mut self, node_id: &str) -> Result<(), FaultError> {
        let record = self
            .nodes
            .get_mut(node_id)
            .ok_or(FaultError::NodeNotRegistered(node_id.to_string()))?;

        record.record_success();
        Ok(())
    }

    pub fn node_heartbeat(&mut self, node_id: &str) -> Result<(), FaultError> {
        let record = self
            .nodes
            .get_mut(node_id)
            .ok_or(FaultError::NodeNotRegistered(node_id.to_string()))?;

        record.heartbeat();
        Ok(())
    }

    // ------------------------------------------------------------------
    // Circuit Breaker
    // ------------------------------------------------------------------

    pub fn can_send_to_node(&mut self, node_id: &str) -> Result<bool, FaultError> {
        let record = self
            .nodes
            .get_mut(node_id)
            .ok_or(FaultError::NodeNotRegistered(node_id.to_string()))?;

        if record.is_isolated {
            return Ok(false);
        }

        Ok(record.circuit_breaker.allow_request())
    }

    pub fn get_node_health(&self, node_id: &str) -> Result<NodeHealth, FaultError> {
        let record = self
            .nodes
            .get(node_id)
            .ok_or(FaultError::NodeNotRegistered(node_id.to_string()))?;

        Ok(record.health.clone())
    }

    // ------------------------------------------------------------------
    // Recovery
    // ------------------------------------------------------------------

    pub fn detect_and_recover(&mut self) -> Vec<RecoveryEntry> {
        // First pass: collect nodes needing recovery
        let candidates: Vec<(String, RecoveryStrategy)> = self
            .nodes
            .values()
            .filter(|record| {
                !record.is_isolated
                    && (matches!(
                        record.health,
                        NodeHealth::Unhealthy | NodeHealth::Degraded
                    ) || record.is_stale(self.config.heartbeat_timeout))
            })
            .filter_map(|record| {
                let strategy = self.select_recovery_strategy(record)?;
                if record.recovery_attempts < self.config.max_recovery_attempts {
                    Some((record.node_id.clone(), strategy))
                } else {
                    None
                }
            })
            .collect();

        // Second pass: apply recovery
        let mut recoveries = Vec::new();
        for (node_id, strategy) in candidates {
            if let Some(record) = self.nodes.get_mut(&node_id) {
                if record.attempt_recovery() {
                    recoveries.push(RecoveryEntry {
                        node_id,
                        strategy,
                        timestamp: Instant::now(),
                        attempt: record.recovery_attempts,
                    });
                }
            }
        }

        self.recovery_log.extend(recoveries.clone());
        recoveries
    }

    pub fn select_recovery_strategy(&self, record: &NodeFaultRecord) -> Option<RecoveryStrategy> {
        if record.is_isolated || record.recovery_attempts >= self.config.max_recovery_attempts {
            return Some(RecoveryStrategy::IsolateAndReplace);
        }

        match record.health {
            NodeHealth::Unhealthy => {
                if record.consecutive_failures >= self.config.isolation_threshold {
                    Some(RecoveryStrategy::IsolateAndReplace)
                } else {
                    Some(RecoveryStrategy::Restart)
                }
            }
            NodeHealth::Degraded => Some(RecoveryStrategy::ResyncGradients),
            NodeHealth::Recovering => Some(RecoveryStrategy::RejoinFromCheckpoint),
            _ => None,
        }
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    pub fn get_unhealthy_nodes(&self) -> Vec<String> {
        self.nodes
            .values()
            .filter(|r| matches!(r.health, NodeHealth::Unhealthy))
            .map(|r| r.node_id.clone())
            .collect()
    }

    pub fn get_isolated_nodes(&self) -> Vec<String> {
        self.nodes
            .values()
            .filter(|r| r.is_isolated)
            .map(|r| r.node_id.clone())
            .collect()
    }

    pub fn get_stale_nodes(&self) -> Vec<String> {
        self.nodes
            .values()
            .filter(|r| r.is_stale(self.config.heartbeat_timeout))
            .map(|r| r.node_id.clone())
            .collect()
    }

    pub fn get_healthy_node_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|r| matches!(r.health, NodeHealth::Healthy))
            .count()
    }

    pub fn get_recovery_log(&self) -> &[RecoveryEntry] {
        &self.recovery_log
    }

    pub fn get_node_record(&self, node_id: &str) -> Option<&NodeFaultRecord> {
        self.nodes.get(node_id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for FaultToleranceManager {
    fn default() -> Self {
        Self::new(FaultToleranceConfig::default())
    }
}

// ============================================================================
// Recovery Entry
// ============================================================================

#[derive(Debug, Clone)]
pub struct RecoveryEntry {
    pub node_id: String,
    pub strategy: RecoveryStrategy,
    pub timestamp: Instant,
    pub attempt: u32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = FaultToleranceManager::default();
        assert_eq!(manager.node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut manager = FaultToleranceManager::default();
        assert!(manager.register_node("node-1".to_string()).is_ok());
        assert_eq!(manager.node_count(), 1);
    }

    #[test]
    fn test_register_duplicate_node() {
        let mut manager = FaultToleranceManager::default();
        manager.register_node("node-1".to_string()).unwrap();
        match manager.register_node("node-1".to_string()) {
            Err(FaultError::NodeAlreadyRegistered(id)) => assert_eq!(id, "node-1"),
            _ => panic!("Expected NodeAlreadyRegistered"),
        }
    }

    #[test]
    fn test_unregister_node() {
        let mut manager = FaultToleranceManager::default();
        manager.register_node("node-1".to_string()).unwrap();
        assert!(manager.unregister_node("node-1").is_ok());
        assert_eq!(manager.node_count(), 0);
    }

    #[test]
    fn test_record_success() {
        let mut manager = FaultToleranceManager::default();
        manager.register_node("node-1".to_string()).unwrap();
        assert!(manager.record_node_success("node-1").is_ok());

        let health = manager.get_node_health("node-1").unwrap();
        assert_eq!(health, NodeHealth::Healthy);
    }

    #[test]
    fn test_record_failure_degrades_health() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 3;
        manager.register_node("node-1".to_string()).unwrap();

        manager
            .record_node_failure("node-1", FailureType::Timeout)
            .unwrap();

        let health = manager.get_node_health("node-1").unwrap();
        assert_eq!(health, NodeHealth::Degraded);
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 3;
        manager.register_node("node-1".to_string()).unwrap();

        for _ in 0..3 {
            manager
                .record_node_failure("node-1", FailureType::Timeout)
                .unwrap();
        }

        let health = manager.get_node_health("node-1").unwrap();
        assert_eq!(health, NodeHealth::Unhealthy);
    }

    #[test]
    fn test_circuit_breaker_blocks_requests() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 2;
        manager.config.circuit_reset_timeout = Duration::from_secs(10);
        manager.register_node("node-1".to_string()).unwrap();

        manager
            .record_node_failure("node-1", FailureType::Timeout)
            .unwrap();
        manager
            .record_node_failure("node-1", FailureType::Timeout)
            .unwrap();

        let allowed = manager.can_send_to_node("node-1").unwrap();
        assert!(!allowed);
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 2;
        manager.config.circuit_reset_timeout = Duration::from_millis(100);
        manager.register_node("node-1".to_string()).unwrap();

        manager
            .record_node_failure("node-1", FailureType::Timeout)
            .unwrap();
        manager
            .record_node_failure("node-1", FailureType::Timeout)
            .unwrap();

        std::thread::sleep(Duration::from_millis(150));

        let allowed = manager.can_send_to_node("node-1").unwrap();
        assert!(allowed); // Half-open allows requests
    }

    #[test]
    fn test_node_isolation() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 1;
        manager.config.isolation_threshold = 5;
        manager.register_node("node-1".to_string()).unwrap();

        for _ in 0..5 {
            manager
                .record_node_failure("node-1", FailureType::Timeout)
                .unwrap();
        }

        let isolated = manager.get_isolated_nodes();
        assert_eq!(isolated, vec!["node-1"]);
    }

    #[test]
    fn test_heartbeat() {
        let mut manager = FaultToleranceManager::default();
        manager.register_node("node-1".to_string()).unwrap();
        assert!(manager.node_heartbeat("node-1").is_ok());
    }

    #[test]
    fn test_stale_node_detection() {
        let mut manager = FaultToleranceManager::default();
        manager.config.heartbeat_timeout = Duration::from_millis(100);
        manager.register_node("node-1".to_string()).unwrap();

        std::thread::sleep(Duration::from_millis(150));

        let stale = manager.get_stale_nodes();
        assert_eq!(stale, vec!["node-1"]);
    }

    #[test]
    fn test_auto_recovery() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 2;
        manager.config.auto_recovery_enabled = true;
        manager.register_node("node-1".to_string()).unwrap();

        manager
            .record_node_failure("node-1", FailureType::Timeout)
            .unwrap();

        let recoveries = manager.detect_and_recover();
        assert!(!recoveries.is_empty());
        assert_eq!(recoveries[0].node_id, "node-1");
    }

    #[test]
    fn test_recovery_strategy_selection() {
        let manager = FaultToleranceManager::default();
        let mut record = NodeFaultRecord::new(
            "test".to_string(),
            manager.config.failure_threshold,
            manager.config.circuit_reset_timeout,
        );

        assert!(manager.select_recovery_strategy(&record).is_none());

        record.record_failure(FailureType::Timeout);
        assert_eq!(
            manager.select_recovery_strategy(&record),
            Some(RecoveryStrategy::ResyncGradients)
        );
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::new(3);
        assert!(policy.should_retry(0));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
    }

    #[test]
    fn test_retry_backoff_increases() {
        let policy = RetryPolicy::new(5);
        let backoff_0 = policy.get_backoff(0);
        let backoff_1 = policy.get_backoff(1);
        assert!(backoff_1.as_millis() >= backoff_0.as_millis());
    }

    #[test]
    fn test_retry_backoff_capped() {
        let policy = RetryPolicy::new(10);
        let backoff_100 = policy.get_backoff(100);
        assert!(backoff_100 <= policy.max_backoff);
    }

    #[test]
    fn test_circuit_breaker_creation() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        assert_eq!(cb.state, CircuitState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_transitions() {
        let mut cb = CircuitBreaker::new(2, Duration::from_millis(100));

        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Closed);

        cb.record_failure();
        assert!(matches!(cb.state, CircuitState::Open(_))); // State is Open with some timestamp

        std::thread::sleep(Duration::from_millis(150));
        cb.allow_request();
        assert_eq!(cb.state, CircuitState::HalfOpen);

        cb.record_success();
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_get_unhealthy_nodes() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 1;
        manager.register_node("n1".to_string()).unwrap();
        manager.register_node("n2".to_string()).unwrap();

        manager.record_node_failure("n1", FailureType::Timeout).unwrap();

        let unhealthy = manager.get_unhealthy_nodes();
        assert_eq!(unhealthy, vec!["n1"]);
    }

    #[test]
    fn test_healthy_node_count() {
        let mut manager = FaultToleranceManager::default();
        manager.register_node("n1".to_string()).unwrap();
        manager.register_node("n2".to_string()).unwrap();

        assert_eq!(manager.get_healthy_node_count(), 2);
    }

    #[test]
    fn test_config_validation() {
        let mut config = FaultToleranceConfig::new(1);
        assert!(config.validate().is_ok());

        config.failure_threshold = 0;
        match config.validate() {
            Err(FaultError::InvalidConfig(msg)) => {
                assert!(msg.contains("Failure threshold"));
            }
            _ => panic!("Expected InvalidConfig"),
        }
    }

    #[test]
    fn test_health_display() {
        assert_eq!(format!("{}", NodeHealth::Healthy), "Healthy");
        assert_eq!(format!("{}", NodeHealth::Degraded), "Degraded");
        assert_eq!(format!("{}", NodeHealth::Unhealthy), "Unhealthy");
        assert_eq!(format!("{}", NodeHealth::Recovering), "Recovering");
    }

    #[test]
    fn test_failure_type_display() {
        assert_eq!(
            format!("{}", FailureType::Timeout),
            "Timeout"
        );
        assert_eq!(
            format!("{}", FailureType::GradientCorruption),
            "GradientCorruption"
        );
    }

    #[test]
    fn test_recovery_strategy_display() {
        assert_eq!(
            format!("{}", RecoveryStrategy::Restart),
            "Restart"
        );
        assert_eq!(
            format!("{}", RecoveryStrategy::IsolateAndReplace),
            "IsolateAndReplace"
        );
    }

    #[test]
    fn test_circuit_state_display() {
        assert_eq!(format!("{}", CircuitState::Closed), "Closed");
        assert_eq!(
            format!("{}", CircuitState::HalfOpen),
            "HalfOpen"
        );
    }

    #[test]
    fn test_error_display() {
        match FaultError::NodeNotRegistered("x".into()) {
            e => assert!(format!("{}", e).contains("x")),
            _ => {}
        }
    }

    #[test]
    fn test_recovery_log() {
        let mut manager = FaultToleranceManager::default();
        manager.config.failure_threshold = 2;
        manager.config.auto_recovery_enabled = true;
        manager.register_node("n1".to_string()).unwrap();

        manager.record_node_failure("n1", FailureType::Timeout).unwrap();
        manager.detect_and_recover();

        assert!(!manager.get_recovery_log().is_empty());
    }

    #[test]
    fn test_node_fault_record_recovery() {
        let mut record = NodeFaultRecord::new("test".to_string(), 3, Duration::from_secs(60));
        assert!(record.attempt_recovery());
        assert_eq!(record.health, NodeHealth::Recovering);
        assert_eq!(record.recovery_attempts, 1);
    }

    #[test]
    fn test_isolated_node_cannot_recover() {
        let mut record = NodeFaultRecord::new("test".to_string(), 1, Duration::from_secs(60));
        record.is_isolated = true;
        assert!(!record.attempt_recovery());
    }

    #[test]
    fn test_config_default() {
        let config = FaultToleranceConfig::default();
        assert_eq!(config.failure_threshold, 3);
    }

    #[test]
    fn test_manager_default() {
        let manager = FaultToleranceManager::default();
        assert_eq!(manager.node_count(), 0);
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
    }
}
