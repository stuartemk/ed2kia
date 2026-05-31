//! Integration Contracts — Unified Traits for Evolutionary Pillars.
//!
//! Defines the trait-based interface that all 4 pillars must implement
//! for seamless integration with the ed2kIA orchestration layer.
//!
//! **Design Principles:**
//! - Symbiotic cooperation between pillars and ed2kIA core.
//! - CE (Existential Credit) as the sole merit metric — zero Babylonian financial logic.
//! - Zero-Knowledge privacy: biometric data processed locally (WASM/Edge).
//! - Absolute transparency: all operations auditable via SCT Z-score.
//!
//! **Reference:** Sprint 41 — Cross-Pillar Orchestration

use crate::orchestration::PillarId;

/// Errors shared across pillar operations.
#[derive(Debug, Clone)]
pub enum PillarError {
    /// CE balance insufficient for the requested operation.
    InsufficientCE,
    /// Operation rejected by SCT ethical evaluation (Z < 0).
    EthicalRejection(f32),
    /// WASM execution failure (local compute only).
    WasmExecution(String),
    /// Telemetry violation — data attempted to leave local boundary.
    TelemetryViolation,
    /// Resource type not supported by this pillar.
    UnsupportedResource,
}

impl std::fmt::Display for PillarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PillarError::InsufficientCE => {
                write!(f, "Insufficient Existential Credit for operation")
            }
            PillarError::EthicalRejection(z) => {
                write!(f, "SCT ethical rejection: Z = {:.3} < 0", z)
            }
            PillarError::WasmExecution(msg) => write!(f, "WASM execution error: {}", msg),
            PillarError::TelemetryViolation => {
                write!(f, "Telemetry violation: data must remain LOCAL_ONLY")
            }
            PillarError::UnsupportedResource => {
                write!(f, "Resource type not supported by this pillar")
            }
        }
    }
}

/// CE Voucher — Non-transferable merit credential for physical resource exchange.
///
/// **Invariant:** CE is a measure of symbiotic contribution, not financial value.
/// Signed via Ed25519 for cooperative verification.
#[derive(Debug, Clone)]
pub struct CEVoucher {
    /// Amount of CE committed to this voucher.
    pub ce_amount: f64,
    /// Resource type this voucher authorizes.
    pub resource_type: ResourceType,
    /// Ed25519 signature for cooperative verification.
    pub signature: Vec<u8>,
}

/// Physical or computational resource types exchangeable via CE.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    /// 3D printing hours (Corpuscular Bridge).
    Print3DHours(f32),
    /// Solar energy kWh (Corpuscular Bridge).
    SolarEnergyKwh(f32),
    /// Hydroponic yield units (Corpuscular Bridge).
    HydroponicUnits(f32),
    /// Distributed compute cycles (Maieutic Synthesizer).
    ComputeCycles(f64),
    /// Scientific hypothesis tokens (Maieutic Synthesizer).
    HypothesisToken,
    /// Custom resource type.
    Custom(String),
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::Print3DHours(h) => write!(f, "3DPrint:{}h", h),
            ResourceType::SolarEnergyKwh(k) => write!(f, "Solar:{}kWh", k),
            ResourceType::HydroponicUnits(u) => write!(f, "Hydroponic:{}u", u),
            ResourceType::ComputeCycles(c) => write!(f, "Compute:{}cycles", c),
            ResourceType::HypothesisToken => write!(f, "HypothesisToken"),
            ResourceType::Custom(name) => write!(f, "Custom:{}", name),
        }
    }
}

/// **Core Trait:** Unified interface for all Evolutionary Pillars.
///
/// Every pillar engine must implement this trait to integrate with
/// the `PillarOrchestrator` routing layer.
///
/// **Contract Requirements:**
/// - `id()`: Returns the pillar's unique identifier.
/// - `validate_local_constraint()`: Ensures LOCAL_ONLY compliance (critical for Resonance).
/// - `consume_ce()`: Deducts CE from the node's symbiotic merit balance.
///
/// **Reference:** RFCs 001-004 (Sprint 40)
pub trait PillarInterface {
    /// Return the unique identifier for this pillar.
    fn id() -> PillarId;

    /// Validate that this pillar respects its local execution constraints.
    ///
    /// For the Resonance Interface (RFC 004), this MUST return `true` only
    /// when executing in a LOCAL_ONLY WASM/Edge environment. Zero telemetry.
    fn validate_local_constraint(&self) -> bool;

    /// Consume CE (Existential Credit) for a symbiotic operation.
    ///
    /// **Invariant:** CE is non-transferable merit. No wallets, no swaps, no fiat.
    /// Operations with CE <= 0 are rejected to preserve cooperative equilibrium.
    fn consume_ce(&self, amount: f64) -> Result<(), PillarError>;
}

/// **WASM/Edge Trait:** Local computation interface for biometric & scientific workloads.
///
/// Enforces the Zero-Knowledge privacy constraint: all sensitive data
/// (biometric, epigenetic, molecular) must be processed locally via WASM.
///
/// **Reference:** RFC 002 (Maieutic Synthesizer), RFC 004 (Resonance Interface)
#[cfg(target_arch = "wasm32")]
pub trait LocalComputeTrait {
    /// Execute a WASM module with the provided input bytes.
    ///
    /// **LOCAL_ONLY:** Input and output data must never leave the local execution boundary.
    /// Biometric data (face, voice, rPPG) is strictly confined to the edge/browser runtime.
    fn execute_wasm(&self, input: &[u8]) -> Result<Vec<u8>, PillarError>;

    /// Verify that zero telemetry is enforced for this compute context.
    ///
    /// Returns `true` if the execution environment guarantees no external data transmission.
    fn ensure_zero_telemetry(&self) -> bool;
}

/// **CE Exchange Trait:** Interface for corpuscular resource integration.
///
/// Manages the CE ↔ Physical Resource exchange defined in RFC 001.
/// All operations are atomic: CE is committed before resource execution,
/// with automatic refund on cooperative failure.
///
/// **Reference:** RFC 001 (Corpuscular Bridge)
pub trait CEExchangeTrait {
    /// Request a physical resource commitment via CE voucher.
    ///
    /// Returns a `CEVoucher` signed with Ed25519 for cooperative verification.
    /// The voucher is non-transferable and expires after atomic execution.
    fn request_physical_resource(
        &self,
        resource_type: ResourceType,
    ) -> Result<CEVoucher, PillarError>;

    /// Redeem compute credits (CE) for distributed scientific computation.
    ///
    /// Authorizes the Maieutic Synthesizer to allocate `compute_units`
    /// for hypothesis generation, molecular simulation, or protein folding.
    fn redeem_compute_credit(&self, compute_units: f64) -> Result<(), PillarError>;
}
