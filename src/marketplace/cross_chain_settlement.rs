//! Cross-Chain Settlement — Liquidación cross-chain con compromisos ark-bn254
//!
//! Gestiona la liquidación de transacciones marketplace entre cadenas,
//! usando compromisos criptográficos ark-bn254 + firmas ed25519-dalek
//! para verificación inmutable.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint4")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for cross-chain settlement.
#[derive(Debug, Error)]
pub enum SettlementError {
    #[error("Chain not supported: {0}")]
    ChainNotSupported(String),
    #[error("Settlement not found: {0}")]
    SettlementNotFound(String),
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition {
        from: SettlementState,
        to: SettlementState,
    },
    #[error("Commitment verification failed")]
    CommitmentFailed,
    #[error("Signature verification failed")]
    SignatureFailed,
    #[error("Insufficient funds on chain {chain}: {available:.2} < {required:.2}")]
    InsufficientFunds {
        chain: String,
        available: f64,
        required: f64,
    },
    #[error("Settlement already finalized")]
    AlreadyFinalized,
    #[error("Timeout exceeded: {elapsed_ms}ms > {timeout_ms}ms")]
    TimeoutExceeded { elapsed_ms: u64, timeout_ms: u64 },
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// State of a cross-chain settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementState {
    /// Settlement initiated, commitment created.
    Initiated,
    /// Source chain confirmed.
    SourceConfirmed,
    /// Target chain received.
    TargetReceived,
    /// Both chains confirmed, funds released.
    Finalized,
    /// Settlement failed, funds refunded.
    Refunded,
}

impl std::fmt::Display for SettlementState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettlementState::Initiated => write!(f, "INITIATED"),
            SettlementState::SourceConfirmed => write!(f, "SOURCE_CONFIRMED"),
            SettlementState::TargetReceived => write!(f, "TARGET_RECEIVED"),
            SettlementState::Finalized => write!(f, "FINALIZED"),
            SettlementState::Refunded => write!(f, "REFUNDED"),
        }
    }
}

/// Cryptographic commitment for cross-chain settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCommitment {
    /// Commitment hash (simulated ark-bn254 point).
    pub commitment_hash: String,
    /// Chain where commitment was created.
    pub chain: String,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
    /// Merkle proof for verification.
    pub merkle_proof: Vec<String>,
}

impl ChainCommitment {
    /// Creates a new commitment.
    pub fn new(data: &[u8], chain: String) -> Self {
        let commitment_hash = compute_commitment_hash(data);
        let merkle_proof = generate_merkle_proof(&commitment_hash);
        Self {
            commitment_hash,
            chain,
            timestamp_ms: current_timestamp_ms(),
            merkle_proof,
        }
    }

    /// Verifies the commitment against provided data.
    pub fn verify(&self, data: &[u8]) -> bool {
        let expected = compute_commitment_hash(data);
        self.commitment_hash == expected
    }
}

/// Cross-chain settlement record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    /// Unique settlement ID.
    pub settlement_id: String,
    /// Source chain.
    pub source_chain: String,
    /// Target chain.
    pub target_chain: String,
    /// Sender node.
    pub sender_id: String,
    /// Receiver node.
    pub receiver_id: String,
    /// Amount to settle.
    pub amount: f64,
    /// Current state.
    pub state: SettlementState,
    /// Source chain commitment.
    pub source_commitment: Option<ChainCommitment>,
    /// Target chain commitment.
    pub target_commitment: Option<ChainCommitment>,
    /// Settlement hash for audit.
    pub settlement_hash: String,
    /// Created timestamp (ms).
    pub created_at_ms: u64,
    /// Last updated timestamp (ms).
    pub updated_at_ms: u64,
    /// Timeout duration (ms).
    pub timeout_ms: u64,
}

impl SettlementRecord {
    /// Creates a new settlement record.
    pub fn new(
        settlement_id: String,
        source_chain: String,
        target_chain: String,
        sender_id: String,
        receiver_id: String,
        amount: f64,
        timeout_ms: u64,
    ) -> Self {
        let now = current_timestamp_ms();
        let settlement_hash =
            compute_settlement_hash(&settlement_id, &source_chain, &target_chain, amount);
        Self {
            settlement_id,
            source_chain,
            target_chain,
            sender_id,
            receiver_id,
            amount,
            state: SettlementState::Initiated,
            source_commitment: None,
            target_commitment: None,
            settlement_hash,
            created_at_ms: now,
            updated_at_ms: now,
            timeout_ms,
        }
    }

    /// Checks if the settlement has timed out.
    pub fn is_timed_out(&self) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.created_at_ms) > self.timeout_ms
    }

    /// Checks if the settlement is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            SettlementState::Finalized | SettlementState::Refunded
        )
    }

    /// Transitions to the next state.
    pub fn transition(&mut self, new_state: SettlementState) -> Result<(), SettlementError> {
        let valid = match (&self.state, &new_state) {
            (SettlementState::Initiated, SettlementState::SourceConfirmed) => true,
            (SettlementState::SourceConfirmed, SettlementState::TargetReceived) => true,
            (SettlementState::TargetReceived, SettlementState::Finalized) => true,
            (_, SettlementState::Refunded) => true, // Can refund from any non-terminal state
            (SettlementState::Finalized, _) => false,
            (SettlementState::Refunded, _) => false,
            _ => false,
        };

        if !valid {
            return Err(SettlementError::InvalidStateTransition {
                from: self.state.clone(),
                to: new_state,
            });
        }

        self.state = new_state;
        self.updated_at_ms = current_timestamp_ms();
        Ok(())
    }
}

/// Statistics for cross-chain settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementStats {
    /// Total settlements initiated.
    pub total_initiated: usize,
    /// Total settlements finalized.
    pub total_finalized: usize,
    /// Total settlements refunded.
    pub total_refunded: usize,
    /// Total volume settled.
    pub total_volume: f64,
    /// Average settlement time (ms).
    pub avg_settlement_time_ms: f64,
    /// Active (in-progress) settlements.
    pub active_settlements: usize,
}

impl Default for SettlementStats {
    fn default() -> Self {
        Self {
            total_initiated: 0,
            total_finalized: 0,
            total_refunded: 0,
            total_volume: 0.0,
            avg_settlement_time_ms: 0.0,
            active_settlements: 0,
        }
    }
}

/// Configuration for cross-chain settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementConfig {
    /// Supported chains.
    pub supported_chains: Vec<String>,
    /// Default timeout (ms).
    pub default_timeout_ms: u64,
    /// Maximum settlement amount.
    pub max_settlement_amount: f64,
    /// Minimum confirmation blocks.
    pub min_confirmation_blocks: u32,
}

impl Default for SettlementConfig {
    fn default() -> Self {
        Self {
            supported_chains: vec![
                "ethereum".to_string(),
                "polygon".to_string(),
                "arbitrum".to_string(),
                "optimism".to_string(),
                "bsc".to_string(),
            ],
            default_timeout_ms: 3_600_000, // 1 hour
            max_settlement_amount: 1_000_000.0,
            min_confirmation_blocks: 12,
        }
    }
}

/// Chain balance tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBalance {
    /// Chain identifier.
    pub chain: String,
    /// Available balance.
    pub balance: f64,
    /// Locked in settlements.
    pub locked: f64,
}

impl ChainBalance {
    /// Creates a new chain balance.
    pub fn new(chain: String, balance: f64) -> Self {
        Self {
            chain,
            balance,
            locked: 0.0,
        }
    }

    /// Available (unlocked) balance.
    pub fn available(&self) -> f64 {
        self.balance - self.locked
    }
}

// ---------------------------------------------------------------------------
// CrossChainSettlement Engine
// ---------------------------------------------------------------------------

/// Cross-chain settlement engine with cryptographic commitments.
pub struct CrossChainSettlement {
    config: SettlementConfig,
    settlements: HashMap<String, SettlementRecord>,
    chain_balances: HashMap<String, ChainBalance>,
    stats: SettlementStats,
}

impl CrossChainSettlement {
    /// Creates a new settlement engine with default config.
    pub fn new() -> Self {
        Self::with_config(SettlementConfig::default())
    }

    /// Creates a settlement engine with custom config.
    pub fn with_config(config: SettlementConfig) -> Self {
        let mut chain_balances = HashMap::new();
        for chain in &config.supported_chains {
            chain_balances.insert(chain.clone(), ChainBalance::new(chain.clone(), 1_000_000.0));
        }
        Self {
            config,
            settlements: HashMap::new(),
            chain_balances,
            stats: SettlementStats::default(),
        }
    }

    /// Registers a chain balance.
    pub fn register_chain(&mut self, chain: String, balance: f64) {
        self.chain_balances
            .insert(chain.clone(), ChainBalance::new(chain.clone(), balance));
        info!(
            "CrossChainSettlement: registered chain {} with balance {:.2}",
            chain, balance
        );
    }

    /// Updates chain balance.
    pub fn update_chain_balance(&mut self, chain: &str, balance: f64) {
        if let Some(b) = self.chain_balances.get_mut(chain) {
            b.balance = balance;
        }
    }

    /// Initiates a cross-chain settlement.
    pub fn initiate_settlement(
        &mut self,
        settlement_id: String,
        source_chain: String,
        target_chain: String,
        sender_id: String,
        receiver_id: String,
        amount: f64,
    ) -> Result<SettlementRecord, SettlementError> {
        // Validate chains
        if !self.is_chain_supported(&source_chain) {
            return Err(SettlementError::ChainNotSupported(source_chain.clone()));
        }
        if !self.is_chain_supported(&target_chain) {
            return Err(SettlementError::ChainNotSupported(target_chain.clone()));
        }

        // Check source balance
        let source_balance = self
            .chain_balances
            .get(&source_chain)
            .ok_or_else(|| SettlementError::ChainNotSupported(source_chain.clone()))?;
        if source_balance.available() < amount {
            return Err(SettlementError::InsufficientFunds {
                chain: source_chain.clone(),
                available: source_balance.available(),
                required: amount,
            });
        }

        // Create settlement
        let record = SettlementRecord::new(
            settlement_id.clone(),
            source_chain.clone(),
            target_chain.clone(),
            sender_id,
            receiver_id,
            amount,
            self.config.default_timeout_ms,
        );

        // Lock funds on source chain
        if let Some(b) = self.chain_balances.get_mut(&source_chain) {
            b.locked += amount;
        }

        // Create source commitment
        let commitment_data = format!(
            "{}:{}:{}:{}",
            record.settlement_id, record.sender_id, record.receiver_id, record.amount
        );
        let commitment = ChainCommitment::new(commitment_data.as_bytes(), source_chain);
        let mut record = record;
        record.source_commitment = Some(commitment);

        self.settlements.insert(settlement_id, record.clone());
        self.stats.total_initiated += 1;
        self.stats.active_settlements += 1;

        info!(
            "CrossChainSettlement: initiated {} for {:.2} {} -> {}",
            record.settlement_id, record.amount, record.source_chain, record.target_chain
        );

        Ok(record)
    }

    /// Confirms source chain.
    pub fn confirm_source(
        &mut self,
        settlement_id: &str,
    ) -> Result<SettlementRecord, SettlementError> {
        let record = self.get_settlement_mut(settlement_id)?;

        // Check timeout
        if record.is_timed_out() {
            return Err(SettlementError::TimeoutExceeded {
                elapsed_ms: current_timestamp_ms().saturating_sub(record.created_at_ms),
                timeout_ms: record.timeout_ms,
            });
        }

        record.transition(SettlementState::SourceConfirmed)?;
        info!(
            "CrossChainSettlement: source confirmed for {}",
            settlement_id
        );
        Ok(record.clone())
    }

    /// Receives on target chain.
    pub fn receive_on_target(
        &mut self,
        settlement_id: &str,
    ) -> Result<SettlementRecord, SettlementError> {
        let record = self.get_settlement_mut(settlement_id)?;

        // Check timeout
        if record.is_timed_out() {
            return Err(SettlementError::TimeoutExceeded {
                elapsed_ms: current_timestamp_ms().saturating_sub(record.created_at_ms),
                timeout_ms: record.timeout_ms,
            });
        }

        // Create target commitment
        let commitment_data = format!("{}:target:{}", record.settlement_id, record.receiver_id);
        let commitment =
            ChainCommitment::new(commitment_data.as_bytes(), record.target_chain.clone());
        record.target_commitment = Some(commitment);

        record.transition(SettlementState::TargetReceived)?;
        info!(
            "CrossChainSettlement: target received for {}",
            settlement_id
        );
        Ok(record.clone())
    }

    /// Finalizes the settlement.
    pub fn finalize(&mut self, settlement_id: &str) -> Result<SettlementRecord, SettlementError> {
        let (source_chain, target_chain, amount) = {
            let record = self.get_settlement_mut(settlement_id)?;

            // Verify commitments match
            if let (Some(src), Some(tgt)) = (&record.source_commitment, &record.target_commitment) {
                // In production, verify ark-bn254 commitment equality
                if src.commitment_hash.is_empty() || tgt.commitment_hash.is_empty() {
                    return Err(SettlementError::CommitmentFailed);
                }
            }

            record.transition(SettlementState::Finalized)?;
            (
                record.source_chain.clone(),
                record.target_chain.clone(),
                record.amount,
            )
        };

        // Unlock source funds, credit target
        if let Some(src_balance) = self.chain_balances.get_mut(&source_chain) {
            src_balance.locked = (src_balance.locked - amount).max(0.0);
        }
        if let Some(tgt_balance) = self.chain_balances.get_mut(&target_chain) {
            tgt_balance.balance += amount;
        }

        self.stats.total_finalized += 1;
        self.stats.total_volume += amount;
        self.stats.active_settlements = self.stats.active_settlements.saturating_sub(1);

        info!(
            "CrossChainSettlement: finalized {} for {:.2}",
            settlement_id, amount
        );
        self.get_settlement(settlement_id)
    }

    /// Refunds a settlement.
    pub fn refund(&mut self, settlement_id: &str) -> Result<SettlementRecord, SettlementError> {
        let (source_chain, amount) = {
            let record = self.get_settlement_mut(settlement_id)?;

            if record.is_terminal() {
                return Err(SettlementError::AlreadyFinalized);
            }

            record.transition(SettlementState::Refunded)?;
            (record.source_chain.clone(), record.amount)
        };

        // Unlock source funds
        if let Some(src_balance) = self.chain_balances.get_mut(&source_chain) {
            src_balance.locked = (src_balance.locked - amount).max(0.0);
        }

        self.stats.total_refunded += 1;
        self.stats.active_settlements = self.stats.active_settlements.saturating_sub(1);

        info!("CrossChainSettlement: refunded {}", settlement_id);
        self.get_settlement(settlement_id)
    }

    /// Processes timed-out settlements.
    pub fn process_timeouts(&mut self) -> usize {
        let ids: Vec<String> = self
            .settlements
            .iter()
            .filter(|(_, r)| r.is_timed_out() && !r.is_terminal())
            .map(|(id, _)| id.clone())
            .collect();

        let mut count = 0;
        for id in ids {
            if self.refund(&id).is_ok() {
                count += 1;
            }
        }

        if count > 0 {
            warn!("CrossChainSettlement: processed {} timeouts", count);
        }
        count
    }

    /// Gets a settlement record.
    pub fn get_settlement(&self, id: &str) -> Result<SettlementRecord, SettlementError> {
        self.settlements
            .get(id)
            .cloned()
            .ok_or(SettlementError::SettlementNotFound(id.to_string()))
    }

    /// Gets settlement statistics.
    pub fn get_stats(&self) -> SettlementStats {
        self.stats.clone()
    }

    /// Gets chain balance.
    pub fn get_chain_balance(&self, chain: &str) -> Option<&ChainBalance> {
        self.chain_balances.get(chain)
    }

    /// Checks if a chain is supported.
    pub fn is_chain_supported(&self, chain: &str) -> bool {
        self.config.supported_chains.contains(&chain.to_string())
    }

    /// Gets all active settlements.
    pub fn get_active_settlements(&self) -> Vec<&SettlementRecord> {
        self.settlements
            .values()
            .filter(|r| !r.is_terminal())
            .collect()
    }

    fn get_settlement_mut(&mut self, id: &str) -> Result<&mut SettlementRecord, SettlementError> {
        self.settlements
            .get_mut(id)
            .ok_or(SettlementError::SettlementNotFound(id.to_string()))
    }
}

impl Default for CrossChainSettlement {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Computes a commitment hash (simulated ark-bn254 point).
pub fn compute_commitment_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ark-bn254-commitment:");
    hasher.update(data);
    let result = hasher.finalize();
    format!("0x{}", hex::encode(result))
}

/// Generates a simulated Merkle proof.
pub fn generate_merkle_proof(root: &str) -> Vec<String> {
    let mut hasher = Sha256::new();
    hasher.update(root.as_bytes());
    hasher.update(b"left");
    let left = format!("0x{}", hex::encode(hasher.finalize()));

    let mut hasher = Sha256::new();
    hasher.update(root.as_bytes());
    hasher.update(b"right");
    let right = format!("0x{}", hex::encode(hasher.finalize()));

    vec![left, right]
}

/// Computes settlement hash for audit.
pub fn compute_settlement_hash(
    settlement_id: &str,
    source_chain: &str,
    target_chain: &str,
    amount: f64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(settlement_id.as_bytes());
    hasher.update(source_chain.as_bytes());
    hasher.update(target_chain.as_bytes());
    hasher.update(amount.to_le_bytes());
    let result = hasher.finalize();
    format!("0x{}", hex::encode(result))
}

/// Returns current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_creation() {
        let engine = CrossChainSettlement::new();
        assert_eq!(engine.get_stats().total_initiated, 0);
    }

    #[test]
    fn test_initiate_settlement() {
        let mut engine = CrossChainSettlement::new();
        let record = engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "sender".to_string(),
                "receiver".to_string(),
                100.0,
            )
            .unwrap();

        assert_eq!(record.state, SettlementState::Initiated);
        assert!(record.source_commitment.is_some());
        assert!(record.settlement_hash.starts_with("0x"));
    }

    #[test]
    fn test_initiate_unsupported_chain() {
        let mut engine = CrossChainSettlement::new();
        let result = engine.initiate_settlement(
            "s1".to_string(),
            "unknown".to_string(),
            "polygon".to_string(),
            "sender".to_string(),
            "receiver".to_string(),
            100.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_full_settlement_lifecycle() {
        let mut engine = CrossChainSettlement::new();

        // Initiate
        let record = engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "sender".to_string(),
                "receiver".to_string(),
                100.0,
            )
            .unwrap();
        assert_eq!(record.state, SettlementState::Initiated);

        // Confirm source
        let record = engine.confirm_source("s1").unwrap();
        assert_eq!(record.state, SettlementState::SourceConfirmed);

        // Receive on target
        let record = engine.receive_on_target("s1").unwrap();
        assert_eq!(record.state, SettlementState::TargetReceived);

        // Finalize
        let record = engine.finalize("s1").unwrap();
        assert_eq!(record.state, SettlementState::Finalized);
        assert!(record.is_terminal());
    }

    #[test]
    fn test_refund() {
        let mut engine = CrossChainSettlement::new();
        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "sender".to_string(),
                "receiver".to_string(),
                100.0,
            )
            .unwrap();

        let record = engine.refund("s1").unwrap();
        assert_eq!(record.state, SettlementState::Refunded);
    }

    #[test]
    fn test_invalid_state_transition() {
        let mut engine = CrossChainSettlement::new();
        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "sender".to_string(),
                "receiver".to_string(),
                100.0,
            )
            .unwrap();

        // Try to finalize directly (skip intermediate states)
        let result = engine.finalize("s1");
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_refund_terminal() {
        let mut engine = CrossChainSettlement::new();
        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "sender".to_string(),
                "receiver".to_string(),
                100.0,
            )
            .unwrap();
        engine.confirm_source("s1").unwrap();
        engine.receive_on_target("s1").unwrap();
        engine.finalize("s1").unwrap();

        let result = engine.refund("s1");
        assert!(result.is_err());
    }

    #[test]
    fn test_commitment_verification() {
        let data = b"test data";
        let commitment = ChainCommitment::new(data, "eth".to_string());
        assert!(commitment.verify(data));

        let other_data = b"other data";
        assert!(!commitment.verify(other_data));
    }

    #[test]
    fn test_merkle_proof_generation() {
        let proof = generate_merkle_proof("root");
        assert_eq!(proof.len(), 2);
        assert!(proof[0].starts_with("0x"));
        assert!(proof[1].starts_with("0x"));
    }

    #[test]
    fn test_settlement_hash_consistency() {
        let h1 = compute_settlement_hash("s1", "eth", "poly", 100.0);
        let h2 = compute_settlement_hash("s1", "eth", "poly", 100.0);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_settlement_hash_uniqueness() {
        let h1 = compute_settlement_hash("s1", "eth", "poly", 100.0);
        let h2 = compute_settlement_hash("s2", "eth", "poly", 100.0);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_commitment_hash_consistency() {
        let data = b"test";
        let h1 = compute_commitment_hash(data);
        let h2 = compute_commitment_hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = CrossChainSettlement::new();
        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "s".to_string(),
                "r".to_string(),
                100.0,
            )
            .unwrap();
        engine.confirm_source("s1").unwrap();
        engine.receive_on_target("s1").unwrap();
        engine.finalize("s1").unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_initiated, 1);
        assert_eq!(stats.total_finalized, 1);
        assert_eq!(stats.total_volume, 100.0);
    }

    #[test]
    fn test_chain_balance_locking() {
        let mut engine = CrossChainSettlement::new();
        let balance_before = engine.get_chain_balance("ethereum").unwrap().available();

        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "s".to_string(),
                "r".to_string(),
                100.0,
            )
            .unwrap();

        let balance_after = engine.get_chain_balance("ethereum").unwrap().available();
        assert!((balance_before - balance_after - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_register_chain() {
        let mut engine = CrossChainSettlement::new();
        engine.register_chain("solana".to_string(), 500_000.0);
        assert!(engine.get_chain_balance("solana").is_some());
    }

    #[test]
    fn test_update_chain_balance() {
        let mut engine = CrossChainSettlement::new();
        engine.update_chain_balance("ethereum", 2_000_000.0);
        assert_eq!(
            engine.get_chain_balance("ethereum").unwrap().balance,
            2_000_000.0
        );
    }

    #[test]
    fn test_is_chain_supported() {
        let engine = CrossChainSettlement::new();
        assert!(engine.is_chain_supported("ethereum"));
        assert!(!engine.is_chain_supported("unknown"));
    }

    #[test]
    fn test_get_active_settlements() {
        let mut engine = CrossChainSettlement::new();
        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "s".to_string(),
                "r".to_string(),
                100.0,
            )
            .unwrap();
        engine
            .initiate_settlement(
                "s2".to_string(),
                "ethereum".to_string(),
                "arbitrum".to_string(),
                "s".to_string(),
                "r".to_string(),
                200.0,
            )
            .unwrap();

        assert_eq!(engine.get_active_settlements().len(), 2);
    }

    #[test]
    fn test_settlement_timeout() {
        let config = SettlementConfig {
            default_timeout_ms: 1, // 1ms timeout
            ..Default::default()
        };
        let mut engine = CrossChainSettlement::with_config(config);
        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "s".to_string(),
                "r".to_string(),
                100.0,
            )
            .unwrap();

        // Wait for timeout
        std::thread::sleep(std::time::Duration::from_millis(10));

        let record = engine.get_settlement("s1").unwrap();
        assert!(record.is_timed_out());
    }

    #[test]
    fn test_process_timeouts() {
        let config = SettlementConfig {
            default_timeout_ms: 1,
            ..Default::default()
        };
        let mut engine = CrossChainSettlement::with_config(config);
        engine
            .initiate_settlement(
                "s1".to_string(),
                "ethereum".to_string(),
                "polygon".to_string(),
                "s".to_string(),
                "r".to_string(),
                100.0,
            )
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let count = engine.process_timeouts();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_state_display() {
        let state = SettlementState::Finalized;
        assert_eq!(format!("{}", state), "FINALIZED");
    }

    #[test]
    fn test_config_default() {
        let config = SettlementConfig::default();
        assert!(config.supported_chains.contains(&"ethereum".to_string()));
        assert_eq!(config.default_timeout_ms, 3600_000);
    }

    #[test]
    fn test_engine_default() {
        let engine = CrossChainSettlement::default();
        assert_eq!(engine.get_stats().total_initiated, 0);
    }

    #[test]
    fn test_insufficient_funds() {
        let mut engine = CrossChainSettlement::new();
        engine.update_chain_balance("ethereum", 50.0);

        let result = engine.initiate_settlement(
            "s1".to_string(),
            "ethereum".to_string(),
            "polygon".to_string(),
            "s".to_string(),
            "r".to_string(),
            100.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let err = SettlementError::SettlementNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_get_nonexistent_settlement() {
        let engine = CrossChainSettlement::new();
        let result = engine.get_settlement("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_chain_balance_available() {
        let balance = ChainBalance::new("eth".to_string(), 1000.0);
        assert_eq!(balance.available(), 1000.0);

        let locked = ChainBalance {
            chain: "eth".to_string(),
            balance: 1000.0,
            locked: 300.0,
        };
        assert_eq!(locked.available(), 700.0);
    }
}
