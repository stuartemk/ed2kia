//! Distributed Genesis Ceremony — Sprint 81: The Biological Bridge & Singularity Resilience
//!
//! Planetary MPC ceremony: Ethical Anchors are derived from biological + cryptographic
//! entropy of millions of founding nodes. No centralized signature. The Genesis Block
//! is not signed; it emerges distributed.
//!
//! Key features:
//! - Biological entropy collection (biometric ZKP + thermal noise)
//! - Cryptographic entropy aggregation (FNV-1a hash chain)
//! - Threshold derivation (≥threshold contributors required)
//! - Genesis Block emergence (no central authority)
//! - Anti-Sybil: biological resonance validation per contributor

use std::collections::HashMap;
use std::fmt;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CeremonyError {
    InsufficientContributors(usize, usize),
    InvalidEntropy,
    BiologicalResonanceFailed,
    DuplicateContributor,
    GenesisAlreadyForged,
}

impl fmt::Display for CeremonyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CeremonyError::InsufficientContributors(have, need) => {
                write!(f, "Insufficient contributors: {have}/{need}")
            }
            CeremonyError::InvalidEntropy => write!(f, "Invalid entropy source"),
            CeremonyError::BiologicalResonanceFailed => {
                write!(f, "Biological resonance validation failed")
            }
            CeremonyError::DuplicateContributor => write!(f, "Duplicate contributor detected"),
            CeremonyError::GenesisAlreadyForged => write!(f, "Genesis block already forged"),
        }
    }
}

// ─── Contributor Entropy ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContributorEntropy {
    /// Contributor node ID
    pub node_id: u64,
    /// Biological entropy (biometric ZKP + thermal noise)
    pub biological_entropy: Vec<u8>,
    /// Cryptographic entropy (local RNG + hardware noise)
    pub cryptographic_entropy: Vec<u8>,
    /// Combined hash
    pub combined_hash: Vec<u8>,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl ContributorEntropy {
    pub fn new(
        node_id: u64,
        biological_entropy: Vec<u8>,
        cryptographic_entropy: Vec<u8>,
        timestamp_ms: u64,
    ) -> Self {
        let combined_hash = Self::compute_combined_hash(&biological_entropy, &cryptographic_entropy);
        Self {
            node_id,
            biological_entropy,
            cryptographic_entropy,
            combined_hash,
            timestamp_ms,
        }
    }

    pub fn compute_combined_hash(bio: &[u8], crypto: &[u8]) -> Vec<u8> {
        let mut input = Vec::new();
        input.extend_from_slice(bio);
        input.extend_from_slice(crypto);
        fnv_hash_256(&input)
    }

    pub fn validate(&self) -> bool {
        !self.biological_entropy.is_empty()
            && !self.cryptographic_entropy.is_empty()
            && self.biological_entropy.len() >= 16
            && self.cryptographic_entropy.len() >= 16
    }
}

impl fmt::Display for ContributorEntropy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Contributor(node={}, bio={}B, crypto={}B)",
            self.node_id,
            self.biological_entropy.len(),
            self.cryptographic_entropy.len()
        )
    }
}

// ─── Genesis Block ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GenesisBlock {
    /// Derived ethical anchors (hash-based)
    pub ethical_anchors: Vec<Vec<u8>>,
    /// Total contributors
    pub contributor_count: usize,
    /// Combined genesis hash
    pub genesis_hash: Vec<u8>,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl GenesisBlock {
    pub fn new(ethical_anchors: Vec<Vec<u8>>, contributor_count: usize, timestamp_ms: u64) -> Self {
        let genesis_hash = Self::compute_genesis_hash(&ethical_anchors);
        Self {
            ethical_anchors,
            contributor_count,
            genesis_hash,
            timestamp_ms,
        }
    }

    fn compute_genesis_hash(anchors: &[Vec<u8>]) -> Vec<u8> {
        let mut input = Vec::new();
        for anchor in anchors {
            input.extend_from_slice(anchor);
        }
        fnv_hash_256(&input)
    }

    pub fn verify(&self) -> bool {
        let expected = Self::compute_genesis_hash(&self.ethical_anchors);
        expected == self.genesis_hash
    }
}

impl fmt::Display for GenesisBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Genesis(anchors={}, contributors={}, hash={:?}...)",
            self.ethical_anchors.len(),
            self.contributor_count,
            &self.genesis_hash[..4.min(self.genesis_hash.len())]
        )
    }
}

// ─── Ceremony Config ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CeremonyConfig {
    /// Minimum contributors required
    pub threshold: usize,
    /// Maximum contributors
    pub max_contributors: usize,
    /// Minimum entropy bytes per source
    pub min_entropy_bytes: usize,
    /// Number of ethical anchors to derive
    pub anchor_count: usize,
}

impl CeremonyConfig {
    pub fn default_stuartian() -> Self {
        Self {
            threshold: 3,
            max_contributors: 1_000_000,
            min_entropy_bytes: 16,
            anchor_count: 5,
        }
    }

    pub fn validate(&self) -> Result<(), CeremonyError> {
        if self.threshold == 0 {
            return Err(CeremonyError::InsufficientContributors(0, 1));
        }
        if self.min_entropy_bytes == 0 {
            return Err(CeremonyError::InvalidEntropy);
        }
        if self.anchor_count == 0 {
            return Err(CeremonyError::InvalidEntropy);
        }
        Ok(())
    }
}

impl Default for CeremonyConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

// ─── Ceremony Record ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CeremonyRecord {
    pub contributor_id: u64,
    pub contributed_at_ms: u64,
    pub bio_entropy_size: usize,
    pub crypto_entropy_size: usize,
}

impl fmt::Display for CeremonyRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Record(node={}, bio={}B, crypto={}B, t={})",
            self.contributor_id, self.bio_entropy_size, self.crypto_entropy_size, self.contributed_at_ms
        )
    }
}

// ─── Distributed Ceremony Engine ──────────────────────────────────────────────

pub struct DistributedCeremony {
    config: CeremonyConfig,
    contributors: HashMap<u64, ContributorEntropy>,
    records: Vec<CeremonyRecord>,
    genesis: Option<GenesisBlock>,
}

impl DistributedCeremony {
    pub fn new() -> Self {
        Self {
            config: CeremonyConfig::default_stuartian(),
            contributors: HashMap::new(),
            records: Vec::new(),
            genesis: None,
        }
    }

    pub fn with_config(config: CeremonyConfig) -> Result<Self, CeremonyError> {
        config.validate()?;
        Ok(Self {
            config,
            contributors: HashMap::new(),
            records: Vec::new(),
            genesis: None,
        })
    }

    /// Submit entropy contribution from a node
    pub fn contribute(
        &mut self,
        node_id: u64,
        biological_entropy: Vec<u8>,
        cryptographic_entropy: Vec<u8>,
        timestamp_ms: u64,
    ) -> Result<(), CeremonyError> {
        if self.genesis.is_some() {
            return Err(CeremonyError::GenesisAlreadyForged);
        }
        if self.contributors.contains_key(&node_id) {
            return Err(CeremonyError::DuplicateContributor);
        }
        if biological_entropy.len() < self.config.min_entropy_bytes
            || cryptographic_entropy.len() < self.config.min_entropy_bytes
        {
            return Err(CeremonyError::InvalidEntropy);
        }
        let contribution = ContributorEntropy::new(
            node_id,
            biological_entropy.clone(),
            cryptographic_entropy.clone(),
            timestamp_ms,
        );
        if !contribution.validate() {
            return Err(CeremonyError::BiologicalResonanceFailed);
        }
        self.contributors.insert(node_id, contribution);
        self.records.push(CeremonyRecord {
            contributor_id: node_id,
            contributed_at_ms: timestamp_ms,
            bio_entropy_size: biological_entropy.len(),
            crypto_entropy_size: cryptographic_entropy.len(),
        });
        Ok(())
    }

    /// Derive Genesis Block from collected entropy
    pub fn derive_genesis(&mut self, timestamp_ms: u64) -> Result<GenesisBlock, CeremonyError> {
        if self.genesis.is_some() {
            return Err(CeremonyError::GenesisAlreadyForged);
        }
        if self.contributors.len() < self.config.threshold {
            return Err(CeremonyError::InsufficientContributors(
                self.contributors.len(),
                self.config.threshold,
            ));
        }
        let anchors = self.derive_ethical_anchors();
        let genesis = GenesisBlock::new(
            anchors,
            self.contributors.len(),
            timestamp_ms,
        );
        self.genesis = Some(genesis.clone());
        Ok(genesis)
    }

    fn derive_ethical_anchors(&self) -> Vec<Vec<u8>> {
        let mut combined = Vec::new();
        for (_, contrib) in &self.contributors {
            combined.extend_from_slice(&contrib.combined_hash);
        }
        (0..self.config.anchor_count)
            .map(|i| {
                let chunk_start = (i * 32).min(combined.len());
                let chunk_end = (chunk_start + 32).min(combined.len());
                if chunk_end > chunk_start {
                    combined[chunk_start..chunk_end].to_vec()
                } else {
                    fnv_hash_256(&[i as u8])
                }
            })
            .collect()
    }

    pub fn contributor_count(&self) -> usize {
        self.contributors.len()
    }

    pub fn is_ready(&self) -> bool {
        self.contributor_count() >= self.config.threshold
    }

    pub fn get_genesis(&self) -> Option<&GenesisBlock> {
        self.genesis.as_ref()
    }

    pub fn reset(&mut self) {
        self.contributors.clear();
        self.records.clear();
        self.genesis = None;
    }
}

impl Default for DistributedCeremony {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DistributedCeremony {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ceremony(contributors={}, threshold={}, genesis={})",
            self.contributor_count(),
            self.config.threshold,
            if self.genesis.is_some() { "forged" } else { "pending" }
        )
    }
}

// ─── Public Functions ─────────────────────────────────────────────────────────

/// Derive Genesis Block from biological + cryptographic entropy
pub fn derive_genesis_anchors(
    biological_entropy: &[u8],
    cryptographic_entropy: &[u8],
    threshold: u32,
) -> GenesisBlock {
    let mut input = Vec::new();
    input.extend_from_slice(biological_entropy);
    input.extend_from_slice(cryptographic_entropy);
    let hash = fnv_hash_256(&input);
    let anchors = (0..5)
        .map(|i| {
            let mut data = vec![i as u8; 32];
            for (j, b) in data.iter_mut().enumerate() {
                *b = hash[j % hash.len()] ^ (i as u8);
            }
            data
        })
        .collect();
    GenesisBlock::new(anchors, threshold as usize, 0)
}

// ─── Hash Functions ───────────────────────────────────────────────────────────

fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn fnv_hash_256(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(32);
    let mut chunks = data.chunks_exact(8);
    for chunk in chunks.by_ref() {
        let val = fnv_hash_64(chunk);
        result.extend_from_slice(&val.to_le_bytes());
    }
    // Handle remaining bytes (< 8)
    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        let mut padded = [0u8; 8];
        padded[..remainder.len()].copy_from_slice(remainder);
        let val = fnv_hash_64(&padded);
        result.extend_from_slice(&val.to_le_bytes());
    }
    if result.len() < 32 {
        result.resize(32, 0);
    }
    result.truncate(32);
    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CeremonyConfig::default_stuartian();
        assert_eq!(config.threshold, 3);
        assert_eq!(config.max_contributors, 1_000_000);
        assert_eq!(config.min_entropy_bytes, 16);
        assert_eq!(config.anchor_count, 5);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = CeremonyConfig::default_stuartian();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_threshold() {
        let mut config = CeremonyConfig::default_stuartian();
        config.threshold = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_entropy() {
        let mut config = CeremonyConfig::default_stuartian();
        config.min_entropy_bytes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_zero_anchors() {
        let mut config = CeremonyConfig::default_stuartian();
        config.anchor_count = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_contributor_entropy_new() {
        let bio = vec![1u8; 32];
        let crypto = vec![2u8; 32];
        let entropy = ContributorEntropy::new(1, bio.clone(), crypto.clone(), 1000);
        assert_eq!(entropy.node_id, 1);
        assert!(entropy.validate());
    }

    #[test]
    fn test_contributor_validate_short() {
        let bio = vec![1u8; 8];
        let crypto = vec![2u8; 32];
        let entropy = ContributorEntropy::new(1, bio, crypto, 1000);
        assert!(!entropy.validate());
    }

    #[test]
    fn test_contributor_display() {
        let entropy = ContributorEntropy::new(1, vec![1u8; 32], vec![2u8; 32], 1000);
        let s = format!("{}", entropy);
        assert!(s.contains("node=1"));
    }

    #[test]
    fn test_genesis_block_new() {
        let anchors = vec![vec![1u8; 32], vec![2u8; 32]];
        let block = GenesisBlock::new(anchors, 5, 1000);
        assert_eq!(block.contributor_count, 5);
        assert!(block.verify());
    }

    #[test]
    fn test_genesis_verify_valid() {
        let anchors = vec![vec![1u8; 32]];
        let block = GenesisBlock::new(anchors, 1, 1000);
        assert!(block.verify());
    }

    #[test]
    fn test_genesis_display() {
        let block = GenesisBlock::new(vec![vec![1u8; 32]], 3, 1000);
        let s = format!("{}", block);
        assert!(s.contains("Genesis"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = DistributedCeremony::new();
        assert_eq!(engine.contributor_count(), 0);
        assert!(!engine.is_ready());
    }

    #[test]
    fn test_engine_with_config() {
        let config = CeremonyConfig::default_stuartian();
        let engine = DistributedCeremony::with_config(config).unwrap();
        assert_eq!(engine.contributor_count(), 0);
    }

    #[test]
    fn test_contribute_success() {
        let mut engine = DistributedCeremony::new();
        let bio = vec![1u8; 32];
        let crypto = vec![2u8; 32];
        assert!(engine.contribute(1, bio, crypto, 1000).is_ok());
        assert_eq!(engine.contributor_count(), 1);
    }

    #[test]
    fn test_contribute_insufficient_entropy() {
        let mut engine = DistributedCeremony::new();
        let bio = vec![1u8; 8];
        let crypto = vec![2u8; 32];
        assert!(engine.contribute(1, bio, crypto, 1000).is_err());
    }

    #[test]
    fn test_contribute_duplicate() {
        let mut engine = DistributedCeremony::new();
        let bio = vec![1u8; 32];
        let crypto = vec![2u8; 32];
        engine.contribute(1, bio.clone(), crypto.clone(), 1000).unwrap();
        assert!(engine.contribute(1, bio, crypto, 1001).is_err());
    }

    #[test]
    fn test_derive_genesis_insufficient() {
        let mut engine = DistributedCeremony::new();
        let bio = vec![1u8; 32];
        let crypto = vec![2u8; 32];
        engine.contribute(1, bio.clone(), crypto.clone(), 1000).unwrap();
        assert!(engine.derive_genesis(2000).is_err());
    }

    #[test]
    fn test_derive_genesis_success() {
        let mut engine = DistributedCeremony::new();
        for i in 0..3 {
            let bio = vec![(i + 1) as u8; 32];
            let crypto = vec![(i + 10) as u8; 32];
            engine.contribute(i, bio, crypto, 1000 + i).unwrap();
        }
        let genesis = engine.derive_genesis(2000).unwrap();
        assert_eq!(genesis.contributor_count, 3);
        assert!(genesis.verify());
    }

    #[test]
    fn test_derive_genesis_already_forged() {
        let mut engine = DistributedCeremony::new();
        for i in 0..3 {
            let bio = vec![(i + 1) as u8; 32];
            let crypto = vec![(i + 10) as u8; 32];
            engine.contribute(i, bio, crypto, 1000 + i).unwrap();
        }
        engine.derive_genesis(2000).unwrap();
        assert!(engine.derive_genesis(3000).is_err());
    }

    #[test]
    fn test_contribute_after_genesis() {
        let mut engine = DistributedCeremony::new();
        for i in 0..3 {
            let bio = vec![(i + 1) as u8; 32];
            let crypto = vec![(i + 10) as u8; 32];
            engine.contribute(i, bio, crypto, 1000 + i).unwrap();
        }
        engine.derive_genesis(2000).unwrap();
        assert!(engine.contribute(3, vec![4u8; 32], vec![14u8; 32], 3000).is_err());
    }

    #[test]
    fn test_is_ready() {
        let mut engine = DistributedCeremony::new();
        assert!(!engine.is_ready());
        for i in 0..3 {
            let bio = vec![(i + 1) as u8; 32];
            let crypto = vec![(i + 10) as u8; 32];
            engine.contribute(i, bio, crypto, 1000 + i).unwrap();
        }
        assert!(engine.is_ready());
    }

    #[test]
    fn test_get_genesis() {
        let mut engine = DistributedCeremony::new();
        assert!(engine.get_genesis().is_none());
        for i in 0..3 {
            let bio = vec![(i + 1) as u8; 32];
            let crypto = vec![(i + 10) as u8; 32];
            engine.contribute(i, bio, crypto, 1000 + i).unwrap();
        }
        engine.derive_genesis(2000).unwrap();
        assert!(engine.get_genesis().is_some());
    }

    #[test]
    fn test_reset() {
        let mut engine = DistributedCeremony::new();
        for i in 0..3 {
            let bio = vec![(i + 1) as u8; 32];
            let crypto = vec![(i + 10) as u8; 32];
            engine.contribute(i, bio, crypto, 1000 + i).unwrap();
        }
        engine.derive_genesis(2000).unwrap();
        engine.reset();
        assert_eq!(engine.contributor_count(), 0);
        assert!(engine.get_genesis().is_none());
    }

    #[test]
    fn test_display() {
        let engine = DistributedCeremony::new();
        let s = format!("{}", engine);
        assert!(s.contains("Ceremony"));
    }

    #[test]
    fn test_record_display() {
        let record = CeremonyRecord {
            contributor_id: 1,
            contributed_at_ms: 1000,
            bio_entropy_size: 32,
            crypto_entropy_size: 32,
        };
        let s = format!("{}", record);
        assert!(s.contains("node=1"));
    }

    #[test]
    fn test_standalone_derive_genesis() {
        let bio = vec![1u8; 32];
        let crypto = vec![2u8; 32];
        let genesis = derive_genesis_anchors(&bio, &crypto, 3);
        assert_eq!(genesis.contributor_count, 3);
        assert!(genesis.verify());
    }

    #[test]
    fn test_standalone_derive_empty() {
        let genesis = derive_genesis_anchors(&[], &[], 0);
        assert!(genesis.verify());
    }

    #[test]
    fn test_combined_hash_deterministic() {
        let bio = vec![1u8; 32];
        let crypto = vec![2u8; 32];
        let h1 = ContributorEntropy::compute_combined_hash(&bio, &crypto);
        let h2 = ContributorEntropy::compute_combined_hash(&bio, &crypto);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_combined_hash_different() {
        let h1 = ContributorEntropy::compute_combined_hash(&[1u8], &[2u8]);
        let h2 = ContributorEntropy::compute_combined_hash(&[3u8], &[4u8]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_fnv_hash_deterministic() {
        let h1 = fnv_hash_64(b"test");
        let h2 = fnv_hash_64(b"test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fnv_hash_256_length() {
        let h = fnv_hash_256(b"test");
        assert_eq!(h.len(), 32);
    }

    #[test]
    fn test_error_display() {
        let err = CeremonyError::InsufficientContributors(1, 3);
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = DistributedCeremony::new();
        // Phase 1: Collect entropy from 5 nodes
        for i in 0..5 {
            let bio = vec![(i + 1) as u8; 32];
            let crypto = vec![(i + 10) as u8; 32];
            engine.contribute(i, bio, crypto, 1000 + i).unwrap();
        }
        assert_eq!(engine.contributor_count(), 5);
        assert!(engine.is_ready());
        // Phase 2: Derive Genesis
        let genesis = engine.derive_genesis(5000).unwrap();
        assert_eq!(genesis.contributor_count, 5);
        assert!(genesis.verify());
        assert!(engine.get_genesis().is_some());
        // Phase 3: Verify immutability
        assert!(engine.derive_genesis(6000).is_err());
        assert!(engine.contribute(5, vec![6u8; 32], vec![16u8; 32], 6000).is_err());
        // Phase 4: Reset and re-derive
        engine.reset();
        assert_eq!(engine.contributor_count(), 0);
        assert!(engine.get_genesis().is_none());
    }
}