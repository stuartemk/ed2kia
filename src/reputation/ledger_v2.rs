//! Reputation Ledger v2 — Immutable, cryptographically signed reputation event ledger.
//!
//! Features:
//! - Append-only event log with hash chaining
//! - ed25519-dalek style signature verification (simulated)
//! - Merkle root computation for batch verification
//! - Zero financial logic: reputation = compute credits + governance weight

use std::collections::{BTreeMap, VecDeque};

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum LedgerError {
    InvalidSignature(String),
    ChainBroken { expected: String, got: String },
    NodeNotFound(String),
    DuplicateEvent(String),
    CorruptEntry(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSignature(id) => write!(f, "Invalid signature for: {}", id),
            Self::ChainBroken { expected, got } => {
                write!(f, "Chain broken: expected {}, got {}", expected, got)
            }
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::DuplicateEvent(id) => write!(f, "Duplicate event: {}", id),
            Self::CorruptEntry(id) => write!(f, "Corrupt entry: {}", id),
        }
    }
}

impl std::error::Error for LedgerError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct LedgerConfig {
    pub max_events: usize,
    pub merkle_batch_size: usize,
    pub retention_days: u64,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            max_events: 100_000,
            merkle_batch_size: 128,
            retention_days: 90,
        }
    }
}

// ─── Reputation Event ───

#[derive(Debug, Clone)]
pub enum EventType {
    Contribution,
    Review,
    GovernanceVote,
    ComputeCredit,
    SybilPenalty,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contribution => write!(f, "contribution"),
            Self::Review => write!(f, "review"),
            Self::GovernanceVote => write!(f, "governance_vote"),
            Self::ComputeCredit => write!(f, "compute_credit"),
            Self::SybilPenalty => write!(f, "sybil_penalty"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReputationEvent {
    pub event_id: String,
    pub node_id: String,
    pub event_type: EventType,
    pub score_delta: f64,
    pub signature: String,
    pub prev_hash: String,
    pub hash: String,
    pub timestamp_ms: u64,
}

impl ReputationEvent {
    pub fn new(
        event_id: String,
        node_id: String,
        event_type: EventType,
        score_delta: f64,
        prev_hash: String,
    ) -> Self {
        let signature = compute_signature(&event_id, &node_id, score_delta);
        let data = format!(
            "{}:{}:{}:{}:{}",
            event_id,
            node_id,
            score_delta,
            prev_hash,
            current_timestamp_ms()
        );
        let hash = compute_hash(&data);
        Self {
            event_id,
            node_id,
            event_type,
            score_delta,
            signature,
            prev_hash,
            hash,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    pub fn verify_signature(&self) -> bool {
        let expected = compute_signature(&self.event_id, &self.node_id, self.score_delta);
        self.signature == expected
    }

    pub fn verify_chain(&self, expected_prev: &str) -> bool {
        self.prev_hash == expected_prev
    }
}

// ─── Reputation Profile ───

#[derive(Debug, Clone)]
pub struct ReputationProfile {
    pub node_id: String,
    pub total_score: f64,
    pub event_count: u64,
    pub first_event_ms: u64,
    pub last_event_ms: u64,
    pub merkle_leaf: String,
}

impl ReputationProfile {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            total_score: 0.0,
            event_count: 0,
            first_event_ms: 0,
            last_event_ms: 0,
            merkle_leaf: String::new(),
        }
    }

    pub fn apply_event(&mut self, event: &ReputationEvent) {
        self.total_score = (self.total_score + event.score_delta).clamp(0.0, 1000.0);
        self.event_count += 1;
        if self.first_event_ms == 0 {
            self.first_event_ms = event.timestamp_ms;
        }
        self.last_event_ms = event.timestamp_ms;
        self.merkle_leaf = event.hash.clone();
    }
}

// ─── Ledger Stats ───

#[derive(Debug, Clone, Default)]
pub struct LedgerStats {
    pub total_events: u64,
    pub total_nodes: u64,
    pub chain_length: u64,
    pub last_hash: String,
    pub verifications_passed: u64,
    pub verifications_failed: u64,
}

// ─── Ledger ───

pub struct ReputationLedgerV2 {
    config: LedgerConfig,
    events: VecDeque<ReputationEvent>,
    profiles: BTreeMap<String, ReputationProfile>,
    stats: LedgerStats,
    genesis_hash: String,
}

impl ReputationLedgerV2 {
    pub fn new(config: LedgerConfig) -> Self {
        let genesis_hash = compute_hash("genesis");
        Self {
            config,
            events: VecDeque::new(),
            profiles: BTreeMap::new(),
            stats: LedgerStats::default(),
            genesis_hash,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(LedgerConfig::default())
    }

    pub fn record_event(
        &mut self,
        event_id: String,
        node_id: String,
        event_type: EventType,
        score_delta: f64,
    ) -> Result<ReputationEvent, LedgerError> {
        // Check duplicate
        if self.events.iter().any(|e| e.event_id == event_id) {
            return Err(LedgerError::DuplicateEvent(event_id));
        }

        // Get previous hash
        let prev_hash = self
            .events
            .back()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| self.genesis_hash.clone());

        let event = ReputationEvent::new(
            event_id.clone(),
            node_id.clone(),
            event_type,
            score_delta,
            prev_hash,
        );

        // Verify signature
        if !event.verify_signature() {
            self.stats.verifications_failed += 1;
            return Err(LedgerError::InvalidSignature(event_id));
        }
        self.stats.verifications_passed += 1;

        // Append
        self.events.push_back(event.clone());
        if self.events.len() > self.config.max_events {
            self.events.pop_front();
        }

        // Update profile
        self.stats.chain_length += 1;
        self.stats.last_hash = event.hash.clone();
        self.stats.total_events += 1;

        let profile = self.profiles.entry(node_id.clone()).or_insert_with(|| {
            self.stats.total_nodes += 1;
            ReputationProfile::new(node_id)
        });
        profile.apply_event(&event);

        Ok(event)
    }

    pub fn get_profile(&self, node_id: &str) -> Option<&ReputationProfile> {
        self.profiles.get(node_id)
    }

    pub fn verify_chain(&self) -> Result<(), LedgerError> {
        let mut expected_prev = self.genesis_hash.clone();
        for event in &self.events {
            if !event.verify_chain(&expected_prev) {
                return Err(LedgerError::ChainBroken {
                    expected: expected_prev,
                    got: event.prev_hash.clone(),
                });
            }
            expected_prev = event.hash.clone();
        }
        Ok(())
    }

    pub fn compute_merkle_root(&self) -> String {
        if self.events.is_empty() {
            return self.genesis_hash.clone();
        }
        let leaves: Vec<String> = self.events.iter().map(|e| e.hash.clone()).collect();
        compute_merkle_root(&leaves)
    }

    pub fn get_stats(&self) -> &LedgerStats {
        &self.stats
    }

    pub fn get_config(&self) -> &LedgerConfig {
        &self.config
    }

    pub fn get_recent_events(&self, limit: usize) -> Vec<&ReputationEvent> {
        self.events.iter().rev().take(limit).collect()
    }

    pub fn get_node_events(&self, node_id: &str) -> Vec<&ReputationEvent> {
        self.events
            .iter()
            .filter(|e| e.node_id == node_id)
            .collect()
    }
}

impl Default for ReputationLedgerV2 {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Crypto Utilities (Simulated) ───

fn compute_hash(data: &str) -> String {
    let mut h: u64 = 5381;
    for byte in data.bytes() {
        h = h.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{:016x}", h)
}

fn compute_signature(event_id: &str, node_id: &str, score: f64) -> String {
    let data = format!("{}:{}:{}", event_id, node_id, score);
    compute_hash(&data)
}

fn compute_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return compute_hash("empty");
    }
    if leaves.len() == 1 {
        return leaves[0].clone();
    }
    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        for chunk in current.chunks(2) {
            let combined = if chunk.len() == 2 {
                format!("{}{}", chunk[0], chunk[1])
            } else {
                chunk[0].clone()
            };
            next.push(compute_hash(&combined));
        }
        current = next;
    }
    current
        .into_iter()
        .next()
        .unwrap_or_else(|| compute_hash("none"))
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_creation() {
        let ledger = ReputationLedgerV2::with_defaults();
        assert_eq!(ledger.get_stats().total_events, 0);
    }

    #[test]
    fn test_record_event() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        let event = ledger
            .record_event(
                "e1".to_string(),
                "n1".to_string(),
                EventType::Contribution,
                10.0,
            )
            .unwrap();
        assert_eq!(event.event_id, "e1");
        assert_eq!(ledger.get_stats().total_events, 1);
    }

    #[test]
    fn test_duplicate_event() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                "n1".to_string(),
                EventType::Contribution,
                10.0,
            )
            .unwrap();
        let result = ledger.record_event(
            "e1".to_string(),
            "n1".to_string(),
            EventType::Contribution,
            5.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_chain_verification() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                "n1".to_string(),
                EventType::Contribution,
                10.0,
            )
            .unwrap();
        ledger
            .record_event("e2".to_string(), "n1".to_string(), EventType::Review, 5.0)
            .unwrap();
        ledger.verify_chain().unwrap();
    }

    #[test]
    fn test_profile_update() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                "n1".to_string(),
                EventType::Contribution,
                10.0,
            )
            .unwrap();
        let profile = ledger.get_profile("n1").unwrap();
        assert!((profile.total_score - 10.0).abs() < 0.01);
        assert_eq!(profile.event_count, 1);
    }

    #[test]
    fn test_merkle_root() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                "n1".to_string(),
                EventType::Contribution,
                10.0,
            )
            .unwrap();
        ledger
            .record_event("e2".to_string(), "n2".to_string(), EventType::Review, 5.0)
            .unwrap();
        let root = ledger.compute_merkle_root();
        assert!(!root.is_empty());
    }

    #[test]
    fn test_event_type_display() {
        let t = EventType::Contribution;
        assert_eq!(t.to_string(), "contribution");
    }

    #[test]
    fn test_signature_verification() {
        let event = ReputationEvent::new(
            "e1".to_string(),
            "n1".to_string(),
            EventType::Contribution,
            10.0,
            "prev".to_string(),
        );
        assert!(event.verify_signature());
    }

    #[test]
    fn test_score_clamping() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                "n1".to_string(),
                EventType::Contribution,
                1500.0,
            )
            .unwrap();
        let profile = ledger.get_profile("n1").unwrap();
        assert!(profile.total_score <= 1000.0);
    }

    #[test]
    fn test_recent_events() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        for i in 0..10 {
            ledger
                .record_event(
                    format!("e{}", i),
                    "n1".to_string(),
                    EventType::Contribution,
                    1.0,
                )
                .unwrap();
        }
        let recent = ledger.get_recent_events(5);
        assert_eq!(recent.len(), 5);
    }

    #[test]
    fn test_node_events() {
        let mut ledger = ReputationLedgerV2::with_defaults();
        ledger
            .record_event(
                "e1".to_string(),
                "n1".to_string(),
                EventType::Contribution,
                1.0,
            )
            .unwrap();
        ledger
            .record_event("e2".to_string(), "n2".to_string(), EventType::Review, 1.0)
            .unwrap();
        assert_eq!(ledger.get_node_events("n1").len(), 1);
    }

    #[test]
    fn test_error_display() {
        let e = LedgerError::InvalidSignature("x".to_string());
        assert!(!e.to_string().is_empty());
    }
}
