//! Omni-Node — Unified Pillar Integration with SCT Guard Supervision.
//!
//! Integrates the 4 Evolutionary Pillars (Corpuscular, Maieutic, Steganographic, Resonance)
//! under absolute Stuartian Context Tensor (SCT) supervision. Every inter-pillar communication
//! must pass through the SCT Guard with Z >= 0 for ethical approval.
//!
//! **Architecture Principles:**
//! - Symbiotic integration: pillars cooperate, never compete.
//! - SCT Guard Supreme: Z < 0 means automatic ethical rejection.
//! - CE Ledger: Existential Credit tracking for cooperative merit.
//! - Zero telemetry: biometric data remains LOCAL_ONLY.
//!
//! **Reference:** Sprint 47 — Omni-Node Integration & Symbiotic Ignition Sequence

use crate::orchestration::{PillarId, PillarResponse, PillarStatus};
#[cfg(any(feature = "v1.4-sprint1", feature = "v3.0-wasm-runtime", feature = "v3.0-pillar-messaging", feature = "v3.0-privacy-guard"))]
use crate::runtime::pillar_messaging::PillarMessage;
#[cfg(feature = "v2.1-sct-core")]
use crate::alignment::sct_core::{StuartianTensor, SCTDecision};
use std::collections::HashMap;

/// Maximum SCT Z-score threshold for ethical approval.
const SCT_Z_THRESHOLD: f32 = 0.0;

/// Errors in Omni-Node routing and integration.
#[derive(Debug, Clone)]
pub enum RoutingError {
    /// SCT ethical rejection: Z < 0.
    EthicalRejection { z: f32 },
    /// Source pillar not registered.
    SourceNotRegistered(PillarId),
    /// Target pillar not registered.
    TargetNotRegistered(PillarId),
    /// Insufficient CE for inter-pillar operation.
    InsufficientCE,
    /// Invalid Ed25519 signature.
    InvalidSignature,
    /// Channel send failure.
    ChannelClosed,
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::EthicalRejection { z } => {
                write!(f, "SCT ethical rejection: Z = {:.3} < {:.3}", z, SCT_Z_THRESHOLD)
            }
            RoutingError::SourceNotRegistered(id) => write!(f, "Source pillar not registered: {}", id),
            RoutingError::TargetNotRegistered(id) => write!(f, "Target pillar not registered: {}", id),
            RoutingError::InsufficientCE => write!(f, "Insufficient CE for inter-pillar operation"),
            RoutingError::InvalidSignature => write!(f, "Invalid Ed25519 signature"),
            RoutingError::ChannelClosed => write!(f, "Message channel closed"),
        }
    }
}

/// Symbiosis Validator — SCT Guard for inter-pillar communication.
///
/// Every message between pillars must pass through this validator.
/// Z < 0 results in automatic ethical rejection and audit logging.
#[derive(Debug, Clone)]
pub struct SymbiosisValidator {
    /// Minimum Z-score for ethical approval.
    pub z_threshold: f32,
    /// Audit log of rejected trajectories.
    pub rejection_log: Vec<RejectionRecord>,
}

/// Record of an SCT ethical rejection.
#[derive(Debug, Clone)]
pub struct RejectionRecord {
    /// Source pillar of the rejected message.
    pub from: PillarId,
    /// Target pillar of the rejected message.
    pub to: PillarId,
    /// SCT Z-score that triggered rejection.
    pub z_score: f32,
    /// Timestamp of rejection (milliseconds).
    pub timestamp_ms: u64,
}

impl Default for SymbiosisValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbiosisValidator {
    /// Create a new Symbiosis Validator with default threshold (Z >= 0).
    pub fn new() -> Self {
        Self {
            z_threshold: SCT_Z_THRESHOLD,
            rejection_log: Vec::new(),
        }
    }

    /// Create with custom Z threshold.
    pub fn with_threshold(z_threshold: f32) -> Self {
        Self {
            z_threshold,
            rejection_log: Vec::new(),
        }
    }

    /// Validate a trajectory via Stuartian Context Tensor.
    ///
    /// Returns `SCTDecision::Approved` if Z >= threshold,
    /// or `SCTDecision::Rejected` if Z < threshold.
    pub fn validate(&self, tensor: &StuartianTensor) -> SCTDecision {
        if tensor.z >= self.z_threshold {
            SCTDecision::Approved(tensor.z)
        } else {
            SCTDecision::Rejected(tensor.z)
        }
    }

    /// Validate inter-pillar message and record rejection if needed.
    ///
    /// Returns `Ok(())` if ethically approved, or `Err(RoutingError::EthicalRejection)` if rejected.
    pub fn validate_inter_pillar(
        &mut self,
        from: PillarId,
        to: PillarId,
        tensor: &StuartianTensor,
    ) -> Result<(), RoutingError> {
        let decision = self.validate(tensor);
        match decision {
            SCTDecision::Approved(_) => Ok(()),
            SCTDecision::Rejected(z) => {
                self.rejection_log.push(RejectionRecord {
                    from,
                    to,
                    z_score: z,
                    timestamp_ms: Self::now_ms(),
                });
                Err(RoutingError::EthicalRejection { z })
            }
        }
    }

    /// Get the number of recorded rejections.
    pub fn rejection_count(&self) -> usize {
        self.rejection_log.len()
    }

    /// Get all rejection records.
    pub fn get_rejections(&self) -> &[RejectionRecord] {
        &self.rejection_log
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    #[cfg(target_arch = "wasm32")]
    fn now_ms() -> u64 {
        0
    }
}

/// Existential Credit Ledger — Cooperative merit tracking.
///
/// CE is non-transferable, non-speculative, and represents
/// symbiotic contribution to the network.
#[derive(Debug, Clone)]
pub struct ExistentialCreditLedger {
    /// CE balance per pillar.
    balances: HashMap<PillarId, f64>,
    /// Total CE emitted.
    total_emitted: f64,
    /// Total CE consumed.
    total_consumed: f64,
}

impl Default for ExistentialCreditLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ExistentialCreditLedger {
    /// Create a new CE Ledger with zero balances.
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            total_emitted: 0.0,
            total_consumed: 0.0,
        }
    }

    /// Deposit CE for a pillar.
    pub fn deposit(&mut self, pillar: PillarId, amount: f64) {
        if amount > 0.0 {
            *self.balances.entry(pillar).or_insert(0.0) += amount;
            self.total_emitted += amount;
        }
    }

    /// Withdraw CE from a pillar. Returns `true` if successful.
    pub fn withdraw(&mut self, pillar: PillarId, amount: f64) -> bool {
        if amount <= 0.0 {
            return false;
        }
        let balance = self.balances.entry(pillar).or_insert(0.0);
        if *balance >= amount {
            *balance -= amount;
            self.total_consumed += amount;
            true
        } else {
            false
        }
    }

    /// Get CE balance for a pillar.
    pub fn balance(&self, pillar: PillarId) -> f64 {
        *self.balances.get(&pillar).unwrap_or(&0.0)
    }

    /// Get total CE emitted across all pillars.
    pub fn total_emitted(&self) -> f64 {
        self.total_emitted
    }

    /// Get total CE consumed across all pillars.
    pub fn total_consumed(&self) -> f64 {
        self.total_consumed
    }

    /// Get CE balance for all registered pillars.
    pub fn all_balances(&self) -> HashMap<PillarId, f64> {
        self.balances.clone()
    }
}

/// Symbiotic Router — Inter-pillar message routing with SCT Guard.
///
/// Routes messages between pillars, enforcing SCT validation
/// and CE consumption for every inter-pillar transfer.
#[derive(Debug)]
pub struct SymbioticRouter {
    /// SCT Validator for ethical supervision.
    sct_validator: SymbiosisValidator,
    /// CE Ledger for merit tracking.
    ce_ledger: ExistentialCreditLedger,
    /// Registered pillars.
    registered_pillars: Vec<PillarId>,
}

impl Default for SymbioticRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbioticRouter {
    /// Create a new Symbiotic Router.
    pub fn new() -> Self {
        Self {
            sct_validator: SymbiosisValidator::new(),
            ce_ledger: ExistentialCreditLedger::new(),
            registered_pillars: Vec::new(),
        }
    }

    /// Register a pillar with the router.
    pub fn register_pillar(&mut self, pillar: PillarId, initial_ce: f64) {
        if !self.registered_pillars.contains(&pillar) {
            self.registered_pillars.push(pillar);
            if initial_ce > 0.0 {
                self.ce_ledger.deposit(pillar, initial_ce);
            }
        }
    }

    /// Check if a pillar is registered.
    pub fn is_registered(&self, pillar: PillarId) -> bool {
        self.registered_pillars.contains(&pillar)
    }

    /// Get all registered pillars.
    pub fn registered_pillars(&self) -> &[PillarId] {
        &self.registered_pillars
    }

    /// Route a message between pillars with SCT validation.
    ///
    /// 1. Verify source and target are registered.
    /// 2. Validate SCT tensor (Z >= 0).
    /// 3. Consume CE from source pillar.
    /// 4. Return routing result.
    pub fn route_inter_pillar(
        &mut self,
        from: PillarId,
        to: PillarId,
        message: &PillarMessage,
        sct_tensor: &StuartianTensor,
    ) -> Result<PillarResponse, RoutingError> {
        // Verify source registered
        if !self.is_registered(from) {
            return Err(RoutingError::SourceNotRegistered(from));
        }

        // Verify target registered
        if !self.is_registered(to) {
            return Err(RoutingError::TargetNotRegistered(to));
        }

        // SCT Guard validation
        self.sct_validator
            .validate_inter_pillar(from, to, sct_tensor)?;

        // Verify signature (non-empty)
        if message.signature.is_empty() {
            return Err(RoutingError::InvalidSignature);
        }

        // Consume CE
        if !self.ce_ledger.withdraw(from, message.ce_weight) {
            return Err(RoutingError::InsufficientCE);
        }

        // Route successful
        Ok(PillarResponse {
            data: message.payload.clone(),
            ce_consumed: message.ce_weight,
            sct_z_score: sct_tensor.z,
            status: PillarStatus::Success,
        })
    }

    /// Get the SCT validator.
    pub fn sct_validator(&self) -> &SymbiosisValidator {
        &self.sct_validator
    }

    /// Get the CE ledger.
    pub fn ce_ledger(&self) -> &ExistentialCreditLedger {
        &self.ce_ledger
    }

    /// Get mutable access to the CE ledger.
    pub fn ce_ledger_mut(&mut self) -> &mut ExistentialCreditLedger {
        &mut self.ce_ledger
    }
}

/// Pillar Registry — Maps pillar IDs to their status.
#[derive(Debug, Clone)]
pub struct PillarRegistry {
    pillars: HashMap<PillarId, PillarStatus>,
}

impl Default for PillarRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PillarRegistry {
    /// Create an empty pillar registry.
    pub fn new() -> Self {
        Self {
            pillars: HashMap::new(),
        }
    }

    /// Register a pillar with initial status.
    pub fn register(&mut self, id: PillarId, status: PillarStatus) {
        self.pillars.insert(id, status);
    }

    /// Get the status of a pillar.
    pub fn get_status(&self, id: PillarId) -> Option<PillarStatus> {
        self.pillars.get(&id).cloned()
    }

    /// Update the status of a pillar.
    pub fn update_status(&mut self, id: PillarId, status: PillarStatus) {
        self.pillars.insert(id, status);
    }

    /// Check if a pillar is registered.
    pub fn is_registered(&self, id: PillarId) -> bool {
        self.pillars.contains_key(&id)
    }

    /// Get all registered pillar IDs.
    pub fn pillar_ids(&self) -> Vec<PillarId> {
        self.pillars.keys().cloned().collect()
    }
}

/// Omni-Node — Unified integration of all 4 Evolutionary Pillars.
///
/// The Omni-Node is the central coordination entity that:
/// - Registers and manages all 4 pillars.
/// - Routes inter-pillar messages via SymbioticRouter.
/// - Enforces SCT Guard on every inter-pillar communication.
/// - Tracks CE distribution via ExistentialCreditLedger.
///
/// **SCT Guard Supreme:** No pillar can communicate with another
/// without passing through the SCT Guard. Z < 0 = automatic rejection.
pub struct OmniNode {
    /// Pillar registry.
    pillars: PillarRegistry,
    /// Symbiotic router with SCT Guard.
    router: SymbioticRouter,
    /// SCT validator (reference for direct access).
    sct_guard: SymbiosisValidator,
    /// CE ledger (reference for direct access).
    ce_ledger: ExistentialCreditLedger,
}

impl Default for OmniNode {
    fn default() -> Self {
        Self::new()
    }
}

impl OmniNode {
    /// Create a new Omni-Node with default configuration.
    pub fn new() -> Self {
        Self {
            pillars: PillarRegistry::new(),
            router: SymbioticRouter::new(),
            sct_guard: SymbiosisValidator::new(),
            ce_ledger: ExistentialCreditLedger::new(),
        }
    }

    /// Initialize all 4 Evolutionary Pillars with initial CE.
    pub fn initialize_pillars(&mut self, initial_ce: f64) {
        let all_pillars = [
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            PillarId::SteganographicSurvival,
            PillarId::ResonanceInterface,
        ];

        for pillar in &all_pillars {
            self.pillars.register(*pillar, PillarStatus::Success);
            self.router.register_pillar(*pillar, initial_ce);
        }
    }

    /// Register a single pillar.
    pub fn register_pillar(&mut self, id: PillarId, initial_ce: f64) {
        self.pillars.register(id, PillarStatus::Success);
        self.router.register_pillar(id, initial_ce);
    }

    /// Route a message between pillars with full SCT Guard supervision.
    pub fn route_message(
        &mut self,
        from: PillarId,
        to: PillarId,
        message: &PillarMessage,
        sct_tensor: &StuartianTensor,
    ) -> Result<PillarResponse, RoutingError> {
        let result = self.router.route_inter_pillar(from, to, message, sct_tensor);

        // Update pillar status based on routing result
        match &result {
            Ok(_) => {
                self.pillars.update_status(from, PillarStatus::Success);
                self.pillars.update_status(to, PillarStatus::Success);
            }
            Err(RoutingError::EthicalRejection { z }) => {
                self.pillars.update_status(from, PillarStatus::EthicalRejection(*z));
            }
            Err(_) => {
                self.pillars.update_status(from, PillarStatus::Pending);
            }
        }

        result
    }

    /// Deposit CE into a pillar.
    pub fn deposit_ce(&mut self, pillar: PillarId, amount: f64) {
        self.router.ce_ledger_mut().deposit(pillar, amount);
    }

    /// Get the status of a pillar.
    pub fn get_pillar_status(&self, id: PillarId) -> Option<PillarStatus> {
        self.pillars.get_status(id)
    }

    /// Get all registered pillar IDs.
    pub fn registered_pillars(&self) -> Vec<PillarId> {
        self.pillars.pillar_ids()
    }

    /// Get the SCT validator.
    pub fn sct_guard(&self) -> &SymbiosisValidator {
        &self.sct_guard
    }

    /// Get the CE ledger.
    pub fn ce_ledger(&self) -> &ExistentialCreditLedger {
        self.router.ce_ledger()
    }

    /// Get the number of SCT rejections.
    pub fn rejection_count(&self) -> usize {
        self.router.sct_validator().rejection_count()
    }

    /// Get all rejection records.
    pub fn get_rejections(&self) -> &[RejectionRecord] {
        self.router.sct_validator().get_rejections()
    }

    /// Run a diagnostic check on all pillars.
    ///
    /// Returns a map of pillar ID to diagnostic status.
    pub fn diagnose(&self) -> HashMap<PillarId, String> {
        let mut diagnostics = HashMap::new();
        for pillar in self.registered_pillars() {
            let status = self.get_pillar_status(pillar).unwrap_or(PillarStatus::Pending);
            let ce = self.ce_ledger().balance(pillar);
            diagnostics.insert(
                pillar,
                format!("Status: {:?}, CE: {:.2}", status, ce),
            );
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(pillar_id: PillarId, ce_weight: f64) -> PillarMessage {
        PillarMessage::new(
            b"test payload".to_vec(),
            b"valid_signature".to_vec(),
            pillar_id,
            1000,
            1,
            ce_weight,
        )
    }

    fn make_valid_tensor(z: f32) -> StuartianTensor {
        StuartianTensor { x: 0.7, y: 0.3, z }
    }

    #[test]
    fn test_omni_node_creation() {
        let node = OmniNode::new();
        assert_eq!(node.registered_pillars().len(), 0);
    }

    #[test]
    fn test_initialize_all_pillars() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);
        assert_eq!(node.registered_pillars().len(), 4);
        assert_eq!(node.ce_ledger().balance(PillarId::CorpuscularBridge), 100.0);
    }

    #[test]
    fn test_route_message_valid() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        let msg = make_message(PillarId::MaieuticSynthesizer, 5.0);
        let tensor = make_valid_tensor(0.5);

        let result = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, PillarStatus::Success);
        assert_eq!(response.ce_consumed, 5.0);
    }

    #[test]
    fn test_route_message_sct_rejection() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        let msg = make_message(PillarId::ResonanceInterface, 5.0);
        let tensor = make_valid_tensor(-0.5);

        let result = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::ResonanceInterface,
            &msg,
            &tensor,
        );
        assert!(result.is_err());
        match result {
            Err(RoutingError::EthicalRejection { z }) => assert!(z < 0.0),
            _ => unreachable!(),
        }
        assert_eq!(node.rejection_count(), 1);
    }

    #[test]
    fn test_route_message_insufficient_ce() {
        let mut node = OmniNode::new();
        node.initialize_pillars(10.0);

        let msg = make_message(PillarId::MaieuticSynthesizer, 50.0);
        let tensor = make_valid_tensor(0.5);

        let result = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );
        assert!(matches!(result, Err(RoutingError::InsufficientCE)));
    }

    #[test]
    fn test_route_message_invalid_signature() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        let msg = PillarMessage::new(
            b"test".to_vec(),
            vec![], // Empty signature
            PillarId::MaieuticSynthesizer,
            1000,
            1,
            5.0,
        );
        let tensor = make_valid_tensor(0.5);

        let result = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );
        assert!(matches!(result, Err(RoutingError::InvalidSignature)));
    }

    #[test]
    fn test_route_message_unregistered_target() {
        let mut node = OmniNode::new();
        node.register_pillar(PillarId::CorpuscularBridge, 100.0);

        let msg = make_message(PillarId::MaieuticSynthesizer, 5.0);
        let tensor = make_valid_tensor(0.5);

        let result = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );
        assert!(matches!(result, Err(RoutingError::TargetNotRegistered(_))));
    }

    #[test]
    fn test_ce_ledger_deposit_withdraw() {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.deposit(PillarId::CorpuscularBridge, 100.0);
        assert_eq!(ledger.balance(PillarId::CorpuscularBridge), 100.0);

        assert!(ledger.withdraw(PillarId::CorpuscularBridge, 30.0));
        assert_eq!(ledger.balance(PillarId::CorpuscularBridge), 70.0);
        assert_eq!(ledger.total_emitted(), 100.0);
        assert_eq!(ledger.total_consumed(), 30.0);
    }

    #[test]
    fn test_ce_ledger_withdraw_insufficient() {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.deposit(PillarId::CorpuscularBridge, 10.0);
        assert!(!ledger.withdraw(PillarId::CorpuscularBridge, 50.0));
        assert_eq!(ledger.balance(PillarId::CorpuscularBridge), 10.0);
    }

    #[test]
    fn test_symbiosis_validator_approved() {
        let validator = SymbiosisValidator::new();
        let tensor = make_valid_tensor(0.5);
        let decision = validator.validate(&tensor);
        assert!(matches!(decision, SCTDecision::Approved(0.5)));
    }

    #[test]
    fn test_symbiosis_validator_rejected() {
        let mut validator = SymbiosisValidator::new();
        let tensor = make_valid_tensor(-0.3);
        let decision = validator.validate(&tensor);
        assert!(matches!(decision, SCTDecision::Rejected(-0.3)));

        // Test inter-pillar validation
        let result = validator.validate_inter_pillar(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &tensor,
        );
        assert!(result.is_err());
        assert_eq!(validator.rejection_count(), 1);
    }

    #[test]
    fn test_custom_threshold() {
        let validator = SymbiosisValidator::with_threshold(0.5);
        let tensor_low = make_valid_tensor(0.3);
        let tensor_high = make_valid_tensor(0.7);

        assert!(matches!(validator.validate(&tensor_low), SCTDecision::Rejected(_)));
        assert!(matches!(validator.validate(&tensor_high), SCTDecision::Approved(_)));
    }

    #[test]
    fn test_pillar_registry() {
        let mut registry = PillarRegistry::new();
        registry.register(PillarId::CorpuscularBridge, PillarStatus::Success);
        assert!(registry.is_registered(PillarId::CorpuscularBridge));
        assert!(!registry.is_registered(PillarId::MaieuticSynthesizer));
        assert_eq!(
            registry.get_status(PillarId::CorpuscularBridge),
            Some(PillarStatus::Success)
        );
    }

    #[test]
    fn test_diagnose() {
        let mut node = OmniNode::new();
        node.initialize_pillars(50.0);
        let diagnostics = node.diagnose();
        assert_eq!(diagnostics.len(), 4);
        for (_, diag) in &diagnostics {
            assert!(diag.contains("CE: 50.00"));
        }
    }

    #[test]
    fn test_deposit_ce() {
        let mut node = OmniNode::new();
        node.initialize_pillars(0.0);
        node.deposit_ce(PillarId::CorpuscularBridge, 200.0);
        assert_eq!(node.ce_ledger().balance(PillarId::CorpuscularBridge), 200.0);
    }

    #[test]
    fn test_rejection_record() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        let msg = make_message(PillarId::ResonanceInterface, 5.0);
        let tensor = make_valid_tensor(-0.8);

        let _ = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::ResonanceInterface,
            &msg,
            &tensor,
        );

        let rejections = node.get_rejections();
        assert_eq!(rejections.len(), 1);
        let record = &rejections[0];
        assert_eq!(record.from, PillarId::CorpuscularBridge);
        assert_eq!(record.to, PillarId::ResonanceInterface);
        assert_eq!(record.z_score, -0.8);
    }

    #[test]
    fn test_default() {
        let node = OmniNode::default();
        assert_eq!(node.registered_pillars().len(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = RoutingError::EthicalRejection { z: -0.5 };
        let msg = format!("{}", err);
        assert!(msg.contains("ethical rejection"));

        let err = RoutingError::InsufficientCE;
        let msg = format!("{}", err);
        assert!(msg.contains("CE"));
    }
}
