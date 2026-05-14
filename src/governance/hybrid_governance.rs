//! Hybrid Governance v5 — Hybrid execution engine with off-chain validation and on-chain registration.
//!
//! Provides hybrid execution patterns where technical validation occurs off-chain
//! and registration happens on-chain for transparency and auditability.

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::HashMap;

    // ---------------------------------------------------------------------
    // Public types
    // ---------------------------------------------------------------------

    /// Error types for hybrid governance operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum HybridGovernanceError {
        /// Proposal not found in the system.
        ProposalNotFound,
        /// Session not found for the given ID.
        SessionNotFound,
        /// Validation failed due to technical constraints.
        ValidationFailed(String),
        /// Registration failed on-chain.
        RegistrationFailed(String),
        /// Time-lock not yet expired.
        TimeLockActive,
        /// Quorum not reached for execution.
        QuorumNotReached,
        /// Approval threshold not met.
        ApprovalThresholdNotMet,
        /// Session already exists.
        SessionExists,
        /// Maximum sessions reached.
        MaxSessionsReached,
        /// Invalid execution state transition.
        InvalidStateTransition,
    }

    impl std::fmt::Display for HybridGovernanceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                HybridGovernanceError::ProposalNotFound => write!(f, "Proposal not found"),
                HybridGovernanceError::SessionNotFound => write!(f, "Session not found"),
                HybridGovernanceError::ValidationFailed(msg) => {
                    write!(f, "Validation failed: {}", msg)
                }
                HybridGovernanceError::RegistrationFailed(msg) => {
                    write!(f, "Registration failed: {}", msg)
                }
                HybridGovernanceError::TimeLockActive => write!(f, "Time-lock still active"),
                HybridGovernanceError::QuorumNotReached => write!(f, "Quorum not reached"),
                HybridGovernanceError::ApprovalThresholdNotMet => {
                    write!(f, "Approval threshold not met")
                }
                HybridGovernanceError::SessionExists => write!(f, "Session already exists"),
                HybridGovernanceError::MaxSessionsReached => write!(f, "Max sessions reached"),
                HybridGovernanceError::InvalidStateTransition => {
                    write!(f, "Invalid state transition")
                }
            }
        }
    }

    /// Configuration for the hybrid governance engine.
    #[derive(Debug, Clone)]
    pub struct HybridGovernanceConfig {
        /// Maximum concurrent execution sessions.
        pub max_sessions: usize,
        /// Minimum quorum participation ratio (0.0 - 1.0).
        pub quorum_threshold: f64,
        /// Minimum approval ratio (0.0 - 1.0).
        pub approval_threshold: f64,
        /// Default time-lock duration in hours.
        pub default_timelock_hours: u64,
        /// Extended time-lock for critical proposals in hours.
        pub critical_timelock_hours: u64,
        /// Enable emergency bypass for time-lock.
        pub emergency_bypass: bool,
        /// Enable on-chain registration simulation.
        pub on_chain_registration: bool,
        /// Enable off-chain validation.
        pub off_chain_validation: bool,
    }

    impl Default for HybridGovernanceConfig {
        fn default() -> Self {
            Self {
                max_sessions: 100,
                quorum_threshold: 0.30,
                approval_threshold: 0.51,
                default_timelock_hours: 24,
                critical_timelock_hours: 72,
                emergency_bypass: false,
                on_chain_registration: true,
                off_chain_validation: true,
            }
        }
    }

    /// Current state of hybrid execution for a proposal.
    #[derive(Debug, Clone, PartialEq)]
    pub struct HybridExecutionState {
        /// Associated proposal ID.
        pub proposal_id: String,
        /// Off-chain technical validation completed.
        pub off_chain_validated: bool,
        /// On-chain light registration completed.
        pub on_chain_registered: bool,
        /// Cryptographic hash of the execution.
        pub execution_hash: String,
        /// Timestamp when validation completed (ms).
        pub validated_at_ms: u64,
        /// Timestamp when registration completed (ms).
        pub registered_at_ms: Option<u64>,
        /// Timestamp when execution completed (ms).
        pub executed_at_ms: Option<u64>,
        /// Time-lock expiry timestamp (ms).
        pub timelock_until_ms: Option<u64>,
        /// Session status.
        pub status: SessionStatus,
    }

    /// Session status for hybrid execution.
    #[derive(Debug, Clone, PartialEq)]
    pub enum SessionStatus {
        /// Session created, awaiting validation.
        Created,
        /// Off-chain validation in progress.
        Validating,
        /// Off-chain validation completed.
        Validated,
        /// Time-lock period active.
        TimeLocked,
        /// On-chain registration in progress.
        Registering,
        /// On-chain registration completed.
        Registered,
        /// Execution completed successfully.
        Executed,
        /// Session failed.
        Failed(String),
    }

    impl std::fmt::Display for SessionStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SessionStatus::Created => write!(f, "Created"),
                SessionStatus::Validating => write!(f, "Validating"),
                SessionStatus::Validated => write!(f, "Validated"),
                SessionStatus::TimeLocked => write!(f, "TimeLocked"),
                SessionStatus::Registering => write!(f, "Registering"),
                SessionStatus::Registered => write!(f, "Registered"),
                SessionStatus::Executed => write!(f, "Executed"),
                SessionStatus::Failed(msg) => write!(f, "Failed: {}", msg),
            }
        }
    }

    /// Metrics for hybrid governance operations.
    #[derive(Debug, Clone)]
    pub struct HybridGovernanceMetrics {
        /// Total sessions created.
        pub total_sessions: usize,
        /// Currently active sessions.
        pub active_sessions: usize,
        /// Successfully executed sessions.
        pub executed_sessions: usize,
        /// Failed sessions.
        pub failed_sessions: usize,
        /// Average validation time in milliseconds.
        pub avg_validation_time_ms: f64,
        /// Average registration time in milliseconds.
        pub avg_registration_time_ms: f64,
        /// Total validation time accumulated.
        pub total_validation_time_ms: u64,
        /// Total registration time accumulated.
        pub total_registration_time_ms: u64,
        /// Sessions using emergency bypass.
        pub emergency_bypass_count: usize,
    }

    impl Default for HybridGovernanceMetrics {
        fn default() -> Self {
            Self {
                total_sessions: 0,
                active_sessions: 0,
                executed_sessions: 0,
                failed_sessions: 0,
                avg_validation_time_ms: 0.0,
                avg_registration_time_ms: 0.0,
                total_validation_time_ms: 0,
                total_registration_time_ms: 0,
                emergency_bypass_count: 0,
            }
        }
    }

    impl HybridGovernanceMetrics {
        /// Record a completed validation step.
        pub fn record_validation(&mut self, time_ms: u64) {
            self.total_validation_time_ms += time_ms;
            let validations = self.executed_sessions.max(1);
            self.avg_validation_time_ms =
                (self.total_validation_time_ms as f64) / (validations as f64);
        }

        /// Record a completed registration step.
        pub fn record_registration(&mut self, time_ms: u64) {
            self.total_registration_time_ms += time_ms;
            let registrations = self.executed_sessions.max(1);
            self.avg_registration_time_ms =
                (self.total_registration_time_ms as f64) / (registrations as f64);
        }

        /// Record a successful execution.
        pub fn record_execution(&mut self) {
            self.executed_sessions += 1;
            self.active_sessions = self.active_sessions.saturating_sub(1);
        }

        /// Record a failed session.
        pub fn record_failure(&mut self) {
            self.failed_sessions += 1;
            self.active_sessions = self.active_sessions.saturating_sub(1);
        }

        /// Record a new session creation.
        pub fn record_session_creation(&mut self) {
            self.total_sessions += 1;
            self.active_sessions += 1;
        }
    }

    /// Hybrid Governance Engine — Coordinates off-chain validation and on-chain registration.
    pub struct HybridGovernance {
        config: HybridGovernanceConfig,
        sessions: HashMap<String, HybridExecutionState>,
        metrics: HybridGovernanceMetrics,
    }

    impl HybridGovernance {
        /// Create a new hybrid governance engine with the given configuration.
        pub fn new(config: HybridGovernanceConfig) -> Self {
            Self {
                config,
                sessions: HashMap::new(),
                metrics: HybridGovernanceMetrics::default(),
            }
        }

        /// Create a new execution session for a proposal.
        pub fn create_session(
            &mut self,
            session_id: String,
            proposal_id: String,
            is_critical: bool,
            current_time_ms: u64,
        ) -> Result<(), HybridGovernanceError> {
            if self.sessions.contains_key(&session_id) {
                return Err(HybridGovernanceError::SessionExists);
            }
            if self.sessions.len() >= self.config.max_sessions {
                return Err(HybridGovernanceError::MaxSessionsReached);
            }

            let timelock_hours = if is_critical {
                self.config.critical_timelock_hours
            } else {
                self.config.default_timelock_hours
            };

            let timelock_until_ms = if timelock_hours > 0 {
                Some(current_time_ms + timelock_hours * 3600 * 1000)
            } else {
                None
            };

            let state = HybridExecutionState {
                proposal_id: proposal_id.clone(),
                off_chain_validated: false,
                on_chain_registered: false,
                execution_hash: compute_execution_hash(&session_id, &proposal_id),
                validated_at_ms: 0,
                registered_at_ms: None,
                executed_at_ms: None,
                timelock_until_ms,
                status: SessionStatus::Created,
            };

            self.sessions.insert(session_id, state);
            self.metrics.record_session_creation();
            Ok(())
        }

        /// Execute off-chain technical validation.
        pub fn validate_off_chain(
            &mut self,
            session_id: &str,
            current_time_ms: u64,
        ) -> Result<(), HybridGovernanceError> {
            if !self.config.off_chain_validation {
                return Err(HybridGovernanceError::ValidationFailed(
                    "Off-chain validation disabled".to_string(),
                ));
            }

            let state = self
                .sessions
                .get_mut(session_id)
                .ok_or(HybridGovernanceError::SessionNotFound)?;

            if state.status != SessionStatus::Created && state.status != SessionStatus::Validating
            {
                return Err(HybridGovernanceError::InvalidStateTransition);
            }

            state.status = SessionStatus::Validating;
            // Simulate validation work
            state.status = SessionStatus::Validated;
            state.off_chain_validated = true;
            state.validated_at_ms = current_time_ms;

            self.metrics
                .record_validation(current_time_ms.saturating_sub(state.validated_at_ms));

            Ok(())
        }

        /// Check if time-lock has expired for a session.
        pub fn check_timelock(
            &self,
            session_id: &str,
            current_time_ms: u64,
        ) -> Result<bool, HybridGovernanceError> {
            let state = self
                .sessions
                .get(session_id)
                .ok_or(HybridGovernanceError::SessionNotFound)?;

            match state.timelock_until_ms {
                Some(until_ms) => {
                    if current_time_ms < until_ms {
                        return Ok(false); // Time-lock still active
                    }
                    Ok(true)
                }
                None => Ok(true), // No time-lock
            }
        }

        /// Bypass time-lock using emergency authority.
        pub fn bypass_timelock(
            &mut self,
            session_id: &str,
        ) -> Result<(), HybridGovernanceError> {
            if !self.config.emergency_bypass {
                return Err(HybridGovernanceError::TimeLockActive);
            }

            let state = self
                .sessions
                .get_mut(session_id)
                .ok_or(HybridGovernanceError::SessionNotFound)?;

            state.timelock_until_ms = None;
            self.metrics.emergency_bypass_count += 1;

            Ok(())
        }

        /// Execute on-chain light registration.
        pub fn register_on_chain(
            &mut self,
            session_id: &str,
            current_time_ms: u64,
        ) -> Result<(), HybridGovernanceError> {
            if !self.config.on_chain_registration {
                return Err(HybridGovernanceError::RegistrationFailed(
                    "On-chain registration disabled".to_string(),
                ));
            }

            let state = self
                .sessions
                .get_mut(session_id)
                .ok_or(HybridGovernanceError::SessionNotFound)?;

            if !state.off_chain_validated {
                return Err(HybridGovernanceError::InvalidStateTransition);
            }

            // Check time-lock
            if let Some(until_ms) = state.timelock_until_ms {
                if current_time_ms < until_ms {
                    return Err(HybridGovernanceError::TimeLockActive);
                }
            }

            state.status = SessionStatus::Registering;
            // Simulate registration
            state.status = SessionStatus::Registered;
            state.on_chain_registered = true;
            state.registered_at_ms = Some(current_time_ms);

            self.metrics
                .record_registration(current_time_ms.saturating_sub(state.validated_at_ms));

            Ok(())
        }

        /// Execute the proposal after all checks pass.
        pub fn execute(
            &mut self,
            session_id: &str,
            current_time_ms: u64,
        ) -> Result<(), HybridGovernanceError> {
            let state = self
                .sessions
                .get_mut(session_id)
                .ok_or(HybridGovernanceError::SessionNotFound)?;

            if !state.off_chain_validated || !state.on_chain_registered {
                return Err(HybridGovernanceError::InvalidStateTransition);
            }

            state.status = SessionStatus::Executed;
            state.executed_at_ms = Some(current_time_ms);

            self.metrics.record_execution();

            Ok(())
        }

        /// Fail a session with a reason.
        pub fn fail_session(
            &mut self,
            session_id: &str,
            reason: String,
        ) -> Result<(), HybridGovernanceError> {
            let state = self
                .sessions
                .get_mut(session_id)
                .ok_or(HybridGovernanceError::SessionNotFound)?;

            state.status = SessionStatus::Failed(reason);
            self.metrics.record_failure();

            Ok(())
        }

        /// Get the current state of a session.
        pub fn get_session(&self, session_id: &str) -> Option<&HybridExecutionState> {
            self.sessions.get(session_id)
        }

        /// Get current metrics.
        pub fn metrics(&self) -> &HybridGovernanceMetrics {
            &self.metrics
        }

        /// Reset metrics to default.
        pub fn reset_metrics(&mut self) {
            self.metrics = HybridGovernanceMetrics::default();
        }

        /// Remove a completed session.
        pub fn remove_session(&mut self, session_id: &str) -> Result<(), HybridGovernanceError> {
            self.sessions
                .remove(session_id)
                .ok_or(HybridGovernanceError::SessionNotFound)?;
            Ok(())
        }

        /// Count active sessions.
        pub fn active_session_count(&self) -> usize {
            self.sessions
                .values()
                .filter(|s| {
                    !matches!(
                        s.status,
                        SessionStatus::Executed | SessionStatus::Failed(_)
                    )
                })
                .count()
        }
    }

    impl Default for HybridGovernance {
        fn default() -> Self {
            Self::new(HybridGovernanceConfig::default())
        }
    }

    // ---------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------

    fn compute_execution_hash(session_id: &str, proposal_id: &str) -> String {
        let data = format!("{}:{}", session_id, proposal_id);
        compute_sha256(&data)
    }

    fn compute_sha256(data: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    // ---------------------------------------------------------------------
    // Unit tests
    // ---------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn current_time_ms() -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }

        #[test]
        fn test_engine_creation() {
            let engine = HybridGovernance::default();
            assert_eq!(engine.active_session_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = HybridGovernanceConfig {
                max_sessions: 50,
                quorum_threshold: 0.40,
                ..Default::default()
            };
            let engine = HybridGovernance::new(config);
            assert_eq!(engine.active_session_count(), 0);
        }

        #[test]
        fn test_create_session() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            assert!(engine
                .create_session("s1".to_string(), "p1".to_string(), false, time)
                .is_ok());
            assert_eq!(engine.active_session_count(), 1);
        }

        #[test]
        fn test_create_session_duplicate() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            assert!(engine
                .create_session("s1".to_string(), "p1".to_string(), false, time)
                .is_ok());
            assert_eq!(
                engine.create_session("s1".to_string(), "p2".to_string(), false, time),
                Err(HybridGovernanceError::SessionExists)
            );
        }

        #[test]
        fn test_create_session_max_reached() {
            let config = HybridGovernanceConfig {
                max_sessions: 2,
                ..Default::default()
            };
            let mut engine = HybridGovernance::new(config);
            let time = current_time_ms();
            assert!(engine
                .create_session("s1".to_string(), "p1".to_string(), false, time)
                .is_ok());
            assert!(engine
                .create_session("s2".to_string(), "p2".to_string(), false, time)
                .is_ok());
            assert_eq!(
                engine.create_session("s3".to_string(), "p3".to_string(), false, time),
                Err(HybridGovernanceError::MaxSessionsReached)
            );
        }

        #[test]
        fn test_validate_off_chain() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            assert!(engine.validate_off_chain("s1", time + 100).is_ok());
            let state = engine.get_session("s1").unwrap();
            assert!(state.off_chain_validated);
            assert_eq!(state.status, SessionStatus::Validated);
        }

        #[test]
        fn test_validate_off_chain_disabled() {
            let config = HybridGovernanceConfig {
                off_chain_validation: false,
                ..Default::default()
            };
            let mut engine = HybridGovernance::new(config);
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            let result = engine.validate_off_chain("s1", time + 100);
            assert!(matches!(result, Err(HybridGovernanceError::ValidationFailed(_))));
        }

        #[test]
        fn test_timelock_check() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            // Time-lock should be active
            assert!(!engine.check_timelock("s1", time + 1000).unwrap());
            // After 24h should be expired
            let after_timelock = time + 25 * 3600 * 1000;
            assert!(engine.check_timelock("s1", after_timelock).unwrap());
        }

        #[test]
        fn test_critical_timelock() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine
                .create_session("s1".to_string(), "p1".to_string(), true, time)
                .unwrap();
            // 72h time-lock for critical
            let after_48h = time + 48 * 3600 * 1000;
            assert!(!engine.check_timelock("s1", after_48h).unwrap());
            let after_72h = time + 73 * 3600 * 1000;
            assert!(engine.check_timelock("s1", after_72h).unwrap());
        }

        #[test]
        fn test_bypass_timelock() {
            let config = HybridGovernanceConfig {
                emergency_bypass: true,
                ..Default::default()
            };
            let mut engine = HybridGovernance::new(config);
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            assert!(engine.bypass_timelock("s1").is_ok());
            assert!(engine.check_timelock("s1", time + 1000).unwrap());
        }

        #[test]
        fn test_bypass_timelock_disabled() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            assert_eq!(
                engine.bypass_timelock("s1"),
                Err(HybridGovernanceError::TimeLockActive)
            );
        }

        #[test]
        fn test_register_on_chain() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            engine.validate_off_chain("s1", time + 100).unwrap();
            // Wait for time-lock to expire
            let after_timelock = time + 25 * 3600 * 1000;
            assert!(engine.register_on_chain("s1", after_timelock).is_ok());
            let state = engine.get_session("s1").unwrap();
            assert!(state.on_chain_registered);
        }

        #[test]
        fn test_register_on_chain_timelock_active() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            engine.validate_off_chain("s1", time + 100).unwrap();
            assert_eq!(
                engine.register_on_chain("s1", time + 1000),
                Err(HybridGovernanceError::TimeLockActive)
            );
        }

        #[test]
        fn test_register_on_chain_not_validated() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            let after_timelock = time + 25 * 3600 * 1000;
            assert_eq!(
                engine.register_on_chain("s1", after_timelock),
                Err(HybridGovernanceError::InvalidStateTransition)
            );
        }

        #[test]
        fn test_execute() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            engine.validate_off_chain("s1", time + 100).unwrap();
            let after_timelock = time + 25 * 3600 * 1000;
            engine.register_on_chain("s1", after_timelock).unwrap();
            assert!(engine.execute("s1", after_timelock + 100).is_ok());
            let state = engine.get_session("s1").unwrap();
            assert_eq!(state.status, SessionStatus::Executed);
        }

        #[test]
        fn test_execute_without_validation() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            assert_eq!(
                engine.execute("s1", time + 1000),
                Err(HybridGovernanceError::InvalidStateTransition)
            );
        }

        #[test]
        fn test_fail_session() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            assert!(engine
                .fail_session("s1", "Test failure".to_string())
                .is_ok());
            let state = engine.get_session("s1").unwrap();
            assert!(matches!(state.status, SessionStatus::Failed(_)));
        }

        #[test]
        fn test_full_workflow() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            // Create session
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            // Validate
            engine.validate_off_chain("s1", time + 100).unwrap();
            // Wait for time-lock
            let after_timelock = time + 25 * 3600 * 1000;
            // Register
            engine.register_on_chain("s1", after_timelock).unwrap();
            // Execute
            engine.execute("s1", after_timelock + 100).unwrap();
            // Check metrics
            assert_eq!(engine.metrics().executed_sessions, 1);
        }

        #[test]
        fn test_remove_session() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            assert!(engine.remove_session("s1").is_ok());
            assert_eq!(engine.sessions.len(), 0);
        }

        #[test]
        fn test_remove_session_not_found() {
            let mut engine = HybridGovernance::default();
            assert_eq!(
                engine.remove_session("nonexistent"),
                Err(HybridGovernanceError::SessionNotFound)
            );
        }

        #[test]
        fn test_metrics_tracking() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            engine.validate_off_chain("s1", time + 100).unwrap();
            let after_timelock = time + 25 * 3600 * 1000;
            engine.register_on_chain("s1", after_timelock).unwrap();
            engine.execute("s1", after_timelock + 100).unwrap();
            assert_eq!(engine.metrics().total_sessions, 1);
            assert_eq!(engine.metrics().executed_sessions, 1);
        }

        #[test]
        fn test_reset_metrics() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            engine.reset_metrics();
            assert_eq!(engine.metrics().total_sessions, 0);
        }

        #[test]
        fn test_config_default() {
            let config = HybridGovernanceConfig::default();
            assert_eq!(config.max_sessions, 100);
            assert_eq!(config.quorum_threshold, 0.30);
            assert_eq!(config.approval_threshold, 0.51);
            assert_eq!(config.default_timelock_hours, 24);
            assert_eq!(config.critical_timelock_hours, 72);
        }

        #[test]
        fn test_metrics_default() {
            let metrics = HybridGovernanceMetrics::default();
            assert_eq!(metrics.total_sessions, 0);
            assert_eq!(metrics.executed_sessions, 0);
            assert_eq!(metrics.failed_sessions, 0);
        }

        #[test]
        fn test_error_display() {
            let err = HybridGovernanceError::ProposalNotFound;
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_session_status_display() {
            let status = SessionStatus::Created;
            assert_eq!(format!("{}", status), "Created");
        }

        #[test]
        fn test_execution_hash_generated() {
            let mut engine = HybridGovernance::default();
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            let state = engine.get_session("s1").unwrap();
            assert!(!state.execution_hash.is_empty());
            assert_eq!(state.execution_hash.len(), 64); // SHA-256 hex
        }

        #[test]
        fn test_no_timelock_when_zero() {
            let config = HybridGovernanceConfig {
                default_timelock_hours: 0,
                critical_timelock_hours: 0,
                ..Default::default()
            };
            let mut engine = HybridGovernance::new(config);
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            engine.validate_off_chain("s1", time + 100).unwrap();
            // Should be able to register immediately
            assert!(engine.register_on_chain("s1", time + 200).is_ok());
        }

        #[test]
        fn test_session_not_found_errors() {
            let engine = HybridGovernance::default();
            assert_eq!(
                engine.check_timelock("nonexistent", 0),
                Err(HybridGovernanceError::SessionNotFound)
            );
        }

        #[test]
        fn test_emergency_bypass_count() {
            let config = HybridGovernanceConfig {
                emergency_bypass: true,
                ..Default::default()
            };
            let mut engine = HybridGovernance::new(config);
            let time = current_time_ms();
            engine.create_session("s1".to_string(), "p1".to_string(), false, time)
                .unwrap();
            engine.create_session("s2".to_string(), "p2".to_string(), false, time)
                .unwrap();
            engine.bypass_timelock("s1").unwrap();
            engine.bypass_timelock("s2").unwrap();
            assert_eq!(engine.metrics().emergency_bypass_count, 2);
        }
    }
}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;
