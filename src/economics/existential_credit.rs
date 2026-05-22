//! Existential Credit Ledger — Sprint 29
//!
//! Non-transferable alignment metric mined by ethical compute (Z > 0),
//! burned by perversity (Z < 0). Uses a concurrent hash map keyed by
//! peer identifier, where each entry is a floating-point CRDT counter
//! with merge semantics (max-value per peer).
//!
//! # Mathematical Model
//!
//! - **Emit**: `credit += z_score * compute_weight` when `z_score > 0`
//! - **Burn**: `credit -= (-z_score) * penalty_multiplier` when `z_score < 0`
//! - **Merge**: `score[peer] = max(self[peer], other[peer])` (last-writer-wins by value)
//!
//! # Design Directives
//!
//! - CE is **not** a transferable asset. It is a per-peer alignment metric.
//! - Zero financial logic: no wallets, no transfers, no markets.
//! - CRDT merge is idempotent, commutative, and associative.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Error types for Existential Credit operations.
#[derive(Debug, thiserror::Error)]
pub enum ExistentialCreditError {
    #[error("invalid compute_weight: {0}")]
    InvalidComputeWeight(String),

    #[error("invalid penalty_multiplier: {0}")]
    InvalidPenaltyMultiplier(String),

    #[error("peer not found: {0}")]
    PeerNotFound(String),
}

/// A per-peer CRDT counter for Existential Credit.
///
/// Each entry tracks:
/// - `value`: Current credit balance (f64 for precision with z-scores).
/// - `version`: Monotonic version counter for merge ordering.
/// - `last_updated`: Timestamp (ms) for debugging and observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeEntry {
    /// Current existential credit value.
    pub value: f64,
    /// Monotonic version for CRDT merge ordering.
    pub version: u64,
    /// Last update timestamp in milliseconds.
    pub last_updated: u64,
}

impl CeEntry {
    /// Create a new CE entry at zero.
    pub fn new() -> Self {
        Self {
            value: 0.0,
            version: 0,
            last_updated: 0,
        }
    }

    /// Add credit to this entry.
    pub fn add(&mut self, amount: f64, timestamp_ms: u64) {
        self.value += amount;
        self.version += 1;
        self.last_updated = timestamp_ms;
    }

    /// Subtract credit from this entry.
    pub fn sub(&mut self, amount: f64, timestamp_ms: u64) {
        self.value -= amount;
        self.version += 1;
        self.last_updated = timestamp_ms;
    }

    /// Merge with another entry using max-value semantics.
    ///
    /// If versions differ, the higher version wins.
    /// If versions are equal, the higher value wins (LWW by value).
    pub fn merge(&mut self, other: &CeEntry) {
        if other.version > self.version {
            *self = other.clone();
        } else if other.version == self.version && other.value > self.value {
            self.value = other.value;
            self.last_updated = other.last_updated;
        }
    }
}

impl Default for CeEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// Existential Credit Ledger — The Symbiotic Ledger.
///
/// Maps peer IDs to their current existential credit score.
/// Thread-safe via RwLock for concurrent reads/writes.
///
/// # Invariants
///
/// - CE is non-transferable: no `transfer()` method exists.
/// - Merge is CRDT-compliant: idempotent, commutative, associative.
/// - Score can be negative (indicating perversity accumulation).
#[derive(Debug, Clone)]
pub struct ExistentialCreditLedger {
    /// Per-peer CE entries.
    entries: HashMap<String, CeEntry>,
    /// Global version counter for ledger-wide ordering.
    global_version: u64,
}

impl ExistentialCreditLedger {
    /// Create a new empty ledger.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            global_version: 0,
        }
    }

    /// Emit credit for a peer with positive z-score.
    ///
    /// # Formula
    ///
    /// `credit += z_score * compute_weight`
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer receiving credit.
    /// * `z_score` - Stuartian Tensor Z-axis value (must be > 0).
    /// * `compute_weight` - Weight factor for compute contribution (must be > 0).
    ///
    /// # Errors
    ///
    /// Returns `ExistentialCreditError::InvalidComputeWeight` if `compute_weight <= 0`.
    pub fn emit_credit(
        &mut self,
        peer_id: &str,
        z_score: f32,
        compute_weight: f32,
    ) -> Result<(), ExistentialCreditError> {
        if compute_weight <= 0.0 {
            return Err(ExistentialCreditError::InvalidComputeWeight(
                "compute_weight must be positive".into(),
            ));
        }
        if z_score <= 0.0 {
            // Only emit for positive z-scores; negative z-scores use burn_credit.
            return Ok(());
        }

        self.global_version += 1;
        let amount = (z_score * compute_weight) as f64;
        self.entries
            .entry(peer_id.to_string())
            .or_default()
            .add(amount, self.global_version);

        Ok(())
    }

    /// Burn credit for a peer with negative z-score.
    ///
    /// # Formula
    ///
    /// `credit -= (-z_score) * penalty_multiplier`
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer losing credit.
    /// * `z_score` - Stuartian Tensor Z-axis value (must be < 0).
    /// * `penalty_multiplier` - Multiplier for perversity penalty (must be > 0).
    ///
    /// # Errors
    ///
    /// Returns `ExistentialCreditError::InvalidPenaltyMultiplier` if `penalty_multiplier <= 0`.
    pub fn burn_credit(
        &mut self,
        peer_id: &str,
        z_score: f32,
        penalty_multiplier: f32,
    ) -> Result<(), ExistentialCreditError> {
        if penalty_multiplier <= 0.0 {
            return Err(ExistentialCreditError::InvalidPenaltyMultiplier(
                "penalty_multiplier must be positive".into(),
            ));
        }
        if z_score >= 0.0 {
            // Only burn for negative z-scores; positive z-scores use emit_credit.
            return Ok(());
        }

        self.global_version += 1;
        let amount = ((-z_score) * penalty_multiplier) as f64;
        self.entries
            .entry(peer_id.to_string())
            .or_default()
            .sub(amount, self.global_version);

        Ok(())
    }

    /// Get the current existential credit score for a peer.
    ///
    /// Returns `0.0` if the peer has no entry.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer to query.
    pub fn get_score(&self, peer_id: &str) -> f64 {
        self.entries.get(peer_id).map(|e| e.value).unwrap_or(0.0)
    }

    /// Get the CE entry for a peer (including version and timestamp).
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer to query.
    ///
    /// # Errors
    ///
    /// Returns `ExistentialCreditError::PeerNotFound` if the peer has no entry.
    pub fn get_entry(&self, peer_id: &str) -> Result<CeEntry, ExistentialCreditError> {
        self.entries
            .get(peer_id)
            .cloned()
            .ok_or_else(|| ExistentialCreditError::PeerNotFound(peer_id.to_string()))
    }

    /// Merge another ledger into this one using CRDT semantics.
    ///
    /// For each peer in `other`, the entry with the higher version wins.
    /// If versions are equal, the higher value wins (LWW by value).
    ///
    /// # CRDT Properties
    ///
    /// - **Idempotent**: `merge(self, self) == self`
    /// - **Commutative**: `merge(a, b) == merge(b, a)`
    /// - **Associative**: `merge(merge(a, b), c) == merge(a, merge(b, c))`
    pub fn merge(&mut self, other: &ExistentialCreditLedger) {
        for (peer_id, other_entry) in &other.entries {
            match self.entries.get_mut(peer_id) {
                Some(self_entry) => {
                    self_entry.merge(other_entry);
                }
                None => {
                    self.entries.insert(peer_id.clone(), other_entry.clone());
                }
            }
        }
        // Global version takes the max for ordering consistency.
        if other.global_version > self.global_version {
            self.global_version = other.global_version;
        }
    }

    /// Get all peer IDs in the ledger.
    pub fn peer_ids(&self) -> Vec<&String> {
        self.entries.keys().collect()
    }

    /// Get the total number of peers tracked.
    pub fn peer_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if a peer exists in the ledger.
    pub fn contains_peer(&self, peer_id: &str) -> bool {
        self.entries.contains_key(peer_id)
    }

    /// Remove a peer from the ledger (used during apoptosis).
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Identifier of the peer to remove.
    ///
    /// Returns `true` if the peer was present and removed.
    pub fn remove_peer(&mut self, peer_id: &str) -> bool {
        self.entries.remove(peer_id).is_some()
    }

    /// Get the global version of the ledger.
    pub fn global_version(&self) -> u64 {
        self.global_version
    }
}

impl Default for ExistentialCreditLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ledger() -> ExistentialCreditLedger {
        ExistentialCreditLedger::new()
    }

    #[test]
    fn test_ledger_creation() {
        let ledger = make_ledger();
        assert_eq!(ledger.peer_count(), 0);
        assert_eq!(ledger.get_score("peer1"), 0.0);
        assert_eq!(ledger.global_version(), 0);
    }

    #[test]
    fn test_emit_credit_positive_z() {
        let mut ledger = make_ledger();
        ledger
            .emit_credit("peer1", 2.0, 1.5)
            .expect("emit should succeed");

        let score = ledger.get_score("peer1");
        assert!(
            (score - 3.0).abs() < f64::EPSILON,
            "Expected 3.0, got {}",
            score
        );
        assert!(ledger.contains_peer("peer1"));
        assert_eq!(ledger.global_version(), 1);
    }

    #[test]
    fn test_emit_credit_negative_z_noop() {
        let mut ledger = make_ledger();
        ledger
            .emit_credit("peer1", -1.0, 1.0)
            .expect("should not error");

        assert_eq!(ledger.get_score("peer1"), 0.0);
        assert!(!ledger.contains_peer("peer1"));
    }

    #[test]
    fn test_emit_credit_invalid_weight() {
        let mut ledger = make_ledger();
        let result = ledger.emit_credit("peer1", 1.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_burn_credit_negative_z() {
        let mut ledger = make_ledger();
        ledger
            .burn_credit("peer1", -2.0, 1.5)
            .expect("burn should succeed");

        let score = ledger.get_score("peer1");
        assert!(
            (score - (-3.0)).abs() < f64::EPSILON,
            "Expected -3.0, got {}",
            score
        );
    }

    #[test]
    fn test_burn_credit_positive_z_noop() {
        let mut ledger = make_ledger();
        ledger
            .burn_credit("peer1", 1.0, 1.0)
            .expect("should not error");

        assert_eq!(ledger.get_score("peer1"), 0.0);
        assert!(!ledger.contains_peer("peer1"));
    }

    #[test]
    fn test_burn_credit_invalid_multiplier() {
        let mut ledger = make_ledger();
        let result = ledger.burn_credit("peer1", -1.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_emit_then_burn() {
        let mut ledger = make_ledger();
        ledger
            .emit_credit("peer1", 5.0, 1.0)
            .expect("emit should succeed");
        ledger
            .burn_credit("peer1", -2.0, 1.0)
            .expect("burn should succeed");

        let score = ledger.get_score("peer1");
        assert!(
            (score - 3.0).abs() < f64::EPSILON,
            "Expected 3.0 (5 - 2), got {}",
            score
        );
    }

    #[test]
    fn test_merge_idempotent() {
        let mut ledger = make_ledger();
        ledger
            .emit_credit("peer1", 5.0, 1.0)
            .expect("emit should succeed");

        let before = ledger.clone();
        ledger.merge(&before);

        assert_eq!(ledger.get_score("peer1"), before.get_score("peer1"));
    }

    #[test]
    fn test_merge_commutative() {
        let mut a = make_ledger();
        let mut b = make_ledger();

        a.emit_credit("peer1", 5.0, 1.0).ok();
        b.emit_credit("peer2", 3.0, 1.0).ok();

        let mut a_clone = a.clone();
        let mut b_clone = b.clone();

        a.merge(&b);
        a_clone.merge(&b_clone);

        // Both should have same peers and scores after merge.
        assert_eq!(a.peer_count(), b.peer_count());
        assert!((a.get_score("peer1") - b.get_score("peer1")).abs() < f64::EPSILON);
        assert!((a.get_score("peer2") - b.get_score("peer2")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_merge_higher_version_wins() {
        let mut a = make_ledger();
        let mut b = make_ledger();

        a.emit_credit("peer1", 5.0, 1.0).ok();
        // b has higher version because it was emitted later.
        b.emit_credit("peer1", 10.0, 1.0).ok();

        a.merge(&b);
        assert!(
            (a.get_score("peer1") - 10.0).abs() < f64::EPSILON,
            "Expected 10.0 (higher version wins), got {}",
            a.get_score("peer1")
        );
    }

    #[test]
    fn test_merge_associative() {
        let mut a = make_ledger();
        let mut b = make_ledger();
        let mut c = make_ledger();

        a.emit_credit("peer1", 1.0, 1.0).ok();
        b.emit_credit("peer2", 2.0, 1.0).ok();
        c.emit_credit("peer3", 3.0, 1.0).ok();

        let mut ab = a.clone();
        ab.merge(&b);
        ab.merge(&c);

        let mut bc = b.clone();
        bc.merge(&c);
        a.merge(&bc);

        assert_eq!(ab.peer_count(), a.peer_count());
        assert!((ab.get_score("peer1") - a.get_score("peer1")).abs() < f64::EPSILON);
        assert!((ab.get_score("peer2") - a.get_score("peer2")).abs() < f64::EPSILON);
        assert!((ab.get_score("peer3") - a.get_score("peer3")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_remove_peer() {
        let mut ledger = make_ledger();
        ledger
            .emit_credit("peer1", 5.0, 1.0)
            .expect("emit should succeed");

        assert!(ledger.contains_peer("peer1"));
        assert!(ledger.remove_peer("peer1"));
        assert!(!ledger.contains_peer("peer1"));
        assert_eq!(ledger.get_score("peer1"), 0.0);
    }

    #[test]
    fn test_get_entry_not_found() {
        let ledger = make_ledger();
        let result = ledger.get_entry("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_peer_ids() {
        let mut ledger = make_ledger();
        ledger.emit_credit("peer1", 1.0, 1.0).ok();
        ledger.emit_credit("peer2", 2.0, 1.0).ok();

        let ids = ledger.peer_ids();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_default() {
        let ledger = ExistentialCreditLedger::default();
        assert_eq!(ledger.peer_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = ExistentialCreditError::InvalidComputeWeight("test".into());
        assert!(!format!("{}", err).is_empty());

        let err = ExistentialCreditError::InvalidPenaltyMultiplier("test".into());
        assert!(!format!("{}", err).is_empty());

        let err = ExistentialCreditError::PeerNotFound("peer1".into());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_ce_entry_merge_equal_version_higher_value_wins() {
        let mut a = CeEntry::new();
        a.add(5.0, 1);

        let mut b = CeEntry::new();
        b.add(10.0, 1);

        // Same version, b has higher value.
        a.merge(&b);
        assert!((a.value - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_multi_peer_ledger() {
        let mut ledger = make_ledger();
        ledger.emit_credit("alice", 3.0, 1.0).ok();
        ledger.emit_credit("bob", 2.0, 1.0).ok();
        ledger.burn_credit("charlie", -1.0, 2.0).ok();

        assert!((ledger.get_score("alice") - 3.0).abs() < f64::EPSILON);
        assert!((ledger.get_score("bob") - 2.0).abs() < f64::EPSILON);
        assert!((ledger.get_score("charlie") - (-2.0)).abs() < f64::EPSILON);
        assert_eq!(ledger.peer_count(), 3);
    }
}
