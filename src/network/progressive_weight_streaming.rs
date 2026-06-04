//! Progressive Weight Streaming â€” Sprint 78: Invariant Architecture & Planetary-Scale Resilience
//!
//! Progressive weight streaming for cold start optimization. Micro-SAE (1MB) boots in <500ms,
//! then full SAE weights are streamed via WebRTC P2P (BitTorrent-style). Value from t=0.

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by the progressive weight streaming system.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingError {
    /// Chunk size is zero or exceeds maximum.
    InvalidChunkSize(usize),
    /// Peer count below minimum required for streaming.
    InsufficientPeers(usize),
    /// Chunk not found in any peer.
    ChunkNotFound(u64),
    /// Transfer rate below minimum threshold.
    RateTooLow(f64),
    /// Micro-SAE load exceeded timeout.
    TimeoutExceeded(u64),
    /// Dimension mismatch between micro and full model.
    DimensionMismatch { micro: usize, full: usize },
}

impl fmt::Display for StreamingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamingError::InvalidChunkSize(s) => {
                write!(f, "invalid chunk size: {s}")
            }
            StreamingError::InsufficientPeers(n) => {
                write!(f, "insufficient peers: {n} (minimum 1 required)")
            }
            StreamingError::ChunkNotFound(id) => {
                write!(f, "chunk {id} not found in any peer")
            }
            StreamingError::RateTooLow(rate) => {
                write!(f, "transfer rate {rate:.2} KB/s below minimum threshold")
            }
            StreamingError::TimeoutExceeded(ms) => {
                write!(f, "timeout {ms}ms exceeded during micro-SAE load")
            }
            StreamingError::DimensionMismatch { micro, full } => {
                write!(f, "dimension mismatch: micro={micro}, full={full}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for progressive weight streaming.
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum chunk size in bytes for P2P transfer.
    pub max_chunk_size: usize,
    /// Minimum transfer rate in KB/s.
    pub min_transfer_rate_kb: f64,
    /// Maximum timeout for micro-SAE load in milliseconds.
    pub micro_load_timeout_ms: u64,
    /// Target proof size for streaming metadata in bytes.
    pub metadata_size_bytes: usize,
    /// Minimum peer count for streaming.
    pub min_peer_count: usize,
}

impl StreamingConfig {
    /// Default Topological configuration.
    pub fn default_Topological() -> Self {
        Self {
            max_chunk_size: 64 * 1024, // 64 KB chunks
            min_transfer_rate_kb: 10.0,
            micro_load_timeout_ms: 500,
            metadata_size_bytes: 1024,
            min_peer_count: 1,
        }
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<(), StreamingError> {
        if self.max_chunk_size == 0 {
            return Err(StreamingError::InvalidChunkSize(self.max_chunk_size));
        }
        if self.min_transfer_rate_kb <= 0.0 {
            return Err(StreamingError::RateTooLow(self.min_transfer_rate_kb));
        }
        if self.micro_load_timeout_ms == 0 {
            return Err(StreamingError::TimeoutExceeded(self.micro_load_timeout_ms));
        }
        Ok(())
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

// ---------------------------------------------------------------------------
// Chunk representation
// ---------------------------------------------------------------------------

/// A weight chunk for P2P streaming.
#[derive(Debug, Clone)]
pub struct WeightChunk {
    /// Unique chunk identifier.
    pub chunk_id: u64,
    /// Chunk data (simulated weights).
    pub data: Vec<u8>,
    /// Chunk hash for integrity verification.
    pub hash: u64,
    /// Peer IDs that have this chunk.
    pub available_peers: Vec<u64>,
}

impl WeightChunk {
    /// Create a new weight chunk.
    pub fn new(chunk_id: u64, data: Vec<u8>) -> Self {
        let hash = fnv_hash_bytes(&data);
        Self {
            chunk_id,
            data,
            hash,
            available_peers: Vec::new(),
        }
    }

    /// Size of this chunk in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Add a peer that has this chunk.
    pub fn add_peer(&mut self, peer_id: u64) {
        if !self.available_peers.contains(&peer_id) {
            self.available_peers.push(peer_id);
        }
    }
}

impl fmt::Display for WeightChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WeightChunk(id={}, size={}B, peers={}, hash={:x})",
            self.chunk_id,
            self.size(),
            self.available_peers.len(),
            self.hash
        )
    }
}

// ---------------------------------------------------------------------------
// Peer representation
// ---------------------------------------------------------------------------

/// A peer in the P2P streaming mesh.
#[derive(Debug, Clone)]
pub struct StreamingPeer {
    /// Unique peer identifier.
    pub peer_id: u64,
    /// Peer address (simulated).
    pub address: String,
    /// Available bandwidth in KB/s.
    pub bandwidth_kb: f64,
    /// Chunks available from this peer.
    pub available_chunks: Vec<u64>,
    /// Active flag.
    pub active: bool,
}

impl StreamingPeer {
    /// Create a new streaming peer.
    pub fn new(peer_id: u64, address: String, bandwidth_kb: f64) -> Self {
        Self {
            peer_id,
            address,
            bandwidth_kb,
            available_chunks: Vec::new(),
            active: true,
        }
    }

    /// Add an available chunk to this peer.
    pub fn add_chunk(&mut self, chunk_id: u64) {
        if !self.available_chunks.contains(&chunk_id) {
            self.available_chunks.push(chunk_id);
        }
    }
}

impl fmt::Display for StreamingPeer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StreamingPeer(id={}, addr=\"{}\", bw={:.1}KB/s, chunks={}, active={})",
            self.peer_id,
            self.address,
            self.bandwidth_kb,
            self.available_chunks.len(),
            self.active
        )
    }
}

// ---------------------------------------------------------------------------
// Transfer record
// ---------------------------------------------------------------------------

/// Record of a weight transfer.
#[derive(Debug, Clone)]
pub struct TransferRecord {
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Chunk ID transferred.
    pub chunk_id: u64,
    /// Source peer ID.
    pub source_peer: u64,
    /// Transfer size in bytes.
    pub size_bytes: usize,
    /// Transfer duration in milliseconds.
    pub duration_ms: u64,
    /// Transfer rate in KB/s.
    pub rate_kb: f64,
    /// Success flag.
    pub success: bool,
}

impl fmt::Display for TransferRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TransferRecord(chunk={}, peer={}, size={}B, rate={:.2}KB/s, success={})",
            self.chunk_id, self.source_peer, self.size_bytes, self.rate_kb, self.success
        )
    }
}

// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct ProgressiveWeightStreaming {
    config: StreamingConfig,
    peers: HashMap<u64, StreamingPeer>,
    chunks: HashMap<u64, WeightChunk>,
    downloaded_chunks: HashMap<u64, WeightChunk>,
    records: Vec<TransferRecord>,
    micro_loaded: bool,
    micro_load_time_ms: u64,
}

impl ProgressiveWeightStreaming {
    /// Create a new streaming engine with default config.
    pub fn new() -> Self {
        Self {
            config: StreamingConfig::default_Topological(),
            peers: HashMap::new(),
            chunks: HashMap::new(),
            downloaded_chunks: HashMap::new(),
            records: Vec::new(),
            micro_loaded: false,
            micro_load_time_ms: 0,
        }
    }

    /// Create with explicit configuration.
    pub fn with_config(config: StreamingConfig) -> Result<Self, StreamingError> {
        config.validate()?;
        Ok(Self {
            config,
            peers: HashMap::new(),
            chunks: HashMap::new(),
            downloaded_chunks: HashMap::new(),
            records: Vec::new(),
            micro_loaded: false,
            micro_load_time_ms: 0,
        })
    }

    /// Register a peer in the streaming mesh.
    pub fn register_peer(&mut self, peer: StreamingPeer) -> Result<(), StreamingError> {
        self.peers.insert(peer.peer_id, peer);
        Ok(())
    }

    /// Remove a peer from the mesh.
    pub fn remove_peer(&mut self, peer_id: u64) -> bool {
        self.peers.remove(&peer_id).is_some()
    }

    /// Register a chunk available in the mesh.
    pub fn register_chunk(&mut self, chunk: WeightChunk) -> Result<(), StreamingError> {
        if chunk.size() > self.config.max_chunk_size {
            return Err(StreamingError::InvalidChunkSize(chunk.size()));
        }
        self.chunks.insert(chunk.chunk_id, chunk);
        Ok(())
    }

    /// Simulate micro-SAE load.
    pub fn load_micro_sae(&mut self, simulated_time_ms: u64) -> Result<(), StreamingError> {
        if simulated_time_ms > self.config.micro_load_timeout_ms {
            return Err(StreamingError::TimeoutExceeded(simulated_time_ms));
        }
        self.micro_loaded = true;
        self.micro_load_time_ms = simulated_time_ms;
        Ok(())
    }

    /// Find the best peer for a given chunk (highest bandwidth).
    pub fn find_best_peer_for_chunk(&self, chunk_id: u64) -> Option<u64> {
        let chunk = self.chunks.get(&chunk_id)?;
        let mut best_peer: Option<u64> = None;
        let mut best_bandwidth = 0.0;

        for peer_id in &chunk.available_peers {
            if let Some(peer) = self.peers.get(peer_id) {
                if peer.active && peer.bandwidth_kb > best_bandwidth {
                    best_bandwidth = peer.bandwidth_kb;
                    best_peer = Some(*peer_id);
                }
            }
        }
        best_peer
    }

    /// Simulate downloading a chunk from the best available peer.
    pub fn download_chunk(
        &mut self,
        chunk_id: u64,
        timestamp_ms: u64,
    ) -> Result<TransferRecord, StreamingError> {
        // Validate peer count
        let active_peers: Vec<&StreamingPeer> = self.peers.values().filter(|p| p.active).collect();
        if active_peers.len() < self.config.min_peer_count {
            return Err(StreamingError::InsufficientPeers(active_peers.len()));
        }

        // Find chunk
        let chunk = self
            .chunks
            .get(&chunk_id)
            .ok_or(StreamingError::ChunkNotFound(chunk_id))?;

        // Find best peer
        let source_peer = self
            .find_best_peer_for_chunk(chunk_id)
            .ok_or(StreamingError::ChunkNotFound(chunk_id))?;

        // Simulate transfer
        let peer_bandwidth = self
            .peers
            .get(&source_peer)
            .map(|p| p.bandwidth_kb)
            .unwrap_or(1.0);

        let size_kb = chunk.size() as f64 / 1024.0;
        let duration_ms = if peer_bandwidth > 0.0 {
            ((size_kb / peer_bandwidth) * 1000.0) as u64
        } else {
            self.config.micro_load_timeout_ms
        };

        let rate_kb = if duration_ms > 0 {
            size_kb / (duration_ms as f64 / 1000.0)
        } else {
            0.0
        };

        // Check rate threshold
        if rate_kb < self.config.min_transfer_rate_kb && duration_ms > 0 {
            let record = TransferRecord {
                timestamp_ms,
                chunk_id,
                source_peer,
                size_bytes: chunk.size(),
                duration_ms,
                rate_kb,
                success: false,
            };
            self.records.push(record.clone());
            return Err(StreamingError::RateTooLow(rate_kb));
        }

        // Success â€” mark as downloaded
        let downloaded_chunk = chunk.clone();
        self.downloaded_chunks.insert(chunk_id, downloaded_chunk);

        let record = TransferRecord {
            timestamp_ms,
            chunk_id,
            source_peer,
            size_bytes: chunk.size(),
            duration_ms,
            rate_kb,
            success: true,
        };
        self.records.push(record.clone());
        Ok(record)
    }

    /// Compute overall progress as fraction of total chunks downloaded.
    pub fn progress(&self) -> f64 {
        if self.chunks.is_empty() {
            return 0.0;
        }
        self.downloaded_chunks.len() as f64 / self.chunks.len() as f64
    }

    /// Total downloaded bytes.
    pub fn total_downloaded_bytes(&self) -> usize {
        self.downloaded_chunks.values().map(|c| c.size()).sum()
    }

    /// Average transfer rate across all successful transfers.
    pub fn average_rate_kb(&self) -> Option<f64> {
        let successful: Vec<&TransferRecord> = self.records.iter().filter(|r| r.success).collect();
        if successful.is_empty() {
            return None;
        }
        let total_rate: f64 = successful.iter().map(|r| r.rate_kb).sum();
        Some(total_rate / successful.len() as f64)
    }

    /// Check if micro-SAE is loaded.
    pub fn is_micro_loaded(&self) -> bool {
        self.micro_loaded
    }

    /// Get micro-SAE load time.
    pub fn micro_load_time(&self) -> u64 {
        self.micro_load_time_ms
    }

    /// Get active peer count.
    pub fn active_peer_count(&self) -> usize {
        self.peers.values().filter(|p| p.active).count()
    }

    /// Get downloaded chunk count.
    pub fn downloaded_count(&self) -> usize {
        self.downloaded_chunks.len()
    }

    /// Get total chunk count.
    pub fn total_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get transfer records.
    pub fn records(&self) -> &[TransferRecord] {
        &self.records
    }

    /// Reset state (preserves config, clears transfers).
    pub fn reset(&mut self) {
        self.downloaded_chunks.clear();
        self.records.clear();
        self.micro_loaded = false;
        self.micro_load_time_ms = 0;
    }
}

impl Default for ProgressiveWeightStreaming {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProgressiveWeightStreaming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProgressiveWeightStreaming(peers={}, chunks={}/{}, micro_loaded={}, progress={:.0}%)",
            self.active_peer_count(),
            self.downloaded_count(),
            self.total_chunk_count(),
            self.micro_loaded,
            self.progress() * 100.0
        )
    }
}

// ---------------------------------------------------------------------------
// Standalone functions
// ---------------------------------------------------------------------------

/// FNV-1a hash for byte slices.
pub fn fnv_hash_bytes(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Compute estimated transfer time in milliseconds.
pub fn estimate_transfer_time(size_bytes: usize, rate_kb: f64) -> u64 {
    if rate_kb <= 0.0 {
        return u64::MAX;
    }
    let size_kb = size_bytes as f64 / 1024.0;
    (size_kb / rate_kb * 1000.0) as u64
}

/// Compute optimal chunk size for a given bandwidth and latency.
pub fn optimal_chunk_size(bandwidth_kb: f64, latency_ms: u64) -> usize {
    // Target: chunk transfer time â‰ˆ 2 * latency (pipelining optimization)
    let target_time_ms = latency_ms.saturating_mul(2);
    let target_size_kb = bandwidth_kb * (target_time_ms as f64 / 1000.0);
    let size = target_size_kb as usize * 1024;
    // Clamp to reasonable bounds
    size.max(1024).min(1024 * 1024) // 1 KB to 1 MB
}

/// Check if a chunk is available from any active peer.
pub fn is_chunk_available(
    chunk_id: u64,
    chunks: &HashMap<u64, WeightChunk>,
    peers: &HashMap<u64, StreamingPeer>,
) -> bool {
    let chunk = match chunks.get(&chunk_id) {
        Some(c) => c,
        None => return false,
    };
    chunk
        .available_peers
        .iter()
        .any(|peer_id| peers.get(peer_id).map(|p| p.active).unwrap_or(false))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config tests ---

    #[test]
    fn test_config_default() {
        let config = StreamingConfig::default_Topological();
        assert!(config.max_chunk_size > 0);
        assert!(config.min_transfer_rate_kb > 0.0);
        assert!(config.micro_load_timeout_ms > 0);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = StreamingConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_chunk_size() {
        let config = StreamingConfig {
            max_chunk_size: 0,
            ..StreamingConfig::default_Topological()
        };
        assert!(matches!(
            config.validate(),
            Err(StreamingError::InvalidChunkSize(0))
        ));
    }

    #[test]
    fn test_config_zero_rate() {
        let config = StreamingConfig {
            min_transfer_rate_kb: 0.0,
            ..StreamingConfig::default_Topological()
        };
        assert!(matches!(
            config.validate(),
            Err(StreamingError::RateTooLow(0.0))
        ));
    }

    #[test]
    fn test_config_zero_timeout() {
        let config = StreamingConfig {
            micro_load_timeout_ms: 0,
            ..StreamingConfig::default_Topological()
        };
        assert!(matches!(
            config.validate(),
            Err(StreamingError::TimeoutExceeded(0))
        ));
    }

    // --- Chunk tests ---

    #[test]
    fn test_chunk_creation() {
        let data = vec![1, 2, 3, 4];
        let chunk = WeightChunk::new(1, data);
        assert_eq!(chunk.chunk_id, 1);
        assert_eq!(chunk.size(), 4);
    }

    #[test]
    fn test_chunk_add_peer() {
        let mut chunk = WeightChunk::new(1, vec![1, 2, 3]);
        chunk.add_peer(100);
        chunk.add_peer(200);
        assert_eq!(chunk.available_peers.len(), 2);
    }

    #[test]
    fn test_chunk_add_duplicate_peer() {
        let mut chunk = WeightChunk::new(1, vec![1, 2, 3]);
        chunk.add_peer(100);
        chunk.add_peer(100);
        assert_eq!(chunk.available_peers.len(), 1);
    }

    #[test]
    fn test_chunk_display() {
        let chunk = WeightChunk::new(42, vec![0; 100]);
        let s = format!("{chunk}");
        assert!(s.contains("id=42"));
    }

    // --- Peer tests ---

    #[test]
    fn test_peer_creation() {
        let peer = StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0);
        assert_eq!(peer.peer_id, 1);
        assert!(peer.active);
    }

    #[test]
    fn test_peer_add_chunk() {
        let mut peer = StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0);
        peer.add_chunk(10);
        peer.add_chunk(20);
        assert_eq!(peer.available_chunks.len(), 2);
    }

    #[test]
    fn test_peer_display() {
        let peer = StreamingPeer::new(1, "addr".to_string(), 50.0);
        let s = format!("{peer}");
        assert!(s.contains("id=1"));
    }

    // --- Engine tests ---

    #[test]
    fn test_engine_creation() {
        let engine = ProgressiveWeightStreaming::new();
        assert_eq!(engine.active_peer_count(), 0);
        assert!(!engine.is_micro_loaded());
    }

    #[test]
    fn test_engine_with_config() {
        let config = StreamingConfig::default_Topological();
        let engine = ProgressiveWeightStreaming::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_register_peer() {
        let mut engine = ProgressiveWeightStreaming::new();
        let peer = StreamingPeer::new(1, "addr".to_string(), 100.0);
        assert!(engine.register_peer(peer).is_ok());
        assert_eq!(engine.active_peer_count(), 1);
    }

    #[test]
    fn test_remove_peer() {
        let mut engine = ProgressiveWeightStreaming::new();
        engine
            .register_peer(StreamingPeer::new(1, "addr".to_string(), 100.0))
            .unwrap();
        assert!(engine.remove_peer(1));
        assert_eq!(engine.active_peer_count(), 0);
    }

    #[test]
    fn test_register_chunk() {
        let mut engine = ProgressiveWeightStreaming::new();
        let chunk = WeightChunk::new(1, vec![0; 100]);
        assert!(engine.register_chunk(chunk).is_ok());
        assert_eq!(engine.total_chunk_count(), 1);
    }

    #[test]
    fn test_register_chunk_too_large() {
        let mut engine = ProgressiveWeightStreaming::new();
        let large_data = vec![0; engine.config.max_chunk_size + 1];
        let chunk = WeightChunk::new(1, large_data);
        assert!(matches!(
            engine.register_chunk(chunk),
            Err(StreamingError::InvalidChunkSize(_))
        ));
    }

    #[test]
    fn test_load_micro_sae_success() {
        let mut engine = ProgressiveWeightStreaming::new();
        assert!(engine.load_micro_sae(200).is_ok());
        assert!(engine.is_micro_loaded());
        assert_eq!(engine.micro_load_time(), 200);
    }

    #[test]
    fn test_load_micro_sae_timeout() {
        let mut engine = ProgressiveWeightStreaming::new();
        assert!(matches!(
            engine.load_micro_sae(600),
            Err(StreamingError::TimeoutExceeded(600))
        ));
        assert!(!engine.is_micro_loaded());
    }

    #[test]
    fn test_download_chunk_success() {
        let mut engine = ProgressiveWeightStreaming::new();
        // Register peer
        engine
            .register_peer(StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0))
            .unwrap();
        // Register chunk with peer availability
        let mut chunk = WeightChunk::new(1, vec![0; 1024]);
        chunk.add_peer(1);
        engine.register_chunk(chunk).unwrap();

        let result = engine.download_chunk(1, 1000);
        assert!(result.is_ok());
        let record = result.unwrap();
        assert!(record.success);
    }

    #[test]
    fn test_download_chunk_not_found() {
        let mut engine = ProgressiveWeightStreaming::new();
        engine
            .register_peer(StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0))
            .unwrap();
        let result = engine.download_chunk(999, 1000);
        assert!(matches!(result, Err(StreamingError::ChunkNotFound(999))));
    }

    #[test]
    fn test_download_chunk_no_peers() {
        let mut engine = ProgressiveWeightStreaming::new();
        let result = engine.download_chunk(1, 1000);
        assert!(matches!(result, Err(StreamingError::InsufficientPeers(0))));
    }

    #[test]
    fn test_progress_empty() {
        let engine = ProgressiveWeightStreaming::new();
        assert!((engine.progress() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_progress_partial() {
        let mut engine = ProgressiveWeightStreaming::new();
        engine
            .register_peer(StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0))
            .unwrap();
        // Register 2 chunks
        let mut chunk1 = WeightChunk::new(1, vec![0; 100]);
        chunk1.add_peer(1);
        engine.register_chunk(chunk1).unwrap();
        let mut chunk2 = WeightChunk::new(2, vec![0; 100]);
        chunk2.add_peer(1);
        engine.register_chunk(chunk2).unwrap();

        // Download 1 chunk
        engine.download_chunk(1, 1000).unwrap();
        assert!((engine.progress() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_total_downloaded_bytes() {
        let mut engine = ProgressiveWeightStreaming::new();
        engine
            .register_peer(StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0))
            .unwrap();
        let mut chunk = WeightChunk::new(1, vec![0; 2048]);
        chunk.add_peer(1);
        engine.register_chunk(chunk).unwrap();
        engine.download_chunk(1, 1000).unwrap();
        assert_eq!(engine.total_downloaded_bytes(), 2048);
    }

    #[test]
    fn test_average_rate() {
        let mut engine = ProgressiveWeightStreaming::new();
        engine
            .register_peer(StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0))
            .unwrap();
        let mut chunk = WeightChunk::new(1, vec![0; 1024]);
        chunk.add_peer(1);
        engine.register_chunk(chunk).unwrap();
        engine.download_chunk(1, 1000).unwrap();
        let avg = engine.average_rate_kb();
        assert!(avg.is_some());
        assert!(avg.unwrap() > 0.0);
    }

    #[test]
    fn test_average_rate_empty() {
        let engine = ProgressiveWeightStreaming::new();
        assert!(engine.average_rate_kb().is_none());
    }

    #[test]
    fn test_reset() {
        let mut engine = ProgressiveWeightStreaming::new();
        engine
            .register_peer(StreamingPeer::new(1, "127.0.0.1:8080".to_string(), 100.0))
            .unwrap();
        let mut chunk = WeightChunk::new(1, vec![0; 100]);
        chunk.add_peer(1);
        engine.register_chunk(chunk).unwrap();
        engine.download_chunk(1, 1000).unwrap();
        engine.load_micro_sae(200).unwrap();

        engine.reset();
        assert_eq!(engine.downloaded_count(), 0);
        assert_eq!(engine.records().len(), 0);
        assert!(!engine.is_micro_loaded());
        // Peers and chunks preserved
        assert_eq!(engine.active_peer_count(), 1);
        assert_eq!(engine.total_chunk_count(), 1);
    }

    #[test]
    fn test_display() {
        let engine = ProgressiveWeightStreaming::new();
        let s = format!("{engine}");
        assert!(s.contains("ProgressiveWeightStreaming"));
    }

    #[test]
    fn test_record_display() {
        let record = TransferRecord {
            timestamp_ms: 1000,
            chunk_id: 1,
            source_peer: 100,
            size_bytes: 1024,
            duration_ms: 10,
            rate_kb: 100.0,
            success: true,
        };
        let s = format!("{record}");
        assert!(s.contains("TransferRecord"));
    }

    // --- Standalone function tests ---

    #[test]
    fn test_fnv_hash_deterministic() {
        let data = vec![1, 2, 3, 4];
        assert_eq!(fnv_hash_bytes(&data), fnv_hash_bytes(&data));
    }

    #[test]
    fn test_fnv_hash_different_data() {
        let data1 = vec![1, 2, 3];
        let data2 = vec![1, 2, 4];
        assert_ne!(fnv_hash_bytes(&data1), fnv_hash_bytes(&data2));
    }

    #[test]
    fn test_estimate_transfer_time() {
        let time = estimate_transfer_time(10240, 10.0); // 10 KB at 10 KB/s
        assert_eq!(time, 1000); // 1 second
    }

    #[test]
    fn test_estimate_transfer_time_zero_rate() {
        let time = estimate_transfer_time(1024, 0.0);
        assert_eq!(time, u64::MAX);
    }

    #[test]
    fn test_optimal_chunk_size() {
        let size = optimal_chunk_size(100.0, 50); // 100 KB/s, 50ms latency
        assert!(size >= 1024);
        assert!(size <= 1024 * 1024);
    }

    #[test]
    fn test_is_chunk_available() {
        let mut chunks = HashMap::new();
        let mut peers = HashMap::new();

        let mut chunk = WeightChunk::new(1, vec![0; 100]);
        chunk.add_peer(10);
        chunks.insert(1, chunk);

        let peer = StreamingPeer::new(10, "addr".to_string(), 100.0);
        peers.insert(10, peer);

        assert!(is_chunk_available(1, &chunks, &peers));
    }

    #[test]
    fn test_is_chunk_not_available() {
        let chunks = HashMap::new();
        let peers = HashMap::new();
        assert!(!is_chunk_available(1, &chunks, &peers));
    }

    #[test]
    fn test_error_display() {
        let err = StreamingError::InvalidChunkSize(0);
        let s = format!("{err}");
        assert!(!s.is_empty());
    }

    // --- Workflow test ---

    #[test]
    fn test_full_workflow() {
        let mut engine = ProgressiveWeightStreaming::new();

        // Load micro-SAE first (fast boot)
        assert!(engine.load_micro_sae(200).is_ok());
        assert!(engine.is_micro_loaded());

        // Register peers
        engine
            .register_peer(StreamingPeer::new(
                1,
                "peer1.example.com:8080".to_string(),
                200.0,
            ))
            .unwrap();
        engine
            .register_peer(StreamingPeer::new(
                2,
                "peer2.example.com:8080".to_string(),
                150.0,
            ))
            .unwrap();

        // Register chunks
        let mut chunk1 = WeightChunk::new(1, vec![0; 1024]);
        chunk1.add_peer(1);
        engine.register_chunk(chunk1).unwrap();

        let mut chunk2 = WeightChunk::new(2, vec![0; 2048]);
        chunk2.add_peer(2);
        engine.register_chunk(chunk2).unwrap();

        // Download chunks progressively
        assert!(engine.download_chunk(1, 1000).is_ok());
        assert!(engine.download_chunk(2, 2000).is_ok());

        // Verify progress
        assert!((engine.progress() - 1.0).abs() < f64::EPSILON);
        assert_eq!(engine.total_downloaded_bytes(), 1024 + 2048);
        assert!(engine.average_rate_kb().is_some());

        // Verify display
        let s = format!("{engine}");
        assert!(s.contains("ProgressiveWeightStreaming"));
    }
}
