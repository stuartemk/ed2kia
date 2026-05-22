//! Proof of Symbiosis Validator — Sprint 29
//!
//! Consensus mechanism where validation weight depends on accumulated
//! Existential Credit (CE). Provides anti-Sybil protection through
//! dynamic threshold calculation based on network load.
//!
//! # Mathematical Model
//!
//! - **Dynamic Threshold**: `threshold = base_threshold * (1.0 + network_load_factor)`
//! - **Validation Weight**: `weight = ce_score / total_ce` (proportional to CE)
//! - **Committee Threshold**: Sum of committee weights >= dynamic threshold
//!
//! # Design Directives
//!
//! - PoS is NOT Proof of Stake: validation weight comes from ethical alignment (CE), not tokens.
//! - Anti-Sybil: Creating N fake nodes splits CE across them, reducing individual weight.
//! - Dynamic threshold adapts to network load for security under stress.

use crate::economics::existential_credit::ExistentialCreditLedger;

/// Error types for Proof of Symbiosis operations.
#[derive(Debug, thiserror::Error)]
pub enum SymbiosisError {
    #[error("invalid base_threshold: {0}")]
    InvalidBaseThreshold(String),

    #[error("invalid network_load_factor: {0}")]
    InvalidNetworkLoadFactor(String),

    #[error("empty committee")]
    EmptyCommittee,

    #[error("peer not found in ledger: {0}")]
    PeerNotFound(String),
}

/// Trait for validating Proof of Symbiosis proposals.
///
/// Implementations must provide:
/// - `validate_committee()`: Check if a committee meets the dynamic threshold.
/// - `calculate_weight()`: Compute validation weight for a peer based on CE.
pub trait SymbiosisValidator {
    /// Validate if a committee meets the current dynamic threshold.
    ///
    /// # Arguments
    ///
    /// * `committee` - List of peer IDs forming the committee.
    /// * `ledger` - Existential Credit Ledger for score lookup.
    /// * `base_threshold` - Base threshold value (must be in [0, 1]).
    /// * `network_load_factor` - Network load factor (must be >= 0).
    ///
    /// # Returns
    ///
    /// `true` if the sum of committee weights >= dynamic threshold.
    fn validate_committee(
        &self,
        committee: &[&str],
        ledger: &ExistentialCreditLedger,
        base_threshold: f64,
        network_load_factor: f64,
    ) -> Result<bool, SymbiosisError>;

    /// Calculate the validation weight for a peer.
    ///
    /// Weight is proportional to the peer's CE score relative to total network CE.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer.
    /// * `ledger` - Existential Credit Ledger for score lookup.
    ///
    /// # Returns
    ///
    /// Weight in range [0, 1] representing the peer's validation power.
    fn calculate_weight(
        &self,
        peer_id: &str,
        ledger: &ExistentialCreditLedger,
    ) -> Result<f64, SymbiosisError>;
}

/// Default implementation of SymbiosisValidator.
///
/// Uses proportional weighting: `weight = ce_score / total_ce`.
/// Peers with negative CE have zero weight.
#[derive(Debug, Clone)]
pub struct DefaultSymbiosisValidator;

impl DefaultSymbiosisValidator {
    /// Create a new default validator.
    pub fn new() -> Self {
        Self
    }

    /// Calculate the dynamic threshold.
    ///
    /// `threshold = base_threshold * (1.0 + network_load_factor)`
    ///
    /// # Arguments
    ///
    /// * `base_threshold` - Base threshold (must be in [0, 1]).
    /// * `network_load_factor` - Network load factor (must be >= 0).
    pub fn calculate_dynamic_threshold(
        base_threshold: f64,
        network_load_factor: f64,
    ) -> Result<f64, SymbiosisError> {
        if base_threshold < 0.0 || base_threshold > 1.0 {
            return Err(SymbiosisError::InvalidBaseThreshold(
                "base_threshold must be in [0, 1]".into(),
            ));
        }
        if network_load_factor < 0.0 {
            return Err(SymbiosisError::InvalidNetworkLoadFactor(
                "network_load_factor must be >= 0".into(),
            ));
        }
        Ok(base_threshold * (1.0 + network_load_factor))
    }

    /// Calculate total CE across all peers (only positive scores count).
    fn total_positive_ce(ledger: &ExistentialCreditLedger) -> f64 {
        let ids = ledger.peer_ids();
        ids.iter()
            .map(|id| ledger.get_score(id))
            .filter(|&score| score > 0.0)
            .sum()
    }
}

impl SymbiosisValidator for DefaultSymbiosisValidator {
    fn validate_committee(
        &self,
        committee: &[&str],
        ledger: &ExistentialCreditLedger,
        base_threshold: f64,
        network_load_factor: f64,
    ) -> Result<bool, SymbiosisError> {
        if committee.is_empty() {
            return Err(SymbiosisError::EmptyCommittee);
        }

        let dynamic_threshold =
            Self::calculate_dynamic_threshold(base_threshold, network_load_factor)?;

        let total_weight: f64 = committee
            .iter()
            .map(|peer_id| self.calculate_weight(peer_id, ledger))
            .sum::<Result<f64, SymbiosisError>>()?;

        Ok(total_weight >= dynamic_threshold)
    }

    fn calculate_weight(
        &self,
        peer_id: &str,
        ledger: &ExistentialCreditLedger,
    ) -> Result<f64, SymbiosisError> {
        let score = ledger.get_score(peer_id);
        if score <= 0.0 {
            return Ok(0.0);
        }

        let total_ce = Self::total_positive_ce(ledger);
        if total_ce <= 0.0 {
            return Ok(0.0);
        }

        Ok(score / total_ce)
    }
}

impl Default for DefaultSymbiosisValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to check if a committee meets the threshold.
///
/// Uses the default validator with proportional weighting.
///
/// # Arguments
///
/// * `committee` - List of peer IDs forming the committee.
/// * `ledger` - Existential Credit Ledger for score lookup.
/// * `base_threshold` - Base threshold (must be in [0, 1]).
/// * `network_load_factor` - Network load factor (must be >= 0).
///
/// # Returns
///
/// `true` if the committee is valid under current conditions.
pub fn committee_threshold_met(
    committee: &[&str],
    ledger: &ExistentialCreditLedger,
    base_threshold: f64,
    network_load_factor: f64,
) -> Result<bool, SymbiosisError> {
    let validator = DefaultSymbiosisValidator::new();
    validator.validate_committee(committee, ledger, base_threshold, network_load_factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_ledger() -> ExistentialCreditLedger {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("alice", 10.0, 1.0).ok();
        ledger.emit_credit("bob", 5.0, 1.0).ok();
        ledger.emit_credit("charlie", 5.0, 1.0).ok();
        ledger
    }

    #[test]
    fn test_validator_creation() {
        let validator = DefaultSymbiosisValidator::new();
        let _ = validator;
    }

    #[test]
    fn test_calculate_weight_proportional() {
        let ledger = setup_ledger();
        let validator = DefaultSymbiosisValidator::new();

        // Total CE = 20.0, alice has 10.0 -> weight = 0.5
        let alice_weight = validator
            .calculate_weight("alice", &ledger)
            .expect("weight should succeed");
        assert!(
            (alice_weight - 0.5).abs() < f64::EPSILON,
            "Expected 0.5, got {}",
            alice_weight
        );

        // bob has 5.0 -> weight = 0.25
        let bob_weight = validator
            .calculate_weight("bob", &ledger)
            .expect("weight should succeed");
        assert!(
            (bob_weight - 0.25).abs() < f64::EPSILON,
            "Expected 0.25, got {}",
            bob_weight
        );
    }

    #[test]
    fn test_calculate_weight_negative_ce() {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("good", 10.0, 1.0).ok();
        ledger.burn_credit("bad", -5.0, 1.0).ok();

        let validator = DefaultSymbiosisValidator::new();
        let bad_weight = validator
            .calculate_weight("bad", &ledger)
            .expect("weight should succeed");
        assert_eq!(bad_weight, 0.0);
    }

    #[test]
    fn test_calculate_weight_unknown_peer() {
        let ledger = setup_ledger();
        let validator = DefaultSymbiosisValidator::new();

        let weight = validator
            .calculate_weight("unknown", &ledger)
            .expect("weight should succeed");
        assert_eq!(weight, 0.0);
    }

    #[test]
    fn test_dynamic_threshold() {
        // base = 0.5, load = 0.0 -> threshold = 0.5
        let threshold = DefaultSymbiosisValidator::calculate_dynamic_threshold(0.5, 0.0)
            .expect("threshold should succeed");
        assert!((threshold - 0.5).abs() < f64::EPSILON);

        // base = 0.5, load = 1.0 -> threshold = 1.0
        let threshold = DefaultSymbiosisValidator::calculate_dynamic_threshold(0.5, 1.0)
            .expect("threshold should succeed");
        assert!((threshold - 1.0).abs() < f64::EPSILON);

        // base = 0.5, load = 0.5 -> threshold = 0.75
        let threshold = DefaultSymbiosisValidator::calculate_dynamic_threshold(0.5, 0.5)
            .expect("threshold should succeed");
        assert!((threshold - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dynamic_threshold_invalid_base() {
        let result = DefaultSymbiosisValidator::calculate_dynamic_threshold(1.5, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_dynamic_threshold_invalid_load() {
        let result = DefaultSymbiosisValidator::calculate_dynamic_threshold(0.5, -0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_committee_threshold_met() {
        let ledger = setup_ledger();
        // alice (0.5) + bob (0.25) = 0.75 >= 0.5 threshold
        let result = committee_threshold_met(&["alice", "bob"], &ledger, 0.5, 0.0)
            .expect("validation should succeed");
        assert!(result, "Committee should meet threshold");
    }

    #[test]
    fn test_committee_threshold_not_met() {
        let ledger = setup_ledger();
        // charlie (0.25) < 0.5 threshold
        let result = committee_threshold_met(&["charlie"], &ledger, 0.5, 0.0)
            .expect("validation should succeed");
        assert!(!result, "Single peer should not meet threshold");
    }

    #[test]
    fn test_committee_empty() {
        let ledger = setup_ledger();
        let result = committee_threshold_met(&[], &ledger, 0.5, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_committee_with_high_load() {
        let ledger = setup_ledger();
        // alice (0.5) + bob (0.25) = 0.75
        // threshold = 0.5 * (1.0 + 1.0) = 1.0
        // 0.75 < 1.0 -> not met
        let result = committee_threshold_met(&["alice", "bob"], &ledger, 0.5, 1.0)
            .expect("validation should succeed");
        assert!(
            !result,
            "High load should raise threshold above committee weight"
        );
    }

    #[test]
    fn test_full_committee_meets_threshold() {
        let ledger = setup_ledger();
        // alice (0.5) + bob (0.25) + charlie (0.25) = 1.0 >= 0.5
        let result = committee_threshold_met(&["alice", "bob", "charlie"], &ledger, 0.5, 0.0)
            .expect("validation should succeed");
        assert!(result, "Full committee should meet threshold");
    }

    #[test]
    fn test_anti_sybil_protection() {
        // If one node splits into 3 fakes, each gets 1/3 the CE.
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("real", 30.0, 1.0).ok();
        ledger.emit_credit("fake1", 10.0, 1.0).ok();
        ledger.emit_credit("fake2", 10.0, 1.0).ok();
        ledger.emit_credit("fake3", 10.0, 1.0).ok();

        let validator = DefaultSymbiosisValidator::new();

        // Real node has 30/60 = 0.5 weight
        let real_weight = validator
            .calculate_weight("real", &ledger)
            .expect("weight should succeed");
        assert!((real_weight - 0.5).abs() < f64::EPSILON);

        // Each fake has 10/60 = 0.167 weight
        let fake1_weight = validator
            .calculate_weight("fake1", &ledger)
            .expect("weight should succeed");
        assert!((fake1_weight - 1.0 / 6.0).abs() < 1e-9);

        // All 3 fakes combined = 0.5 (same as real node)
        // But they need 3x the coordination to achieve same weight
    }

    #[test]
    fn test_default() {
        let validator = DefaultSymbiosisValidator::default();
        let _ = validator;
    }

    #[test]
    fn test_error_display() {
        let err = SymbiosisError::InvalidBaseThreshold("test".into());
        assert!(!format!("{}", err).is_empty());

        let err = SymbiosisError::InvalidNetworkLoadFactor("test".into());
        assert!(!format!("{}", err).is_empty());

        let err = SymbiosisError::EmptyCommittee;
        assert!(!format!("{}", err).is_empty());

        let err = SymbiosisError::PeerNotFound("peer1".into());
        assert!(!format!("{}", err).is_empty());
    }
}
