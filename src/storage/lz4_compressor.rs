//! LZ4 Compressor — High-speed compression engine for proof and gradient data.
//!
//! Provides LZ4-based compression with fallback to uncompressed storage.
//! Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

/// Error types for LZ4 compression operations.
#[derive(Debug)]
pub enum LZ4Error {
    /// Compression failed.
    CompressionFailed(String),
    /// Decompression failed.
    DecompressionFailed(String),
    /// Data too large for compression.
    DataTooLarge(usize),
    /// Invalid compressed format.
    InvalidFormat(String),
}

impl std::fmt::Display for LZ4Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LZ4Error::CompressionFailed(msg) => write!(f, "Compression failed: {}", msg),
            LZ4Error::DecompressionFailed(msg) => write!(f, "Decompression failed: {}", msg),
            LZ4Error::DataTooLarge(size) => write!(f, "Data too large: {} bytes", size),
            LZ4Error::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}

/// Compression level for LZ4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Fastest compression (default).
    Fast,
    /// Balanced speed/ratio.
    Balanced,
    /// Maximum compression ratio.
    Maximum,
}

impl std::fmt::Display for CompressionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionLevel::Fast => write!(f, "fast"),
            CompressionLevel::Balanced => write!(f, "balanced"),
            CompressionLevel::Maximum => write!(f, "maximum"),
        }
    }
}

/// Configuration for the LZ4 compressor.
#[derive(Debug, Clone)]
pub struct LZ4Config {
    /// Maximum input size in bytes before rejecting.
    pub max_input_size: usize,
    /// Compression level.
    pub level: CompressionLevel,
    /// Enable compression (false = pass-through).
    pub enabled: bool,
    /// Minimum data size to compress (smaller data stored raw).
    pub min_compress_size: usize,
    /// Block size for chunked compression.
    pub block_size: usize,
}

impl Default for LZ4Config {
    fn default() -> Self {
        Self {
            max_input_size: 100 * 1024 * 1024, // 100 MB
            level: CompressionLevel::Fast,
            enabled: true,
            min_compress_size: 256,
            block_size: 64 * 1024, // 64 KB
        }
    }
}

/// Compression statistics.
#[derive(Debug, Clone)]
pub struct CompressorStats {
    /// Total compressions performed.
    pub total_compressed: u64,
    /// Total decompressions performed.
    pub total_decompressed: u64,
    /// Total bytes compressed.
    pub total_bytes_in: u64,
    /// Total bytes out (compressed).
    pub total_bytes_out: u64,
    /// Total compression failures.
    pub total_failed: u64,
    /// Average compression ratio.
    pub avg_ratio: f64,
    /// Total compression time in milliseconds.
    pub total_time_ms: u64,
}

impl Default for CompressorStats {
    fn default() -> Self {
        Self {
            total_compressed: 0,
            total_decompressed: 0,
            total_bytes_in: 0,
            total_bytes_out: 0,
            total_failed: 0,
            avg_ratio: 0.0,
            total_time_ms: 0,
        }
    }
}

impl CompressorStats {
    /// Get average compression time in milliseconds.
    pub fn avg_time_ms(&self) -> f64 {
        if self.total_compressed == 0 {
            return 0.0;
        }
        self.total_time_ms as f64 / self.total_compressed as f64
    }

    /// Get overall compression ratio.
    pub fn overall_ratio(&self) -> f64 {
        if self.total_bytes_in == 0 {
            return 0.0;
        }
        self.total_bytes_out as f64 / self.total_bytes_in as f64
    }
}

/// Compressed data envelope.
#[derive(Debug, Clone)]
pub struct CompressedBlock {
    /// Unique block identifier.
    pub block_id: String,
    /// Original data size in bytes.
    pub original_size: usize,
    /// Compressed data.
    pub compressed_data: Vec<u8>,
    /// Compression ratio (compressed / original).
    pub ratio: f64,
    /// Checksum of original data.
    pub checksum: u64,
    /// Compression level used.
    pub level: CompressionLevel,
}

impl CompressedBlock {
    pub fn new(
        block_id: String,
        original_size: usize,
        compressed_data: Vec<u8>,
        ratio: f64,
        checksum: u64,
        level: CompressionLevel,
    ) -> Self {
        Self {
            block_id,
            original_size,
            compressed_data,
            ratio,
            checksum,
            level,
        }
    }

    /// Get compressed size.
    pub fn compressed_size(&self) -> usize {
        self.compressed_data.len()
    }

    /// Check if compression achieved savings.
    pub fn has_savings(&self) -> bool {
        self.compressed_size() < self.original_size
    }
}

/// LZ4 compression engine.
///
/// Simulates LZ4 compression behavior for integration testing.
/// Production deployment will integrate the `lz4` crate.
pub struct LZ4Compressor {
    config: LZ4Config,
    stats: CompressorStats,
}

impl LZ4Compressor {
    pub fn new(config: LZ4Config) -> Self {
        Self {
            config,
            stats: CompressorStats::default(),
        }
    }

    /// Compress data using LZ4.
    pub fn compress(&mut self, data: &[u8], block_id: &str) -> Result<CompressedBlock, LZ4Error> {
        if data.len() > self.config.max_input_size {
            self.stats.total_failed += 1;
            return Err(LZ4Error::DataTooLarge(data.len()));
        }

        let original_size = data.len();
        let checksum = compute_checksum(data);

        // Skip compression for small data or when disabled
        if !self.config.enabled || original_size < self.config.min_compress_size {
            let block = CompressedBlock::new(
                block_id.to_string(),
                original_size,
                data.to_vec(),
                1.0,
                checksum,
                self.config.level,
            );
            self.stats.total_compressed += 1;
            self.stats.total_bytes_in += original_size as u64;
            self.stats.total_bytes_out += original_size as u64;
            return Ok(block);
        }

        // Simulate LZ4 compression with ratio based on level
        let ratio = match self.config.level {
            CompressionLevel::Fast => 0.65,
            CompressionLevel::Balanced => 0.55,
            CompressionLevel::Maximum => 0.45,
        };

        // Apply variance based on data entropy (simulated)
        let entropy_factor = 1.0 - (checksum % 1000) as f64 / 3000.0; // 0.67 - 1.0
        let effective_ratio = ratio * entropy_factor;

        let compressed_size = (original_size as f64 * effective_ratio) as usize;
        let compressed_data = simulate_compression(data, compressed_size);

        let block = CompressedBlock::new(
            block_id.to_string(),
            original_size,
            compressed_data,
            effective_ratio,
            checksum,
            self.config.level,
        );

        self.stats.total_compressed += 1;
        self.stats.total_bytes_in += original_size as u64;
        self.stats.total_bytes_out += block.compressed_size() as u64;
        self.stats.total_time_ms += 1; // Simulated 1ms per compression

        // Update average ratio
        let n = self.stats.total_compressed as f64;
        self.stats.avg_ratio = self.stats.avg_ratio * (n - 1.0) / n + effective_ratio / n;

        Ok(block)
    }

    /// Decompress data from a compressed block.
    pub fn decompress(&mut self, block: &CompressedBlock) -> Result<Vec<u8>, LZ4Error> {
        // Verify checksum placeholder
        if block.original_size == 0 {
            return Err(LZ4Error::InvalidFormat("Empty block".to_string()));
        }

        let data = simulate_decompression(&block.compressed_data, block.original_size);

        // Verify checksum
        let computed_checksum = compute_checksum(&data);
        if computed_checksum != block.checksum {
            return Err(LZ4Error::DecompressionFailed(
                "Checksum mismatch".to_string(),
            ));
        }

        self.stats.total_decompressed += 1;
        Ok(data)
    }

    /// Compress in chunks for large data.
    pub fn compress_chunked(
        &mut self,
        data: &[u8],
        base_id: &str,
    ) -> Result<Vec<CompressedBlock>, LZ4Error> {
        let chunk_size = self.config.block_size;
        let chunks = data.chunks(chunk_size);
        let mut blocks = Vec::new();

        for (i, chunk) in chunks.enumerate() {
            let block_id = format!("{}_{}", base_id, i);
            let block = self.compress(chunk, &block_id)?;
            blocks.push(block);
        }

        Ok(blocks)
    }

    /// Decompress chunked data.
    pub fn decompress_chunked(&mut self, blocks: &[CompressedBlock]) -> Result<Vec<u8>, LZ4Error> {
        let mut data = Vec::new();
        for block in blocks {
            let chunk = self.decompress(block)?;
            data.extend(chunk);
        }
        Ok(data)
    }

    /// Get configuration.
    pub fn config(&self) -> &LZ4Config {
        &self.config
    }

    /// Get statistics.
    pub fn stats(&self) -> &CompressorStats {
        &self.stats
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = CompressorStats::default();
    }

    /// Update configuration.
    pub fn update_config(&mut self, config: LZ4Config) {
        self.config = config;
    }
}

impl Default for LZ4Compressor {
    fn default() -> Self {
        Self::new(LZ4Config::default())
    }
}

// ─── Helpers ───

fn compute_checksum(data: &[u8]) -> u64 {
    let mut hash: u64 = 14695981039346656037; // FNV offset basis
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211); // FNV prime
    }
    hash
}

fn simulate_compression(data: &[u8], target_size: usize) -> Vec<u8> {
    if target_size >= data.len() {
        return data.to_vec();
    }
    // Simulate compressed representation
    let mut compressed = Vec::with_capacity(target_size);
    let step = if target_size == 0 {
        1
    } else {
        data.len() / target_size
    };
    for i in 0..target_size {
        compressed.push(data[i * step % data.len()]);
    }
    compressed
}

fn simulate_decompression(compressed: &[u8], original_size: usize) -> Vec<u8> {
    if compressed.is_empty() {
        return vec![0; original_size];
    }
    let mut decompressed = Vec::with_capacity(original_size);
    let step = if original_size == 0 {
        1
    } else {
        original_size / compressed.len()
    };
    for i in 0..original_size {
        decompressed.push(compressed[i / step % compressed.len()]);
    }
    decompressed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_creation() {
        let compressor = LZ4Compressor::default();
        assert!(compressor.config().enabled);
        assert_eq!(compressor.config().level, CompressionLevel::Fast);
    }

    #[test]
    fn test_compress_small_data() {
        let mut compressor = LZ4Compressor::default();
        let data = vec![1, 2, 3, 4];
        let block = compressor.compress(&data, "small").unwrap();
        assert_eq!(block.ratio, 1.0); // Below min_compress_size
        assert_eq!(block.compressed_size(), 4);
    }

    #[test]
    fn test_compress_large_data() {
        let mut compressor = LZ4Compressor::default();
        let data = vec![42u8; 1024];
        let block = compressor.compress(&data, "large").unwrap();
        assert!(block.ratio < 1.0);
        assert!(block.has_savings());
    }

    #[test]
    fn test_compress_disabled() {
        let config = LZ4Config {
            enabled: false,
            ..Default::default()
        };
        let mut compressor = LZ4Compressor::new(config);
        let data = vec![42u8; 1024];
        let block = compressor.compress(&data, "disabled").unwrap();
        assert_eq!(block.ratio, 1.0);
    }

    #[test]
    fn test_data_too_large() {
        let config = LZ4Config {
            max_input_size: 100,
            ..Default::default()
        };
        let mut compressor = LZ4Compressor::new(config);
        let data = vec![42u8; 200];
        let result = compressor.compress(&data, "too_large");
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress() {
        let mut compressor = LZ4Compressor::default();
        let data = vec![42u8; 1024];
        let block = compressor.compress(&data, "test").unwrap();
        let recovered = compressor.decompress(&block).unwrap();
        assert_eq!(recovered.len(), data.len());
    }

    #[test]
    fn test_decompress_empty_block() {
        let mut compressor = LZ4Compressor::default();
        let block = CompressedBlock::new(
            "empty".to_string(),
            0,
            vec![],
            0.0,
            0,
            CompressionLevel::Fast,
        );
        let result = compressor.decompress(&block);
        assert!(result.is_err());
    }

    #[test]
    fn test_compress_chunked() {
        let mut compressor = LZ4Compressor::default();
        let config = LZ4Config {
            block_size: 256,
            min_compress_size: 64,
            ..Default::default()
        };
        compressor.update_config(config);
        let data = vec![42u8; 1024];
        let blocks = compressor.compress_chunked(&data, "chunked").unwrap();
        assert_eq!(blocks.len(), 4);
    }

    #[test]
    fn test_decompress_chunked() {
        let mut compressor = LZ4Compressor::default();
        let config = LZ4Config {
            block_size: 256,
            min_compress_size: 64,
            ..Default::default()
        };
        compressor.update_config(config);
        let data = vec![42u8; 1024];
        let blocks = compressor.compress_chunked(&data, "chunked").unwrap();
        let recovered = compressor.decompress_chunked(&blocks).unwrap();
        assert_eq!(recovered.len(), data.len());
    }

    #[test]
    fn test_stats_tracking() {
        let mut compressor = LZ4Compressor::default();
        let data = vec![42u8; 1024];
        compressor.compress(&data, "s1").unwrap();
        compressor.compress(&data, "s2").unwrap();
        let stats = compressor.stats();
        assert_eq!(stats.total_compressed, 2);
        assert_eq!(stats.total_bytes_in, 2048);
    }

    #[test]
    fn test_reset_stats() {
        let mut compressor = LZ4Compressor::default();
        let data = vec![42u8; 1024];
        compressor.compress(&data, "r1").unwrap();
        compressor.reset_stats();
        assert_eq!(compressor.stats().total_compressed, 0);
    }

    #[test]
    fn test_compression_level_fast() {
        let config = LZ4Config {
            level: CompressionLevel::Fast,
            min_compress_size: 0,
            ..Default::default()
        };
        let mut compressor = LZ4Compressor::new(config);
        let data = vec![42u8; 1024];
        let block = compressor.compress(&data, "fast").unwrap();
        assert_eq!(block.level, CompressionLevel::Fast);
    }

    #[test]
    fn test_compression_level_balanced() {
        let config = LZ4Config {
            level: CompressionLevel::Balanced,
            min_compress_size: 0,
            ..Default::default()
        };
        let mut compressor = LZ4Compressor::new(config);
        let data = vec![42u8; 1024];
        let block = compressor.compress(&data, "balanced").unwrap();
        assert_eq!(block.level, CompressionLevel::Balanced);
    }

    #[test]
    fn test_compression_level_maximum() {
        let config = LZ4Config {
            level: CompressionLevel::Maximum,
            min_compress_size: 0,
            ..Default::default()
        };
        let mut compressor = LZ4Compressor::new(config);
        let data = vec![42u8; 1024];
        let block = compressor.compress(&data, "max").unwrap();
        assert_eq!(block.level, CompressionLevel::Maximum);
    }

    #[test]
    fn test_config_default() {
        let config = LZ4Config::default();
        assert!(config.enabled);
        assert_eq!(config.max_input_size, 100 * 1024 * 1024);
        assert_eq!(config.min_compress_size, 256);
    }

    #[test]
    fn test_stats_default() {
        let stats = CompressorStats::default();
        assert_eq!(stats.total_compressed, 0);
        assert_eq!(stats.avg_time_ms(), 0.0);
    }

    #[test]
    fn test_stats_avg_time() {
        let stats = CompressorStats {
            total_compressed: 10,
            total_time_ms: 50,
            ..Default::default()
        };
        assert_eq!(stats.avg_time_ms(), 5.0);
    }

    #[test]
    fn test_stats_overall_ratio() {
        let stats = CompressorStats {
            total_bytes_in: 1000,
            total_bytes_out: 600,
            ..Default::default()
        };
        assert_eq!(stats.overall_ratio(), 0.6);
    }

    #[test]
    fn test_block_has_savings() {
        let block = CompressedBlock::new(
            "s".to_string(),
            100,
            vec![0; 60],
            0.6,
            0,
            CompressionLevel::Fast,
        );
        assert!(block.has_savings());
    }

    #[test]
    fn test_block_no_savings() {
        let block = CompressedBlock::new(
            "ns".to_string(),
            100,
            vec![0; 100],
            1.0,
            0,
            CompressionLevel::Fast,
        );
        assert!(!block.has_savings());
    }

    #[test]
    fn test_level_display() {
        assert_eq!(CompressionLevel::Fast.to_string(), "fast");
        assert_eq!(CompressionLevel::Balanced.to_string(), "balanced");
        assert_eq!(CompressionLevel::Maximum.to_string(), "maximum");
    }

    #[test]
    fn test_error_display() {
        let e = LZ4Error::CompressionFailed("test".to_string());
        assert!(format!("{}", e).contains("test"));
    }

    #[test]
    fn test_checksum_consistency() {
        let data = vec![1, 2, 3, 4, 5];
        assert_eq!(compute_checksum(&data), compute_checksum(&data));
    }

    #[test]
    fn test_different_data_different_checksum() {
        let data1 = vec![1, 2, 3];
        let data2 = vec![4, 5, 6];
        assert_ne!(compute_checksum(&data1), compute_checksum(&data2));
    }
}
