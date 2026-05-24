//! Pillar Router — Request routing, CE/SCT validation & endpoint dispatch.
//!
//! Orchestrates the flow of requests from the ed2kIA core to the appropriate
//! Evolutionary Pillar endpoint (local WASM, edge, or remote).
//!
//! **Reference:** Sprint 41 — Cross-Pillar Orchestration

use std::collections::HashMap;

/// Unique identifier for each Evolutionary Pillar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PillarId {
    /// RFC 001: Corpuscular Bridge — IoT Simbiótico & Economía CE
    CorpuscularBridge,
    /// RFC 002: Maieutic Synthesizer — Motor de Sabiduría
    MaieuticSynthesizer,
    /// RFC 003: Steganographic Survival — Preservación de Red
    SteganographicSurvival,
    /// RFC 004: Resonance Interface — Biorretroalimentación
    ResonanceInterface,
}

impl std::fmt::Display for PillarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PillarId::CorpuscularBridge => write!(f, "corpuscular-bridge"),
            PillarId::MaieuticSynthesizer => write!(f, "maieutic-synthesizer"),
            PillarId::SteganographicSurvival => write!(f, "steganographic-survival"),
            PillarId::ResonanceInterface => write!(f, "resonance-interface"),
        }
    }
}

/// Execution environment for pillar endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PillarEndpoint {
    /// Local WASM module (browser or edge runtime).
    /// Enforces LOCAL_ONLY constraint for biometric data.
    LocalWasm,
    /// Edge server (wasm32-wasi target).
    Edge,
    /// Remote P2P node (libp2p stream).
    Remote(String),
}

/// Incoming payload routed to a pillar.
///
/// All payloads must be Ed25519-signed to ensure cooperative integrity.
#[derive(Debug, Clone)]
pub struct PillarPayload {
    /// Node ID of the requester.
    pub requester_id: String,
    /// Raw payload data (Protobuf/JSON encoded).
    pub data: Vec<u8>,
    /// Ed25519 signature over `data`.
    pub signature: Vec<u8>,
    /// Associated CE cost for this operation.
    pub ce_cost: f64,
}

/// Response from a pillar execution.
#[derive(Debug, Clone)]
pub struct PillarResponse {
    /// Result data from the pillar.
    pub data: Vec<u8>,
    /// CE consumed during execution.
    pub ce_consumed: f64,
    /// SCT Z-score of the operation (Z > 0 = constructive).
    pub sct_z_score: f32,
    /// Execution status.
    pub status: PillarStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PillarStatus {
    Success,
    Pending,
    EthicalRejection(f32),
    InsufficientCE,
    InvalidSignature,
}

/// Errors in cross-pillar orchestration.
#[derive(Debug, Clone)]
pub enum OrchestrationError {
    PillarNotFound(PillarId),
    InvalidSignature,
    InsufficientCE,
    EthicalRejection(f32),
    EndpointUnavailable(PillarId),
}

impl std::fmt::Display for OrchestrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestrationError::PillarNotFound(id) => write!(f, "Pillar not found: {}", id),
            OrchestrationError::InvalidSignature => write!(f, "Invalid Ed25519 signature"),
            OrchestrationError::InsufficientCE => write!(f, "Insufficient Existential Credit (CE > 0 required)"),
            OrchestrationError::EthicalRejection(z) => write!(f, "Ethical rejection: SCT Z-score {:.3} < 0", z),
            OrchestrationError::EndpointUnavailable(id) => write!(f, "Endpoint unavailable for pillar: {}", id),
        }
    }
}

/// Central orchestrator for cross-pillar integration.
///
/// Coordinates request routing, CE/SCT validation, and endpoint dispatch
/// across the 4 Evolutionary Pillars of ed2kIA v3.0.
///
/// **Integration Points:**
/// - `ExistentialCreditLedger`: CE balance verification.
/// - `SymbiosisValidator`: SCT Z-score evaluation.
/// - `PillarRegistry`: Maps PillarId → PillarEndpoint.
///
/// **Reference:** Sprint 41 — Cross-Pillar Orchestration
pub struct PillarOrchestrator {
    /// Registry mapping pillar IDs to their execution endpoints.
    pillar_registry: HashMap<PillarId, PillarEndpoint>,
}

impl PillarOrchestrator {
    /// Create a new orchestrator with an empty pillar registry.
    pub fn new() -> Self {
        Self {
            pillar_registry: HashMap::new(),
        }
    }

    /// Register a pillar endpoint.
    ///
    /// Establishes the integration point between a pillar and its execution environment.
    pub fn register_pillar(&mut self, pillar_id: PillarId, endpoint: PillarEndpoint) {
        self.pillar_registry.insert(pillar_id, endpoint);
    }

    /// Route a request to the appropriate pillar.
    ///
    /// **Validation Flow:**
    /// 1. Verify Ed25519 signature of the payload.
    /// 2. Confirm requester has CE > 0 (via ExistentialCreditLedger).
    /// 3. Evaluate SCT Z-score (Z > 0 required for constructive integration).
    /// 4. Dispatch to registered `PillarEndpoint`.
    /// 5. Return response with CE consumption metrics and SCT state.
    ///
    /// **LOCAL_ONLY Constraint:** Requests to `ResonanceInterface` (RFC 004)
    /// must route to `LocalWasm` endpoint. Zero telemetry enforced.
    ///
    /// TODO: Phase 10 Implementation — Wire CE Ledger, SCT Validator & Ed25519 verification.
    pub fn route_request(
        &self,
        pillar: PillarId,
        payload: &PillarPayload,
    ) -> Result<PillarResponse, OrchestrationError> {
        // Step 1: Verify pillar is registered.
        let endpoint = self.pillar_registry.get(&pillar)
            .ok_or(OrchestrationError::PillarNotFound(pillar))?;

        // Step 2: Validate Ed25519 signature.
        // TODO: Wire ed25519_dalek verification against requester's public key.
        if payload.signature.is_empty() {
            return Err(OrchestrationError::InvalidSignature);
        }

        // Step 3: Verify CE > 0.
        // TODO: Query ExistentialCreditLedger for requester_id balance.
        if payload.ce_cost <= 0.0 {
            return Err(OrchestrationError::InsufficientCE);
        }

        // Step 4: SCT Z-score evaluation.
        // TODO: Evaluate payload through SymbiosisValidator → ensure Z > 0.
        let sct_z_score = 0.0; // Placeholder — pending SCT integration.

        // Step 5: Enforce LOCAL_ONLY for Resonance Interface.
        if pillar == PillarId::ResonanceInterface {
            match endpoint {
                PillarEndpoint::LocalWasm => {}, // ✅ Compliant — biometric data stays local.
                _ => return Err(OrchestrationError::EndpointUnavailable(pillar)),
            }
        }

        // Step 6: Dispatch to endpoint.
        // TODO: Route to WASM module, edge server, or P2P stream based on endpoint type.
        Ok(PillarResponse {
            data: Vec::new(),
            ce_consumed: payload.ce_cost,
            sct_z_score,
            status: PillarStatus::Success,
        })
    }

    /// Retrieve the registered endpoint for a pillar.
    pub fn get_endpoint(&self, pillar: PillarId) -> Option<&PillarEndpoint> {
        self.pillar_registry.get(&pillar)
    }

    /// List all registered pillars.
    pub fn registered_pillars(&self) -> Vec<PillarId> {
        self.pillar_registry.keys().cloned().collect()
    }
}

impl Default for PillarOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_payload() -> PillarPayload {
        PillarPayload {
            requester_id: "test-node-001".to_string(),
            data: vec![1, 2, 3],
            signature: vec![42; 64], // Mock Ed25519 signature.
            ce_cost: 1.0,
        }
    }

    #[test]
    fn test_orchestrator_creation() {
        let orch = PillarOrchestrator::new();
        assert!(orch.registered_pillars().is_empty());
    }

    #[test]
    fn test_register_and_get_endpoint() {
        let mut orch = PillarOrchestrator::new();
        orch.register_pillar(PillarId::CorpuscularBridge, PillarEndpoint::Edge);
        assert!(orch.get_endpoint(PillarId::CorpuscularBridge).is_some());
    }

    #[test]
    fn test_route_request_pillar_not_found() {
        let orch = PillarOrchestrator::new();
        let result = orch.route_request(PillarId::CorpuscularBridge, &test_payload());
        assert!(matches!(result, Err(OrchestrationError::PillarNotFound(_))));
    }

    #[test]
    fn test_route_request_invalid_signature() {
        let mut orch = PillarOrchestrator::new();
        orch.register_pillar(PillarId::CorpuscularBridge, PillarEndpoint::Edge);
        let payload = PillarPayload {
            signature: vec![], // Empty signature.
            ..test_payload()
        };
        let result = orch.route_request(PillarId::CorpuscularBridge, &payload);
        assert!(matches!(result, Err(OrchestrationError::InvalidSignature)));
    }

    #[test]
    fn test_route_request_insufficient_ce() {
        let mut orch = PillarOrchestrator::new();
        orch.register_pillar(PillarId::CorpuscularBridge, PillarEndpoint::Edge);
        let payload = PillarPayload {
            ce_cost: 0.0,
            ..test_payload()
        };
        let result = orch.route_request(PillarId::CorpuscularBridge, &payload);
        assert!(matches!(result, Err(OrchestrationError::InsufficientCE)));
    }

    #[test]
    fn test_resonance_local_only_enforcement() {
        let mut orch = PillarOrchestrator::new();
        // Resonance must use LocalWasm — remote endpoint rejected.
        orch.register_pillar(PillarId::ResonanceInterface, PillarEndpoint::Remote("remote".into()));
        let result = orch.route_request(PillarId::ResonanceInterface, &test_payload());
        assert!(matches!(result, Err(OrchestrationError::EndpointUnavailable(_))));
    }

    #[test]
    fn test_resonance_local_wasm_success() {
        let mut orch = PillarOrchestrator::new();
        orch.register_pillar(PillarId::ResonanceInterface, PillarEndpoint::LocalWasm);
        let result = orch.route_request(PillarId::ResonanceInterface, &test_payload());
        assert!(result.is_ok());
    }

    #[test]
    fn test_pillar_id_display() {
        assert_eq!(format!("{}", PillarId::CorpuscularBridge), "corpuscular-bridge");
        assert_eq!(format!("{}", PillarId::MaieuticSynthesizer), "maieutic-synthesizer");
        assert_eq!(format!("{}", PillarId::SteganographicSurvival), "steganographic-survival");
        assert_eq!(format!("{}", PillarId::ResonanceInterface), "resonance-interface");
    }

    #[test]
    fn test_error_display() {
        let err = OrchestrationError::InsufficientCE;
        assert!(!format!("{}", err).is_empty());
    }
}
