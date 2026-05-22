//! Geographic Routing v1 — Peer proximity scoring + RTT-based routing for P2P mesh.
//!
//! Integrates with libp2p swarm to measure RTT/ping per peer, compute geographic
//! proximity scores, and prioritize leases based on latency + region affinity.
//! Falls back to Kademlia DHT when geographic data is unavailable.

mod internal {
    use std::collections::HashMap;
    use std::fmt;

    /// Error types for geographic routing operations.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum GeoRoutingError {
        /// Peer not found in routing table.
        PeerNotFound,
        /// Invalid coordinates (latitude/longitude out of bounds).
        InvalidCoordinates,
        /// No peers available for routing.
        NoPeersAvailable,
        /// RTT measurement timeout.
        RttTimeout,
    }

    impl fmt::Display for GeoRoutingError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                GeoRoutingError::PeerNotFound => write!(f, "Peer not found in routing table"),
                GeoRoutingError::InvalidCoordinates => write!(f, "Invalid geographic coordinates"),
                GeoRoutingError::NoPeersAvailable => write!(f, "No peers available for routing"),
                GeoRoutingError::RttTimeout => write!(f, "RTT measurement timed out"),
            }
        }
    }

    impl std::error::Error for GeoRoutingError {}

    /// Geographic coordinates for a peer node.
    #[derive(Debug, Clone, PartialEq)]
    pub struct GeoCoordinates {
        /// Latitude in degrees (-90.0 to 90.0).
        pub latitude: f64,
        /// Longitude in degrees (-180.0 to 180.0).
        pub longitude: f64,
        /// Optional region code (e.g., "us-east-1", "eu-west-2").
        pub region: Option<String>,
    }

    impl GeoCoordinates {
        /// Create new coordinates, validating bounds.
        pub fn new(latitude: f64, longitude: f64) -> Result<Self, GeoRoutingError> {
            if latitude < -90.0 || latitude > 90.0 {
                return Err(GeoRoutingError::InvalidCoordinates);
            }
            if longitude < -180.0 || longitude > 180.0 {
                return Err(GeoRoutingError::InvalidCoordinates);
            }
            Ok(Self {
                latitude,
                longitude,
                region: None,
            })
        }

        /// Create coordinates with region code.
        pub fn with_region(mut self, region: String) -> Self {
            self.region = Some(region);
            self
        }

        /// Compute approximate great-circle distance (Haversine formula) to another coordinate.
        /// Returns distance in kilometers.
        pub fn distance_to(&self, other: &GeoCoordinates) -> f64 {
            let earth_radius_km = 6_371.0;
            let lat_diff = (other.latitude - self.latitude).to_radians();
            let lon_diff = (other.longitude - self.longitude).to_radians();
            let lat_rad_self = self.latitude.to_radians();
            let lat_rad_other = other.latitude.to_radians();

            let a = (lat_diff / 2.0).sin() * (lat_diff / 2.0).sin()
                + lat_rad_self.cos()
                    * lat_rad_other.cos()
                    * (lon_diff / 2.0).sin()
                    * (lon_diff / 2.0).sin();
            let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

            earth_radius_km * c
        }
    }

    /// RTT (Round Trip Time) measurement record for a peer.
    #[derive(Debug, Clone)]
    pub struct RttRecord {
        /// Peer identifier.
        pub peer_id: String,
        /// Average RTT in milliseconds (EMA smoothed).
        pub avg_rtt_ms: f64,
        /// Minimum observed RTT in milliseconds.
        pub min_rtt_ms: f64,
        /// Maximum observed RTT in milliseconds.
        pub max_rtt_ms: f64,
        /// Number of samples collected.
        pub sample_count: u64,
        /// Last measurement timestamp (ms since epoch).
        pub last_measurement_ms: u64,
    }

    impl RttRecord {
        /// Create a new RTT record with initial measurement.
        pub fn new(peer_id: String, initial_rtt_ms: f64, timestamp_ms: u64) -> Self {
            Self {
                peer_id,
                avg_rtt_ms: initial_rtt_ms,
                min_rtt_ms: initial_rtt_ms,
                max_rtt_ms: initial_rtt_ms,
                sample_count: 1,
                last_measurement_ms: timestamp_ms,
            }
        }

        /// Update RTT with new measurement using exponential moving average.
        pub fn update(&mut self, new_rtt_ms: f64, alpha: f64, timestamp_ms: u64) {
            self.avg_rtt_ms = alpha * new_rtt_ms + (1.0 - alpha) * self.avg_rtt_ms;
            self.min_rtt_ms = self.min_rtt_ms.min(new_rtt_ms);
            self.max_rtt_ms = self.max_rtt_ms.max(new_rtt_ms);
            self.sample_count += 1;
            self.last_measurement_ms = timestamp_ms;
        }

        /// Check if this record is stale (no measurements in the last `stale_threshold_ms`).
        pub fn is_stale(&self, current_ms: u64, stale_threshold_ms: u64) -> bool {
            current_ms.saturating_sub(self.last_measurement_ms) > stale_threshold_ms
        }
    }

    /// Combined peer entry with geographic and RTT data.
    #[derive(Debug, Clone)]
    pub struct GeoPeerEntry {
        /// Peer identifier.
        pub peer_id: String,
        /// Geographic coordinates (if available).
        pub coordinates: Option<GeoCoordinates>,
        /// RTT measurement record.
        pub rtt: RttRecord,
        /// Current lease priority score (higher = better).
        pub lease_priority: f64,
    }

    impl GeoPeerEntry {
        /// Create a new peer entry with RTT data only.
        pub fn new(peer_id: String, initial_rtt_ms: f64, timestamp_ms: u64) -> Self {
            Self {
                peer_id: peer_id.clone(),
                coordinates: None,
                rtt: RttRecord::new(peer_id, initial_rtt_ms, timestamp_ms),
                lease_priority: 1.0,
            }
        }

        /// Create a new peer entry with geographic coordinates.
        pub fn with_coordinates(
            peer_id: String,
            coordinates: GeoCoordinates,
            initial_rtt_ms: f64,
            timestamp_ms: u64,
        ) -> Self {
            Self {
                peer_id: peer_id.clone(),
                coordinates: Some(coordinates),
                rtt: RttRecord::new(peer_id, initial_rtt_ms, timestamp_ms),
                lease_priority: 1.0,
            }
        }

        /// Compute composite routing score: lower RTT + closer distance = higher score.
        /// Base score is 1000, penalized by RTT (1 point per ms) and distance (0.1 per km).
        pub fn routing_score(&self, origin: &Option<GeoCoordinates>) -> f64 {
            let base_score = 1000.0;
            let rtt_penalty = self.rtt.avg_rtt_ms;

            let distance_penalty = match (&self.coordinates, origin) {
                (Some(coords), Some(origin_coords)) => coords.distance_to(origin_coords) * 0.1,
                _ => 0.0, // No geographic penalty if data unavailable
            };

            base_score - rtt_penalty - distance_penalty
        }
    }

    /// Configuration for the geographic routing table.
    #[derive(Debug, Clone)]
    pub struct GeoRoutingConfig {
        /// RTT EMA smoothing factor (0.0 = no smoothing, 1.0 = only latest sample).
        pub rtt_alpha: f64,
        /// Stale threshold in milliseconds (records older than this are ignored).
        pub stale_threshold_ms: u64,
        /// Maximum number of peers in routing table.
        pub max_peers: usize,
        /// Minimum samples before a peer is considered for routing.
        pub min_samples: u64,
        /// Region affinity weight (0.0 = ignore regions, 1.0 = strong preference).
        pub region_affinity_weight: f64,
    }

    impl Default for GeoRoutingConfig {
        fn default() -> Self {
            Self {
                rtt_alpha: 0.3,
                stale_threshold_ms: 30_000, // 30 seconds
                max_peers: 256,
                min_samples: 3,
                region_affinity_weight: 0.5,
            }
        }
    }

    /// Geographic routing table — manages peer proximity scoring and lease prioritization.
    pub struct GeoRoutingTable {
        /// Configuration.
        config: GeoRoutingConfig,
        /// Peer entries indexed by peer_id.
        peers: HashMap<String, GeoPeerEntry>,
        /// Origin coordinates for score computation.
        origin: Option<GeoCoordinates>,
    }

    impl GeoRoutingTable {
        /// Create a new routing table with default configuration.
        pub fn new() -> Self {
            Self {
                config: GeoRoutingConfig::default(),
                peers: HashMap::new(),
                origin: None,
            }
        }

        /// Create a new routing table with custom configuration.
        pub fn with_config(config: GeoRoutingConfig) -> Self {
            Self {
                config,
                peers: HashMap::new(),
                origin: None,
            }
        }

        /// Set the origin coordinates for score computation.
        pub fn set_origin(&mut self, coordinates: GeoCoordinates) {
            self.origin = Some(coordinates);
        }

        /// Add or update a peer in the routing table.
        pub fn add_peer(
            &mut self,
            peer_id: String,
            initial_rtt_ms: f64,
            timestamp_ms: u64,
        ) -> Result<(), GeoRoutingError> {
            if self.peers.len() >= self.config.max_peers && !self.peers.contains_key(&peer_id) {
                return Err(GeoRoutingError::NoPeersAvailable);
            }

            let entry = GeoPeerEntry::new(peer_id.clone(), initial_rtt_ms, timestamp_ms);
            self.peers.insert(peer_id, entry);
            Ok(())
        }

        /// Add a peer with geographic coordinates.
        pub fn add_peer_with_geo(
            &mut self,
            peer_id: String,
            coordinates: GeoCoordinates,
            initial_rtt_ms: f64,
            timestamp_ms: u64,
        ) -> Result<(), GeoRoutingError> {
            if self.peers.len() >= self.config.max_peers && !self.peers.contains_key(&peer_id) {
                return Err(GeoRoutingError::NoPeersAvailable);
            }

            let entry = GeoPeerEntry::with_coordinates(
                peer_id.clone(),
                coordinates,
                initial_rtt_ms,
                timestamp_ms,
            );
            self.peers.insert(peer_id, entry);
            Ok(())
        }

        /// Update RTT measurement for an existing peer.
        pub fn update_rtt(
            &mut self,
            peer_id: &str,
            new_rtt_ms: f64,
            timestamp_ms: u64,
        ) -> Result<(), GeoRoutingError> {
            match self.peers.get_mut(peer_id) {
                Some(entry) => {
                    entry
                        .rtt
                        .update(new_rtt_ms, self.config.rtt_alpha, timestamp_ms);
                    Ok(())
                }
                None => Err(GeoRoutingError::PeerNotFound),
            }
        }

        /// Get the best peer for routing based on composite score.
        pub fn select_best_peer(&self, current_ms: u64) -> Result<String, GeoRoutingError> {
            let mut best_peer: Option<(String, f64)> = None;

            for entry in self.peers.values() {
                // Skip stale entries
                if entry
                    .rtt
                    .is_stale(current_ms, self.config.stale_threshold_ms)
                {
                    continue;
                }
                // Skip peers with insufficient samples
                if entry.rtt.sample_count < self.config.min_samples {
                    continue;
                }

                let score = entry.routing_score(&self.origin);
                match best_peer {
                    Some((_, best_score)) if score > best_score => {
                        best_peer = Some((entry.peer_id.clone(), score));
                    }
                    None => {
                        best_peer = Some((entry.peer_id.clone(), score));
                    }
                    _ => {}
                }
            }

            best_peer
                .map(|(id, _)| id)
                .ok_or(GeoRoutingError::NoPeersAvailable)
        }

        /// Get top N peers sorted by routing score.
        pub fn select_top_peers(&self, n: usize, current_ms: u64) -> Vec<String> {
            let mut scored_peers: Vec<(String, f64)> = self
                .peers
                .values()
                .filter(|entry| {
                    !entry
                        .rtt
                        .is_stale(current_ms, self.config.stale_threshold_ms)
                        && entry.rtt.sample_count >= self.config.min_samples
                })
                .map(|entry| (entry.peer_id.clone(), entry.routing_score(&self.origin)))
                .collect();

            scored_peers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scored_peers.into_iter().take(n).map(|(id, _)| id).collect()
        }

        /// Get peer entry by ID.
        pub fn get_peer(&self, peer_id: &str) -> Option<&GeoPeerEntry> {
            self.peers.get(peer_id)
        }

        /// Remove a peer from the routing table.
        pub fn remove_peer(&mut self, peer_id: &str) -> bool {
            self.peers.remove(peer_id).is_some()
        }

        /// Clean up stale entries. Returns number of removed entries.
        pub fn cleanup_stale(&mut self, current_ms: u64) -> usize {
            let stale_ids: Vec<String> = self
                .peers
                .values()
                .filter(|entry| {
                    entry
                        .rtt
                        .is_stale(current_ms, self.config.stale_threshold_ms)
                })
                .map(|entry| entry.peer_id.clone())
                .collect();

            let count = stale_ids.len();
            for id in stale_ids {
                self.peers.remove(&id);
            }
            count
        }

        /// Get the number of active (non-stale) peers.
        pub fn active_peer_count(&self, current_ms: u64) -> usize {
            self.peers
                .values()
                .filter(|entry| {
                    !entry
                        .rtt
                        .is_stale(current_ms, self.config.stale_threshold_ms)
                })
                .count()
        }

        /// Check if routing should fall back to Kademlia DHT.
        /// Returns true if fewer than `min_samples` peers have sufficient data.
        pub fn should_fallback_kad(&self, current_ms: u64, min_peers: usize) -> bool {
            let qualified = self
                .peers
                .values()
                .filter(|entry| {
                    !entry
                        .rtt
                        .is_stale(current_ms, self.config.stale_threshold_ms)
                        && entry.rtt.sample_count >= self.config.min_samples
                })
                .count();
            qualified < min_peers
        }
    }

    impl Default for GeoRoutingTable {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn test_origin() -> GeoCoordinates {
            GeoCoordinates::new(40.7128, -74.0060).unwrap() // New York
        }

        fn test_eu_peer() -> GeoCoordinates {
            GeoCoordinates::new(51.5074, -0.1278).unwrap() // London
        }

        fn test_jp_peer() -> GeoCoordinates {
            GeoCoordinates::new(35.6762, 139.6503).unwrap() // Tokyo
        }

        #[test]
        fn test_geo_coordinates_creation() {
            let coords = GeoCoordinates::new(40.7128, -74.0060).unwrap();
            assert!((coords.latitude - 40.7128).abs() < f64::EPSILON);
            assert!((coords.longitude - (-74.0060)).abs() < f64::EPSILON);
            assert!(coords.region.is_none());
        }

        #[test]
        fn test_geo_coordinates_with_region() {
            let coords = GeoCoordinates::new(40.7128, -74.0060)
                .unwrap()
                .with_region("us-east-1".to_string());
            assert_eq!(coords.region, Some("us-east-1".to_string()));
        }

        #[test]
        fn test_geo_coordinates_invalid_latitude() {
            assert!(GeoCoordinates::new(91.0, 0.0).is_err());
            assert!(GeoCoordinates::new(-91.0, 0.0).is_err());
        }

        #[test]
        fn test_geo_coordinates_invalid_longitude() {
            assert!(GeoCoordinates::new(0.0, 181.0).is_err());
            assert!(GeoCoordinates::new(0.0, -181.0).is_err());
        }

        #[test]
        fn test_haversine_distance() {
            let ny = test_origin();
            let london = test_eu_peer();
            let distance = ny.distance_to(&london);
            // NYC to London is approximately 5,570 km
            assert!(
                distance > 5000.0 && distance < 6000.0,
                "Expected ~5570km, got {}",
                distance
            );
        }

        #[test]
        fn test_same_coordinates_zero_distance() {
            let coords = test_origin();
            let distance = coords.distance_to(&coords);
            assert!((distance - 0.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_rtt_record_creation() {
            let record = RttRecord::new("peer1".to_string(), 50.0, 1000);
            assert_eq!(record.peer_id, "peer1");
            assert!((record.avg_rtt_ms - 50.0).abs() < f64::EPSILON);
            assert_eq!(record.sample_count, 1);
        }

        #[test]
        fn test_rtt_record_update() {
            let mut record = RttRecord::new("peer1".to_string(), 50.0, 1000);
            record.update(60.0, 0.3, 2000);
            // EMA: 0.3 * 60 + 0.7 * 50 = 18 + 35 = 53
            assert!((record.avg_rtt_ms - 53.0).abs() < 0.01);
            assert_eq!(record.sample_count, 2);
            assert!((record.min_rtt_ms - 50.0).abs() < f64::EPSILON);
            assert!((record.max_rtt_ms - 60.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_rtt_record_stale() {
            let record = RttRecord::new("peer1".to_string(), 50.0, 1000);
            assert!(!record.is_stale(1000, 30_000));
            assert!(record.is_stale(35_000, 30_000));
        }

        #[test]
        fn test_routing_table_add_peer() {
            let mut table = GeoRoutingTable::new();
            assert!(table.add_peer("peer1".to_string(), 50.0, 1000).is_ok());
            assert!(table.get_peer("peer1").is_some());
        }

        #[test]
        fn test_routing_table_add_peer_with_geo() {
            let mut table = GeoRoutingTable::new();
            let coords = test_origin();
            assert!(table
                .add_peer_with_geo("peer1".to_string(), coords, 50.0, 1000)
                .is_ok());
            let entry = table.get_peer("peer1").unwrap();
            assert!(entry.coordinates.is_some());
        }

        #[test]
        fn test_routing_table_update_rtt() {
            let mut table = GeoRoutingTable::new();
            table.add_peer("peer1".to_string(), 50.0, 1000).unwrap();
            assert!(table.update_rtt("peer1", 60.0, 2000).is_ok());
            let entry = table.get_peer("peer1").unwrap();
            assert!((entry.rtt.avg_rtt_ms - 53.0).abs() < 0.01);
        }

        #[test]
        fn test_routing_table_update_rtt_not_found() {
            let mut table = GeoRoutingTable::new();
            assert!(table.update_rtt("unknown", 50.0, 1000).is_err());
        }

        #[test]
        fn test_select_best_peer() {
            let mut table = GeoRoutingTable::new();
            // Add peers with enough samples
            table.add_peer("fast".to_string(), 20.0, 1000).unwrap();
            table.add_peer("slow".to_string(), 100.0, 1000).unwrap();

            // Manually bump sample counts for test
            for _ in 0..5 {
                table.update_rtt("fast", 20.0, 1000).unwrap();
                table.update_rtt("slow", 100.0, 1000).unwrap();
            }

            let best = table.select_best_peer(1000).unwrap();
            assert_eq!(best, "fast");
        }

        #[test]
        fn test_select_best_peer_no_qualified() {
            let table = GeoRoutingTable::new();
            assert!(table.select_best_peer(1000).is_err());
        }

        #[test]
        fn test_select_top_peers() {
            let mut table = GeoRoutingTable::new();
            table.add_peer("p1".to_string(), 10.0, 1000).unwrap();
            table.add_peer("p2".to_string(), 20.0, 1000).unwrap();
            table.add_peer("p3".to_string(), 30.0, 1000).unwrap();

            for _ in 0..5 {
                table.update_rtt("p1", 10.0, 1000).unwrap();
                table.update_rtt("p2", 20.0, 1000).unwrap();
                table.update_rtt("p3", 30.0, 1000).unwrap();
            }

            let top = table.select_top_peers(2, 1000);
            assert_eq!(top.len(), 2);
            assert_eq!(top[0], "p1");
            assert_eq!(top[1], "p2");
        }

        #[test]
        fn test_routing_score_with_geography() {
            let mut table = GeoRoutingTable::new();
            table.set_origin(test_origin());

            let eu = test_eu_peer();
            let jp = test_jp_peer();

            table
                .add_peer_with_geo("eu".to_string(), eu, 50.0, 1000)
                .unwrap();
            table
                .add_peer_with_geo("jp".to_string(), jp, 50.0, 1000)
                .unwrap();

            for _ in 0..5 {
                table.update_rtt("eu", 50.0, 1000).unwrap();
                table.update_rtt("jp", 50.0, 1000).unwrap();
            }

            let eu_score = table.get_peer("eu").unwrap().routing_score(&table.origin);
            let jp_score = table.get_peer("jp").unwrap().routing_score(&table.origin);
            // EU should score higher (closer to NYC)
            assert!(
                eu_score > jp_score,
                "EU score {} > JP score {}",
                eu_score,
                jp_score
            );
        }

        #[test]
        fn test_cleanup_stale() {
            let mut table = GeoRoutingTable::new();
            table.add_peer("fresh".to_string(), 50.0, 10_000).unwrap();
            table.add_peer("stale".to_string(), 50.0, 1_000).unwrap();

            let removed = table.cleanup_stale(40_000);
            assert_eq!(removed, 1);
            assert!(table.get_peer("fresh").is_some());
            assert!(table.get_peer("stale").is_none());
        }

        #[test]
        fn test_should_fallback_kad() {
            let table = GeoRoutingTable::new();
            assert!(table.should_fallback_kad(1000, 3));
        }

        #[test]
        fn test_remove_peer() {
            let mut table = GeoRoutingTable::new();
            table.add_peer("peer1".to_string(), 50.0, 1000).unwrap();
            assert!(table.remove_peer("peer1"));
            assert!(table.get_peer("peer1").is_none());
            assert!(!table.remove_peer("peer1"));
        }

        #[test]
        fn test_active_peer_count() {
            let mut table = GeoRoutingTable::new();
            table.add_peer("fresh".to_string(), 50.0, 10_000).unwrap();
            table.add_peer("stale".to_string(), 50.0, 1_000).unwrap();

            assert_eq!(table.active_peer_count(10_000), 2);
            assert_eq!(table.active_peer_count(40_000), 1);
        }

        #[test]
        fn test_config_default() {
            let config = GeoRoutingConfig::default();
            assert!((config.rtt_alpha - 0.3).abs() < f64::EPSILON);
            assert_eq!(config.stale_threshold_ms, 30_000);
            assert_eq!(config.max_peers, 256);
            assert_eq!(config.min_samples, 3);
        }

        #[test]
        fn test_error_display() {
            assert_eq!(
                format!("{}", GeoRoutingError::PeerNotFound),
                "Peer not found in routing table"
            );
            assert_eq!(
                format!("{}", GeoRoutingError::InvalidCoordinates),
                "Invalid geographic coordinates"
            );
        }

        #[test]
        fn test_max_peers_limit() {
            let mut config = GeoRoutingConfig::default();
            config.max_peers = 2;
            let mut table = GeoRoutingTable::with_config(config);

            assert!(table.add_peer("p1".to_string(), 50.0, 1000).is_ok());
            assert!(table.add_peer("p2".to_string(), 50.0, 1000).is_ok());
            // Table is full, adding new peer should fail
            assert!(table.add_peer("p3".to_string(), 50.0, 1000).is_err());
            // Updating existing peer should still work
            assert!(table.add_peer("p1".to_string(), 30.0, 2000).is_ok());
        }
    }
}

pub use internal::{
    GeoCoordinates, GeoPeerEntry, GeoRoutingConfig, GeoRoutingError, GeoRoutingTable, RttRecord,
};
