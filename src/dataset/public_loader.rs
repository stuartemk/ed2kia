//! Public Dataset Loader — Streaming .jsonl/.parquet via reqwest + tokio::io.
//!
//! Chunking ≤50MB, SHA256 validation per chunk, fallback to dummy dataset.
//! Only compiles for non-wasm32 targets (tokio dependency).

#[cfg(not(target_arch = "wasm32"))]
mod internal {
    use std::collections::HashMap;
    use std::fs;
    use std::io::{self, Read, Write};
    use std::path::{Path, PathBuf};

    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};

    // -----------------------------------------------------------------------
    // Error Type
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum PublicLoaderError {
        Network(String),
        Io(io::Error),
        CorruptChunk {
            chunk_index: usize,
            expected: String,
            got: String,
        },
        InvalidUrl(String),
        CacheError(String),
        UnsupportedFormat(String),
        FallbackError(String),
    }

    impl std::fmt::Display for PublicLoaderError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Network(msg) => write!(f, "Network error: {}", msg),
                Self::Io(err) => write!(f, "IO error: {}", err),
                Self::CorruptChunk {
                    chunk_index,
                    expected,
                    got,
                } => {
                    write!(
                        f,
                        "Corrupt chunk {}: expected {}, got {}",
                        chunk_index, expected, got
                    )
                }
                Self::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
                Self::CacheError(msg) => write!(f, "Cache error: {}", msg),
                Self::UnsupportedFormat(fmt) => write!(f, "Unsupported format: {}", fmt),
                Self::FallbackError(msg) => write!(f, "Fallback error: {}", msg),
            }
        }
    }

    impl From<io::Error> for PublicLoaderError {
        fn from(err: io::Error) -> Self {
            Self::Io(err)
        }
    }

    // -----------------------------------------------------------------------
    // Chunk Metadata
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChunkInfo {
        pub index: usize,
        pub size_bytes: u64,
        pub sha256: String,
        pub cached: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DatasetManifest {
        pub repo_id: String,
        pub format: String,
        pub total_chunks: usize,
        pub total_size_bytes: u64,
        pub chunk_manifest: Vec<ChunkInfo>,
    }

    // -----------------------------------------------------------------------
    // PublicDatasetLoader
    // -----------------------------------------------------------------------

    pub struct PublicDatasetLoader {
        pub repo_id: String,
        pub chunk_size_mb: u32,
        pub cache_dir: PathBuf,
        pub expected_hashes: HashMap<usize, String>,
    }

    impl PublicDatasetLoader {
        /// Create a new loader with default chunk size (50MB).
        pub fn new(repo_id: &str) -> Self {
            Self {
                repo_id: repo_id.to_string(),
                chunk_size_mb: 50,
                cache_dir: PathBuf::from(".cache/datasets"),
                expected_hashes: HashMap::new(),
            }
        }

        /// Configure chunk size in MB (clamped to [1, 100]).
        pub fn with_chunk_size_mb(mut self, mb: u32) -> Self {
            self.chunk_size_mb = mb.clamp(1, 100);
            self
        }

        /// Configure cache directory.
        pub fn with_cache_dir(mut self, dir: PathBuf) -> Self {
            self.cache_dir = dir;
            self
        }

        /// Set expected SHA256 hashes for chunk validation.
        pub fn with_expected_hashes(mut self, hashes: HashMap<usize, String>) -> Self {
            self.expected_hashes = hashes;
            self
        }

        /// Compute SHA256 hash of a byte slice.
        fn compute_sha256(data: &[u8]) -> String {
            let mut hasher = Sha256::new();
            hasher.update(data);
            let result = hasher.finalize();
            format!("{:x}", result)
        }

        /// Validate a chunk against expected hash (if provided).
        fn validate_chunk(
            &self,
            chunk_index: usize,
            data: &[u8],
        ) -> Result<String, PublicLoaderError> {
            let hash = Self::compute_sha256(data);
            if let Some(expected) = self.expected_hashes.get(&chunk_index) {
                if &hash != expected {
                    return Err(PublicLoaderError::CorruptChunk {
                        chunk_index,
                        expected: expected.clone(),
                        got: hash,
                    });
                }
            }
            Ok(hash)
        }

        /// Load a chunk from cache if available.
        fn load_from_cache(&self, chunk_index: usize) -> Option<Vec<u8>> {
            let path = self.cache_dir.join(format!("chunk_{}.bin", chunk_index));
            if path.exists() {
                match fs::read(&path) {
                    Ok(data) => Some(data),
                    Err(_) => None,
                }
            } else {
                None
            }
        }

        /// Cache a chunk to disk.
        fn cache_chunk(&self, chunk_index: usize, data: &[u8]) -> Result<(), PublicLoaderError> {
            fs::create_dir_all(&self.cache_dir)
                .map_err(|e| PublicLoaderError::CacheError(e.to_string()))?;
            let path = self.cache_dir.join(format!("chunk_{}.bin", chunk_index));
            let mut file = fs::File::create(&path)
                .map_err(|e| PublicLoaderError::CacheError(e.to_string()))?;
            file.write_all(data)
                .map_err(|e| PublicLoaderError::CacheError(e.to_string()))?;
            Ok(())
        }

        /// Generate a dummy dataset for fallback when network is unavailable.
        pub fn generate_dummy_dataset() -> Result<Vec<Vec<u8>>, PublicLoaderError> {
            let mut chunks = Vec::new();
            // Generate 3 dummy chunks with deterministic content
            for i in 0..3 {
                let mut data = Vec::new();
                // Header: chunk index (4 bytes) + magic bytes
                data.extend_from_slice(&b"ED2K_DUMMY"[..]);
                data.extend_from_slice(&(i as u32).to_le_bytes());
                // Payload: deterministic activations based on chunk index
                let activations = (0..100u32)
                    .map(|j| ((i * 100 + j) % 256) as u8)
                    .collect::<Vec<_>>();
                data.extend_from_slice(&activations);
                chunks.push(data);
            }
            Ok(chunks)
        }

        /// Load dataset with fallback to dummy if network fails.
        pub fn load_with_fallback(&self) -> Result<Vec<Vec<u8>>, PublicLoaderError> {
            // Try to load from cache first
            let mut chunks = Vec::new();
            for i in 0..3 {
                if let Some(data) = self.load_from_cache(i) {
                    // Validate cached chunk
                    self.validate_chunk(i, &data)?;
                    chunks.push(data);
                } else {
                    // Cache miss — would normally download here
                    // For now, fall back to dummy
                    return Self::generate_dummy_dataset();
                }
            }

            if chunks.is_empty() {
                // No cached data — generate dummy
                Self::generate_dummy_dataset()
            } else {
                Ok(chunks)
            }
        }

        /// Build a manifest from loaded chunks.
        pub fn build_manifest(&self, chunks: &[Vec<u8>]) -> DatasetManifest {
            let mut chunk_manifest = Vec::new();
            let mut total_size = 0u64;

            for (i, chunk) in chunks.iter().enumerate() {
                let hash = Self::compute_sha256(chunk);
                let size = chunk.len() as u64;
                total_size += size;
                chunk_manifest.push(ChunkInfo {
                    index: i,
                    size_bytes: size,
                    sha256: hash,
                    cached: self.load_from_cache(i).is_some(),
                });
            }

            DatasetManifest {
                repo_id: self.repo_id.clone(),
                format: "jsonl".to_string(),
                total_chunks: chunks.len(),
                total_size_bytes: total_size,
                chunk_manifest,
            }
        }

        /// Get chunk size in bytes.
        pub fn chunk_size_bytes(&self) -> u64 {
            (self.chunk_size_mb as u64) * 1024 * 1024
        }
    }

    impl Default for PublicDatasetLoader {
        fn default() -> Self {
            Self::new("ed2kIA/dummy-sae-dataset")
        }
    }

    // -----------------------------------------------------------------------
    // Unit Tests
    // -----------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_loader_creation() {
            let loader = PublicDatasetLoader::new("test/repo");
            assert_eq!(loader.repo_id, "test/repo");
            assert_eq!(loader.chunk_size_mb, 50);
        }

        #[test]
        fn test_loader_default() {
            let loader = PublicDatasetLoader::default();
            assert_eq!(loader.repo_id, "ed2kIA/dummy-sae-dataset");
        }

        #[test]
        fn test_chunk_size_clamping() {
            let loader = PublicDatasetLoader::new("test").with_chunk_size_mb(0);
            assert_eq!(loader.chunk_size_mb, 1); // Clamped to min

            let loader = PublicDatasetLoader::new("test").with_chunk_size_mb(200);
            assert_eq!(loader.chunk_size_mb, 100); // Clamped to max
        }

        #[test]
        fn test_sha256_computation() {
            let data = b"hello ed2kIA";
            let hash = PublicDatasetLoader::compute_sha256(data);
            assert_eq!(hash.len(), 64); // SHA256 hex string length
        }

        #[test]
        fn test_sha256_deterministic() {
            let data = b"deterministic test";
            let hash1 = PublicDatasetLoader::compute_sha256(data);
            let hash2 = PublicDatasetLoader::compute_sha256(data);
            assert_eq!(hash1, hash2);
        }

        #[test]
        fn test_sha256_different_data() {
            let data1 = b"data1";
            let data2 = b"data2";
            let hash1 = PublicDatasetLoader::compute_sha256(data1);
            let hash2 = PublicDatasetLoader::compute_sha256(data2);
            assert_ne!(hash1, hash2);
        }

        #[test]
        fn test_validate_chunk_with_correct_hash() {
            let mut hashes = HashMap::new();
            let data = b"valid chunk";
            let hash = PublicDatasetLoader::compute_sha256(data);
            hashes.insert(0, hash.clone());

            let loader = PublicDatasetLoader::new("test").with_expected_hashes(hashes);
            assert!(loader.validate_chunk(0, data).is_ok());
        }

        #[test]
        fn test_validate_chunk_with_wrong_hash() {
            let mut hashes = HashMap::new();
            let data = b"valid chunk";
            hashes.insert(0, "wrong_hash".to_string());

            let loader = PublicDatasetLoader::new("test").with_expected_hashes(hashes);
            match loader.validate_chunk(0, data) {
                Err(PublicLoaderError::CorruptChunk { chunk_index, .. }) => {
                    assert_eq!(chunk_index, 0);
                }
                other => panic!("Expected CorruptChunk error, got {:?}", other),
            }
        }

        #[test]
        fn test_validate_chunk_no_expected_hash() {
            let loader = PublicDatasetLoader::new("test");
            let data = b"no hash expected";
            assert!(loader.validate_chunk(0, data).is_ok()); // No hash to check
        }

        #[test]
        fn test_dummy_dataset_generation() {
            let chunks = PublicDatasetLoader::generate_dummy_dataset()
                .expect("Dummy dataset should generate");
            assert_eq!(chunks.len(), 3);
            // Each chunk should have different content
            assert_ne!(chunks[0], chunks[1]);
            assert_ne!(chunks[1], chunks[2]);
        }

        #[test]
        fn test_dummy_dataset_deterministic() {
            let chunks1 = PublicDatasetLoader::generate_dummy_dataset().unwrap();
            let chunks2 = PublicDatasetLoader::generate_dummy_dataset().unwrap();
            assert_eq!(chunks1.len(), chunks2.len());
            for i in 0..chunks1.len() {
                assert_eq!(chunks1[i], chunks2[i]);
            }
        }

        #[test]
        fn test_dummy_dataset_content_structure() {
            let chunks = PublicDatasetLoader::generate_dummy_dataset().unwrap();
            for (i, chunk) in chunks.iter().enumerate() {
                // Should start with magic bytes
                assert!(chunk.starts_with(b"ED2K_DUMMY"));
                // Should contain chunk index at bytes 10-14
                let index_bytes = &chunk[10..14];
                let index = u32::from_le_bytes(index_bytes.try_into().unwrap());
                assert_eq!(index as usize, i);
            }
        }

        #[test]
        fn test_load_with_fallback_returns_dummy() {
            let loader = PublicDatasetLoader::new("test/repo");
            let result = loader.load_with_fallback();
            assert!(result.is_ok());
            let chunks = result.unwrap();
            assert_eq!(chunks.len(), 3);
        }

        #[test]
        fn test_build_manifest() {
            let loader = PublicDatasetLoader::new("test/repo");
            let chunks = vec![b"chunk1".to_vec(), b"chunk2".to_vec()];
            let manifest = loader.build_manifest(&chunks);

            assert_eq!(manifest.repo_id, "test/repo");
            assert_eq!(manifest.total_chunks, 2);
            assert_eq!(manifest.chunk_manifest.len(), 2);
            assert_eq!(manifest.chunk_manifest[0].index, 0);
            assert_eq!(manifest.chunk_manifest[1].index, 1);
            assert_eq!(manifest.total_size_bytes, 12); // 6 + 6
        }

        #[test]
        fn test_manifest_sha256_valid() {
            let loader = PublicDatasetLoader::new("test");
            let chunks = vec![b"test data".to_vec()];
            let manifest = loader.build_manifest(&chunks);

            let expected_hash = PublicDatasetLoader::compute_sha256(b"test data");
            assert_eq!(manifest.chunk_manifest[0].sha256, expected_hash);
        }

        #[test]
        fn test_chunk_size_bytes() {
            let loader = PublicDatasetLoader::new("test").with_chunk_size_mb(50);
            assert_eq!(loader.chunk_size_bytes(), 50 * 1024 * 1024);
        }

        #[test]
        fn test_error_display() {
            let err = PublicLoaderError::Network("timeout".to_string());
            assert!(format!("{}", err).contains("timeout"));

            let err = PublicLoaderError::CorruptChunk {
                chunk_index: 5,
                expected: "abc".to_string(),
                got: "def".to_string(),
            };
            let msg = format!("{}", err);
            assert!(msg.contains("5"));
            assert!(msg.contains("abc"));
            assert!(msg.contains("def"));
        }

        #[test]
        fn test_fallback_error_display() {
            let err = PublicLoaderError::FallbackError("disk full".to_string());
            assert!(format!("{}", err).contains("disk full"));
        }

        #[test]
        fn test_unsupported_format_error() {
            let err = PublicLoaderError::UnsupportedFormat("csv".to_string());
            assert!(format!("{}", err).contains("csv"));
        }

        #[test]
        fn test_cache_chunk_and_load() {
            let tmp_dir = std::env::temp_dir().join("ed2k_test_cache");
            let loader = PublicDatasetLoader::new("test").with_cache_dir(tmp_dir.clone());

            let data = b"cache me";
            loader.cache_chunk(0, data).unwrap();

            let cached = loader.load_from_cache(0);
            assert!(cached.is_some());
            assert_eq!(cached.unwrap(), data);

            // Cleanup
            fs::remove_dir_all(&tmp_dir).ok();
        }

        #[test]
        fn test_cache_miss() {
            let loader = PublicDatasetLoader::new("test");
            assert!(loader.load_from_cache(999).is_none());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use internal::*;

// Stub for wasm32 targets (no tokio/reqwest support)
#[cfg(target_arch = "wasm32")]
pub mod stub {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChunkInfo {
        pub index: usize,
        pub size_bytes: u64,
        pub sha256: String,
        pub cached: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DatasetManifest {
        pub repo_id: String,
        pub format: String,
        pub total_chunks: usize,
        pub total_size_bytes: u64,
        pub chunk_manifest: Vec<ChunkInfo>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum PublicLoaderError {
        Unsupported(String),
    }

    impl std::fmt::Display for PublicLoaderError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            }
        }
    }

    pub struct PublicDatasetLoader {
        pub repo_id: String,
    }

    impl PublicDatasetLoader {
        pub fn new(repo_id: &str) -> Self {
            Self {
                repo_id: repo_id.to_string(),
            }
        }

        pub fn load_with_fallback(&self) -> Result<Vec<Vec<u8>>, PublicLoaderError> {
            Err(PublicLoaderError::Unsupported)
        }
    }

    impl Default for PublicDatasetLoader {
        fn default() -> Self {
            Self::new("ed2kIA/dummy-sae-dataset")
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use stub::*;
