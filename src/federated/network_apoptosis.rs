//! Network Immune System — Sprint 29
//!
//! Biological metaphor for network health management:
//! - **Healthy**: Peer score >= 0, normal operation.
//! - **Pain**: Peer score < 0, early warning signals emitted.
//! - **Apoptosis**: Peer score < -100.0, cell death — peer blocklisted and disconnected.
//!
//! # Design Directives
//!
//! - Apoptosis is irreversible within the current swarm context.
//! - Pain state emits warnings but allows continued operation.
//! - Blocklisted peers are tracked to prevent re-entry.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::economics::existential_credit::ExistentialCreditLedger;

/// Error types for Network Immune System operations.
#[derive(Debug, thiserror::Error)]
pub enum ApoptosisError {
    #[error("peer already blocklisted: {0}")]
    AlreadyBlocklisted(String),

    #[error("peer not found: {0}")]
    PeerNotFound(String),

    #[error("invalid apoptosis_threshold: {0}")]
    InvalidThreshold(String),

    #[error("swarm disconnect failed for peer: {0}")]
    SwarmDisconnectFailed(String),
}

/// Immune state of a peer in the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImmuneState {
    /// Peer is healthy (score >= 0).
    Healthy,
    /// Peer is in pain (score < 0), emitting warnings.
    Pain,
    /// Peer is undergoing apoptosis (score < -100.0), blocklisted.
    Apoptosis,
}

impl std::fmt::Display for ImmuneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImmuneState::Healthy => write!(f, "Healthy"),
            ImmuneState::Pain => write!(f, "Pain"),
            ImmuneState::Apoptosis => write!(f, "Apoptosis"),
        }
    }
}

/// Configuration for the Network Immune System.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImmuneConfig {
    /// Threshold for apoptosis (default: -100.0).
    pub apoptosis_threshold: f64,
    /// Maximum number of blocklisted peers to retain (default: 1000).
    pub max_blocklist_size: usize,
    /// Enable automatic apoptosis trigger (default: true).
    pub auto_apoptosis: bool,
}

impl Default for ImmuneConfig {
    fn default() -> Self {
        Self {
            apoptosis_threshold: -100.0,
            max_blocklist_size: 1000,
            auto_apoptosis: true,
        }
    }
}

/// Callback type for swarm disconnection.
///
/// Allows integration with libp2p Swarm or other network layers.
pub type DisconnectCallback = Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// Network Immune System — Monitors peer health and triggers apoptosis.
///
/// Uses the Existential Credit Ledger to evaluate peer states:
/// - `score >= 0` → Healthy
/// - `score < 0` → Pain (warnings emitted)
/// - `score < apoptosis_threshold` → Apoptosis (blocklist + disconnect)
pub struct NetworkImmuneSystem {
    /// Configuration parameters.
    config: ImmuneConfig,
    /// Set of blocklisted peer IDs.
    blocklist: HashSet<String>,
    /// Optional callback for swarm disconnection.
    disconnect_callback: Option<DisconnectCallback>,
    /// History of apoptosis events for observability.
    apoptosis_log: Vec<ApoptosisEvent>,
}

/// Record of an apoptosis event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApoptosisEvent {
    /// Peer ID that was blocklisted.
    pub peer_id: String,
    /// Score at time of apoptosis.
    pub score: f64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Reason for apoptosis.
    pub reason: String,
}

impl NetworkImmuneSystem {
    /// Create a new immune system with default configuration.
    pub fn new() -> Self {
        Self {
            config: ImmuneConfig::default(),
            blocklist: HashSet::new(),
            disconnect_callback: None,
            apoptosis_log: Vec::new(),
        }
    }

    /// Create a new immune system with custom configuration.
    pub fn with_config(config: ImmuneConfig) -> Result<Self, ApoptosisError> {
        if config.apoptosis_threshold > 0.0 {
            return Err(ApoptosisError::InvalidThreshold(
                "apoptosis_threshold must be <= 0".into(),
            ));
        }
        Ok(Self {
            config,
            blocklist: HashSet::new(),
            disconnect_callback: None,
            apoptosis_log: Vec::new(),
        })
    }

    /// Set the disconnect callback for swarm integration.
    pub fn set_disconnect_callback(&mut self, callback: DisconnectCallback) {
        self.disconnect_callback = Some(callback);
    }

    /// Evaluate a peer's immune state based on current CE score.
    ///
    /// # State Transitions
    ///
    /// - `score >= 0` → Healthy
    /// - `0 > score > apoptosis_threshold` → Pain
    /// - `score <= apoptosis_threshold` → Apoptosis
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer to evaluate.
    /// * `ledger` - Existential Credit Ledger for score lookup.
    ///
    /// # Returns
    ///
    /// Current immune state of the peer.
    pub fn evaluate_peer(
        &self,
        peer_id: &str,
        ledger: &ExistentialCreditLedger,
    ) -> ImmuneState {
        // Blocklisted peers are always in Apoptosis state.
        if self.blocklist.contains(peer_id) {
            return ImmuneState::Apoptosis;
        }

        let score = ledger.get_score(peer_id);

        if score <= self.config.apoptosis_threshold {
            ImmuneState::Apoptosis
        } else if score < 0.0 {
            ImmuneState::Pain
        } else {
            ImmuneState::Healthy
        }
    }

    /// Trigger apoptosis for a peer: blocklist + disconnect.
    ///
    /// This is irreversible within the current swarm context.
    /// The peer is added to the blocklist and disconnected from the swarm.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer to apoptosis.
    /// * `ledger` - Existential Credit Ledger for score recording.
    /// * `timestamp_ms` - Current timestamp in milliseconds.
    /// * `reason` - Human-readable reason for apoptosis.
    ///
    /// # Errors
    ///
    /// Returns `ApoptosisError::AlreadyBlocklisted` if peer is already blocklisted.
    pub fn trigger_apoptosis(
        &mut self,
        peer_id: &str,
        ledger: &ExistentialCreditLedger,
        timestamp_ms: u64,
        reason: &str,
    ) -> Result<(), ApoptosisError> {
        if self.blocklist.contains(peer_id) {
            return Err(ApoptosisError::AlreadyBlocklisted(peer_id.to_string()));
        }

        let score = ledger.get_score(peer_id);

        // Add to blocklist.
        self.blocklist.insert(peer_id.to_string());

        // Log the event.
        self.apoptosis_log.push(ApoptosisEvent {
            peer_id: peer_id.to_string(),
            score,
            timestamp_ms,
            reason: reason.to_string(),
        });

        // Disconnect from swarm if callback is set.
        if let Some(ref callback) = self.disconnect_callback {
            callback(peer_id).map_err(|e| {
                ApoptosisError::SwarmDisconnectFailed(format!("{}: {}", peer_id, e))
            })?;
        }

        Ok(())
    }

    /// Evaluate all peers and trigger apoptosis for those below threshold.
    ///
    /// If `auto_apoptosis` is enabled in config, peers below the threshold
    /// will be automatically blocklisted and disconnected.
    ///
    /// # Arguments
    ///
    /// * `ledger` - Existential Credit Ledger for score lookup.
    /// * `timestamp_ms` - Current timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// List of peers that underwent apoptosis during this evaluation.
    pub fn evaluate_all(
        &mut self,
        ledger: &ExistentialCreditLedger,
        timestamp_ms: u64,
    ) -> Vec<String> {
        let mut apoptosed = Vec::new();

        let peer_ids = ledger.peer_ids();
        for peer_id in peer_ids {
            let state = self.evaluate_peer(peer_id, ledger);
            if state == ImmuneState::Apoptosis && !self.blocklist.contains(peer_id) {
                if self.config.auto_apoptosis {
                    if self
                        .trigger_apoptosis(
                            peer_id,
                            ledger,
                            timestamp_ms,
                            "score below apoptosis_threshold",
                        )
                        .is_ok()
                    {
                        apoptosed.push(peer_id.to_string());
                    }
                }
            }
        }

        apoptosed
    }

    /// Check if a peer is blocklisted.
    pub fn is_blocklisted(&self, peer_id: &str) -> bool {
        self.blocklist.contains(peer_id)
    }

    /// Get the current blocklist.
    pub fn get_blocklist(&self) -> &HashSet<String> {
        &self.blocklist
    }

    /// Get the blocklist size.
    pub fn blocklist_size(&self) -> usize {
        self.blocklist.len()
    }

    /// Get the apoptosis log.
    pub fn get_apoptosis_log(&self) -> &[ApoptosisEvent] {
        &self.apoptosis_log
    }

    /// Get the configuration.
    pub fn config(&self) -> &ImmuneConfig {
        &self.config
    }

    /// Clear the blocklist (for testing or emergency reset).
    pub fn clear_blocklist(&mut self) {
        self.blocklist.clear();
    }

    /// Remove a peer from the blocklist (manual override).
    pub fn unblock_peer(&mut self, peer_id: &str) {
        self.blocklist.remove(peer_id);
    }
}

impl Default for NetworkImmuneSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for NetworkImmuneSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkImmuneSystem")
            .field("config", &self.config)
            .field("blocklist_size", &self.blocklist.len())
            .field("has_disconnect_callback", &self.disconnect_callback.is_some())
            .field("apoptosis_log_len", &self.apoptosis_log.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_ledger_healthy() -> ExistentialCreditLedger {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("healthy_peer", 50.0, 1.0).ok();
        ledger
    }

    fn setup_ledger_pain() -> ExistentialCreditLedger {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.burn_credit("pain_peer", -30.0, 1.0).ok();
        ledger
    }

    fn setup_ledger_apoptosis() -> ExistentialCreditLedger {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.burn_credit("dead_peer", -150.0, 1.0).ok();
        ledger
    }

    #[test]
    fn test_immune_system_creation() {
        let immune = NetworkImmuneSystem::new();
        assert_eq!(immune.blocklist_size(), 0);
        assert_eq!(immune.get_apoptosis_log().len(), 0);
    }

    #[test]
    fn test_immune_system_with_config() {
        let config = ImmuneConfig {
            apoptosis_threshold: -50.0,
            max_blocklist_size: 100,
            auto_apoptosis: false,
        };
        let immune = NetworkImmuneSystem::with_config(config).expect("config should be valid");
        assert_eq!(immune.config().apoptosis_threshold, -50.0);
    }

    #[test]
    fn test_invalid_config_positive_threshold() {
        let config = ImmuneConfig {
            apoptosis_threshold: 50.0,
            ..Default::default()
        };
        let result = NetworkImmuneSystem::with_config(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_healthy_peer() {
        let immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_healthy();

        let state = immune.evaluate_peer("healthy_peer", &ledger);
        assert_eq!(state, ImmuneState::Healthy);
    }

    #[test]
    fn test_evaluate_pain_peer() {
        let immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_pain();

        let state = immune.evaluate_peer("pain_peer", &ledger);
        assert_eq!(state, ImmuneState::Pain);
    }

    #[test]
    fn test_evaluate_apoptosis_peer() {
        let immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_apoptosis();

        let state = immune.evaluate_peer("dead_peer", &ledger);
        assert_eq!(state, ImmuneState::Apoptosis);
    }

    #[test]
    fn test_evaluate_unknown_peer() {
        let immune = NetworkImmuneSystem::new();
        let ledger = ExistentialCreditLedger::new();

        // Unknown peer has score 0.0 -> Healthy
        let state = immune.evaluate_peer("unknown", &ledger);
        assert_eq!(state, ImmuneState::Healthy);
    }

    #[test]
    fn test_trigger_apoptosis() {
        let mut immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_apoptosis();

        immune
            .trigger_apoptosis("dead_peer", &ledger, 1000, "test apoptosis")
            .expect("apoptosis should succeed");

        assert!(immune.is_blocklisted("dead_peer"));
        assert_eq!(immune.blocklist_size(), 1);
        assert_eq!(immune.get_apoptosis_log().len(), 1);
    }

    #[test]
    fn test_trigger_apoptosis_already_blocklisted() {
        let mut immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_apoptosis();

        immune
            .trigger_apoptosis("dead_peer", &ledger, 1000, "first")
            .ok();

        let result = immune.trigger_apoptosis("dead_peer", &ledger, 2000, "second");
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_all_auto_apoptosis() {
        let mut immune = NetworkImmuneSystem::new();
        let mut ledger = ExistentialCreditLedger::new();

        ledger.emit_credit("good", 50.0, 1.0).ok();
        ledger.burn_credit("bad", -150.0, 1.0).ok();
        ledger.burn_credit("dying", -200.0, 1.0).ok();

        let apoptosed = immune.evaluate_all(&ledger, 1000);
        assert_eq!(apoptosed.len(), 2);
        assert!(immune.is_blocklisted("bad"));
        assert!(immune.is_blocklisted("dying"));
        assert!(!immune.is_blocklisted("good"));
    }

    #[test]
    fn test_evaluate_all_no_auto_apoptosis() {
        let mut immune = NetworkImmuneSystem::with_config(ImmuneConfig {
            auto_apoptosis: false,
            ..Default::default()
        })
        .expect("config should be valid");

        let mut ledger = ExistentialCreditLedger::new();
        ledger.burn_credit("bad", -150.0, 1.0).ok();

        let apoptosed = immune.evaluate_all(&ledger, 1000);
        assert_eq!(apoptosed.len(), 0);
        assert!(!immune.is_blocklisted("bad"));
    }

    #[test]
    fn test_blocklisted_peer_always_apoptosis() {
        let mut immune = NetworkImmuneSystem::new();
        let mut ledger = ExistentialCreditLedger::new();

        // Peer starts healthy.
        ledger.emit_credit("peer1", 50.0, 1.0).ok();
        assert_eq!(
            immune.evaluate_peer("peer1", &ledger),
            ImmuneState::Healthy
        );

        // Manually blocklist.
        immune
            .trigger_apoptosis("peer1", &ledger, 1000, "manual block")
            .ok();

        // Even if score improves, still Apoptosis.
        ledger.emit_credit("peer1", 100.0, 1.0).ok();
        assert_eq!(
            immune.evaluate_peer("peer1", &ledger),
            ImmuneState::Apoptosis
        );
    }

    #[test]
    fn test_clear_blocklist() {
        let mut immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_apoptosis();

        immune
            .trigger_apoptosis("dead_peer", &ledger, 1000, "test")
            .ok();
        assert!(immune.is_blocklisted("dead_peer"));

        immune.clear_blocklist();
        assert!(!immune.is_blocklisted("dead_peer"));
    }

    #[test]
    fn test_unblock_peer() {
        let mut immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_apoptosis();

        immune
            .trigger_apoptosis("dead_peer", &ledger, 1000, "test")
            .ok();
        immune.unblock_peer("dead_peer");
        assert!(!immune.is_blocklisted("dead_peer"));
    }

    #[test]
    fn test_disconnect_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let mut immune = NetworkImmuneSystem::new();
        immune.set_disconnect_callback(Box::new(move |peer_id| {
            called_clone.store(true, Ordering::SeqCst);
            Ok(())
        }));

        let ledger = setup_ledger_apoptosis();
        immune
            .trigger_apoptosis("dead_peer", &ledger, 1000, "test")
            .expect("apoptosis should succeed");

        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_disconnect_callback_failure() {
        let mut immune = NetworkImmuneSystem::new();
        immune.set_disconnect_callback(Box::new(|peer_id| {
            Err(format!("cannot disconnect {}", peer_id))
        }));

        let ledger = setup_ledger_apoptosis();
        let result = immune.trigger_apoptosis("dead_peer", &ledger, 1000, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_apoptosis_event_log() {
        let mut immune = NetworkImmuneSystem::new();
        let ledger = setup_ledger_apoptosis();

        immune
            .trigger_apoptosis("dead_peer", &ledger, 12345, "test reason")
            .ok();

        let log = immune.get_apoptosis_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].peer_id, "dead_peer");
        assert_eq!(log[0].timestamp_ms, 12345);
        assert_eq!(log[0].reason, "test reason");
        assert!((log[0].score - (-150.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default() {
        let immune = NetworkImmuneSystem::default();
        assert_eq!(immune.blocklist_size(), 0);
    }

    #[test]
    fn test_config_default() {
        let config = ImmuneConfig::default();
        assert_eq!(config.apoptosis_threshold, -100.0);
        assert_eq!(config.max_blocklist_size, 1000);
        assert!(config.auto_apoptosis);
    }

    #[test]
    fn test_error_display() {
        let err = ApoptosisError::AlreadyBlocklisted("peer1".into());
        assert!(!format!("{}", err).is_empty());

        let err = ApoptosisError::PeerNotFound("peer1".into());
        assert!(!format!("{}", err).is_empty());

        let err = ApoptosisError::InvalidThreshold("test".into());
        assert!(!format!("{}", err).is_empty());

        let err = ApoptosisError::SwarmDisconnectFailed("test".into());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_immune_state_display() {
        assert_eq!(format!("{}", ImmuneState::Healthy), "Healthy");
        assert_eq!(format!("{}", ImmuneState::Pain), "Pain");
        assert_eq!(format!("{}", ImmuneState::Apoptosis), "Apoptosis");
    }

    #[test]
    fn test_custom_threshold() {
        let mut immune = NetworkImmuneSystem::with_config(ImmuneConfig {
            apoptosis_threshold: -50.0,
            ..Default::default()
        })
        .expect("config should be valid");

        let mut ledger = ExistentialCreditLedger::new();
        // Score -60 is below -50 threshold but above default -100.
        ledger.burn_credit("peer1", -60.0, 1.0).ok();

        assert_eq!(
            immune.evaluate_peer("peer1", &ledger),
            ImmuneState::Apoptosis
        );

        let apoptosed = immune.evaluate_all(&ledger, 1000);
        assert_eq!(apoptosed.len(), 1);
    }
}
