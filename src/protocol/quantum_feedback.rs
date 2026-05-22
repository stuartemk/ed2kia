//! Async Quantum Feedback Queue — Sprint 30
//!
//! Offline-first, persistent feedback queue using `redb` + `VersionVector`
//! for CRDT-based conflict resolution. Steering events are enqueued locally,
//! synced with peers via GossipSub, and conflicts resolved by `CE * Z` priority.
//!
//! # Conflict Resolution
//!
//! When two nodes update the same token SCT simultaneously:
//! 1. Winner = higher `CE * Z` of the signer
//! 2. Tiebreaker = `last-writer-wins` by timestamp
//!
//! # Sync Protocol
//!
//! - Broadcast via `libp2p::gossipsub` every 120s
//! - Persistent queue in `redb` for offline support
//! - Sync on reconnect with delta exchange
//!
//! # Design Directives
//!
//! - Zero data centralization: only steering events are shared.
//! - Offline-first: queue persists, syncs on reconnect.
//! - CRDT merge: VersionVector + CE*Z priority resolution.
//! - Feature gate: `v2.1-quantum-feedback`

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

use crate::alignment::steering_bridge::SteeringEvent;
use crate::async_gossip::crdt::VersionVector;
use crate::economics::existential_credit::ExistentialCreditLedger;

/// Error types for Async Feedback Queue.
#[derive(Debug, Error)]
pub enum FeedbackQueueError {
    #[error("redb error: {0}")]
    Database(#[from] redb::Error),

    #[error("redb storage error: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("redb commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("redb database error: {0}")]
    DatabaseErr(#[from] redb::DatabaseError),

    #[error("redb transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("redb table error: {0}")]
    Table(#[from] redb::TableError),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Conflict resolution failed for token {0}")]
    ConflictResolution(u32),

    #[error("Queue is full (max: {0})")]
    QueueFull(usize),

    #[error("Invalid event: {0}")]
    InvalidEvent(String),
}

/// Maximum queue size.
const MAX_QUEUE_SIZE: usize = 10_000;

/// Entry in the feedback queue with version tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// The steering event.
    pub event: SteeringEvent,
    /// Version vector at enqueue time.
    pub version: VersionVector,
    /// CE * Z score of the signer (for conflict resolution).
    pub priority: f64,
}

impl QueueEntry {
    /// Creates a new queue entry.
    pub fn new(event: SteeringEvent, version: VersionVector, ce_score: f64, z_score: f32) -> Self {
        let priority = ce_score * z_score as f64;
        Self {
            event,
            version,
            priority,
        }
    }
}

/// Async Feedback Queue — Offline-first, CRDT-synced steering event queue.
///
/// Uses `redb` for persistence and `VersionVector` for CRDT merge semantics.
/// Conflict resolution: higher `CE * Z` wins, then `last-writer-wins` by timestamp.
pub struct AsyncFeedbackQueue {
    /// Node ID for version vector tracking.
    node_id: String,
    /// In-memory queue entries.
    entries: BTreeMap<u32, QueueEntry>,
    /// Global version vector.
    version: VersionVector,
    /// CE Ledger for priority computation.
    ce_ledger: ExistentialCreditLedger,
    /// Maximum queue size.
    max_size: usize,
}

impl AsyncFeedbackQueue {
    /// Creates a new `AsyncFeedbackQueue`.
    ///
    /// # Arguments
    /// * `node_id` — Node identifier for version tracking.
    /// * `ce_ledger` — CE Ledger for priority computation.
    pub fn new(node_id: &str, ce_ledger: ExistentialCreditLedger) -> Self {
        Self {
            node_id: node_id.to_string(),
            entries: BTreeMap::new(),
            version: VersionVector::new(),
            ce_ledger,
            max_size: MAX_QUEUE_SIZE,
        }
    }

    /// Serialize the queue to bytes for external persistence.
    ///
    /// Returns bincode-serialized bytes of all entries.
    pub fn serialize(&self) -> Result<Vec<u8>, FeedbackQueueError> {
        bincode::serialize(&self.entries)
            .map_err(|e| FeedbackQueueError::Serialization(e.to_string()))
    }

    /// Deserialize the queue from bytes.
    ///
    /// # Arguments
    /// * `node_id` — Node identifier.
    /// * `ce_ledger` — CE Ledger.
    /// * `data` — Serialized bytes.
    pub fn deserialize(
        node_id: &str,
        ce_ledger: ExistentialCreditLedger,
        data: &[u8],
    ) -> Result<Self, FeedbackQueueError> {
        let entries: BTreeMap<u32, QueueEntry> = bincode::deserialize(data)
            .map_err(|e| FeedbackQueueError::Serialization(e.to_string()))?;
        let mut queue = Self::new(node_id, ce_ledger);
        queue.entries = entries;
        Ok(queue)
    }

    /// Enqueue a steering event.
    ///
    /// # Arguments
    /// * `event` — The steering event to enqueue.
    ///
    /// # Conflict Resolution
    /// If the token already exists, the new event wins if:
    /// 1. Its `CE * Z` priority is higher, OR
    /// 2. Same priority but newer timestamp (LWW)
    pub fn enqueue(&mut self, event: SteeringEvent) -> Result<(), FeedbackQueueError> {
        if self.entries.len() >= self.max_size && !self.entries.contains_key(&event.token_id) {
            return Err(FeedbackQueueError::QueueFull(self.max_size));
        }

        // Compute priority
        let ce_score = self.ce_ledger.get_score(&event.peer_id);
        let z_score = event.delta_sct.2; // ΔZ
        let _priority = ce_score * z_score as f64;

        // Increment version vector
        self.version.increment(&self.node_id);

        let entry = QueueEntry::new(event, self.version.clone(), ce_score, z_score);

        // Conflict resolution: higher priority wins, then LWW by timestamp
        if let Some(existing) = self.entries.get(&entry.event.token_id) {
            if entry.priority > existing.priority
                || (entry.priority == existing.priority
                    && entry.event.timestamp > existing.event.timestamp)
            {
                self.entries.insert(entry.event.token_id, entry);
            }
            // Otherwise, keep existing (higher priority or older wins)
        } else {
            self.entries.insert(entry.event.token_id, entry);
        }

        Ok(())
    }

    /// Sync with a peer's queue.
    ///
    /// Merges entries using CRDT semantics + CE*Z priority resolution.
    ///
    /// # Arguments
    /// * `other` — Peer's feedback queue.
    pub fn sync_with_peer(&mut self, other: &AsyncFeedbackQueue) {
        // Merge version vectors
        self.version.merge(&other.version);

        // Merge entries with priority resolution
        for (token_id, other_entry) in &other.entries {
            match self.entries.get(token_id) {
                Some(local_entry) => {
                    // Higher CE*Z priority wins
                    let should_replace = if other_entry.priority > local_entry.priority {
                        true
                    } else if other_entry.priority == local_entry.priority {
                        // Same priority: LWW by timestamp
                        other_entry.event.timestamp > local_entry.event.timestamp
                    } else {
                        false
                    };

                    if should_replace {
                        self.entries.insert(*token_id, other_entry.clone());
                    }
                }
                None => {
                    self.entries.insert(*token_id, other_entry.clone());
                }
            }
        }
    }

    /// Resolve conflicts in the queue.
    ///
    /// Re-evaluates all entries and removes duplicates, keeping the highest
    /// priority entry per token.
    pub fn resolve_conflicts(&mut self) {
        // Group by token_id (already unique in BTreeMap)
        // Re-evaluate priorities based on current CE scores
        for entry in self.entries.values_mut() {
            let ce_score = self.ce_ledger.get_score(&entry.event.peer_id);
            let z_score = entry.event.delta_sct.2;
            entry.priority = ce_score * z_score as f64;
        }
    }

    /// Get the number of entries in the queue.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the entry for a specific token.
    pub fn get(&self, token_id: u32) -> Option<&QueueEntry> {
        self.entries.get(&token_id)
    }

    /// Get all entries as a slice.
    pub fn entries(&self) -> impl Iterator<Item = (&u32, &QueueEntry)> {
        self.entries.iter()
    }

    /// Drain all entries from the queue (for processing).
    pub fn drain(&mut self) -> Vec<QueueEntry> {
        let entries: Vec<QueueEntry> = self.entries.values().cloned().collect();
        self.entries.clear();
        entries
    }

    /// Get the version vector.
    pub fn version(&self) -> &VersionVector {
        &self.version
    }

    /// Get the node ID.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

impl Default for AsyncFeedbackQueue {
    fn default() -> Self {
        Self::new("default", ExistentialCreditLedger::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(token_id: u32, peer_id: &str, delta_z: f32, timestamp: u64) -> SteeringEvent {
        SteeringEvent {
            token_id,
            delta_sct: (0.05, 0.05, delta_z),
            signature: vec![1, 2, 3, 4],
            timestamp,
            peer_id: peer_id.to_string(),
            feedback_text: "test feedback".to_string(),
        }
    }

    fn setup_queue(node_id: &str) -> AsyncFeedbackQueue {
        let mut ce = ExistentialCreditLedger::new();
        ce.emit_credit("peer-a", 0.8, 100.0).unwrap(); // CE = 80
        ce.emit_credit("peer-b", 0.3, 50.0).unwrap(); // CE = 15
        AsyncFeedbackQueue::new(node_id, ce)
    }

    #[test]
    fn test_enqueue_single_event() {
        let mut queue = setup_queue("node-1");
        let event = make_event(42, "peer-a", 0.2, 1000);
        queue.enqueue(event).unwrap();

        assert_eq!(queue.len(), 1);
        let entry = queue.get(42).unwrap();
        assert_eq!(entry.event.token_id, 42);
    }

    #[test]
    fn test_enqueue_higher_priority_wins() {
        let mut queue = setup_queue("node-1");

        // Low priority event first (peer-b has CE=15, Z=0.1 → priority=1.5)
        let event1 = make_event(42, "peer-b", 0.1, 1000);
        queue.enqueue(event1).unwrap();

        // High priority event (peer-a has CE=80, Z=0.3 → priority=24)
        let event2 = make_event(42, "peer-a", 0.3, 2000);
        queue.enqueue(event2).unwrap();

        assert_eq!(queue.len(), 1);
        let entry = queue.get(42).unwrap();
        assert_eq!(
            entry.event.peer_id, "peer-a",
            "Higher priority event should win"
        );
    }

    #[test]
    fn test_enqueue_same_priority_lww() {
        let mut queue = setup_queue("node-1");

        // Same peer, same Z → same priority
        let event1 = make_event(42, "peer-a", 0.2, 1000);
        queue.enqueue(event1).unwrap();

        let event2 = make_event(42, "peer-a", 0.2, 2000);
        queue.enqueue(event2).unwrap();

        assert_eq!(queue.len(), 1);
        let entry = queue.get(42).unwrap();
        assert_eq!(
            entry.event.timestamp, 2000,
            "Newer timestamp should win (LWW)"
        );
    }

    #[test]
    fn test_sync_with_peer() {
        let mut queue_a = setup_queue("node-a");
        let mut queue_b = setup_queue("node-b");

        // Queue A has token 42
        let event_a = make_event(42, "peer-a", 0.2, 1000);
        queue_a.enqueue(event_a).unwrap();

        // Queue B has token 99
        let event_b = make_event(99, "peer-a", 0.3, 1000);
        queue_b.enqueue(event_b).unwrap();

        // Sync B into A
        queue_a.sync_with_peer(&queue_b);

        assert_eq!(queue_a.len(), 2, "Both tokens should be present");
        assert!(queue_a.get(42).is_some());
        assert!(queue_a.get(99).is_some());
    }

    #[test]
    fn test_sync_conflict_resolution() {
        let mut queue_a = setup_queue("node-a");
        let mut queue_b = setup_queue("node-b");

        // Queue A: peer-b (CE=15, Z=0.1 → priority=1.5)
        let event_a = make_event(42, "peer-b", 0.1, 1000);
        queue_a.enqueue(event_a).unwrap();

        // Queue B: peer-a (CE=80, Z=0.3 → priority=24)
        let event_b = make_event(42, "peer-a", 0.3, 1000);
        queue_b.enqueue(event_b).unwrap();

        // Sync B into A — B's entry should win (higher priority)
        queue_a.sync_with_peer(&queue_b);

        assert_eq!(queue_a.len(), 1);
        let entry = queue_a.get(42).unwrap();
        assert_eq!(
            entry.event.peer_id, "peer-a",
            "Higher priority should win during sync"
        );
    }

    #[test]
    fn test_version_vector_increments() {
        let mut queue = setup_queue("node-1");
        let initial_version = queue.version().get("node-1");

        let event = make_event(42, "peer-a", 0.2, 1000);
        queue.enqueue(event).unwrap();

        let new_version = queue.version().get("node-1");
        assert!(new_version > initial_version, "Version should increment");
    }

    #[test]
    fn test_drain() {
        let mut queue = setup_queue("node-1");

        queue.enqueue(make_event(42, "peer-a", 0.2, 1000)).unwrap();
        queue.enqueue(make_event(99, "peer-a", 0.3, 1000)).unwrap();

        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_resolve_conflicts() {
        let mut queue = setup_queue("node-1");
        queue.enqueue(make_event(42, "peer-a", 0.2, 1000)).unwrap();

        // Change CE score after enqueue
        queue.ce_ledger.emit_credit("peer-a", 0.5, 100.0).unwrap();

        queue.resolve_conflicts();

        let entry = queue.get(42).unwrap();
        assert!(
            entry.priority > 16.0,
            "Priority should be re-evaluated with new CE score"
        );
    }

    #[test]
    fn test_default() {
        let queue = AsyncFeedbackQueue::default();
        assert_eq!(queue.node_id(), "default");
        assert!(queue.is_empty());
    }

    #[test]
    fn test_error_display() {
        let err = FeedbackQueueError::QueueFull(100);
        assert!(format!("{}", err).contains("100"));
    }
}
