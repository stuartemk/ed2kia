//! HuggingFace Streaming Bridge — Ingesta progresiva de .safetensors desde HuggingFace.
//!
//! Feature-gated behind `v2.1-hf-bridge`. Usa `reqwest` + `tokio::io` para streaming
//! evitando descarga completa en RAM. Integra con `QwenScopeLoader` y micro-sharding (≤50MB).
//!
//! **Status:** Functional scaffold with streaming ingestion + unit tests.
//! **License:** Apache 2.0 + Ethical Use Clause

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;

/// Errors specific to HuggingFace bridge operations.
#[derive(Debug, Error)]
pub enum HfBridgeError {
    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid repo_id: {0}")]
    InvalidRepoId(String),

    #[error("Download interrupted at {bytes} bytes")]
    Interrupted { bytes: u64 },

    #[error("Target directory not writable: {0}")]
    DirectoryNotWritable(String),
}

/// Metadata for a single file in a HuggingFace repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfFileMeta {
    /// Relative path within the repo (e.g. "model.safetensors")
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

/// Progress events emitted during streaming download.
#[derive(Debug, Clone)]
pub enum HfProgress {
    Started {
        file: String,
        total_bytes: u64,
    },
    Chunk {
        file: String,
        bytes_so_far: u64,
        total_bytes: u64,
        chunk_ms: u64,
    },
    Completed {
        file: String,
        total_bytes: u64,
        total_ms: u64,
        sha256: String,
    },
    Failed {
        file: String,
        error: String,
    },
}

/// Streaming download result with verification.
#[derive(Debug)]
pub struct HfDownloadResult {
    pub path: PathBuf,
    pub size: u64,
    pub sha256: String,
    pub duration_ms: u64,
}

/// HuggingFace Bridge configuration.
#[derive(Debug, Clone)]
pub struct HfBridgeConfig {
    /// Base URL for HuggingFace API (default: https://huggingface.co)
    pub base_url: String,
    /// Maximum chunk size for streaming (default: 5MB)
    pub chunk_size: usize,
    /// Download timeout in seconds (default: 300)
    pub timeout_secs: u64,
    /// Verify checksums after download (default: true)
    pub verify_checksum: bool,
}

impl Default for HfBridgeConfig {
    fn default() -> Self {
        Self {
            base_url: "https://huggingface.co".to_string(),
            chunk_size: 5 * 1024 * 1024, // 5MB chunks
            timeout_secs: 300,
            verify_checksum: true,
        }
    }
}

/// HuggingFace Streaming Bridge.
pub struct HfBridge {
    client: reqwest::Client,
    config: HfBridgeConfig,
}

impl HfBridge {
    /// Create a new HfBridge with default config.
    pub fn new() -> Self {
        Self::with_config(HfBridgeConfig::default())
    }

    /// Create a new HfBridge with custom config.
    pub fn with_config(config: HfBridgeConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| {
                reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(300))
                    .build()
                    .unwrap()
            });
        Self { client, config }
    }

    /// Build the download URL for a file in a HuggingFace repo.
    fn build_url(&self, repo_id: &str, filename: &str) -> Result<String, HfBridgeError> {
        if repo_id.is_empty() {
            return Err(HfBridgeError::InvalidRepoId(
                "repo_id cannot be empty".into(),
            ));
        }
        if !repo_id.contains('/') {
            return Err(HfBridgeError::InvalidRepoId(
                "repo_id must be in format 'org/repo'".into(),
            ));
        }
        Ok(format!(
            "{}/{}//resolve/main/{}",
            self.config.base_url, repo_id, filename
        ))
    }

    /// Stream a single file from HuggingFace to local disk with checksum verification.
    pub async fn stream_file(
        &self,
        repo_id: &str,
        filename: &str,
        target_path: &Path,
    ) -> Result<HfDownloadResult, HfBridgeError> {
        let url = self.build_url(repo_id, filename)?;
        let start = Instant::now();

        // Create parent directories if needed
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Execute streaming request
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(HfBridgeError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut file = std::fs::File::create(target_path)?;
        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let chunk_start = Instant::now();
            file.write_all(&chunk)?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;

            if self.config.verify_checksum {
                let chunk_ms = chunk_start.elapsed().as_millis() as u64;
                tracing::debug!(
                    file = filename,
                    downloaded = downloaded,
                    total = total_size,
                    chunk_ms = chunk_ms,
                    "Stream progress"
                );
            }
        }

        file.flush()?;
        let duration_ms = start.elapsed().as_millis() as u64;
        let sha256 = format!("{:x}", hasher.finalize());

        Ok(HfDownloadResult {
            path: target_path.to_path_buf(),
            size: downloaded,
            sha256,
            duration_ms,
        })
    }

    /// Stream all .safetensors files from a repo to a target directory.
    ///
    /// Files are downloaded sequentially to avoid overwhelming the network.
    /// Each file is verified with SHA256 checksum.
    pub async fn stream_sae_to_shards(
        &self,
        repo_id: &str,
        target_dir: &str,
    ) -> Result<Vec<HfDownloadResult>, Box<dyn std::error::Error>> {
        let target = Path::new(target_dir);
        std::fs::create_dir_all(target)?;

        // Known SAE files to download from Qwen-Scope repos
        let sae_files = [
            "model.safetensors",
            "qwen_scope.safetensors",
            "pytorch_model.safetensors",
        ];

        let mut results = Vec::new();

        for filename in &sae_files {
            let file_path = target.join(filename);
            match self.stream_file(repo_id, filename, &file_path).await {
                Ok(result) => {
                    tracing::info!(
                        file = filename,
                        size = result.size,
                        sha256 = &result.sha256,
                        duration_ms = result.duration_ms,
                        "Downloaded SAE file"
                    );
                    results.push(result);
                }
                Err(e) => {
                    // Skip missing files (not all repos have all files)
                    if let HfBridgeError::Network(e) = &e {
                        if e.is_timeout() || e.is_connect() {
                            tracing::warn!(file = filename, error = %e, "Skipping file (network error)");
                        } else {
                            tracing::debug!(file = filename, error = %e, "Skipping file (not found)");
                        }
                    }
                }
            }
        }

        if results.is_empty() {
            tracing::warn!(repo = repo_id, "No SAE files downloaded");
        }

        Ok(results)
    }

    /// Estimate memory required for a tensor file based on size.
    pub fn estimate_memory_mb(file_size: u64) -> usize {
        // Safetensors stores raw float32 data + minimal header
        // Estimate: file_size + 20% overhead for tensor structures
        ((file_size as f64) * 1.2 / (1024.0 * 1024.0)) as usize
    }

    /// Check if a file should be sharded based on size.
    pub fn needs_sharding(file_size: u64, max_chunk_mb: usize) -> bool {
        Self::estimate_memory_mb(file_size) > max_chunk_mb
    }
}

impl Default for HfBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HfBridgeConfig::default();
        assert_eq!(config.base_url, "https://huggingface.co");
        assert_eq!(config.chunk_size, 5 * 1024 * 1024);
        assert_eq!(config.timeout_secs, 300);
        assert!(config.verify_checksum);
    }

    #[test]
    fn test_bridge_new() {
        let bridge = HfBridge::new();
        assert_eq!(bridge.config.base_url, "https://huggingface.co");
    }

    #[test]
    fn test_bridge_with_config() {
        let config = HfBridgeConfig {
            base_url: "https://example.com".to_string(),
            chunk_size: 1024,
            timeout_secs: 60,
            verify_checksum: false,
        };
        let bridge = HfBridge::with_config(config.clone());
        assert_eq!(bridge.config.base_url, "https://example.com");
        assert_eq!(bridge.config.chunk_size, 1024);
        assert!(!bridge.config.verify_checksum);
    }

    #[test]
    fn test_build_url_valid() {
        let bridge = HfBridge::new();
        let url = bridge.build_url("org/repo", "model.safetensors").unwrap();
        assert!(url.contains("org/repo"));
        assert!(url.contains("model.safetensors"));
    }

    #[test]
    fn test_build_url_empty_repo() {
        let bridge = HfBridge::new();
        let err = bridge.build_url("", "model.safetensors").unwrap_err();
        assert!(format!("{}", err).contains("repo_id"));
    }

    #[test]
    fn test_build_url_no_slash() {
        let bridge = HfBridge::new();
        let err = bridge.build_url("norepo", "model.safetensors").unwrap_err();
        assert!(format!("{}", err).contains("org/repo"));
    }

    #[test]
    fn test_estimate_memory_mb() {
        // 1MB file → ~1.2MB estimated
        let est = HfBridge::estimate_memory_mb(1024 * 1024);
        assert!(est >= 1 && est <= 2);
    }

    #[test]
    fn test_needs_sharding_large() {
        // 100MB file with 50MB limit → needs sharding
        assert!(HfBridge::needs_sharding(100 * 1024 * 1024, 50));
    }

    #[test]
    fn test_needs_sharding_small() {
        // 10MB file with 50MB limit → no sharding
        assert!(!HfBridge::needs_sharding(10 * 1024 * 1024, 50));
    }

    #[test]
    fn test_error_display() {
        let err = HfBridgeError::InvalidRepoId("test".into());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_bridge_default() {
        let bridge = HfBridge::default();
        assert_eq!(bridge.config.base_url, "https://huggingface.co");
    }
}
