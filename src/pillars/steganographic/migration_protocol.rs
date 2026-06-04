//! Migration Protocol â€” Cluster Onboarding for Large Data Centers ("Gran MigraciÃ³n").
//!
//! Enables seamless integration of new clusters into the ed2kIA network through
//! cooperative handshake negotiation, transport selection, and SCT-supervised
//! bootstrap routing. This protocol ensures that large-scale migrations maintain
//! network harmony and ethical alignment.
//!
//! **Migration Flow:**
//! ```text
//! New Cluster â†’ MigrationHandshake â†’ negotiate_migration() â†’ MigrationToken
//!     â†’ Bootstrap Routes â†’ SCT Validation â†’ Cluster Integrated
//! ```
//!
//! **Design Principles:**
//! - Cooperative onboarding: No cluster is forced, all participate willingly.
//! - SCT-supervised: Every migration must pass ethical validation (Z >= 0).
//! - Transport negotiation: Dynamic selection based on cluster capabilities.
//! - CE-aware: Migration costs tracked via Existential Credit ledger.
//! - Zero telemetry: Biometric and sensitive data remains local-only.
//!
//! **Reference:** Sprint 47 â€” Omni-Node Integration & Symbiotic Ignition Sequence

use crate::alignment::sct_core::{SCTDecision, TopologicalTensor};
use crate::pillars::steganographic::transport_rotator::{TransportHealth, TransportType};
use std::collections::HashMap;

/// Maximum allowed CE cost per migration handshake.
const MAX_MIGRATION_CE_COST: f64 = 50.0;

/// Minimum SCT Z-score for migration approval.
const MIGRATION_Z_THRESHOLD: f32 = 0.0;

/// Errors specific to cluster migration.
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationError {
    /// Cluster ID already exists in the network.
    ClusterAlreadyExists(String),
    /// Invalid cluster identifier (empty or too long).
    InvalidClusterId(String),
    /// SCT ethical rejection for migration trajectory.
    EthicalRejection { z: f32 },
    /// Insufficient CE for migration cost.
    InsufficientCE { required: f64, available: f64 },
    /// No compatible transport found for cluster.
    NoCompatibleTransport,
    /// Migration handshake signature verification failed.
    InvalidSignature,
    /// Migration token expired.
    TokenExpired,
    /// Cluster capacity exceeds network limits.
    CapacityExceeded { requested: u64, max: u64 },
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationError::ClusterAlreadyExists(id) => {
                write!(f, "Cluster '{}' already exists in the network", id)
            }
            MigrationError::InvalidClusterId(id) => {
                write!(f, "Invalid cluster identifier: '{}'", id)
            }
            MigrationError::EthicalRejection { z } => {
                write!(
                    f,
                    "SCT ethical rejection for migration: Z = {:.3} < {:.3}",
                    z, MIGRATION_Z_THRESHOLD
                )
            }
            MigrationError::InsufficientCE {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient CE for migration: required {:.2}, available {:.2}",
                    required, available
                )
            }
            MigrationError::NoCompatibleTransport => {
                write!(f, "No compatible transport found for cluster onboarding")
            }
            MigrationError::InvalidSignature => {
                write!(f, "Migration handshake signature verification failed")
            }
            MigrationError::TokenExpired => {
                write!(f, "Migration token has expired")
            }
            MigrationError::CapacityExceeded { requested, max } => {
                write!(
                    f,
                    "Cluster capacity exceeds network limits: requested {}, max {}",
                    requested, max
                )
            }
        }
    }
}

/// Migration Handshake â€” Initial contact from a new cluster.
///
/// Contains the cluster's capabilities, preferred transports,
/// and cryptographic signature for cooperative onboarding.
#[derive(Debug, Clone)]
pub struct MigrationHandshake {
    /// Unique cluster identifier.
    pub cluster_id: String,
    /// Cluster compute capacity (in arbitrary units).
    pub capacity: u64,
    /// Preferred transport types for communication.
    pub transports: Vec<TransportType>,
    /// Transport health reports from the cluster.
    pub health_reports: Vec<TransportHealth>,
    /// Cryptographic signature for handshake integrity.
    pub signature: Vec<u8>,
    /// Timestamp of handshake creation (milliseconds).
    pub timestamp_ms: u64,
    /// CE budget allocated for this migration.
    pub ce_budget: f64,
}

impl MigrationHandshake {
    /// Create a new migration handshake.
    pub fn new(
        cluster_id: String,
        capacity: u64,
        transports: Vec<TransportType>,
        signature: Vec<u8>,
        ce_budget: f64,
    ) -> Result<Self, MigrationError> {
        // Validate cluster ID
        if cluster_id.is_empty() || cluster_id.len() > 64 {
            return Err(MigrationError::InvalidClusterId(cluster_id));
        }

        // Validate transports
        if transports.is_empty() {
            return Err(MigrationError::NoCompatibleTransport);
        }

        // Validate CE budget
        if ce_budget <= 0.0 || ce_budget > MAX_MIGRATION_CE_COST {
            return Err(MigrationError::InsufficientCE {
                required: ce_budget,
                available: 0.0,
            });
        }

        Ok(Self {
            cluster_id,
            capacity,
            transports,
            health_reports: Vec::new(),
            signature,
            timestamp_ms: Self::now_ms(),
            ce_budget,
        })
    }

    /// Add transport health reports to the handshake.
    pub fn add_health_report(&mut self, report: TransportHealth) {
        self.health_reports.push(report);
    }

    /// Add multiple transport health reports.
    pub fn add_health_reports(&mut self, reports: Vec<TransportHealth>) {
        self.health_reports.extend(reports);
    }

    /// Get the best transport based on health reports.
    pub fn get_best_transport(&self) -> Option<TransportType> {
        if self.health_reports.is_empty() {
            return self.transports.first().cloned();
        }

        self.health_reports
            .iter()
            .filter(|r| r.is_healthy)
            .max_by(|a, b| {
                a.score()
                    .partial_cmp(&b.score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|r| r.transport.clone())
    }

    /// Verify handshake signature (non-empty check).
    pub fn verify_signature(&self) -> Result<(), MigrationError> {
        if self.signature.is_empty() {
            return Err(MigrationError::InvalidSignature);
        }
        Ok(())
    }

    /// Check if handshake is within validity window (5 minutes).
    pub fn is_valid(&self) -> bool {
        let now = Self::now_ms();
        now.saturating_sub(self.timestamp_ms) < 300_000 // 5 minutes
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

/// Migration Token â€” Bootstrap credentials for integrated clusters.
///
/// Generated after successful negotiation, this token contains
/// the bootstrap routes, SCT thresholds, and CE limits for the
/// newly integrated cluster.
#[derive(Debug, Clone)]
pub struct MigrationToken {
    /// Cluster ID that this token belongs to.
    pub cluster_id: String,
    /// Bootstrap routes for initial network discovery.
    pub bootstrap_routes: Vec<String>,
    /// SCT Z-score threshold for this cluster.
    pub sct_z_threshold: f32,
    /// Initial CE allocation for the cluster.
    pub initial_ce: f64,
    /// Maximum CE limit for the cluster.
    pub max_ce_limit: f64,
    /// Selected transport for primary communication.
    pub primary_transport: TransportType,
    /// Token expiration timestamp (milliseconds).
    pub expires_at_ms: u64,
    /// Token creation timestamp (milliseconds).
    pub created_at_ms: u64,
}

impl MigrationToken {
    /// Create a new migration token.
    pub fn new(
        cluster_id: String,
        bootstrap_routes: Vec<String>,
        primary_transport: TransportType,
        initial_ce: f64,
        max_ce_limit: f64,
    ) -> Self {
        let now = Self::now_ms();
        Self {
            cluster_id,
            bootstrap_routes,
            sct_z_threshold: MIGRATION_Z_THRESHOLD,
            initial_ce,
            max_ce_limit,
            primary_transport,
            expires_at_ms: now + 3_600_000, // 1 hour validity
            created_at_ms: now,
        }
    }

    /// Create with custom SCT threshold.
    pub fn with_sct_threshold(mut self, threshold: f32) -> Self {
        self.sct_z_threshold = threshold;
        self
    }

    /// Check if token is still valid.
    pub fn is_valid(&self) -> bool {
        let now = Self::now_ms();
        now < self.expires_at_ms
    }

    /// Get remaining validity time in milliseconds.
    pub fn remaining_validity_ms(&self) -> u64 {
        let now = Self::now_ms();
        self.expires_at_ms.saturating_sub(now)
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

/// Migration Negotiator â€” Coordinates cluster onboarding.
///
/// Manages the migration negotiation process, including SCT validation,
/// transport selection, and token generation.
#[derive(Debug, Clone)]
pub struct MigrationNegotiator {
    /// Registered cluster IDs.
    registered_clusters: HashMap<String, MigrationToken>,
    /// Maximum cluster capacity allowed.
    max_cluster_capacity: u64,
    /// SCT Z-score threshold for migrations.
    sct_z_threshold: f32,
    /// Migration audit log.
    migration_log: Vec<MigrationRecord>,
}

impl Default for MigrationNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationNegotiator {
    /// Create a new migration negotiator.
    pub fn new() -> Self {
        Self {
            registered_clusters: HashMap::new(),
            max_cluster_capacity: 10_000, // Default max capacity
            sct_z_threshold: MIGRATION_Z_THRESHOLD,
            migration_log: Vec::new(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(max_capacity: u64, sct_threshold: f32) -> Self {
        Self {
            registered_clusters: HashMap::new(),
            max_cluster_capacity: max_capacity,
            sct_z_threshold: sct_threshold,
            migration_log: Vec::new(),
        }
    }

    /// Negotiate migration for a new cluster.
    ///
    /// This is the core migration function that:
    /// 1. Validates the handshake signature.
    /// 2. Checks cluster capacity limits.
    /// 3. Validates SCT ethical alignment.
    /// 4. Selects optimal transport.
    /// 5. Generates migration token.
    /// 6. Records migration in audit log.
    pub fn negotiate_migration(
        &mut self,
        handshake: &MigrationHandshake,
        sct_tensor: &TopologicalTensor,
    ) -> Result<MigrationToken, MigrationError> {
        // Step 1: Verify handshake signature
        handshake.verify_signature()?;

        // Step 2: Check if cluster already exists
        if self.registered_clusters.contains_key(&handshake.cluster_id) {
            return Err(MigrationError::ClusterAlreadyExists(
                handshake.cluster_id.clone(),
            ));
        }

        // Step 3: Validate cluster capacity
        if handshake.capacity > self.max_cluster_capacity {
            return Err(MigrationError::CapacityExceeded {
                requested: handshake.capacity,
                max: self.max_cluster_capacity,
            });
        }

        // Step 4: SCT ethical validation
        let decision = sct_tensor
            .evaluate_trajectory()
            .map_err(|_| MigrationError::EthicalRejection { z: -1.0 })?;
        match decision {
            SCTDecision::Approved(z) => {
                if z < self.sct_z_threshold {
                    return Err(MigrationError::EthicalRejection { z });
                }
            }
            SCTDecision::Rejected(z) => {
                return Err(MigrationError::EthicalRejection { z });
            }
        }

        // Step 5: Select optimal transport
        let primary_transport = handshake
            .get_best_transport()
            .ok_or(MigrationError::NoCompatibleTransport)?;

        // Step 6: Generate bootstrap routes
        let bootstrap_routes = self.generate_bootstrap_routes(&handshake.cluster_id);

        // Step 7: Generate migration token
        let token = MigrationToken::new(
            handshake.cluster_id.clone(),
            bootstrap_routes,
            primary_transport,
            handshake.ce_budget,
            handshake.ce_budget * 2.0, // Max limit is 2x initial
        )
        .with_sct_threshold(self.sct_z_threshold);

        // Step 8: Register cluster
        self.registered_clusters
            .insert(handshake.cluster_id.clone(), token.clone());

        // Step 9: Record migration in audit log
        self.migration_log.push(MigrationRecord {
            cluster_id: handshake.cluster_id.clone(),
            timestamp_ms: Self::now_ms(),
            sct_z_score: sct_tensor.z,
            transport: token.primary_transport.clone(),
            initial_ce: token.initial_ce,
            status: MigrationStatus::Success,
        });

        Ok(token)
    }

    /// Generate bootstrap routes for a new cluster.
    fn generate_bootstrap_routes(&self, cluster_id: &str) -> Vec<String> {
        // Generate deterministic bootstrap routes based on cluster ID
        let mut routes = Vec::new();
        for i in 0..3 {
            routes.push(format!(
                "ed2k://bootstrap-{}.ed2kIA/network/{}",
                i, cluster_id
            ));
        }
        routes
    }

    /// Check if a cluster is registered.
    pub fn is_cluster_registered(&self, cluster_id: &str) -> bool {
        self.registered_clusters.contains_key(cluster_id)
    }

    /// Get the migration token for a cluster.
    pub fn get_token(&self, cluster_id: &str) -> Option<&MigrationToken> {
        self.registered_clusters.get(cluster_id)
    }

    /// Get all registered cluster IDs.
    pub fn get_registered_clusters(&self) -> Vec<String> {
        self.registered_clusters.keys().cloned().collect()
    }

    /// Get the number of registered clusters.
    pub fn cluster_count(&self) -> usize {
        self.registered_clusters.len()
    }

    /// Get the migration audit log.
    pub fn get_migration_log(&self) -> &[MigrationRecord] {
        &self.migration_log
    }

    /// Remove expired tokens.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.registered_clusters.len();
        self.registered_clusters.retain(|_, token| token.is_valid());
        before - self.registered_clusters.len()
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

/// Record of a migration event for audit purposes.
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    /// Cluster ID that was migrated.
    pub cluster_id: String,
    /// Timestamp of migration (milliseconds).
    pub timestamp_ms: u64,
    /// SCT Z-score from migration validation.
    pub sct_z_score: f32,
    /// Selected transport for the cluster.
    pub transport: TransportType,
    /// Initial CE allocation.
    pub initial_ce: f64,
    /// Migration status.
    pub status: MigrationStatus,
}

/// Status of a migration event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationStatus {
    /// Migration completed successfully.
    Success,
    /// Migration was rejected by SCT Guard.
    EthicalRejection,
    /// Migration failed due to technical error.
    Failed,
}

impl std::fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationStatus::Success => write!(f, "success"),
            MigrationStatus::EthicalRejection => write!(f, "ethical_rejection"),
            MigrationStatus::Failed => write!(f, "failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handshake(cluster_id: &str, capacity: u64, ce_budget: f64) -> MigrationHandshake {
        MigrationHandshake::new(
            cluster_id.to_string(),
            capacity,
            vec![TransportType::Tcp, TransportType::Quic],
            b"valid_signature".to_vec(),
            ce_budget,
        )
        .unwrap()
    }

    fn make_valid_tensor(z: f32) -> TopologicalTensor {
        TopologicalTensor { x: 0.7, y: 0.3, z }
    }

    #[test]
    fn test_handshake_creation() {
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        assert_eq!(handshake.cluster_id, "cluster-1");
        assert_eq!(handshake.capacity, 1000);
        assert_eq!(handshake.ce_budget, 25.0);
    }

    #[test]
    fn test_handshake_empty_cluster_id() {
        let result = MigrationHandshake::new(
            String::new(),
            1000,
            vec![TransportType::Tcp],
            b"sig".to_vec(),
            25.0,
        );
        assert!(matches!(result, Err(MigrationError::InvalidClusterId(_))));
    }

    #[test]
    fn test_handshake_long_cluster_id() {
        let long_id = "a".repeat(65);
        let result = MigrationHandshake::new(
            long_id,
            1000,
            vec![TransportType::Tcp],
            b"sig".to_vec(),
            25.0,
        );
        assert!(matches!(result, Err(MigrationError::InvalidClusterId(_))));
    }

    #[test]
    fn test_handshake_no_transports() {
        let result =
            MigrationHandshake::new("cluster-1".to_string(), 1000, vec![], b"sig".to_vec(), 25.0);
        assert!(matches!(result, Err(MigrationError::NoCompatibleTransport)));
    }

    #[test]
    fn test_handshake_invalid_ce_budget() {
        let result = MigrationHandshake::new(
            "cluster-1".to_string(),
            1000,
            vec![TransportType::Tcp],
            b"sig".to_vec(),
            0.0,
        );
        assert!(matches!(result, Err(MigrationError::InsufficientCE { .. })));
    }

    #[test]
    fn test_handshake_ce_budget_exceeds_max() {
        let result = MigrationHandshake::new(
            "cluster-1".to_string(),
            1000,
            vec![TransportType::Tcp],
            b"sig".to_vec(),
            100.0, // Exceeds MAX_MIGRATION_CE_COST
        );
        assert!(matches!(result, Err(MigrationError::InsufficientCE { .. })));
    }

    #[test]
    fn test_handshake_empty_signature() {
        let handshake = MigrationHandshake::new(
            "cluster-1".to_string(),
            1000,
            vec![TransportType::Tcp],
            vec![], // Empty signature
            25.0,
        )
        .unwrap();
        assert!(handshake.verify_signature().is_err());
    }

    #[test]
    fn test_handshake_valid_signature() {
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        assert!(handshake.verify_signature().is_ok());
    }

    #[test]
    fn test_handshake_add_health_reports() {
        let mut handshake = make_handshake("cluster-1", 1000, 25.0);
        let report = TransportHealth::new(TransportType::Tcp, 10.0, 0.01, 1_000_000.0);
        handshake.add_health_report(report);
        assert_eq!(handshake.health_reports.len(), 1);
    }

    #[test]
    fn test_handshake_get_best_transport_no_health() {
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        let best = handshake.get_best_transport();
        assert_eq!(best, Some(TransportType::Tcp));
    }

    #[test]
    fn test_token_creation() {
        let token = MigrationToken::new(
            "cluster-1".to_string(),
            vec!["route1".to_string(), "route2".to_string()],
            TransportType::Quic,
            25.0,
            50.0,
        );
        assert_eq!(token.cluster_id, "cluster-1");
        assert_eq!(token.bootstrap_routes.len(), 2);
        assert_eq!(token.primary_transport, TransportType::Quic);
        assert!(token.is_valid());
    }

    #[test]
    fn test_token_with_sct_threshold() {
        let token = MigrationToken::new(
            "cluster-1".to_string(),
            vec![],
            TransportType::Tcp,
            25.0,
            50.0,
        )
        .with_sct_threshold(0.5);
        assert_eq!(token.sct_z_threshold, 0.5);
    }

    #[test]
    fn test_token_remaining_validity() {
        let token = MigrationToken::new(
            "cluster-1".to_string(),
            vec![],
            TransportType::Tcp,
            25.0,
            50.0,
        );
        assert!(token.remaining_validity_ms() > 0);
    }

    #[test]
    fn test_negotiator_creation() {
        let negotiator = MigrationNegotiator::new();
        assert_eq!(negotiator.cluster_count(), 0);
    }

    #[test]
    fn test_negotiator_custom_config() {
        let negotiator = MigrationNegotiator::with_config(5000, 0.5);
        assert_eq!(negotiator.max_cluster_capacity, 5000);
        assert_eq!(negotiator.sct_z_threshold, 0.5);
    }

    #[test]
    fn test_negotiate_migration_success() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        let tensor = make_valid_tensor(0.5);

        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(result.is_ok());
        let token = result.unwrap();
        assert_eq!(token.cluster_id, "cluster-1");
        assert!(negotiator.is_cluster_registered("cluster-1"));
        assert_eq!(negotiator.cluster_count(), 1);
    }

    #[test]
    fn test_negotiate_migration_duplicate_cluster() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        let tensor = make_valid_tensor(0.5);

        let _ = negotiator.negotiate_migration(&handshake, &tensor);
        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(matches!(
            result,
            Err(MigrationError::ClusterAlreadyExists(_))
        ));
    }

    #[test]
    fn test_negotiate_migration_capacity_exceeded() {
        let mut negotiator = MigrationNegotiator::with_config(500, 0.0);
        let handshake = make_handshake("cluster-1", 1000, 25.0); // Capacity 1000 > max 500
        let tensor = make_valid_tensor(0.5);

        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(matches!(
            result,
            Err(MigrationError::CapacityExceeded { .. })
        ));
    }

    #[test]
    fn test_negotiate_migration_sct_rejection() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        let tensor = make_valid_tensor(-0.5); // Negative Z

        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(matches!(
            result,
            Err(MigrationError::EthicalRejection { .. })
        ));
    }

    #[test]
    fn test_negotiate_migration_invalid_signature() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = MigrationHandshake::new(
            "cluster-1".to_string(),
            1000,
            vec![TransportType::Tcp],
            vec![], // Empty signature
            25.0,
        )
        .unwrap();
        let tensor = make_valid_tensor(0.5);

        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(matches!(result, Err(MigrationError::InvalidSignature)));
    }

    #[test]
    fn test_negotiator_get_token() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        let tensor = make_valid_tensor(0.5);

        let _ = negotiator.negotiate_migration(&handshake, &tensor);
        let token = negotiator.get_token("cluster-1");
        assert!(token.is_some());
        assert_eq!(token.unwrap().cluster_id, "cluster-1");
    }

    #[test]
    fn test_negotiator_get_registered_clusters() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake1 = make_handshake("cluster-1", 1000, 25.0);
        let handshake2 = make_handshake("cluster-2", 2000, 30.0);
        let tensor = make_valid_tensor(0.5);

        let _ = negotiator.negotiate_migration(&handshake1, &tensor);
        let _ = negotiator.negotiate_migration(&handshake2, &tensor);

        let clusters = negotiator.get_registered_clusters();
        assert_eq!(clusters.len(), 2);
        assert!(clusters.contains(&"cluster-1".to_string()));
        assert!(clusters.contains(&"cluster-2".to_string()));
    }

    #[test]
    fn test_negotiator_migration_log() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("cluster-1", 1000, 25.0);
        let tensor = make_valid_tensor(0.5);

        let _ = negotiator.negotiate_migration(&handshake, &tensor);
        let log = negotiator.get_migration_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].cluster_id, "cluster-1");
        assert_eq!(log[0].status, MigrationStatus::Success);
    }

    #[test]
    fn test_negotiator_cleanup_expired() {
        let mut negotiator = MigrationNegotiator::new();
        let cleaned = negotiator.cleanup_expired();
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn test_bootstrap_routes_generation() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("test-cluster", 1000, 25.0);
        let tensor = make_valid_tensor(0.5);

        let _ = negotiator.negotiate_migration(&handshake, &tensor);
        let token = negotiator.get_token("test-cluster").unwrap();
        assert_eq!(token.bootstrap_routes.len(), 3);
        for route in &token.bootstrap_routes {
            assert!(route.contains("test-cluster"));
        }
    }

    #[test]
    fn test_error_display() {
        let err = MigrationError::EthicalRejection { z: -0.5 };
        let msg = format!("{}", err);
        assert!(msg.contains("ethical rejection"));

        let err = MigrationError::ClusterAlreadyExists("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn test_migration_status_display() {
        assert_eq!(format!("{}", MigrationStatus::Success), "success");
        assert_eq!(
            format!("{}", MigrationStatus::EthicalRejection),
            "ethical_rejection"
        );
        assert_eq!(format!("{}", MigrationStatus::Failed), "failed");
    }

    #[test]
    fn test_default() {
        let negotiator = MigrationNegotiator::default();
        assert_eq!(negotiator.cluster_count(), 0);
    }
}
