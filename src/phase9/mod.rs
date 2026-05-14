//! Phase 9 – Feature-gated re-exports for Fase 9 Sprint 1
//!
//! Liquid Governance + Real-Time UI + Async ZKP Federation

#![cfg(feature = "phase9-sprint1")]

// ─── Governance ───────────────────────────────────────────────────────────────

#[allow(unused_imports)]
pub mod governance {
    #[allow(unused_imports)]
    pub mod liquid {
        // FIX: Use governance_v2 module path (E0432)
        pub use crate::governance_v2::liquid::{
            Delegation, GovernanceConfig, GovernanceError, GovernanceResult,
            GovernanceStats, LiquidGovernance, NodeProfile, Proposal, SybilCluster,
        };
    }
}

// ─── UI ───────────────────────────────────────────────────────────────────────

#[allow(unused_imports)]
pub mod ui {
    #[allow(unused_imports)]
    pub mod realtime {
        // FIX: Use ui_v2 module path (E0432)
        pub use crate::ui_v2::realtime::{
            EventType, RealtimeError, RealtimeEvent, RealtimeStats,
            RealtimeUIBackend, SessionState, WsResult,
        };
    }
}

// ─── Federation ───────────────────────────────────────────────────────────────

#[allow(unused_imports)]
pub mod federation {
    #[allow(unused_imports)]
    pub mod async_zkp {
        // FIX: Use federation_v3 module path (E0432)
        pub use crate::federation_v3::async_zkp::{
            AsyncZKPFederation, DeltaProof, MerkleProof, ZKPConfig,
            ZKPError, ZKPResult, ZKPStats,
        };
    }
}

// ─── Sprint Metadata ──────────────────────────────────────────────────────────

/// Phase 9 Sprint 1 sprint identifier.
pub const fn sprint_identifier() -> &'static str {
    "phase9-sprint1"
}

/// Phase 9 Sprint 1 version string.
pub const fn version() -> &'static str {
    "0.9.0-alpha.1"
}

/// Returns the list of enabled features for Phase 9 Sprint 1.
pub fn enabled_features() -> Vec<&'static str> {
    vec![
        "liquid_governance",
        "realtime_ui_websocket",
        "async_zkp_federation",
        "sybil_detection",
        "merkle_fallback",
        "rate_limiting",
    ]
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn test_sprint_identifier() {
        assert_eq!(super::sprint_identifier(), "phase9-sprint1");
    }

    #[test]
    fn test_version() {
        assert_eq!(super::version(), "0.9.0-alpha.1");
    }

    #[test]
    fn test_enabled_features() {
        let features = super::enabled_features();
        assert!(features.contains(&"liquid_governance"));
        assert!(features.contains(&"realtime_ui_websocket"));
        assert!(features.contains(&"async_zkp_federation"));
        assert!(features.contains(&"sybil_detection"));
        assert!(features.contains(&"merkle_fallback"));
        assert!(features.contains(&"rate_limiting"));
    }
}
