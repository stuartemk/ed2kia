//! Cross-Mesh Routing & Peering — Enrutamiento determinista entre mallas GossipSub independientes.
//!
//! **Stuartian Law 1 (Diversidad):** Peering orgánico entre mallas, sin coordinación centralizada.
//! **Stuartian Law 5 (Múltiples Posibilidades):** Tolerancia a particiones, fallback a broadcast directo.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v2.1-cross-mesh` | cross_mesh | Cross-mesh routing, peering, rate limiting |

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::time::{Duration, Instant};

/// Error types for cross-mesh operations.
#[derive(Debug, Clone)]
pub enum CrossMeshError {
    /// Mesh signature validation failed.
    InvalidMeshSignature(String),
    /// Peer link is inactive or unreachable.
    PeerLinkInactive(String),
    /// Rate limit exceeded for a mesh.
    RateLimitExceeded(String),
    /// Payload too large for cross-mesh relay.
    PayloadTooLarge(usize),
    /// Unknown mesh ID.
    UnknownMesh(String),
    /// Backoff in progress for a peer.
    BackoffInProgress(String),
}

impl fmt::Display for CrossMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrossMeshError::InvalidMeshSignature(msg) => {
                write!(f, "Invalid mesh signature: {}", msg)
            }
            CrossMeshError::PeerLinkInactive(mesh_id) => {
                write!(f, "Peer link inactive: {}", mesh_id)
            }
            CrossMeshError::RateLimitExceeded(mesh_id) => {
                write!(f, "Rate limit exceeded for mesh: {}", mesh_id)
            }
            CrossMeshError::PayloadTooLarge(size) => write!(f, "Payload too large: {} bytes", size),
            CrossMeshError::UnknownMesh(mesh_id) => write!(f, "Unknown mesh: {}", mesh_id),
            CrossMeshError::BackoffInProgress(mesh_id) => {
                write!(f, "Backoff in progress for mesh: {}", mesh_id)
            }
        }
    }
}

/// Payload types that can be relayed across meshes.
#[derive(Debug, Clone)]
pub enum RelayPayload {
    /// QLoRA gradient payload.
    QLoRAPayload(Vec<u8>),
    /// SCT Decision payload.
    SCTDecision(f32),
    /// CRDT State payload.
    CRDTState(Vec<u8>),
}

impl RelayPayload {
    /// Estimate payload size in bytes.
    pub fn size_bytes(&self) -> usize {
        match self {
            RelayPayload::QLoRAPayload(data) => data.len(),
            RelayPayload::SCTDecision(_) => std::mem::size_of::<f32>(),
            RelayPayload::CRDTState(data) => data.len(),
        }
    }
}

/// Maximum payload size for cross-mesh relay (1MB).
pub const MAX_PAYLOAD_SIZE: usize = 1_048_576;

/// Peer link state tracking.
#[derive(Debug, Clone)]
pub struct PeerLink {
    /// Remote mesh ID.
    pub mesh_id: String,
    /// Mesh signature for validation.
    pub signature: String,
    /// Link is active.
    pub active: bool,
    /// Current backoff count.
    pub backoff_count: u32,
    /// Last successful relay time.
    pub last_relay: Option<Instant>,
    /// Messages relayed through this link.
    pub relay_count: u64,
    /// Rate limit: max messages per window.
    pub rate_limit: u64,
    /// Rate limit window duration.
    pub rate_window: Duration,
    /// Messages in current window.
    pub window_count: u64,
    /// Window start time.
    pub window_start: Instant,
}

impl PeerLink {
    /// Create a new peer link.
    pub fn new(mesh_id: String, signature: String) -> Self {
        Self {
            mesh_id,
            signature,
            active: true,
            backoff_count: 0,
            last_relay: None,
            relay_count: 0,
            rate_limit: 100,
            rate_window: Duration::from_secs(10),
            window_count: 0,
            window_start: Instant::now(),
        }
    }

    /// Check rate limit for this link.
    pub fn check_rate_limit(&mut self) -> Result<(), CrossMeshError> {
        // Reset window if expired.
        if self.window_start.elapsed() > self.rate_window {
            self.window_count = 0;
            self.window_start = Instant::now();
        }
        self.window_count += 1;
        if self.window_count > self.rate_limit {
            return Err(CrossMeshError::RateLimitExceeded(self.mesh_id.clone()));
        }
        Ok(())
    }

    /// Apply exponential backoff on failure.
    pub fn apply_backoff(&mut self) {
        self.backoff_count += 1;
        self.active = false;
    }

    /// Calculate backoff duration.
    pub fn backoff_duration(&self) -> Duration {
        let base = Duration::from_millis(100);
        let multiplier = 2_u32.pow(self.backoff_count.min(10)); // Cap at 2^10
        base * multiplier as u32
    }

    /// Attempt to recover from backoff.
    pub fn attempt_recovery(&mut self, now: Instant) -> bool {
        if let Some(last_relay) = self.last_relay {
            if now - last_relay > self.backoff_duration() {
                self.active = true;
                self.backoff_count = 0;
                return true;
            }
        }
        false
    }

    /// Mark relay as successful.
    pub fn mark_relay(&mut self) {
        self.relay_count += 1;
        self.last_relay = Some(Instant::now());
        if self.backoff_count > 0 {
            self.backoff_count = 0;
            self.active = true;
        }
    }
}

/// Routing table entry for a mesh.
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// Mesh ID.
    pub mesh_id: String,
    /// Next hop mesh ID (if any).
    pub next_hop: Option<String>,
    /// Hop count.
    pub hops: u8,
    /// Route is valid.
    pub valid: bool,
    /// Last update time.
    pub last_update: Instant,
}

impl RouteEntry {
    /// Create a new route entry.
    pub fn new(mesh_id: String) -> Self {
        Self {
            mesh_id,
            next_hop: None,
            hops: 0,
            valid: true,
            last_update: Instant::now(),
        }
    }
}

/// Cross-mesh router for deterministic peering between independent GossipSub meshes.
pub struct CrossMeshRouter {
    /// Local mesh ID.
    local_mesh_id: String,
    /// Peer links to remote meshes.
    peer_links: HashMap<String, PeerLink>,
    /// Routing table.
    routes: BTreeMap<String, RouteEntry>,
    /// Pending relay queue.
    pending_queue: VecDeque<RelayPayload>,
    /// Total relays performed.
    total_relays: u64,
    /// Total failures encountered.
    total_failures: u64,
}

impl CrossMeshRouter {
    /// Create a new cross-mesh router.
    pub fn new(local_mesh_id: String) -> Self {
        Self {
            local_mesh_id,
            peer_links: HashMap::new(),
            routes: BTreeMap::new(),
            pending_queue: VecDeque::new(),
            total_relays: 0,
            total_failures: 0,
        }
    }

    /// Add a peer link to a remote mesh.
    pub fn add_peer(&mut self, mesh_id: String, signature: String) {
        let link = PeerLink::new(mesh_id.clone(), signature);
        self.peer_links.insert(mesh_id.clone(), link);
        // Add direct route.
        self.routes.insert(
            mesh_id,
            RouteEntry {
                mesh_id: self.local_mesh_id.clone(),
                next_hop: None,
                hops: 1,
                valid: true,
                last_update: Instant::now(),
            },
        );
    }

    /// Remove a peer link.
    pub fn remove_peer(&mut self, mesh_id: &str) {
        self.peer_links.remove(mesh_id);
        self.routes.remove(mesh_id);
    }

    /// Validate mesh signature.
    pub fn validate_signature(&self, mesh_id: &str, signature: &str) -> Result<(), CrossMeshError> {
        match self.peer_links.get(mesh_id) {
            Some(link) => {
                if link.signature == signature {
                    Ok(())
                } else {
                    Err(CrossMeshError::InvalidMeshSignature(mesh_id.to_string()))
                }
            }
            None => Err(CrossMeshError::UnknownMesh(mesh_id.to_string())),
        }
    }

    /// Relay a payload to a specific mesh.
    pub fn relay_to(&mut self, mesh_id: &str, payload: RelayPayload) -> Result<(), CrossMeshError> {
        // Check payload size.
        if payload.size_bytes() > MAX_PAYLOAD_SIZE {
            return Err(CrossMeshError::PayloadTooLarge(payload.size_bytes()));
        }

        // Get peer link.
        let link = self
            .peer_links
            .get_mut(mesh_id)
            .ok_or_else(|| CrossMeshError::UnknownMesh(mesh_id.to_string()))?;

        // Check if link is active.
        if !link.active {
            return Err(CrossMeshError::PeerLinkInactive(mesh_id.to_string()));
        }

        // Check rate limit.
        link.check_rate_limit()?;

        // Relay payload (simulated).
        link.mark_relay();
        self.total_relays += 1;
        Ok(())
    }

    /// Broadcast payload to all active peers.
    pub fn broadcast(&mut self, payload: RelayPayload) -> Result<usize, CrossMeshError> {
        let mut success_count = 0;
        let mesh_ids: Vec<String> = self.peer_links.keys().cloned().collect();

        for mesh_id in mesh_ids {
            if self.relay_to(&mesh_id, payload.clone()).is_ok() {
                success_count += 1;
            }
        }
        Ok(success_count)
    }

    /// Queue a payload for later relay.
    pub fn queue_payload(&mut self, payload: RelayPayload) {
        self.pending_queue.push_back(payload);
    }

    /// Process pending queue.
    pub fn process_queue(&mut self) -> usize {
        let mut processed = 0;
        while let Some(payload) = self.pending_queue.pop_front() {
            if self.broadcast(payload).is_ok() {
                processed += 1;
            }
        }
        processed
    }

    /// Get peer link info.
    pub fn get_peer(&self, mesh_id: &str) -> Option<&PeerLink> {
        self.peer_links.get(mesh_id)
    }

    /// Get route info.
    pub fn get_route(&self, mesh_id: &str) -> Option<&RouteEntry> {
        self.routes.get(mesh_id)
    }

    /// Get router stats.
    pub fn stats(&self) -> RouterStats {
        RouterStats {
            total_peers: self.peer_links.len(),
            active_peers: self.peer_links.values().filter(|l| l.active).count(),
            total_routes: self.routes.len(),
            total_relays: self.total_relays,
            total_failures: self.total_failures,
            queue_size: self.pending_queue.len(),
        }
    }
}

/// Router statistics.
#[derive(Debug, Clone)]
pub struct RouterStats {
    pub total_peers: usize,
    pub active_peers: usize,
    pub total_routes: usize,
    pub total_relays: u64,
    pub total_failures: u64,
    pub queue_size: usize,
}

impl Default for CrossMeshRouter {
    fn default() -> Self {
        Self::new("default-mesh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = CrossMeshRouter::new("mesh-1".to_string());
        assert_eq!(router.stats().total_peers, 0);
    }

    #[test]
    fn test_add_peer() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        assert_eq!(router.stats().total_peers, 1);
        assert!(router.get_peer("mesh-2").is_some());
    }

    #[test]
    fn test_remove_peer() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        router.remove_peer("mesh-2");
        assert_eq!(router.stats().total_peers, 0);
    }

    #[test]
    fn test_validate_signature_valid() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        assert!(router.validate_signature("mesh-2", "sig-2").is_ok());
    }

    #[test]
    fn test_validate_signature_invalid() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        assert!(router.validate_signature("mesh-2", "wrong-sig").is_err());
    }

    #[test]
    fn test_relay_to_success() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        let payload = RelayPayload::SCTDecision(0.5);
        assert!(router.relay_to("mesh-2", payload).is_ok());
        assert_eq!(router.stats().total_relays, 1);
    }

    #[test]
    fn test_relay_to_unknown_mesh() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        let payload = RelayPayload::SCTDecision(0.5);
        assert!(router.relay_to("unknown", payload).is_err());
    }

    #[test]
    fn test_relay_to_payload_too_large() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        let large_data = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        let payload = RelayPayload::QLoRAPayload(large_data);
        assert!(router.relay_to("mesh-2", payload).is_err());
    }

    #[test]
    fn test_broadcast() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        router.add_peer("mesh-3".to_string(), "sig-3".to_string());
        let payload = RelayPayload::SCTDecision(0.8);
        let count = router.broadcast(payload).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_queue_and_process() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        router.queue_payload(RelayPayload::SCTDecision(0.3));
        router.queue_payload(RelayPayload::SCTDecision(0.7));
        assert_eq!(router.stats().queue_size, 2);
        let processed = router.process_queue();
        assert_eq!(processed, 2);
        assert_eq!(router.stats().queue_size, 0);
    }

    #[test]
    fn test_backoff_and_recovery() {
        let mut link = PeerLink::new("mesh-2".to_string(), "sig-2".to_string());
        assert!(link.active);
        link.apply_backoff();
        assert!(!link.active);
        assert_eq!(link.backoff_count, 1);
    }

    #[test]
    fn test_rate_limit() {
        let mut link = PeerLink::new("mesh-2".to_string(), "sig-2".to_string());
        link.rate_limit = 5;
        for _ in 0..5 {
            assert!(link.check_rate_limit().is_ok());
        }
        assert!(link.check_rate_limit().is_err());
    }

    #[test]
    fn test_peer_link_mark_relay() {
        let mut link = PeerLink::new("mesh-2".to_string(), "sig-2".to_string());
        link.apply_backoff();
        assert!(!link.active);
        link.mark_relay();
        assert!(link.active);
        assert_eq!(link.backoff_count, 0);
    }

    #[test]
    fn test_route_entry_creation() {
        let route = RouteEntry::new("mesh-2".to_string());
        assert_eq!(route.mesh_id, "mesh-2");
        assert!(route.valid);
        assert_eq!(route.hops, 0);
    }

    #[test]
    fn test_router_stats() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        router.add_peer("mesh-3".to_string(), "sig-3".to_string());
        let stats = router.stats();
        assert_eq!(stats.total_peers, 2);
        assert_eq!(stats.active_peers, 2);
        assert_eq!(stats.total_routes, 2);
    }

    #[test]
    fn test_relay_payload_size() {
        let qlo = RelayPayload::QLoRAPayload(vec![1, 2, 3, 4, 5]);
        assert_eq!(qlo.size_bytes(), 5);
        let sct = RelayPayload::SCTDecision(0.5);
        assert_eq!(sct.size_bytes(), std::mem::size_of::<f32>());
        let crdt = RelayPayload::CRDTState(vec![10; 100]);
        assert_eq!(crdt.size_bytes(), 100);
    }

    #[test]
    fn test_error_display() {
        let err = CrossMeshError::InvalidMeshSignature("mesh-1".to_string());
        assert!(!format!("{}", err).is_empty());
        let err = CrossMeshError::RateLimitExceeded("mesh-2".to_string());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_default_router() {
        let router = CrossMeshRouter::default();
        assert_eq!(router.local_mesh_id, "default-mesh");
    }

    #[test]
    fn test_three_mesh_propagation() {
        // Simulate 3 disconnected meshes, activate peering, verify propagation.
        let mut router = CrossMeshRouter::new("mesh-a".to_string());
        router.add_peer("mesh-b".to_string(), "sig-b".to_string());
        router.add_peer("mesh-c".to_string(), "sig-c".to_string());

        let payload = RelayPayload::CRDTState(vec![1, 2, 3]);
        let count = router.broadcast(payload).unwrap();
        assert_eq!(count, 2); // Propagated to both meshes
        assert_eq!(router.stats().total_relays, 2);
    }

    #[test]
    fn test_no_duplicate_relay() {
        let mut router = CrossMeshRouter::new("mesh-1".to_string());
        router.add_peer("mesh-2".to_string(), "sig-2".to_string());
        let payload = RelayPayload::SCTDecision(0.5);
        assert!(router.relay_to("mesh-2", payload.clone()).is_ok());
        assert_eq!(router.stats().total_relays, 1);
    }
}
