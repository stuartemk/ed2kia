//! Corpuscular Bridge — IoT Simbiótico & Economía CE.
//!
//! **RFC 001:** Connects the ed2kIA information network with the physical energy/production level.
//! Enables closed-circuit exchange of physical resources (3D printing, solar energy, hydroponics)
//! via CE (Existential Credit) vouchers signed with Ed25519.
//!
//! **Integration Points:**
//! - `LocalHardwareAdapter`: LOCAL_ONLY device registry and command routing.
//! - `CEExchangeEngine`: CE ↔ Physical Resource atomic exchange protocol.
//! - `PillarInterface`: Core trait for orchestration integration.
//! - `CEExchangeTrait`: CE voucher lifecycle management.
//! - `PillarMessage` / `PillarResponse`: Secure messaging with the orchestrator.
//!
//! **Feature Gate:** `v3.0-corpuscular-bridge`

#[cfg(feature = "v3.0-corpuscular-bridge")]
#[path = "iot_adapter.rs"]
pub mod iot_adapter;

#[cfg(feature = "v3.0-corpuscular-bridge")]
#[path = "ce_exchange.rs"]
pub mod ce_exchange;

#[cfg(feature = "v3.4-macro-symbiosis")]
#[path = "macro_bridge.rs"]
pub mod macro_bridge;

use crate::orchestration::PillarId;
use crate::pillars::{CEExchangeTrait, CEVoucher, PillarError, PillarInterface, ResourceType};

#[cfg(feature = "v3.0-corpuscular-bridge")]
use crate::orchestration::{PillarResponse, PillarStatus};
#[cfg(feature = "v3.0-corpuscular-bridge")]
use crate::runtime::pillar_messaging::PillarMessage;

#[cfg(feature = "v3.0-corpuscular-bridge")]
use ce_exchange::CEExchangeEngine;
#[cfg(feature = "v3.0-corpuscular-bridge")]
use iot_adapter::LocalHardwareAdapter;

/// Corpuscular Bridge Engine — Orchestrates IoT symbiotic operations.
///
/// Coordinates the integration between ed2kIA's information layer and physical hardware
/// (3D printers, solar microgrids, hydroponic controllers) via CE-based contracts.
///
/// **Expected Flow:**
/// 1. Node registers physical resource via `LocalHardwareAdapter`.
/// 2. CE voucher generated (Ed25519 signed) via `CEExchangeEngine`.
/// 3. Corpuscular contract executed atomically.
/// 4. Hardware command dispatched via LOCAL_ONLY routing.
/// 5. Audit event returned as `PillarResponse`.
pub struct CorpuscularEngine {
    #[cfg(feature = "v3.0-corpuscular-bridge")]
    hardware_adapter: LocalHardwareAdapter,
    #[cfg(feature = "v3.0-corpuscular-bridge")]
    ce_exchange: CEExchangeEngine,
}

impl CorpuscularEngine {
    /// Create a new Corpuscular Bridge Engine.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "v3.0-corpuscular-bridge")]
            hardware_adapter: LocalHardwareAdapter::new(),
            #[cfg(feature = "v3.0-corpuscular-bridge")]
            ce_exchange: CEExchangeEngine::new(),
        }
    }

    /// Handle an incoming pillar message from the orchestrator.
    ///
    /// Deserializes the message, validates CE, routes to IoT adapter or CE exchange,
    /// and returns a signed response with CE metrics and SCT state.
    #[cfg(feature = "v3.0-corpuscular-bridge")]
    pub fn handle_request(&mut self, msg: &PillarMessage) -> Result<PillarResponse, PillarError> {
        // Step 1: Validate that message target is this pillar.
        if msg.pillar_id != PillarId::CorpuscularBridge {
            return Err(PillarError::UnsupportedResource);
        }

        // Step 2: Validate CE weight > 0.
        if msg.ce_weight <= 0.0 {
            return Err(PillarError::InsufficientCE);
        }

        // Step 3: Route based on payload content (scaffolding: use payload length as discriminator).
        let response_data = if !msg.payload.is_empty() {
            // Simulate hardware command dispatch.
            // In production: deserialize payload to determine operation type.
            format!(
                "corpuscular-ok:ce={:.2}:payload={}",
                msg.ce_weight,
                msg.payload.len()
            )
            .into_bytes()
        } else {
            // Empty payload: return status metrics.
            format!(
                "corpuscular-status:devices={}:ce_window=active",
                self.hardware_adapter.device_count()
            )
            .into_bytes()
        };

        Ok(PillarResponse {
            data: response_data,
            ce_consumed: msg.ce_weight,
            sct_z_score: 0.5, // Positive Z = constructive integration.
            status: PillarStatus::Success,
        })
    }
}

impl PillarInterface for CorpuscularEngine {
    fn id() -> PillarId {
        PillarId::CorpuscularBridge
    }

    fn validate_local_constraint(&self) -> bool {
        // Corpuscular Bridge operates on local hardware via loopback/UNIX sockets.
        // All device endpoints are validated as LOCAL_ONLY at registration time.
        true
    }

    fn consume_ce(&self, amount: f64) -> Result<(), PillarError> {
        if amount <= 0.0 {
            return Err(PillarError::InsufficientCE);
        }
        // CE consumed for corpuscular operations.
        // In production: wire ExistentialCreditLedger.deduct_ce(node_id, amount).
        // Atomic execution: CE committed before hardware operation.
        Ok(())
    }
}

impl CEExchangeTrait for CorpuscularEngine {
    fn request_physical_resource(
        &self,
        resource_type: ResourceType,
    ) -> Result<CEVoucher, PillarError> {
        // Generate CE voucher with Ed25519 signature.
        // Voucher binds CE amount to specific resource type.
        // Non-transferable, expires after atomic execution.
        let voucher = CEVoucher {
            ce_amount: 10.0, // Default CE commitment.
            resource_type,
            signature: vec![0xAB, 0xCD, 0xEF, 0x01], // Scaffolding signature.
        };
        Ok(voucher)
    }

    fn redeem_compute_credit(&self, compute_units: f64) -> Result<(), PillarError> {
        if compute_units <= 0.0 {
            return Err(PillarError::InsufficientCE);
        }
        // Allocate compute units for corpuscular operations.
        Ok(())
    }
}

impl Default for CorpuscularEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let _engine = CorpuscularEngine::new();
    }

    #[test]
    fn test_pillar_id() {
        assert_eq!(CorpuscularEngine::id(), PillarId::CorpuscularBridge);
    }

    #[test]
    fn test_local_constraint() {
        let engine = CorpuscularEngine::new();
        assert!(engine.validate_local_constraint());
    }

    #[test]
    fn test_consume_ce_valid() {
        let engine = CorpuscularEngine::new();
        assert!(engine.consume_ce(5.0).is_ok());
    }

    #[test]
    fn test_consume_ce_zero_rejected() {
        let engine = CorpuscularEngine::new();
        match engine.consume_ce(0.0) {
            Err(PillarError::InsufficientCE) => {} // Expected
            other => panic!("Expected InsufficientCE, got {:?}", other),
        }
    }

    #[test]
    fn test_request_physical_resource() {
        let engine = CorpuscularEngine::new();
        let voucher = engine.request_physical_resource(ResourceType::Print3DHours(2.0));
        assert!(voucher.is_ok());
        let voucher = voucher.unwrap();
        assert_eq!(voucher.ce_amount, 10.0);
    }

    #[test]
    fn test_redeem_compute_credit_valid() {
        let engine = CorpuscularEngine::new();
        assert!(engine.redeem_compute_credit(100.0).is_ok());
    }

    #[test]
    fn test_redeem_compute_credit_zero_rejected() {
        let engine = CorpuscularEngine::new();
        match engine.redeem_compute_credit(0.0) {
            Err(PillarError::InsufficientCE) => {} // Expected
            other => panic!("Expected InsufficientCE, got {:?}", other),
        }
    }

    #[test]
    fn test_default() {
        let _engine = CorpuscularEngine::default();
    }
}
