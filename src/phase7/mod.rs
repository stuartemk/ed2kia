//! Phase 7 – Continuous Alignment Engine + Cross-Net Federation Bridge + Sprint 2 Extensions
//!
//! This module provides feature-gated re-exports for Fase 7 Sprint 1 and Sprint 2,
//! including Alignment Engine, Federation Bridge, Feedback Loop, Dynamic Trust Scorer,
//! and Schema Registry modules.
//!
//! **Version:** `0.7.0-alpha.2`
//!
//! # Features
//!
//! - `phase7-sprint1`: Enables Continuous Alignment Engine and Federation Bridge
//! - `phase7-sprint2`: Enables Feedback Loop, Dynamic Trust Scorer, and Schema Registry
//!
//! # Usage
//!
//! ```rust,ignore
//! #[cfg(feature = "phase7-sprint1")]
//! use ed2kia::phase7::{
//!     alignment::{AlignmentScorer, AlignmentConfig, AlignmentResult},
//!     federation::{FederationBridge, NetworkIdentity, DeltaUpdate},
//! };
//!
//! #[cfg(feature = "phase7-sprint2")]
//! use ed2kia::phase7::{
//!     alignment::feedback_loop::AlignmentFeedbackLoop,
//!     federation::trust_scoring::DynamicTrustScorer,
//!     interoperability::schema_registry::SchemaRegistry,
//! };
//! ```

/// Semantic version for Phase 7 Sprint 2
pub const VERSION: &str = "0.7.0-alpha.2";

/// Sprint 1 identifier
pub const SPRINT1_IDENTIFIER: &str = "phase7-sprint1";

/// Sprint 2 identifier
pub const SPRINT2_IDENTIFIER: &str = "phase7-sprint2";

#[cfg(feature = "phase7-sprint1")]
pub mod alignment {
    /// Re-exports for the Alignment Engine module
    pub mod engine {
        pub use crate::alignment::engine::{
            AlignmentConfig,
            AlignmentError,
            AlignmentFeedback,
            AlignmentResult,
            AlignmentScorer,
        };
    }

    /// Re-exports for Alignment Engine tests (dev-only)
    #[cfg(test)]
    pub mod tests {
        use crate::alignment::tests;
    }
}

#[cfg(feature = "phase7-sprint1")]
pub mod federation {
    /// Re-exports for the Federation Bridge module
    pub mod bridge {
        pub use crate::federation::bridge::{
            BridgeError,
            BridgeResult,
            DeltaUpdate,
            FederationBridge,
            HandshakeMessage,
            NetworkIdentity,
            TrustRecord,
        };
    }

    /// Re-exports for Federation Bridge tests (dev-only)
    #[cfg(test)]
    pub mod tests {
        use crate::federation::tests;
    }
}

/// Sprint 2 modules: Feedback Loop, Dynamic Trust, Schema Registry
#[cfg(feature = "phase7-sprint2")]
pub mod sprint2 {
    /// Re-exports for Alignment Feedback Loop
    pub mod feedback_loop {
        pub use crate::alignment::feedback_loop::{
            AlignmentFeedbackLoop,
            FeedbackLoopConfig,
            FeedbackLoopError,
            LoopResult,
            AuditEntry,
            AuditAction,
            AuditResult,
        };
    }

    /// Re-exports for Dynamic Trust Scoring
    pub mod trust_scoring {
        pub use crate::federation::trust_scoring::{
            DynamicTrustScorer,
            TrustConfig,
            TrustResult,
            TrustScoringError,
            NodeTrustRecord,
            NodeStatus,
            SybilCluster,
            TrustStats,
        };
    }

    /// Re-exports for Schema Registry
    pub mod schema_registry {
        pub use crate::interoperability::schema_registry::{
            SchemaRegistry,
            SchemaRegistryConfig,
            SchemaRegistryError,
            SchemaDefinition,
            SchemaResult,
            CompatibilityMatrix,
            CompatibilityType,
            SchemaStats,
        };
    }

    /// Re-exports for Sprint 2 tests (dev-only)
    #[cfg(test)]
    pub mod tests {
        use crate::alignment::feedback_loop::tests;
        use crate::federation::trust_scoring::tests;
        use crate::interoperability::schema_registry::tests;
    }
}

/// Check if Phase 7 Sprint 1 features are enabled at compile time
///
/// # Returns
///
/// `true` if the `phase7-sprint1` feature is enabled, `false` otherwise
pub fn is_sprint1_enabled() -> bool {
    cfg!(feature = "phase7-sprint1")
}

/// Check if Phase 7 Sprint 2 features are enabled at compile time
///
/// # Returns
///
/// `true` if the `phase7-sprint2` feature is enabled, `false` otherwise
pub fn is_sprint2_enabled() -> bool {
    cfg!(feature = "phase7-sprint2")
}

/// Check if any Phase 7 features are enabled
///
/// # Returns
///
/// `true` if any phase7 feature is enabled
pub fn is_enabled() -> bool {
    cfg!(feature = "phase7-sprint1") || cfg!(feature = "phase7-sprint2")
}

/// Get the list of enabled Phase 7 features
///
/// # Returns
///
/// A vector of feature names currently enabled for Phase 7
pub fn enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    if cfg!(feature = "phase7-sprint1") {
        features.push("phase7-sprint1");
    }
    if cfg!(feature = "phase7-sprint2") {
        features.push("phase7-sprint2");
    }
    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.7.0-alpha.2");
    }

    #[test]
    fn test_sprint1_identifier() {
        assert_eq!(SPRINT1_IDENTIFIER, "phase7-sprint1");
    }

    #[test]
    fn test_sprint2_identifier() {
        assert_eq!(SPRINT2_IDENTIFIER, "phase7-sprint2");
    }

    #[test]
    fn test_is_sprint1_enabled() {
        assert_eq!(is_sprint1_enabled(), cfg!(feature = "phase7-sprint1"));
    }

    #[test]
    fn test_is_sprint2_enabled() {
        assert_eq!(is_sprint2_enabled(), cfg!(feature = "phase7-sprint2"));
    }

    #[test]
    fn test_is_enabled() {
        assert_eq!(is_enabled(), cfg!(feature = "phase7-sprint1") || cfg!(feature = "phase7-sprint2"));
    }

    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        let expected = (if cfg!(feature = "phase7-sprint1") { 1 } else { 0 })
            + (if cfg!(feature = "phase7-sprint2") { 1 } else { 0 });
        assert_eq!(features.len(), expected);
    }

    #[cfg(feature = "phase7-sprint1")]
    #[test]
    fn test_sprint1_feature_detection() {
        assert!(is_sprint1_enabled());
        assert!(is_enabled());
        let features = enabled_features();
        assert!(features.contains(&"phase7-sprint1"));
    }

    #[cfg(feature = "phase7-sprint2")]
    #[test]
    fn test_sprint2_feature_detection() {
        assert!(is_sprint2_enabled());
        assert!(is_enabled());
        let features = enabled_features();
        assert!(features.contains(&"phase7-sprint2"));
    }
}
