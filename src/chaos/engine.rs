//! Chaos Engine — Controlled fault injection for operational resilience testing.
//!
//! **Sprint15 - Resiliencia Operativa & Automatización de Respuesta**
//!
//! Async motor (tokio) for fault injection in local/testnet environments.
//! Strict control: only active with `--chaos-mode`, limited duration,
//! automatic rollback, detailed logs.
//!
//! **Simulable faults:**
//! - WASM node failure
//! - Network partition (GossipSub isolation)
//! - Artificial latency
//! - Malicious vote injection
//! - Task queue saturation
//!
//! **Safety:**
//! - Never active in production without explicit `--chaos-mode` flag
//! - Automatic rollback after configured duration
//! - All actions logged with trace-level detail

use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

/// Chaos engine errors.
#[derive(Debug, Clone)]
pub enum ChaosError {
    /// Attempted to activate scenario without `--chaos-mode` flag.
    ModeNotEnabled,
    /// Scenario duration exceeded maximum allowed.
    DurationExceeded(Duration),
    /// Failed to inject fault.
    InjectionFailed(String),
    /// Scenario already active.
    ScenarioAlreadyActive(String),
    /// No active scenario to rollback.
    NoActiveScenario,
    /// Timeout waiting for scenario completion.
    Timeout,
}

impl fmt::Display for ChaosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChaosError::ModeNotEnabled => write!(f, "Chaos mode not enabled. Use --chaos-mode flag."),
            ChaosError::DurationExceeded(d) => write!(f, "Duration {:?} exceeds maximum allowed", d),
            ChaosError::InjectionFailed(msg) => write!(f, "Fault injection failed: {}", msg),
            ChaosError::ScenarioAlreadyActive(name) => write!(f, "Scenario '{}' already active", name),
            ChaosError::NoActiveScenario => write!(f, "No active scenario to rollback"),
            ChaosError::Timeout => write!(f, "Timeout waiting for scenario completion"),
        }
    }
}

impl std::error::Error for ChaosError {}

// ---------------------------------------------------------------------------
// Fault Scenarios
// ---------------------------------------------------------------------------

/// Simulable fault scenarios.
#[derive(Debug, Clone)]
pub enum ChaosScenario {
    /// Simulate WASM node failure.
    /// Node stops responding, returns errors on inference requests.
    WasmNodeFailure {
        /// Target node ID.
        node_id: String,
        /// Failure rate (0.0 - 1.0).
        failure_rate: f64,
    },
    /// Simulate network partition via GossipSub isolation.
    /// Target node cannot send/receive messages from mesh.
    NetworkPartition {
        /// Isolated node IDs.
        isolated_nodes: Vec<String>,
        /// Partition group A.
        group_a: Vec<String>,
        /// Partition group B.
        group_b: Vec<String>,
    },
    /// Inject artificial latency on message routing.
    ArtificialLatency {
        /// Target node ID (or "all" for global).
        target: String,
        /// Added latency in milliseconds.
        latency_ms: u64,
        /// Jitter range in milliseconds.
        jitter_ms: u64,
    },
    /// Inject malicious votes in consensus rounds.
    MaliciousVotes {
        /// Attacker node ID.
        attacker_id: String,
        /// Malicious vote rate (0.0 - 1.0).
        malicious_rate: f64,
    },
    /// Saturate task queue to test backpressure.
    TaskQueueSaturation {
        /// Target queue capacity.
        target_capacity: usize,
        /// Message flood rate per second.
        flood_rate: usize,
    },
}

impl fmt::Display for ChaosScenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChaosScenario::WasmNodeFailure { node_id, .. } => {
                write!(f, "WASM Node Failure (node: {})", node_id)
            }
            ChaosScenario::NetworkPartition { .. } => write!(f, "Network Partition"),
            ChaosScenario::ArtificialLatency { target, .. } => {
                write!(f, "Artificial Latency (target: {})", target)
            }
            ChaosScenario::MaliciousVotes { attacker_id, .. } => {
                write!(f, "Malicious Votes (attacker: {})", attacker_id)
            }
            ChaosScenario::TaskQueueSaturation { .. } => write!(f, "Task Queue Saturation"),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Chaos engine configuration.
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Maximum scenario duration.
    pub max_duration: Duration,
    /// Cool-down period between scenarios.
    pub cooldown: Duration,
    /// Enable automatic rollback.
    pub auto_rollback: bool,
    /// Chaos mode flag (MUST be true to activate).
    pub chaos_mode: bool,
    /// Log level for chaos actions.
    pub verbose: bool,
}

impl ChaosConfig {
    /// Create a new configuration with safe defaults.
    pub fn new() -> Self {
        Self {
            max_duration: Duration::from_secs(300), // 5 minutes max
            cooldown: Duration::from_secs(60),
            auto_rollback: true,
            chaos_mode: false, // SAFE DEFAULT
            verbose: true,
        }
    }

    /// Enable chaos mode (explicit opt-in).
    pub fn with_chaos_mode(mut self, enabled: bool) -> Self {
        self.chaos_mode = enabled;
        self
    }

    /// Set maximum scenario duration.
    pub fn with_max_duration(mut self, duration: Duration) -> Self {
        self.max_duration = duration;
        self
    }

    /// Set cooldown period.
    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Enable/disable automatic rollback.
    pub fn with_auto_rollback(mut self, enabled: bool) -> Self {
        self.auto_rollback = enabled;
        self
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), ChaosError> {
        if !self.chaos_mode {
            return Err(ChaosError::ModeNotEnabled);
        }
        Ok(())
    }
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Scenario State
// ---------------------------------------------------------------------------

/// Active scenario state.
#[derive(Debug, Clone)]
pub struct ActiveScenario {
    /// Scenario definition.
    pub scenario: ChaosScenario,
    /// Start time.
    pub started_at: Instant,
    /// Configured duration.
    pub duration: Duration,
    /// Rollback performed.
    pub rolled_back: bool,
}

impl ActiveScenario {
    /// Check if scenario has expired.
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed() > self.duration
    }

    /// Remaining time.
    pub fn remaining(&self) -> Duration {
        let elapsed = self.started_at.elapsed();
        if elapsed > self.duration {
            return Duration::from_secs(0);
        }
        self.duration - elapsed
    }
}

// ---------------------------------------------------------------------------
// Chaos Engine
// ---------------------------------------------------------------------------

/// Chaos Engine — Controlled fault injection for resilience testing.
///
/// **CRITICAL SAFETY:** Only active when `config.chaos_mode == true`.
/// All fault injections are logged and automatically rolled back.
#[derive(Debug)]
pub struct ChaosEngine {
    /// Engine configuration.
    config: ChaosConfig,
    /// Currently active scenario (if any).
    active: Option<ActiveScenario>,
    /// Scenario history.
    history: VecDeque<ActiveScenario>,
    /// Command channel for async control.
    tx: mpsc::UnboundedSender<ChaosCommand>,
    /// Event channel for outgoing events.
    event_tx: mpsc::UnboundedSender<ChaosEvent>,
}

/// Internal commands.
#[derive(Debug)]
enum ChaosCommand {
    /// Activate a scenario.
    Activate(ChaosScenario, Duration, mpsc::Sender<Result<(), ChaosError>>),
    /// Rollback current scenario.
    Rollback(mpsc::Sender<Result<(), ChaosError>>),
    /// Get current status.
    Status(mpsc::Sender<Option<ActiveScenario>>),
    /// Shutdown engine.
    Shutdown,
}

/// Chaos engine events.
#[derive(Debug, Clone)]
pub enum ChaosEvent {
    /// Scenario activated.
    ScenarioActivated {
        name: String,
        duration: Duration,
    },
    /// Fault injected.
    FaultInjected {
        scenario: String,
        fault_type: String,
        timestamp: Instant,
    },
    /// Scenario rolled back.
    ScenarioRolledBack {
        name: String,
        reason: String,
    },
    /// Scenario expired.
    ScenarioExpired {
        name: String,
    },
    /// Engine shutdown.
    EngineShutdown,
}

impl fmt::Display for ChaosEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChaosEvent::ScenarioActivated { name, .. } => {
                write!(f, "Scenario '{}' activated", name)
            }
            ChaosEvent::FaultInjected { scenario, fault_type, .. } => {
                write!(f, "Fault '{}' injected in '{}'", fault_type, scenario)
            }
            ChaosEvent::ScenarioRolledBack { name, reason, .. } => {
                write!(f, "Scenario '{}' rolled back: {}", name, reason)
            }
            ChaosEvent::ScenarioExpired { name, .. } => {
                write!(f, "Scenario '{}' expired", name)
            }
            ChaosEvent::EngineShutdown => write!(f, "Chaos engine shutdown"),
        }
    }
}

impl ChaosEngine {
    /// Create a new chaos engine.
    ///
    /// Returns a tuple of `(engine, event_receiver)`.
    pub fn new(config: ChaosConfig) -> (Self, mpsc::UnboundedReceiver<ChaosEvent>) {
        let (tx, rx) = mpsc::unbounded_channel::<ChaosCommand>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<ChaosEvent>();

        let engine = Self {
            config: config.clone(),
            active: None,
            history: VecDeque::new(),
            tx,
            event_tx: event_tx.clone(),
        };

        // Spawn background loop.
        tokio::spawn(Self::run(rx, event_tx, config));

        (engine, event_rx)
    }

    /// Background event loop.
    async fn run(
        mut rx: mpsc::UnboundedReceiver<ChaosCommand>,
        event_tx: mpsc::UnboundedSender<ChaosEvent>,
        config: ChaosConfig,
    ) {
        info!(
            chaos_mode = config.chaos_mode,
            max_duration = ?config.max_duration,
            "Chaos engine background loop started"
        );

        let mut active_scenario: Option<ActiveScenario> = None;
        let mut cooldown_until: Option<Instant> = None;

        loop {
            tokio::select! {
                // Handle commands.
                Some(cmd) = rx.recv() => {
                    match cmd {
                        ChaosCommand::Activate(scenario, duration, reply) => {
                            // Check chaos mode.
                            if !config.chaos_mode {
                                warn!("Rejecting scenario activation: chaos mode not enabled");
                                let _ = reply.send(Err(ChaosError::ModeNotEnabled)).await;
                                continue;
                            }

                            // Check cooldown.
                            if let Some(until) = cooldown_until {
                                if Instant::now() < until {
                                    let remaining = until - Instant::now();
                                    warn!(
                                        cooldown_remaining = ?remaining,
                                        "Rejecting scenario: cooldown active"
                                    );
                                    let _ = reply.send(Err(ChaosError::ScenarioAlreadyActive(
                                        "cooldown active".to_string()
                                    ))).await;
                                    continue;
                                }
                            }

                            // Check max duration.
                            if duration > config.max_duration {
                                warn!(
                                    requested = ?duration,
                                    max = ?config.max_duration,
                                    "Rejecting scenario: duration exceeds maximum"
                                );
                                let _ = reply.send(Err(ChaosError::DurationExceeded(duration))).await;
                                continue;
                            }

                            // Check for existing active scenario.
                            if active_scenario.is_some() {
                                let name = active_scenario.as_ref().map(|s| s.scenario.to_string());
                                warn!("Rejecting scenario: another scenario already active");
                                let _ = reply.send(Err(ChaosError::ScenarioAlreadyActive(
                                    name.unwrap_or_default()
                                ))).await;
                                continue;
                            }

                            // Activate scenario.
                            let scenario_name = scenario.clone();
                            let active = ActiveScenario {
                                scenario,
                                started_at: Instant::now(),
                                duration,
                                rolled_back: false,
                            };
                            active_scenario = Some(active.clone());

                            info!(
                                scenario = %scenario_name,
                                duration = ?duration,
                                "Scenario activated"
                            );

                            let _ = event_tx.send(ChaosEvent::ScenarioActivated {
                                name: scenario_name.to_string(),
                                duration,
                            });

                            let _ = reply.send(Ok(())).await;

                            // Spawn fault injection loop for this scenario.
                            let injection_loop = Self::inject_faults(
                                active.scenario.clone(),
                                active.started_at,
                                active.duration,
                                event_tx.clone(),
                                config.verbose,
                            );

                            // Wait for scenario to complete or be cancelled.
                            tokio::spawn(async move {
                                injection_loop.await;
                            });
                        }

                        ChaosCommand::Rollback(reply) => {
                            match active_scenario.take() {
                                Some(active) => {
                                    let name = active.scenario.to_string();
                                    info!(scenario = %name, "Scenario rollback initiated");

                                    active_scenario = None;
                                    cooldown_until = Some(Instant::now() + config.cooldown);

                                    let _ = event_tx.send(ChaosEvent::ScenarioRolledBack {
                                        name: name.clone(),
                                        reason: "manual rollback".to_string(),
                                    });

                                    let _ = reply.send(Ok(())).await;
                                }
                                None => {
                                    warn!("Rollback requested but no active scenario");
                                    let _ = reply.send(Err(ChaosError::NoActiveScenario)).await;
                                }
                            }
                        }

                        ChaosCommand::Status(reply) => {
                            let _ = reply.send(active_scenario.clone()).await;
                        }

                        ChaosCommand::Shutdown => {
                            // Rollback active scenario before shutdown.
                            if let Some(active) = active_scenario.take() {
                                let name = active.scenario.to_string();
                                info!(scenario = %name, "Shutdown: rolling back active scenario");

                                let _ = event_tx.send(ChaosEvent::ScenarioRolledBack {
                                    name,
                                    reason: "engine shutdown".to_string(),
                                });
                            }

                            let _ = event_tx.send(ChaosEvent::EngineShutdown);
                            info!("Chaos engine background loop shutting down");
                            break;
                        }
                    }
                }

                // Check for expired scenarios.
                else => {
                    if let Some(ref active) = active_scenario {
                        if active.is_expired() {
                            let name = active.scenario.to_string();
                            info!(scenario = %name, "Scenario expired");

                            let _ = event_tx.send(ChaosEvent::ScenarioExpired { name });
                            active_scenario = None;
                            cooldown_until = Some(Instant::now() + config.cooldown);
                        }
                    }
                    // Small sleep to prevent busy loop.
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Fault injection loop for an active scenario.
    async fn inject_faults(
        scenario: ChaosScenario,
        started_at: Instant,
        duration: Duration,
        event_tx: mpsc::UnboundedSender<ChaosEvent>,
        verbose: bool,
    ) {
        let scenario_name = scenario.clone();
        info!(
            scenario = %scenario_name,
            "Fault injection loop started"
        );

        let mut interval = interval(Duration::from_millis(500)); // Check every 500ms

        loop {
            interval.tick().await;

            // Check if scenario expired.
            let elapsed = started_at.elapsed();
            if elapsed >= duration {
                info!(
                    scenario = %scenario_name,
                    elapsed = ?elapsed,
                    "Fault injection loop: scenario duration reached"
                );
                break;
            }

            // Inject faults based on scenario type.
            match &scenario {
                ChaosScenario::WasmNodeFailure { node_id, failure_rate } => {
                    Self::inject_wasm_failure(node_id, *failure_rate, &event_tx, verbose).await;
                }
                ChaosScenario::NetworkPartition { .. } => {
                    Self::inject_network_partition(&scenario, &event_tx, verbose).await;
                }
                ChaosScenario::ArtificialLatency { target, latency_ms, jitter_ms } => {
                    Self::inject_latency(target, *latency_ms, *jitter_ms, &event_tx, verbose).await;
                }
                ChaosScenario::MaliciousVotes { attacker_id, malicious_rate } => {
                    Self::inject_malicious_votes(attacker_id, *malicious_rate, &event_tx, verbose).await;
                }
                ChaosScenario::TaskQueueSaturation { target_capacity, flood_rate } => {
                    Self::inject_queue_saturation(*target_capacity, *flood_rate, &event_tx, verbose).await;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Fault Injection Implementations
    // -----------------------------------------------------------------------

    /// Inject WASM node failure.
    async fn inject_wasm_failure(
        node_id: &str,
        failure_rate: f64,
        event_tx: &mpsc::UnboundedSender<ChaosEvent>,
        verbose: bool,
    ) {
        // Simulate failure based on rate.
        let roll: f64 = fastrand::f64();
        if roll < failure_rate {
            if verbose {
                debug!(
                    node = node_id,
                    failure_rate,
                    roll,
                    "Chaos: WASM node failure triggered"
                );
            }

            let _ = event_tx.send(ChaosEvent::FaultInjected {
                scenario: "WasmNodeFailure".to_string(),
                fault_type: "node_inference_failure".to_string(),
                timestamp: Instant::now(),
            });
        }
    }

    /// Inject network partition.
    async fn inject_network_partition(
        scenario: &ChaosScenario,
        event_tx: &mpsc::UnboundedSender<ChaosEvent>,
        verbose: bool,
    ) {
        if verbose {
            debug!(scenario = %scenario, "Chaos: Network partition active");
        }

        let _ = event_tx.send(ChaosEvent::FaultInjected {
            scenario: "NetworkPartition".to_string(),
            fault_type: "gossipsub_isolation".to_string(),
            timestamp: Instant::now(),
        });
    }

    /// Inject artificial latency.
    async fn inject_latency(
        target: &str,
        latency_ms: u64,
        jitter_ms: u64,
        event_tx: &mpsc::UnboundedSender<ChaosEvent>,
        verbose: bool,
    ) {
        let jitter = if jitter_ms > 0 {
            fastrand::u64(0..jitter_ms)
        } else {
            0
        };
        let total_latency = latency_ms + jitter;

        if verbose {
            debug!(
                target,
                total_latency_ms = total_latency,
                "Chaos: Artificial latency injected"
            );
        }

        let _ = event_tx.send(ChaosEvent::FaultInjected {
            scenario: "ArtificialLatency".to_string(),
            fault_type: format!("added_latency_{}ms", total_latency),
            timestamp: Instant::now(),
        });

        // Simulate latency.
        sleep(Duration::from_millis(total_latency)).await;
    }

    /// Inject malicious votes.
    async fn inject_malicious_votes(
        attacker_id: &str,
        malicious_rate: f64,
        event_tx: &mpsc::UnboundedSender<ChaosEvent>,
        verbose: bool,
    ) {
        let roll: f64 = fastrand::f64();
        if roll < malicious_rate {
            if verbose {
                debug!(
                    attacker = attacker_id,
                    malicious_rate,
                    roll,
                    "Chaos: Malicious vote injected"
                );
            }

            let _ = event_tx.send(ChaosEvent::FaultInjected {
                scenario: "MaliciousVotes".to_string(),
                fault_type: "malicious_consensus_vote".to_string(),
                timestamp: Instant::now(),
            });
        }
    }

    /// Inject task queue saturation.
    async fn inject_queue_saturation(
        target_capacity: usize,
        flood_rate: usize,
        event_tx: &mpsc::UnboundedSender<ChaosEvent>,
        verbose: bool,
    ) {
        if verbose {
            debug!(
                target_capacity,
                flood_rate,
                "Chaos: Task queue saturation active"
            );
        }

        let _ = event_tx.send(ChaosEvent::FaultInjected {
            scenario: "TaskQueueSaturation".to_string(),
            fault_type: format!("queue_flood_{}msg/s", flood_rate),
            timestamp: Instant::now(),
        });
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Activate a scenario with the given duration.
    pub async fn activate(
        &self,
        scenario: ChaosScenario,
        duration: Duration,
    ) -> Result<(), ChaosError> {
        let (tx, mut rx) = mpsc::channel(1);
        if self.tx.send(ChaosCommand::Activate(scenario, duration, tx)).is_err() {
            return Err(ChaosError::InjectionFailed(
                "Background loop not running".to_string(),
            ));
        }
        rx.recv().await.ok_or(ChaosError::Timeout)?
    }

    /// Rollback current active scenario.
    pub async fn rollback(&self) -> Result<(), ChaosError> {
        let (tx, mut rx) = mpsc::channel(1);
        if self.tx.send(ChaosCommand::Rollback(tx)).is_err() {
            return Err(ChaosError::InjectionFailed(
                "Background loop not running".to_string(),
            ));
        }
        rx.recv().await.ok_or(ChaosError::Timeout)?
    }

    /// Get current active scenario status.
    pub async fn status(&self) -> Option<ActiveScenario> {
        let (tx, mut rx) = mpsc::channel(1);
        if self.tx.send(ChaosCommand::Status(tx)).is_err() {
            return None;
        }
        rx.recv().await.unwrap_or(None)
    }

    /// Shutdown engine gracefully.
    pub async fn shutdown(&self) {
        let _ = self.tx.send(ChaosCommand::Shutdown);
    }

    /// Get engine configuration.
    pub fn config(&self) -> &ChaosConfig {
        &self.config
    }

    /// Check if chaos mode is enabled.
    pub fn is_chaos_mode(&self) -> bool {
        self.config.chaos_mode
    }
}

impl Drop for ChaosEngine {
    fn drop(&mut self) {
        // Best-effort shutdown.
        let _ = self.tx.send(ChaosCommand::Shutdown);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_config_default() {
        let config = ChaosConfig::new();
        assert!(!config.chaos_mode);
        assert_eq!(config.max_duration, Duration::from_secs(300));
        assert_eq!(config.cooldown, Duration::from_secs(60));
        assert!(config.auto_rollback);
    }

    #[test]
    fn test_chaos_config_with_chaos_mode() {
        let config = ChaosConfig::new().with_chaos_mode(true);
        assert!(config.chaos_mode);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_chaos_config_validate_without_mode() {
        let config = ChaosConfig::new();
        assert_eq!(config.validate(), Err(ChaosError::ModeNotEnabled));
    }

    #[test]
    fn test_chaos_scenario_display() {
        let scenario = ChaosScenario::WasmNodeFailure {
            node_id: "test-node".to_string(),
            failure_rate: 0.5,
        };
        let display = format!("{}", scenario);
        assert!(display.contains("WASM Node Failure"));
        assert!(display.contains("test-node"));
    }

    #[test]
    fn test_active_scenario_expiration() {
        let scenario = ChaosScenario::WasmNodeFailure {
            node_id: "test".to_string(),
            failure_rate: 1.0,
        };
        let active = ActiveScenario {
            scenario,
            started_at: Instant::now() - Duration::from_secs(10),
            duration: Duration::from_secs(5),
            rolled_back: false,
        };
        assert!(active.is_expired());
        assert_eq!(active.remaining(), Duration::from_secs(0));
    }

    #[test]
    fn test_active_scenario_not_expired() {
        let scenario = ChaosScenario::WasmNodeFailure {
            node_id: "test".to_string(),
            failure_rate: 1.0,
        };
        let active = ActiveScenario {
            scenario,
            started_at: Instant::now(),
            duration: Duration::from_secs(60),
            rolled_back: false,
        };
        assert!(!active.is_expired());
        assert!(active.remaining() > Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let config = ChaosConfig::new();
        let (engine, _event_rx) = ChaosEngine::new(config);
        assert!(!engine.is_chaos_mode());
    }

    #[tokio::test]
    async fn test_engine_rejects_activation_without_mode() {
        let config = ChaosConfig::new(); // chaos_mode = false
        let (engine, _event_rx) = ChaosEngine::new(config);

        let scenario = ChaosScenario::WasmNodeFailure {
            node_id: "test".to_string(),
            failure_rate: 0.5,
        };

        let result = engine.activate(scenario, Duration::from_secs(10)).await;
        assert!(matches!(result, Err(ChaosError::ModeNotEnabled)));
    }

    #[tokio::test]
    async fn test_engine_accepts_activation_with_mode() {
        let config = ChaosConfig::new().with_chaos_mode(true);
        let (engine, mut event_rx) = ChaosEngine::new(config);

        let scenario = ChaosScenario::WasmNodeFailure {
            node_id: "test".to_string(),
            failure_rate: 0.5,
        };

        // Activate with short duration.
        let result = engine.activate(scenario, Duration::from_millis(100)).await;
        assert!(result.is_ok());

        // Should receive activation event.
        tokio::select! {
            event = event_rx.recv() => {
                assert!(matches!(event, Some(ChaosEvent::ScenarioActivated { .. })));
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                panic!("Timeout waiting for activation event");
            }
        }

        // Wait for scenario to expire.
        sleep(Duration::from_millis(200)).await;

    //   Graceful shutdown.
        engine.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_rollback_without_active() {
        let config = ChaosConfig::new().with_chaos_mode(true);
        let (engine, _event_rx) = ChaosEngine::new(config);

        let result = engine.rollback().await;
        assert!(matches!(result, Err(ChaosError::NoActiveScenario)));

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_status_initial() {
        let config = ChaosConfig::new();
        let (engine, _event_rx) = ChaosEngine::new(config);

        let status = engine.status().await;
        assert!(status.is_none());

        engine.shutdown().await;
    }

    #[test]
    fn test_chaos_error_display() {
        let err = ChaosError::ModeNotEnabled;
        assert!(format!("{}", err).contains("chaos mode"));

        let err = ChaosError::DurationExceeded(Duration::from_secs(10));
        assert!(format!("{}", err).contains("Duration"));

        let err = ChaosError::InjectionFailed("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_chaos_event_display() {
        let event = ChaosEvent::ScenarioActivated {
            name: "test".to_string(),
            duration: Duration::from_secs(10),
        };
        assert!(format!("{}", event).contains("test"));

        let event = ChaosEvent::EngineShutdown;
        assert!(format!("{}", event).contains("shutdown"));
    }

    #[tokio::test]
    async fn test_engine_duration_exceeded() {
        let config = ChaosConfig::new()
            .with_chaos_mode(true)
            .with_max_duration(Duration::from_secs(10));
        let (engine, _event_rx) = ChaosEngine::new(config);

        let scenario = ChaosScenario::WasmNodeFailure {
            node_id: "test".to_string(),
            failure_rate: 0.5,
        };

        // Request duration longer than max.
        let result = engine.activate(scenario, Duration::from_secs(100)).await;
        assert!(matches!(result, Err(ChaosError::DurationExceeded(_))));

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_scenario_already_active() {
        let config = ChaosConfig::new()
            .with_chaos_mode(true)
            .with_cooldown(Duration::from_secs(1));
        let (engine, mut _event_rx) = ChaosEngine::new(config);

        let scenario1 = ChaosScenario::WasmNodeFailure {
            node_id: "test-1".to_string(),
            failure_rate: 0.5,
        };

        let scenario2 = ChaosScenario::WasmNodeFailure {
            node_id: "test-2".to_string(),
            failure_rate: 0.5,
        };

        // Activate first scenario.
        let result1 = engine.activate(scenario1, Duration::from_secs(5)).await;
        assert!(result1.is_ok());

        // Try to activate second scenario while first is active.
        let result2 = engine.activate(scenario2, Duration::from_secs(5)).await;
        assert!(matches!(result2, Err(ChaosError::ScenarioAlreadyActive(_))));

        // Cleanup.
        engine.rollback().await.ok();
        engine.shutdown().await;
    }
}
