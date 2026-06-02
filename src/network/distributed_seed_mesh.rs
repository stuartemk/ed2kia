//! Distributed Seed Mesh — Sprint 75: Thermodynamic Hardening & Asynchronous Neuro-Symbolic Pivot
//!
//! Bootstrap mesh across multiple regions/ISPs with cryptographic rotation.
//! Resistance to centralized takedown via geographic and ISP diversity.

use std::collections::HashMap;
use std::fmt;

/// Seed mesh errors.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshError {
    InsufficientRegions(usize),
    InvalidRotationInterval(u64),
    NodeNotFound(u64),
    RegionConflict(String),
    IsbDiversityTooLow(u32),
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeshError::InsufficientRegions(n) => write!(f, "Insufficient regions: {}", n),
            MeshError::InvalidRotationInterval(s) => write!(f, "Invalid rotation interval: {}s", s),
            MeshError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            MeshError::RegionConflict(r) => write!(f, "Region conflict: {}", r),
            MeshError::IsbDiversityTooLow(n) => write!(f, "ISP diversity too low: {}", n),
        }
    }
}

/// Geographic region for seed distribution.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub name: String,
    pub iso_code: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl Region {
    pub fn new(name: String, iso_code: String, latitude: f64, longitude: f64) -> Self {
        Self {
            name,
            iso_code,
            latitude,
            longitude,
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Region {{ {}, {}, lat: {:.2}, lon: {:.2} }}",
            self.name, self.iso_code, self.latitude, self.longitude
        )
    }
}

/// Seed node in the mesh.
#[derive(Debug, Clone)]
pub struct SeedNode {
    pub node_id: u64,
    pub region: Region,
    pub isp: String,
    pub public_key: [u8; 32],
    pub active: bool,
    pub last_rotation_ms: u64,
    pub mesh_score: f64,
}

impl SeedNode {
    pub fn new(node_id: u64, region: Region, isp: String, public_key: [u8; 32]) -> Self {
        Self {
            node_id,
            region,
            isp,
            public_key,
            active: true,
            last_rotation_ms: 0,
            mesh_score: 1.0,
        }
    }
}

impl fmt::Display for SeedNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SeedNode {{ id: {}, region: {}, isp: {}, active: {}, score: {:.3} }}",
            self.node_id, self.region.name, self.isp, self.active, self.mesh_score
        )
    }
}

/// Seed mesh configuration.
#[derive(Debug, Clone)]
pub struct SeedMeshConfig {
    pub regions: Vec<Region>,
    pub isp_diversity: u32,
    pub rotation_interval_s: u64,
    pub min_nodes_per_region: usize,
    pub min_regions: usize,
}

impl SeedMeshConfig {
    pub fn validate(&self) -> Result<(), MeshError> {
        if self.regions.len() < self.min_regions {
            return Err(MeshError::InsufficientRegions(self.regions.len()));
        }
        if self.rotation_interval_s < 300 {
            return Err(MeshError::InvalidRotationInterval(self.rotation_interval_s));
        }
        if self.isp_diversity < 2 {
            return Err(MeshError::IsbDiversityTooLow(self.isp_diversity));
        }
        Ok(())
    }
}

impl fmt::Display for SeedMeshConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SeedMeshConfig {{ regions: {}, isp_diversity: {}, rotation: {}s, min_nodes/region: {} }}",
            self.regions.len(),
            self.isp_diversity,
            self.rotation_interval_s,
            self.min_nodes_per_region
        )
    }
}

/// Rotation record.
#[derive(Debug, Clone)]
pub struct RotationRecord {
    pub node_id: u64,
    pub old_key: [u8; 32],
    pub new_key: [u8; 32],
    pub timestamp_ms: u64,
}

impl fmt::Display for RotationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RotationRecord {{ node: {}, ts: {} }}",
            self.node_id, self.timestamp_ms
        )
    }
}

/// Distributed Seed Mesh engine.
pub struct DistributedSeedMesh {
    pub config: SeedMeshConfig,
    nodes: HashMap<u64, SeedNode>,
    rotation_history: Vec<RotationRecord>,
    total_rotations: usize,
}

impl DistributedSeedMesh {
    pub fn new() -> Self {
        Self {
            config: SeedMeshConfig {
                regions: Vec::new(),
                isp_diversity: 3,
                rotation_interval_s: 3600,
                min_nodes_per_region: 2,
                min_regions: 3,
            },
            nodes: HashMap::new(),
            rotation_history: Vec::new(),
            total_rotations: 0,
        }
    }

    /// Initialize seed mesh with regions, ISP diversity, and rotation interval.
    pub fn initialize_seed_mesh(
        regions: &[Region],
        isp_diversity: u32,
        rotation_interval_s: u64,
    ) -> SeedMeshConfig {
        SeedMeshConfig {
            regions: regions.to_vec(),
            isp_diversity,
            rotation_interval_s,
            min_nodes_per_region: 2,
            min_regions: 3,
        }
    }

    pub fn with_config(config: SeedMeshConfig) -> Result<Self, MeshError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            rotation_history: Vec::new(),
            total_rotations: 0,
        })
    }

    /// Add a seed node to the mesh.
    pub fn add_node(&mut self, node: SeedNode) -> Result<(), MeshError> {
        if self.nodes.contains_key(&node.node_id) {
            return Err(MeshError::RegionConflict(format!(
                "Node {} already exists",
                node.node_id
            )));
        }
        self.nodes.insert(node.node_id, node);
        Ok(())
    }

    /// Rotate public key for a node (cryptographic rotation).
    pub fn rotate_key(
        &mut self,
        node_id: u64,
        new_key: [u8; 32],
        current_ms: u64,
    ) -> Result<RotationRecord, MeshError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(MeshError::NodeNotFound(node_id))?;
        let old_key = node.public_key;
        node.public_key = new_key;
        node.last_rotation_ms = current_ms;

        let record = RotationRecord {
            node_id,
            old_key,
            new_key,
            timestamp_ms: current_ms,
        };
        self.rotation_history.push(record.clone());
        self.total_rotations += 1;
        Ok(record)
    }

    /// Check if rotation is due based on interval.
    pub fn is_rotation_due(&self, node_id: u64, current_ms: u64) -> Result<bool, MeshError> {
        let node = self
            .nodes
            .get(&node_id)
            .ok_or(MeshError::NodeNotFound(node_id))?;
        let interval_ms = self.config.rotation_interval_s * 1000;
        Ok(current_ms - node.last_rotation_ms >= interval_ms)
    }

    /// Get active nodes in a region.
    pub fn get_nodes_in_region(&self, region_name: &str) -> Vec<&SeedNode> {
        self.nodes
            .values()
            .filter(|n| n.active && n.region.name == region_name)
            .collect()
    }

    /// Compute Shannon diversity index across ISPs.
    pub fn isp_diversity_index(&self) -> f64 {
        let mut isp_counts: HashMap<String, usize> = HashMap::new();
        for node in self.nodes.values() {
            *isp_counts.entry(node.isp.clone()).or_insert(0) += 1;
        }
        let total = self.nodes.len();
        if total == 0 {
            return 0.0;
        }
        let total = total as f64;
        let entropy: f64 = isp_counts
            .values()
            .map(|&count| {
                let p = count as f64 / total;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum();
        entropy
    }

    /// Compute geographic spread (variance of lat/lon).
    pub fn geographic_spread(&self) -> f64 {
        let nodes: Vec<&SeedNode> = self.nodes.values().collect();
        if nodes.len() < 2 {
            return 0.0;
        }
        let n = nodes.len() as f64;
        let avg_lat: f64 = nodes.iter().map(|n| n.region.latitude).sum::<f64>() / n;
        let avg_lon: f64 = nodes.iter().map(|n| n.region.longitude).sum::<f64>() / n;

        let lat_var: f64 = nodes
            .iter()
            .map(|n| (n.region.latitude - avg_lat).powi(2))
            .sum::<f64>()
            / n;
        let lon_var: f64 = nodes
            .iter()
            .map(|n| (n.region.longitude - avg_lon).powi(2))
            .sum::<f64>()
            / n;

        (lat_var + lon_var).sqrt()
    }

    /// Deactivate a node (graceful removal).
    pub fn deactivate_node(&mut self, node_id: u64) -> Result<(), MeshError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(MeshError::NodeNotFound(node_id))?;
        node.active = false;
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.nodes.values().filter(|n| n.active).count()
    }

    pub fn region_count(&self) -> usize {
        let mut regions = std::collections::HashSet::new();
        for node in self.nodes.values() {
            regions.insert(node.region.name.clone());
        }
        regions.len()
    }

    pub fn reset(&mut self) {
        self.nodes.clear();
        self.rotation_history.clear();
        self.total_rotations = 0;
    }
}

impl Default for DistributedSeedMesh {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DistributedSeedMesh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DistributedSeedMesh {{ nodes: {}, active: {}, regions: {}, isp_diversity: {:.3}, rotations: {} }}",
            self.nodes.len(),
            self.active_count(),
            self.region_count(),
            self.isp_diversity_index(),
            self.total_rotations
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_region(name: &str, lat: f64, lon: f64) -> Region {
        Region::new(name.to_string(), format!("{}-CODE", name), lat, lon)
    }

    fn make_key(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    #[test]
    fn test_region_creation() {
        let r = make_region("US-East", 40.7, -74.0);
        assert_eq!(r.name, "US-East");
    }

    #[test]
    fn test_node_creation() {
        let node = SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        );
        assert_eq!(node.node_id, 1);
        assert!(node.active);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = SeedMeshConfig {
            regions: vec![
                make_region("US", 40.0, -74.0),
                make_region("EU", 50.0, 10.0),
                make_region("APAC", 35.0, 139.0),
            ],
            isp_diversity: 3,
            rotation_interval_s: 3600,
            min_nodes_per_region: 1,
            min_regions: 3,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_insufficient_regions() {
        let config = SeedMeshConfig {
            regions: vec![make_region("US", 40.0, -74.0)],
            isp_diversity: 3,
            rotation_interval_s: 3600,
            min_nodes_per_region: 1,
            min_regions: 3,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_low_isp_diversity() {
        let mut config = SeedMeshConfig {
            regions: vec![
                make_region("US", 40.0, -74.0),
                make_region("EU", 50.0, 10.0),
                make_region("APAC", 35.0, 139.0),
            ],
            isp_diversity: 1,
            rotation_interval_s: 3600,
            min_nodes_per_region: 1,
            min_regions: 3,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_initialize_seed_mesh() {
        let regions = vec![
            make_region("US", 40.0, -74.0),
            make_region("EU", 50.0, 10.0),
            make_region("APAC", 35.0, 139.0),
        ];
        let config = DistributedSeedMesh::initialize_seed_mesh(&regions, 3, 3600);
        assert_eq!(config.regions.len(), 3);
        assert_eq!(config.rotation_interval_s, 3600);
    }

    #[test]
    fn test_add_node() {
        let mut mesh = DistributedSeedMesh::new();
        let node = SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        );
        assert!(mesh.add_node(node).is_ok());
        assert_eq!(mesh.nodes.len(), 1);
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut mesh = DistributedSeedMesh::new();
        let node = SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        );
        mesh.add_node(node).unwrap();
        let node2 = SeedNode::new(
            1,
            make_region("EU", 50.0, 10.0),
            "Deutsche".to_string(),
            make_key(2),
        );
        assert!(mesh.add_node(node2).is_err());
    }

    #[test]
    fn test_rotate_key() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        let record = mesh.rotate_key(1, make_key(2), 1000).unwrap();
        assert_eq!(record.node_id, 1);
        assert_eq!(mesh.total_rotations, 1);
    }

    #[test]
    fn test_rotation_due() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.config.rotation_interval_s = 60;
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        assert!(!mesh.is_rotation_due(1, 50_000).unwrap()); // < 60s
        assert!(mesh.is_rotation_due(1, 70_000).unwrap()); // > 60s
    }

    #[test]
    fn test_get_nodes_in_region() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        mesh.add_node(SeedNode::new(
            2,
            make_region("EU", 50.0, 10.0),
            "Deutsche".to_string(),
            make_key(2),
        ))
        .unwrap();
        let us_nodes = mesh.get_nodes_in_region("US");
        assert_eq!(us_nodes.len(), 1);
    }

    #[test]
    fn test_isp_diversity_single() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        assert!((mesh.isp_diversity_index() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_isp_diversity_multiple() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        mesh.add_node(SeedNode::new(
            2,
            make_region("US", 40.0, -74.0),
            "Verizon".to_string(),
            make_key(2),
        ))
        .unwrap();
        let diversity = mesh.isp_diversity_index();
        assert!(diversity > 0.0);
    }

    #[test]
    fn test_geographic_spread() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        mesh.add_node(SeedNode::new(
            2,
            make_region("EU", 50.0, 10.0),
            "Deutsche".to_string(),
            make_key(2),
        ))
        .unwrap();
        let spread = mesh.geographic_spread();
        assert!(spread > 0.0);
    }

    #[test]
    fn test_deactivate_node() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        mesh.deactivate_node(1).unwrap();
        assert_eq!(mesh.active_count(), 0);
    }

    #[test]
    fn test_reset() {
        let mut mesh = DistributedSeedMesh::new();
        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        mesh.reset();
        assert_eq!(mesh.nodes.len(), 0);
        assert_eq!(mesh.total_rotations, 0);
    }

    #[test]
    fn test_display() {
        let mesh = DistributedSeedMesh::new();
        let s = format!("{}", mesh);
        assert!(s.contains("DistributedSeedMesh"));
    }

    #[test]
    fn test_full_workflow() {
        let regions = vec![
            make_region("US", 40.0, -74.0),
            make_region("EU", 50.0, 10.0),
            make_region("APAC", 35.0, 139.0),
        ];
        let config = DistributedSeedMesh::initialize_seed_mesh(&regions, 3, 3600);
        let mut mesh = DistributedSeedMesh::with_config(config).unwrap();

        mesh.add_node(SeedNode::new(
            1,
            make_region("US", 40.0, -74.0),
            "AT&T".to_string(),
            make_key(1),
        ))
        .unwrap();
        mesh.add_node(SeedNode::new(
            2,
            make_region("EU", 50.0, 10.0),
            "Deutsche".to_string(),
            make_key(2),
        ))
        .unwrap();
        mesh.add_node(SeedNode::new(
            3,
            make_region("APAC", 35.0, 139.0),
            "NTT".to_string(),
            make_key(3),
        ))
        .unwrap();

        assert_eq!(mesh.active_count(), 3);
        assert_eq!(mesh.region_count(), 3);
        assert!(mesh.isp_diversity_index() > 0.0);
        assert!(mesh.geographic_spread() > 0.0);

        mesh.rotate_key(1, make_key(99), 1000).unwrap();
        assert_eq!(mesh.total_rotations, 1);
    }

    #[test]
    fn test_error_display() {
        let err = MeshError::InsufficientRegions(1);
        assert!(format!("{}", err).contains("1"));
    }
}
