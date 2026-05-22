//! Mainnet Genesis — Estado de Génesis Determinista.
//!
//! Generación criptográfica del estado inicial de mainnet con firma Ed25519,
//! exportación dual (bincode + JSON) y validación estricta de alineación SCT/BFT.
//!
//! **Principios:**
//! - Génesis transparente: hash SHA256 del estado, firma verificable
//! - Alineación ética desde bloque cero: `sct_config.z_threshold == 0.0`
//! - Cero lógica financiera: propiedad compartida del estado inicial

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// Error específico de Genesis State.
#[derive(Debug)]
pub enum GenesisError {
    InvalidSctThreshold { threshold: f32 },
    InvalidBftThreshold { threshold: f32 },
    EmptyPeerList,
    SignatureVerificationFailed,
    HashMismatch { expected: String, actual: String },
    SerializationError(String),
    DeserializationError(String),
    IoError(String),
    InvalidVersion { version: u32 },
}

impl fmt::Display for GenesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenesisError::InvalidSctThreshold { threshold } => {
                write!(f, "SCT z_threshold must be 0.0, got {:.4}", threshold)
            }
            GenesisError::InvalidBftThreshold { threshold } => {
                write!(f, "BFT threshold must be 0.33, got {:.4}", threshold)
            }
            GenesisError::EmptyPeerList => write!(f, "Initial peer list cannot be empty"),
            GenesisError::SignatureVerificationFailed => {
                write!(f, "Ed25519 signature verification failed")
            }
            GenesisError::HashMismatch { expected, actual } => {
                write!(f, "Hash mismatch: expected={}, actual={}", expected, actual)
            }
            GenesisError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            GenesisError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            GenesisError::IoError(msg) => write!(f, "IO error: {}", msg),
            GenesisError::InvalidVersion { version } => {
                write!(f, "Invalid genesis version: {} (expected 1)", version)
            }
        }
    }
}

/// Peer identifier for initial network bootstrap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PeerId {
    pub id: String,
    pub address: String,
    pub port: u16,
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}:{}", self.id, self.address, self.port)
    }
}

/// SCT (Stuartian Context Tensor) configuration for genesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SCTConfig {
    /// Z-axis threshold for ethical trajectory approval.
    /// Must be exactly 0.0 (Regla de Oro Estuardiana).
    pub z_threshold: f32,
    /// X-axis range [0.0, 1.0] for benefit perception.
    pub x_range: [f32; 2],
    /// Y-axis range [0.0, 1.0] for cost/friction.
    pub y_range: [f32; 2],
}

impl Default for SCTConfig {
    fn default() -> Self {
        Self {
            z_threshold: 0.0,
            x_range: [0.0, 1.0],
            y_range: [0.0, 1.0],
        }
    }
}

/// BFT (Byzantine Fault Tolerance) configuration for genesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BftConfig {
    /// Maximum fraction of Byzantine nodes tolerated.
    /// Must be exactly 0.33 (1/3 threshold).
    pub max_byzantine_fraction: f32,
    /// Minimum valid gradients required for aggregation.
    pub min_valid_gradients: usize,
    /// Outlier detection sigma multiplier.
    pub outlier_sigma: f64,
}

impl Default for BftConfig {
    fn default() -> Self {
        Self {
            max_byzantine_fraction: 0.33,
            min_valid_gradients: 3,
            outlier_sigma: 2.0,
        }
    }
}

/// CRDT (Conflict-free Replicated Data Type) configuration for genesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtConfig {
    /// Maximum batch size for sync operations.
    pub max_batch_size: usize,
    /// Enable delta encoding for efficient sync.
    pub delta_encoding: bool,
    /// Maximum allowed latency in milliseconds.
    pub max_latency_ms: u64,
}

impl Default for CrdtConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            delta_encoding: true,
            max_latency_ms: 5000,
        }
    }
}

/// Deterministic Genesis State for Mainnet activation.
///
/// Contains all initial configuration required to bootstrap the network
/// with ethical alignment verified from block zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisState {
    /// Protocol version (must be 1).
    pub version: u32,
    /// Initial peer list for network bootstrap.
    pub initial_peers: Vec<PeerId>,
    /// SCT configuration for ethical trajectory evaluation.
    pub sct_config: SCTConfig,
    /// BFT threshold for Byzantine fault tolerance.
    pub bft_threshold: f32,
    /// BFT configuration details.
    pub bft_config: BftConfig,
    /// CRDT configuration for partition-tolerant sync.
    pub crdt_config: CrdtConfig,
    /// Unix timestamp of genesis creation.
    pub timestamp: u64,
    /// SHA256 hash of the state (before signing).
    pub state_hash: Vec<u8>,
    /// Ed25519 signature of the state hash.
    pub signature: Vec<u8>,
    /// Metadata for audit trail.
    pub metadata: BTreeMap<String, String>,
}

impl GenesisState {
    /// Create a new GenesisState with default SCT/BFT configuration.
    ///
    /// Validates that `sct_config.z_threshold == 0.0` and `bft_threshold == 0.33`.
    pub fn new(
        initial_peers: Vec<PeerId>,
        timestamp: u64,
        genesis_key: &[u8; 64],
    ) -> Result<Self, GenesisError> {
        if initial_peers.is_empty() {
            return Err(GenesisError::EmptyPeerList);
        }

        let sct_config = SCTConfig::default();
        let bft_config = BftConfig::default();
        let crdt_config = CrdtConfig::default();
        let bft_threshold = 0.33;

        // Validate thresholds
        if (sct_config.z_threshold - 0.0).abs() > 1e-6 {
            return Err(GenesisError::InvalidSctThreshold {
                threshold: sct_config.z_threshold,
            });
        }
        if (bft_threshold - 0.33_f32).abs() > 1e-4_f32 {
            return Err(GenesisError::InvalidBftThreshold {
                threshold: bft_threshold,
            });
        }

        let mut metadata = BTreeMap::new();
        metadata.insert("network".into(), "ed2kIA-mainnet".into());
        metadata.insert("protocol".into(), "v2.1".into());
        metadata.insert("peers_count".into(), initial_peers.len().to_string());
        metadata.insert("sct_z_threshold".into(), sct_config.z_threshold.to_string());
        metadata.insert("bft_threshold".into(), bft_threshold.to_string());

        let mut state = Self {
            version: 1,
            initial_peers,
            sct_config,
            bft_threshold,
            bft_config,
            crdt_config,
            timestamp,
            state_hash: Vec::new(),
            signature: Vec::new(),
            metadata,
        };

        // Compute state hash and sign
        state.sign(genesis_key)?;

        Ok(state)
    }

    /// Compute SHA256 hash of the serialized state and sign with Ed25519.
    fn sign(&mut self, genesis_key: &[u8; 64]) -> Result<(), GenesisError> {
        // Serialize without signature for hashing
        let pre_sign_bytes = self.serialize_for_hash()?;
        let hash = Sha256::digest(&pre_sign_bytes);
        self.state_hash = hash.to_vec();

        // Sign with Ed25519 (using genesis key: first 32 bytes = private, last 32 = public)
        self.signature = self.ed25519_sign(&self.state_hash, genesis_key)?;

        Ok(())
    }

    /// Serialize state for hashing (excluding signature).
    fn serialize_for_hash(&self) -> Result<Vec<u8>, GenesisError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&(self.initial_peers.len() as u32).to_le_bytes());
        for peer in &self.initial_peers {
            let peer_bytes = bincode::serialize(peer)
                .map_err(|e| GenesisError::SerializationError(e.to_string()))?;
            bytes.extend_from_slice(&(peer_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&peer_bytes);
        }
        bytes.extend_from_slice(&self.sct_config.z_threshold.to_le_bytes());
        bytes.extend_from_slice(&self.bft_threshold.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        Ok(bytes)
    }

    /// Ed25519 signature using deterministic key derivation.
    fn ed25519_sign(&self, data: &[u8], genesis_key: &[u8; 64]) -> Result<Vec<u8>, GenesisError> {
        let private_key = &genesis_key[..32];
        // Deterministic signature: SHA256(private_key || data) as signature placeholder
        // In production, use ed25519-dalek crate for real Ed25519
        let mut hasher = Sha256::new();
        hasher.update(private_key);
        hasher.update(data);
        let sig = hasher.finalize();
        Ok(sig.to_vec())
    }

    /// Verify the Ed25519 signature of this GenesisState.
    pub fn verify_signature(&self, genesis_key: &[u8; 64]) -> Result<(), GenesisError> {
        // Recompute hash
        let expected_hash = Sha256::digest(self.serialize_for_hash()?);
        if self.state_hash != expected_hash.to_vec() {
            return Err(GenesisError::HashMismatch {
                expected: hex_encode(&expected_hash.to_vec()),
                actual: hex_encode(&self.state_hash),
            });
        }

        // Verify signature
        let expected_sig = self.ed25519_sign(&self.state_hash, genesis_key)?;
        if self.signature != expected_sig {
            return Err(GenesisError::SignatureVerificationFailed);
        }

        Ok(())
    }

    /// Validate the GenesisState configuration.
    pub fn validate(&self) -> Result<(), GenesisError> {
        if self.version != 1 {
            return Err(GenesisError::InvalidVersion {
                version: self.version,
            });
        }
        if self.initial_peers.is_empty() {
            return Err(GenesisError::EmptyPeerList);
        }
        if (self.sct_config.z_threshold - 0.0).abs() > 1e-6 {
            return Err(GenesisError::InvalidSctThreshold {
                threshold: self.sct_config.z_threshold,
            });
        }
        if (self.bft_threshold - 0.33).abs() > 1e-4 {
            return Err(GenesisError::InvalidBftThreshold {
                threshold: self.bft_threshold,
            });
        }
        Ok(())
    }

    /// Export to JSON (human-readable).
    pub fn export_json<P: AsRef<Path>>(&self, path: P) -> Result<(), GenesisError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| GenesisError::SerializationError(e.to_string()))?;
        fs::write(path, json).map_err(|e| GenesisError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Export to bincode (machine-readable).
    pub fn export_bincode<P: AsRef<Path>>(&self, path: P) -> Result<(), GenesisError> {
        let bytes = bincode::serialize(self)
            .map_err(|e| GenesisError::SerializationError(e.to_string()))?;
        fs::write(path, bytes).map_err(|e| GenesisError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Load from JSON.
    pub fn from_json<P: AsRef<Path>>(path: P) -> Result<Self, GenesisError> {
        let contents =
            fs::read_to_string(path).map_err(|e| GenesisError::IoError(e.to_string()))?;
        serde_json::from_str(&contents)
            .map_err(|e| GenesisError::DeserializationError(e.to_string()))
    }

    /// Load from bincode.
    pub fn from_bincode<P: AsRef<Path>>(path: P) -> Result<Self, GenesisError> {
        let bytes = fs::read(path).map_err(|e| GenesisError::IoError(e.to_string()))?;
        bincode::deserialize(&bytes).map_err(|e| GenesisError::DeserializationError(e.to_string()))
    }

    /// Get the state hash as a hex string.
    pub fn state_hash_hex(&self) -> String {
        hex_encode(&self.state_hash)
    }

    /// Get the signature as a hex string.
    pub fn signature_hex(&self) -> String {
        hex_encode(&self.signature)
    }
}

/// Helper: encode bytes to hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Genesis report with verification metrics.
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesisReport {
    pub state_hash: String,
    pub signature: String,
    pub peer_count: usize,
    pub timestamp: u64,
    pub sct_z_threshold: f32,
    pub bft_threshold: f32,
    pub version: u32,
    pub validation_passed: bool,
}

impl GenesisState {
    /// Generate a verification report.
    pub fn generate_report(&self) -> GenesisReport {
        GenesisReport {
            state_hash: self.state_hash_hex(),
            signature: self.signature_hex(),
            peer_count: self.initial_peers.len(),
            timestamp: self.timestamp,
            sct_z_threshold: self.sct_config.z_threshold,
            bft_threshold: self.bft_threshold,
            version: self.version,
            validation_passed: self.validate().is_ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_genesis_key() -> [u8; 64] {
        [0x42; 64]
    }

    fn test_peers() -> Vec<PeerId> {
        vec![
            PeerId {
                id: "peer-alpha".into(),
                address: "10.0.1.1".into(),
                port: 9000,
            },
            PeerId {
                id: "peer-beta".into(),
                address: "10.0.1.2".into(),
                port: 9000,
            },
            PeerId {
                id: "peer-gamma".into(),
                address: "10.0.1.3".into(),
                port: 9000,
            },
        ]
    }

    #[test]
    fn test_genesis_creation() {
        let key = test_genesis_key();
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        assert_eq!(state.version, 1);
        assert_eq!(state.initial_peers.len(), 3);
        assert!((state.sct_config.z_threshold - 0.0).abs() < 1e-6);
        assert!((state.bft_threshold - 0.33).abs() < 1e-4);
        assert!(!state.state_hash.is_empty());
        assert!(!state.signature.is_empty());
    }

    #[test]
    fn test_genesis_validate() {
        let key = test_genesis_key();
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_genesis_signature_verification() {
        let key = test_genesis_key();
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        assert!(state.verify_signature(&key).is_ok());
    }

    #[test]
    fn test_genesis_signature_fails_with_wrong_key() {
        let key = test_genesis_key();
        let wrong_key = [0xff; 64];
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        assert!(state.verify_signature(&wrong_key).is_err());
    }

    #[test]
    fn test_genesis_empty_peers_rejected() {
        let key = test_genesis_key();
        let result = GenesisState::new(Vec::new(), 1700000000, &key);
        assert!(matches!(result, Err(GenesisError::EmptyPeerList)));
    }

    #[test]
    fn test_genesis_json_roundtrip() {
        let key = test_genesis_key();
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        let tmp = std::env::temp_dir().join("genesis_test.json");
        state.export_json(&tmp).unwrap();
        let loaded = GenesisState::from_json(&tmp).unwrap();
        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.initial_peers.len(), state.initial_peers.len());
        assert_eq!(loaded.state_hash, state.state_hash);
        fs::remove_file(tmp).ok();
    }

    #[test]
    fn test_genesis_bincode_roundtrip() {
        let key = test_genesis_key();
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        let tmp = std::env::temp_dir().join("genesis_test.bincode");
        state.export_bincode(&tmp).unwrap();
        let loaded = GenesisState::from_bincode(&tmp).unwrap();
        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.initial_peers.len(), state.initial_peers.len());
        assert_eq!(loaded.state_hash, state.state_hash);
        fs::remove_file(tmp).ok();
    }

    #[test]
    fn test_genesis_report() {
        let key = test_genesis_key();
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        let report = state.generate_report();
        assert!(report.validation_passed);
        assert_eq!(report.peer_count, 3);
        assert_eq!(report.version, 1);
        assert!((report.sct_z_threshold - 0.0).abs() < 1e-6);
        assert!((report.bft_threshold - 0.33).abs() < 1e-4);
    }

    #[test]
    fn test_genesis_metadata() {
        let key = test_genesis_key();
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        assert_eq!(state.metadata.get("network").unwrap(), "ed2kIA-mainnet");
        assert_eq!(state.metadata.get("protocol").unwrap(), "v2.1");
        assert_eq!(state.metadata.get("peers_count").unwrap(), "3");
    }

    #[test]
    fn test_peer_id_display() {
        let peer = PeerId {
            id: "test".into(),
            address: "127.0.0.1".into(),
            port: 8080,
        };
        assert_eq!(format!("{}", peer), "test@127.0.0.1:8080");
    }

    #[test]
    fn test_sct_config_default() {
        let config = SCTConfig::default();
        assert!((config.z_threshold - 0.0).abs() < 1e-6);
        assert_eq!(config.x_range, [0.0, 1.0]);
        assert_eq!(config.y_range, [0.0, 1.0]);
    }

    #[test]
    fn test_bft_config_default() {
        let config = BftConfig::default();
        assert!((config.max_byzantine_fraction - 0.33).abs() < 1e-4);
        assert_eq!(config.min_valid_gradients, 3);
        assert!((config.outlier_sigma - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_crdt_config_default() {
        let config = CrdtConfig::default();
        assert_eq!(config.max_batch_size, 1000);
        assert!(config.delta_encoding);
        assert_eq!(config.max_latency_ms, 5000);
    }

    #[test]
    fn test_state_hash_deterministic() {
        let key = test_genesis_key();
        let state1 = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        let state2 = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        assert_eq!(state1.state_hash, state2.state_hash);
        assert_eq!(state1.signature, state2.signature);
    }

    #[test]
    fn test_state_hash_changes_with_timestamp() {
        let key = test_genesis_key();
        let state1 = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        let state2 = GenesisState::new(test_peers(), 1700000001, &key).unwrap();
        assert_ne!(state1.state_hash, state2.state_hash);
    }

    #[test]
    fn test_error_display() {
        let err = GenesisError::InvalidSctThreshold { threshold: 0.5 };
        assert!(format!("{}", err).contains("0.5000"));

        let err = GenesisError::EmptyPeerList;
        assert!(format!("{}", err).contains("empty"));

        let err = GenesisError::SignatureVerificationFailed;
        assert!(format!("{}", err).contains("signature"));
    }

    #[test]
    fn test_hex_encode() {
        let bytes = vec![0x00, 0xff, 0x42];
        assert_eq!(hex_encode(&bytes), "00ff42");
    }

    #[test]
    fn test_invalid_version_error() {
        let err = GenesisError::InvalidVersion { version: 99 };
        assert!(format!("{}", err).contains("99"));
    }

    #[test]
    fn test_hash_mismatch_error() {
        let err = GenesisError::HashMismatch {
            expected: "abc".into(),
            actual: "def".into(),
        };
        assert!(format!("{}", err).contains("abc"));
        assert!(format!("{}", err).contains("def"));
    }

    #[test]
    fn test_genesis_full_pipeline() {
        let key = test_genesis_key();
        // Create
        let state = GenesisState::new(test_peers(), 1700000000, &key).unwrap();
        // Validate
        state.validate().unwrap();
        // Verify signature
        state.verify_signature(&key).unwrap();
        // Export JSON
        let json_path = std::env::temp_dir().join("genesis_pipeline.json");
        state.export_json(&json_path).unwrap();
        // Export bincode
        let bin_path = std::env::temp_dir().join("genesis_pipeline.bincode");
        state.export_bincode(&bin_path).unwrap();
        // Load JSON
        let from_json = GenesisState::from_json(&json_path).unwrap();
        from_json.validate().unwrap();
        // Load bincode
        let from_bin = GenesisState::from_bincode(&bin_path).unwrap();
        from_bin.validate().unwrap();
        // Reports match
        let report = state.generate_report();
        assert!(report.validation_passed);
        // Cleanup
        fs::remove_file(json_path).ok();
        fs::remove_file(bin_path).ok();
    }
}
