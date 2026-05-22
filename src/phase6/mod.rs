//! Phase 6 – Feature-gated re-exports for Fase 6 (Sprint 1 + Sprint 2)
//!
//! This module provides conditional re-exports for the Fase 6 components:
//! - Sprint 1: `interoperability::TensorAdapter`, `federation::FedAvgAggregator`
//! - Sprint 2: `interoperability::OnnxAdapter`, `api::AuthValidator`, `staking::ResourceRegistry`
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `phase6-core` | Core Sprint 1 modules (adapter + aggregator) |
//! | `phase6-sprint2` | Sprint 2 modules (ONNX adapter, auth, staking, API v2) |
//! | `phase6-experimental` | Full Fase 6 (includes `phase6-core` + `phase6-sprint2`) |
//!
//! # Usage
//!
//! ```rust,ignore
//! #[cfg(feature = "phase6-core")]
//! use ed2kia::phase6::{TensorAdapter, FedAvgAggregator};
//!
//! #[cfg(feature = "phase6-sprint2")]
//! use ed2kia::phase6::{OnnxAdapter, AuthValidator, ResourceRegistry};
//! ```

// ---------------------------------------------------------------------------
// Versioning
// ---------------------------------------------------------------------------

/// Phase 6 core module version
pub const PHASE6_CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Phase 6 Sprint 2 version
#[cfg(feature = "phase6-sprint2")]
pub const PHASE6_SPRINT2_VERSION: &str = "0.6.0-alpha.2";

/// Sprint identifier
#[cfg(not(feature = "phase6-sprint2"))]
pub const SPRINT: &str = "sprint1";

/// Sprint identifier (includes Sprint 2)
#[cfg(feature = "phase6-sprint2")]
pub const SPRINT: &str = "sprint2";

// ---------------------------------------------------------------------------
// Re-exports (feature gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "phase6-core")]
pub mod interoperability {
    // Re-exports for phase6-core consumers
    // Use `allow(unused_imports)` since these are public API re-exports
    #[allow(unused_imports)]
    pub use crate::interoperability::adapter::{
        AdapterError, NormalizedHiddenState, SourceModel, TensorAdapter,
    };
}

#[cfg(feature = "phase6-core")]
pub mod federation {
    // Re-exports for phase6-core consumers
    #[allow(unused_imports)]
    pub use crate::federation::avg_aggregator::{
        AggregationResult, FedAvgAggregator, FedAvgConfig, WeightUpdate,
    };
}

// ---------------------------------------------------------------------------
// Sprint 2 Re-exports (feature gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "phase6-sprint2")]
pub mod onnx {
    // Re-exports for phase6-sprint2 ONNX adapter
    #[allow(unused_imports)]
    pub use crate::interoperability::onnx_adapter::{
        OnnxAdapter, OnnxAdapterConfig, OnnxConversionResult, OnnxError,
    };
}

#[cfg(feature = "phase6-sprint2")]
pub mod auth {
    // Re-exports for phase6-sprint2 auth
    #[allow(unused_imports)]
    pub use crate::api::auth::{AuthConfig, AuthError, AuthValidator, SignatureValidationResult};
}

#[cfg(feature = "phase6-sprint2")]
pub mod staking {
    // Re-exports for phase6-sprint2 staking
    #[allow(unused_imports)]
    pub use crate::staking::registry::{
        NodeStatus, RegistryStats, ResourceCommitment, ResourceRegistry,
    };
}

// ---------------------------------------------------------------------------
// Public API (always available for type compatibility)
// ---------------------------------------------------------------------------

/// Re-export SourceModel for serialization without phase6-core
#[cfg(not(feature = "phase6-core"))]
pub use crate::interoperability::adapter::SourceModel;

/// Re-export WeightUpdate for serialization without phase6-core
#[cfg(not(feature = "phase6-core"))]
pub use crate::federation::avg_aggregator::WeightUpdate;

// ---------------------------------------------------------------------------
// Feature detection helpers
// ---------------------------------------------------------------------------

/// Check if phase6-core is enabled at runtime
pub fn is_phase6_core_enabled() -> bool {
    cfg!(feature = "phase6-core")
}

/// Check if phase6-sprint2 is enabled at runtime
pub fn is_phase6_sprint2_enabled() -> bool {
    cfg!(feature = "phase6-sprint2")
}

/// Check if phase6-experimental is enabled at runtime
pub fn is_phase6_experimental_enabled() -> bool {
    cfg!(feature = "phase6-experimental")
}

/// Get the list of enabled phase6 features
pub fn enabled_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut features = vec![
        #[cfg(feature = "phase6-core")]
        "phase6-core",
    ];
    #[cfg(feature = "phase6-sprint2")]
    features.push("phase6-sprint2");
    #[cfg(feature = "phase6-experimental")]
    features.push("phase6-experimental");
    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_not_empty() {
        assert!(!PHASE6_CORE_VERSION.is_empty());
    }

    #[test]
    fn test_sprint_identifier() {
        #[cfg(feature = "phase6-sprint2")]
        assert_eq!(SPRINT, "sprint2");
        #[cfg(not(feature = "phase6-sprint2"))]
        assert_eq!(SPRINT, "sprint1");
    }

    #[test]
    fn test_feature_detection() {
        // phase6-core should be enabled in this test context
        #[cfg(feature = "phase6-core")]
        assert!(is_phase6_core_enabled());

        #[cfg(not(feature = "phase6-core"))]
        assert!(!is_phase6_core_enabled());
    }

    #[test]
    fn test_sprint2_detection() {
        #[cfg(feature = "phase6-sprint2")]
        assert!(is_phase6_sprint2_enabled());

        #[cfg(not(feature = "phase6-sprint2"))]
        assert!(!is_phase6_sprint2_enabled());
    }

    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        #[cfg(feature = "phase6-core")]
        assert!(features.contains(&"phase6-core"));
        #[cfg(feature = "phase6-sprint2")]
        assert!(features.contains(&"phase6-sprint2"));
    }
}
