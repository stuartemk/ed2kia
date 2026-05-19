//! Reputation Engine — Slashing & Banning matrix for peer trust management.
//!
//! Feature-gated behind `v2.1-reputation-system`. Provides concurrent
//! reputation scoring via DashMap with automatic banning when scores drop
//! below zero.
//!
//! **Status:** Functional with unit tests.
//! **License:** Apache 2.0 + Ethical Use Clause

use dashmap::{DashMap, DashSet};

/// Reputation Engine — Tracks peer scores and maintains a ban list.
///
/// - `+1` point for each matched (consensus) result
/// - `-50` points for each mismatched (poisoned) result
/// - Peer is banned when score drops below 0
pub struct ReputationEngine {
    /// Peer reputation scores (peer_id -> score)
    pub scores: DashMap<String, i32>,
    /// Banned peers (immutable until explicitly unbanned)
    pub ban_list: DashSet<String>,
}

impl ReputationEngine {
    /// Creates a new empty ReputationEngine.
    pub fn new() -> Self {
        Self {
            scores: DashMap::new(),
            ban_list: DashSet::new(),
        }
    }

    /// Updates the score for a peer based on consensus match result.
    ///
    /// # Arguments
    /// * `peer_id` — Identifier of the peer to update
    /// * `matched` — `true` if the peer's result matched consensus, `false` if mismatch
    ///
    /// # Returns
    /// `true` if the peer should be banned (score < 0), `false` otherwise
    pub fn update_score(&self, peer_id: String, matched: bool) -> bool {
        let delta = if matched { 1 } else { -50 };

        let mut score = self.scores.entry(peer_id.clone()).or_insert(0);
        *score += delta;

        if *score < 0 {
            self.ban_list.insert(peer_id);
            true
        } else {
            false
        }
    }

    /// Checks if a peer is banned.
    pub fn is_banned(&self, peer_id: &str) -> bool {
        self.ban_list.contains(peer_id)
    }

    /// Returns the current score for a peer.
    pub fn get_score(&self, peer_id: &str) -> Option<i32> {
        self.scores.get(peer_id).map(|entry| *entry.value())
    }

    /// Returns the count of banned peers.
    pub fn banned_count(&self) -> usize {
        self.ban_list.len()
    }

    /// Returns the count of tracked peers.
    pub fn tracked_count(&self) -> usize {
        self.scores.len()
    }

    /// Explicitly unban a peer (requires human governance decision).
    /// Also resets the score to 0.
    pub fn unban_peer(&self, peer_id: &str) {
        self.ban_list.remove(peer_id);
        if let Some(mut entry) = self.scores.get_mut(peer_id) {
            *entry = 0;
        }
    }

    /// Returns a list of all banned peer IDs.
    pub fn get_banned_peers(&self) -> Vec<String> {
        self.ban_list.iter().map(|k| k.clone()).collect()
    }
}

impl Default for ReputationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ReputationEngine::new();
        assert_eq!(engine.tracked_count(), 0);
        assert_eq!(engine.banned_count(), 0);
    }

    #[test]
    fn test_update_score_match() {
        let engine = ReputationEngine::new();
        let banned = engine.update_score("peer-1".to_string(), true);
        assert!(!banned);
        assert_eq!(engine.get_score("peer-1"), Some(1));
    }

    #[test]
    fn test_update_score_mismatch_bans() {
        let engine = ReputationEngine::new();
        let banned = engine.update_score("peer-1".to_string(), false);
        assert!(banned);
        assert_eq!(engine.get_score("peer-1"), Some(-50));
        assert!(engine.is_banned("peer-1"));
    }

    #[test]
    fn test_gradual_score_increase() {
        let engine = ReputationEngine::new();
        for _ in 0..5 {
            engine.update_score("peer-1".to_string(), true);
        }
        assert_eq!(engine.get_score("peer-1"), Some(5));
    }

    #[test]
    fn test_mismatch_after_positive_score() {
        let engine = ReputationEngine::new();
        // Build up positive score
        for _ in 0..10 {
            engine.update_score("peer-1".to_string(), true);
        }
        assert_eq!(engine.get_score("peer-1"), Some(10));
        // One mismatch: 10 - 50 = -40 → banned
        let banned = engine.update_score("peer-1".to_string(), false);
        assert!(banned);
        assert_eq!(engine.get_score("peer-1"), Some(-40));
        assert!(engine.is_banned("peer-1"));
    }

    #[test]
    fn test_is_banned_unknown_peer() {
        let engine = ReputationEngine::new();
        assert!(!engine.is_banned("unknown"));
    }

    #[test]
    fn test_get_score_unknown_peer() {
        let engine = ReputationEngine::new();
        assert_eq!(engine.get_score("unknown"), None);
    }

    #[test]
    fn test_unban_peer() {
        let engine = ReputationEngine::new();
        engine.update_score("peer-1".to_string(), false);
        assert!(engine.is_banned("peer-1"));
        engine.unban_peer("peer-1");
        assert!(!engine.is_banned("peer-1"));
        assert_eq!(engine.get_score("peer-1"), Some(0));
    }

    #[test]
    fn test_get_banned_peers() {
        let engine = ReputationEngine::new();
        engine.update_score("peer-1".to_string(), false);
        engine.update_score("peer-2".to_string(), false);
        engine.update_score("peer-3".to_string(), true);
        let banned = engine.get_banned_peers();
        assert_eq!(banned.len(), 2);
        assert!(banned.contains(&"peer-1".to_string()));
        assert!(banned.contains(&"peer-2".to_string()));
    }

    #[test]
    fn test_banned_count() {
        let engine = ReputationEngine::new();
        engine.update_score("peer-1".to_string(), false);
        assert_eq!(engine.banned_count(), 1);
        engine.update_score("peer-2".to_string(), true);
        assert_eq!(engine.banned_count(), 1); // peer-2 not banned
    }

    #[test]
    fn test_tracked_count() {
        let engine = ReputationEngine::new();
        engine.update_score("peer-1".to_string(), true);
        engine.update_score("peer-2".to_string(), true);
        assert_eq!(engine.tracked_count(), 2);
    }

    #[test]
    fn test_default() {
        let engine = ReputationEngine::default();
        assert_eq!(engine.tracked_count(), 0);
    }

    #[test]
    fn test_concurrent_updates() {
        use std::sync::Arc;
        use std::thread;

        let engine = Arc::new(ReputationEngine::new());

        let mut handles = vec![];
        for i in 0..10 {
            let engine = Arc::clone(&engine);
            handles.push(thread::spawn(move || {
                let peer_id = format!("peer-{}", i);
                engine.update_score(peer_id, true);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(engine.tracked_count(), 10);
    }
}
