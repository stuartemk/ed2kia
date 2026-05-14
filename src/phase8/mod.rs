//! Phase 8 – Feature-gated re-exports for Fase 8
//!
//! Sprint 1: Marketplace, UI Backend, SLO Engine (`phase8-sprint1`)
//! Sprint 2: Cross-Model Scaling, Continuous Alignment, SLA Enforcer (`phase8-sprint2`)
//!
//! Version: `0.8.0-alpha.2`

// ---------------------------------------------------------------------------
// Sprint 1: Marketplace + UI Backend + SLO Engine
// ---------------------------------------------------------------------------

#[cfg(feature = "phase8-sprint1")]
pub mod marketplace {
    #[allow(unused_imports)]
    pub mod engine {
        pub use crate::marketplace::engine::{
            MarketResult, MarketplaceError, NodeTrustInfo, ResourceListing,
            ResourceMarketplace, ResourceRequest,
        };
    }
}

#[cfg(feature = "phase8-sprint1")]
pub mod ui {
    #[allow(unused_imports)]
    pub mod backend {
        pub use crate::ui::backend::{
            AlignmentStreamEvent, create_router, FederationStatus, RealtimeMetrics,
            UiBackendState, UIResponse,
        };
    }
}

#[cfg(feature = "phase8-sprint1")]
pub mod slo {
    #[allow(unused_imports)]
    pub mod engine {
        pub use crate::slo::engine::{
            DegradationAction, SLOConfig, SLOEngine, SLOError, SLOStatus, SLOResult,
        };
    }
}

// ---------------------------------------------------------------------------
// Sprint 2: Cross-Model Scaling + Continuous Alignment + SLA Enforcer
// ---------------------------------------------------------------------------

#[cfg(feature = "phase8-sprint2")]
pub mod scaling {
    #[allow(unused_imports)]
    pub mod cross_model {
        pub use crate::scaling::cross_model::{
            CrossModelScaler, NodeCapacity, RoutingRequest, ScaleResult, ScalingError, ScalingStats,
        };
    }
}

#[cfg(feature = "phase8-sprint2")]
pub mod alignment {
    #[allow(unused_imports)]
    pub mod continuous {
        pub use crate::alignment::continuous::{
            AlignmentLoopError, AlignmentLoopResult, ContinuousAlignmentLoop,
            ContinuousFeedback, LoopConfig,
        };
    }
}

#[cfg(feature = "phase8-sprint2")]
pub mod slo_enforcer {
    #[allow(unused_imports)]
    pub mod enforcer {
        pub use crate::slo::enforcer::{
            DegradationLevel, EnforcerConfig, EnforcerError, EnforcementResult,
            OpsNotification, SloStatusRecord, SLAEnforcer,
        };
    }
}

// ---------------------------------------------------------------------------
// Sprint identifier
// ---------------------------------------------------------------------------

/// Returns the current Phase 8 sprint identifier.
// FIX: Add return statements to const functions (E0308)
// CLEANUP: Fixed unreachable expressions and needless returns in const fn
pub const fn sprint_identifier() -> &'static str {
    #[cfg(feature = "phase8-sprint2")]
    {
        "phase8-sprint2"
    }
    #[cfg(all(not(feature = "phase8-sprint2"), feature = "phase8-sprint1"))]
    {
        "phase8-sprint1"
    }
    #[cfg(not(any(feature = "phase8-sprint2", feature = "phase8-sprint1")))]
    {
        "phase8"
    }
}

/// Returns the current Phase 8 version.
// CLEANUP: Fixed unreachable expressions and needless returns in const fn
pub const fn version() -> &'static str {
    #[cfg(feature = "phase8-sprint2")]
    {
        "0.8.0-alpha.2"
    }
    #[cfg(all(not(feature = "phase8-sprint2"), feature = "phase8-sprint1"))]
    {
        "0.8.0-alpha.1"
    }
    #[cfg(not(any(feature = "phase8-sprint2", feature = "phase8-sprint1")))]
    {
        "0.8.0-alpha"
    }
}

/// List of enabled Phase 8 features based on compile-time feature flags.
pub fn enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    #[cfg(feature = "phase8-sprint1")]
    {
        features.push("phase8-sprint1");
    }
    #[cfg(feature = "phase8-sprint2")]
    {
        features.push("phase8-sprint2");
    }
    features
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprint_identifier() {
        #[cfg(feature = "phase8-sprint2")]
        assert_eq!(sprint_identifier(), "phase8-sprint2");
        #[cfg(all(feature = "phase8-sprint1", not(feature = "phase8-sprint2")))]
        assert_eq!(sprint_identifier(), "phase8-sprint1");
    }

    #[test]
    fn test_version() {
        #[cfg(feature = "phase8-sprint2")]
        assert_eq!(version(), "0.8.0-alpha.2");
        #[cfg(all(feature = "phase8-sprint1", not(feature = "phase8-sprint2")))]
        assert_eq!(version(), "0.8.0-alpha.1");
    }

    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        #[cfg(feature = "phase8-sprint2")]
        assert!(features.contains(&"phase8-sprint2"));
        #[cfg(feature = "phase8-sprint1")]
        assert!(features.contains(&"phase8-sprint1"));
    }
}
