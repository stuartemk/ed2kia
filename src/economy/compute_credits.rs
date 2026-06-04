//! Compute Credits — Sprint 82: Tactical Pivot & Distributed SAE Audit MVP
//!
//! Exposes CE (Existence Credits) as Compute Credits for the symbiotic exchange:
//! you provide compute, you receive audit capacity.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v9.18-mvp-deployment` | compute_credits | ComputeCredits — CE as audit currency |

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CreditError {
    InsufficientBalance { available: u64, required: u64 },
    NodeNotFound(u64),
    Overflow,
    InvalidAmount,
}

impl fmt::Display for CreditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreditError::InsufficientBalance {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient balance: {} available, {} required",
                    available, required
                )
            }
            CreditError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            CreditError::Overflow => write!(f, "Credit overflow"),
            CreditError::InvalidAmount => write!(f, "Invalid credit amount"),
        }
    }
}

// ============================================================================
// Credit Ledger Entry
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct CreditEntry {
    pub node_id: u64,
    pub amount: u64,
    pub kind: CreditKind,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreditKind {
    Earned { compute_hash: u64 },
    Spent { audit_id: u64 },
    Reward { reason: String },
}

impl fmt::Display for CreditEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match &self.kind {
            CreditKind::Earned { .. } => "earned",
            CreditKind::Spent { .. } => "spent",
            CreditKind::Reward { .. } => "reward",
        };
        write!(
            f,
            "Credit: node={} amount={} kind={} ts={}",
            self.node_id, self.amount, kind_str, self.timestamp_ms
        )
    }
}

// ============================================================================
// Node Balance
// ============================================================================

#[derive(Debug, Clone)]
pub struct NodeBalance {
    pub node_id: u64,
    pub balance: u64,
    pub total_earned: u64,
    pub total_spent: u64,
}

impl NodeBalance {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            balance: 0,
            total_earned: 0,
            total_spent: 0,
        }
    }
}

impl fmt::Display for NodeBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node {} — Balance: {} CE (Earned: {}, Spent: {})",
            self.node_id, self.balance, self.total_earned, self.total_spent
        )
    }
}

// ============================================================================
// ComputeCredits Engine
// ============================================================================

pub struct ComputeCredits {
    balances: HashMap<u64, NodeBalance>,
    ledger: Vec<CreditEntry>,
}

impl ComputeCredits {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            ledger: Vec::new(),
        }
    }

    /// Register a new node with zero balance.
    pub fn register_node(&mut self, node_id: u64) {
        self.balances.insert(node_id, NodeBalance::new(node_id));
    }

    /// Earn credits by providing compute.
    pub fn earn(
        &mut self,
        node_id: u64,
        amount: u64,
        compute_hash: u64,
        timestamp_ms: u64,
    ) -> Result<CreditEntry, CreditError> {
        if amount == 0 {
            return Err(CreditError::InvalidAmount);
        }

        let entry = CreditEntry {
            node_id,
            amount,
            kind: CreditKind::Earned { compute_hash },
            timestamp_ms,
        };

        let balance = self
            .balances
            .get_mut(&node_id)
            .ok_or(CreditError::NodeNotFound(node_id))?;

        // Check overflow
        if balance.balance.checked_add(amount).is_none() {
            return Err(CreditError::Overflow);
        }

        balance.balance += amount;
        balance.total_earned += amount;

        self.ledger.push(entry.clone());
        Ok(entry)
    }

    /// Spend credits to request an audit.
    pub fn spend(
        &mut self,
        node_id: u64,
        amount: u64,
        audit_id: u64,
        timestamp_ms: u64,
    ) -> Result<CreditEntry, CreditError> {
        if amount == 0 {
            return Err(CreditError::InvalidAmount);
        }

        let balance = self
            .balances
            .get_mut(&node_id)
            .ok_or(CreditError::NodeNotFound(node_id))?;

        if balance.balance < amount {
            return Err(CreditError::InsufficientBalance {
                available: balance.balance,
                required: amount,
            });
        }

        let entry = CreditEntry {
            node_id,
            amount,
            kind: CreditKind::Spent { audit_id },
            timestamp_ms,
        };

        balance.balance -= amount;
        balance.total_spent += amount;

        self.ledger.push(entry.clone());
        Ok(entry)
    }

    /// Grant a reward (e.g., governance bonus).
    pub fn reward(
        &mut self,
        node_id: u64,
        amount: u64,
        reason: String,
        timestamp_ms: u64,
    ) -> Result<CreditEntry, CreditError> {
        if amount == 0 {
            return Err(CreditError::InvalidAmount);
        }

        let entry = CreditEntry {
            node_id,
            amount,
            kind: CreditKind::Reward { reason },
            timestamp_ms,
        };

        let balance = self
            .balances
            .get_mut(&node_id)
            .ok_or(CreditError::NodeNotFound(node_id))?;

        if balance.balance.checked_add(amount).is_none() {
            return Err(CreditError::Overflow);
        }

        balance.balance += amount;
        balance.total_earned += amount;

        self.ledger.push(entry.clone());
        Ok(entry)
    }

    /// Get balance for a node.
    pub fn get_balance(&self, node_id: u64) -> Option<&NodeBalance> {
        self.balances.get(&node_id)
    }

    /// Get the full ledger.
    pub fn ledger(&self) -> &[CreditEntry] {
        &self.ledger
    }

    /// Get total credits in circulation.
    pub fn total_circulation(&self) -> u64 {
        self.balances.values().map(|b| b.balance).sum()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.balances.clear();
        self.ledger.clear();
    }
}

impl Default for ComputeCredits {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ComputeCredits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ComputeCredits(nodes={}, circulation={}, entries={})",
            self.balances.len(),
            self.total_circulation(),
            self.ledger.len()
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ComputeCredits::new();
        assert_eq!(engine.total_circulation(), 0);
        assert!(engine.ledger().is_empty());
    }

    #[test]
    fn test_register_node() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        let balance = engine.get_balance(1).unwrap();
        assert_eq!(balance.balance, 0);
        assert_eq!(balance.node_id, 1);
    }

    #[test]
    fn test_earn_credits() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        let entry = engine.earn(1, 100, 0xABCD, 1000).unwrap();
        assert_eq!(entry.amount, 100);
        assert_eq!(engine.get_balance(1).unwrap().balance, 100);
        assert_eq!(engine.get_balance(1).unwrap().total_earned, 100);
    }

    #[test]
    fn test_earn_zero_amount() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        assert!(engine.earn(1, 0, 0, 1000).is_err());
    }

    #[test]
    fn test_earn_unknown_node() {
        let mut engine = ComputeCredits::new();
        assert!(engine.earn(99, 100, 0, 1000).is_err());
    }

    #[test]
    fn test_spend_credits() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        engine.earn(1, 100, 0xABCD, 1000).unwrap();
        let entry = engine.spend(1, 30, 42, 2000).unwrap();
        assert_eq!(entry.amount, 30);
        assert_eq!(engine.get_balance(1).unwrap().balance, 70);
        assert_eq!(engine.get_balance(1).unwrap().total_spent, 30);
    }

    #[test]
    fn test_spend_insufficient() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        engine.earn(1, 50, 0, 1000).unwrap();
        let result = engine.spend(1, 100, 42, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_spend_zero_amount() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        assert!(engine.spend(1, 0, 42, 2000).is_err());
    }

    #[test]
    fn test_reward() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        let entry = engine
            .reward(1, 50, "governance".to_string(), 3000)
            .unwrap();
        assert_eq!(entry.amount, 50);
        assert_eq!(engine.get_balance(1).unwrap().balance, 50);
    }

    #[test]
    fn test_reward_zero_amount() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        assert!(engine.reward(1, 0, "test".to_string(), 3000).is_err());
    }

    #[test]
    fn test_total_circulation() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        engine.register_node(2);
        engine.earn(1, 100, 0, 1000).unwrap();
        engine.earn(2, 200, 0, 1000).unwrap();
        engine.spend(1, 30, 42, 2000).unwrap();
        assert_eq!(engine.total_circulation(), 270);
    }

    #[test]
    fn test_ledger_grows() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        engine.earn(1, 100, 0, 1000).unwrap();
        engine.spend(1, 50, 42, 2000).unwrap();
        assert_eq!(engine.ledger().len(), 2);
    }

    #[test]
    fn test_reset() {
        let mut engine = ComputeCredits::new();
        engine.register_node(1);
        engine.earn(1, 100, 0, 1000).unwrap();
        engine.reset();
        assert_eq!(engine.total_circulation(), 0);
        assert!(engine.ledger().is_empty());
    }

    #[test]
    fn test_balance_display() {
        let balance = NodeBalance {
            node_id: 1,
            balance: 100,
            total_earned: 150,
            total_spent: 50,
        };
        let display = format!("{}", balance);
        assert!(display.contains("100"));
        assert!(display.contains("150"));
    }

    #[test]
    fn test_entry_display() {
        let entry = CreditEntry {
            node_id: 1,
            amount: 100,
            kind: CreditKind::Earned {
                compute_hash: 0xABCD,
            },
            timestamp_ms: 1000,
        };
        let display = format!("{}", entry);
        assert!(display.contains("earned"));
    }

    #[test]
    fn test_engine_display() {
        let engine = ComputeCredits::new();
        let display = format!("{}", engine);
        assert!(display.contains("ComputeCredits"));
    }

    #[test]
    fn test_error_display() {
        let err = CreditError::InsufficientBalance {
            available: 50,
            required: 100,
        };
        assert!(format!("{}", err).contains("50"));
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = ComputeCredits::new();

        // Register nodes
        engine.register_node(1);
        engine.register_node(2);

        // Node 1 earns credits
        engine.earn(1, 1000, 0xDEAD, 1000).unwrap();
        assert_eq!(engine.get_balance(1).unwrap().balance, 1000);

        // Node 1 spends credits for audit
        engine.spend(1, 200, 100, 2000).unwrap();
        assert_eq!(engine.get_balance(1).unwrap().balance, 800);

        // Node 2 earns credits
        engine.earn(2, 500, 0xBEEF, 3000).unwrap();

        // Governance reward
        engine
            .reward(1, 100, "stewardship".to_string(), 4000)
            .unwrap();
        assert_eq!(engine.get_balance(1).unwrap().balance, 900);

        // Verify circulation
        assert_eq!(engine.total_circulation(), 1400);

        // Verify ledger
        assert_eq!(engine.ledger().len(), 4);
    }
}
