//! Hugging Face / ModelScope Sync - Sincronización con repositorios abiertos
//!
//! Descarga y verifica modelos SAE (.safetensors) desde Hugging Face y
//! ModelScope, con rate limiting, cache local y verificación de checksums.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
// FIX: E0599 - writeln! requires Write trait in scope
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{info, warn};
// CLEANUP: removed unused import error

/// Error de sincronización con ecosistema
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Filesystem error: {path}: {msg}")]
    Filesystem { path: String, msg: String },
    #[error("Checksum mismatch: expected={expected}, got={got}")]
    ChecksumMismatch { expected: String, got: String },
    #[error("Rate limit exceeded. Retry after {retry_after:?}")]
    RateLimitExceeded { retry_after: Duration },
    #[error("Invalid repository format: {0}")]
    InvalidRepository(String),
    #[error("Model not found: {repo}/{file}")]
    ModelNotFound { repo: String, file: String },
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Configuración de sincronización
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// URL base de Hugging Face
    pub hf_base_url: String,
    /// URL base de ModelScope
    pub modelscope_base_url: String,
    /// Directorio de cache local
    pub cache_dir: PathBuf,
    /// Rate limit: requests por minuto
    pub rate_limit_rpm: u32,
    /// Timeout por descarga (segundos)
    pub download_timeout_secs: u64,
    /// Chunk size para descargas (bytes)
    pub chunk_size: usize,
    /// Verificar checksums
    pub verify_checksums: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            hf_base_url: "https://huggingface.co".to_string(),
            modelscope_base_url: "https://www.modelscope.cn".to_string(),
            cache_dir: PathBuf::from("./cache/models"),
            rate_limit_rpm: 30,
            download_timeout_secs: 300,
            chunk_size: 64 * 1024, // 64KB
            verify_checksums: true,
        }
    }
}

/// Fuente del modelo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelSource {
    HuggingFace,
    ModelScope,
    Local,
}

impl std::fmt::Display for ModelSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelSource::HuggingFace => write!(f, "HuggingFace"),
            ModelSource::ModelScope => write!(f, "ModelScope"),
            ModelSource::Local => write!(f, "Local"),
        }
    }
}

/// Metadata de un modelo remoto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteModelInfo {
    /// Repositorio (ej: "Qwen-Scope/SAE-Res-Qwen3.5-27B")
    pub repo_id: String,
    /// Nombre del archivo
    pub filename: String,
    /// Tamaño en bytes
    pub size_bytes: u64,
    /// Checksum SHA-256 esperado
    pub expected_sha256: Option<String>,
    /// Fuente
    pub source: ModelSource,
    /// URL de descarga directa
    pub download_url: String,
    /// Timestamp de última modificación remota
    pub last_modified: Option<String>,
}

/// Gestor de sincronización
pub struct HfSyncManager {
    config: SyncConfig,
    client: Client,
    /// Rate limiter: instantes de las últimas requests
    request_timestamps: Vec<Instant>,
}

impl HfSyncManager {
    pub fn new() -> Self {
        Self {
            config: SyncConfig::default(),
            client: Client::new(),
            request_timestamps: Vec::new(),
        }
    }

    pub fn with_config(config: SyncConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .user_agent("ed2kIA/0.5.0")
                .build()
                .unwrap_or_else(|_| Client::new()),
            request_timestamps: Vec::new(),
        }
    }

    /// Parsear repositorio "owner/repo" a URL de descarga
    fn build_download_url(&self, repo: &str, filename: &str, source: &ModelSource) -> String {
        match source {
            ModelSource::HuggingFace => {
                format!(
                    "{}/api/{}/resolve/main/{}",
                    self.config.hf_base_url, repo, filename
                )
            }
            ModelSource::ModelScope => {
                format!(
                    "{}/api/v1/models/{}/repo?fileName={}",
                    self.config.modelscope_base_url, repo, filename
                )
            }
            ModelSource::Local => {
                format!("/{}", repo)
            }
        }
    }

    /// Verificar rate limit y esperar si es necesario
    async fn enforce_rate_limit(&mut self) -> Result<(), SyncError> {
        let now = Instant::now();
        // Limpiar timestamps viejos (> 1 min)
        self.request_timestamps
            .retain(|t| now.duration_since(*t) < Duration::from_secs(60));

        if self.request_timestamps.len() >= self.config.rate_limit_rpm as usize {
            let oldest = self.request_timestamps.first().unwrap();
            let wait_time = Duration::from_secs(60) - now.duration_since(*oldest);

            warn!(
                wait_seconds = wait_time.as_secs_f32(),
                "Rate limit reached, waiting"
            );

            tokio::time::sleep(wait_time).await;
        }

        self.request_timestamps.push(Instant::now());
        Ok(())
    }

    /// Descargar modelo desde repositorio remoto
    pub async fn download_model(
        &mut self,
        repo: &str,
        filename: &str,
        source: &ModelSource,
        expected_sha256: Option<&str>,
    ) -> Result<PathBuf, SyncError> {
        // Verificar cache primero
        let cache_path = self.get_cache_path(repo, filename);
        if cache_path.exists() {
            info!(
                cache_path = %cache_path.display(),
                "Model found in cache, verifying..."
            );

            if self.config.verify_checksums {
                let actual_hash =
                    self.compute_file_hash(&cache_path)
                        .map_err(|e| SyncError::Filesystem {
                            path: cache_path.to_string_lossy().to_string(),
                            msg: e.to_string(),
                        })?;

                if let Some(expected) = expected_sha256 {
                    if actual_hash != expected {
                        info!("Cache checksum mismatch, re-downloading");
                    } else {
                        info!("Cache valid, using cached model");
                        return Ok(cache_path);
                    }
                }
            } else {
                return Ok(cache_path);
            }
        }

        // Rate limit
        self.enforce_rate_limit().await?;

        let url = self.build_download_url(repo, filename, source);
        info!(url = %url, "Starting model download");

        // Crear directorio cache si no existe
        std::fs::create_dir_all(&self.config.cache_dir).map_err(|e| SyncError::Filesystem {
            path: self.config.cache_dir.to_string_lossy().to_string(),
            msg: e.to_string(),
        })?;

        // Descargar
        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|_e| SyncError::ModelNotFound {
                repo: repo.to_string(),
                file: filename.to_string(),
            })?;

        // Leer contenido
        let bytes = response.bytes().await.map_err(SyncError::Network)?; // CLEANUP: redundant closure

        // Escribir a disco
        std::fs::write(&cache_path, &bytes).map_err(|e| SyncError::Filesystem {
            path: cache_path.to_string_lossy().to_string(),
            msg: e.to_string(),
        })?;

        info!(
            cache_path = %cache_path.display(),
            size_bytes = bytes.len(),
            "Model downloaded successfully"
        );

        // Verificar checksum
        if self.config.verify_checksums {
            let actual_hash =
                self.compute_file_hash(&cache_path)
                    .map_err(|e| SyncError::Filesystem {
                        path: cache_path.to_string_lossy().to_string(),
                        msg: e.to_string(),
                    })?;

            if let Some(expected) = expected_sha256 {
                if actual_hash != expected {
                    return Err(SyncError::ChecksumMismatch {
                        expected: expected.to_string(),
                        got: actual_hash,
                    });
                }
            }
        }

        Ok(cache_path)
    }

    /// Obtener info de modelo remoto (sin descargar)
    pub async fn get_model_info(
        &mut self,
        repo: &str,
        filename: &str,
        source: &ModelSource,
    ) -> Result<RemoteModelInfo, SyncError> {
        self.enforce_rate_limit().await?;

        let url = self.build_download_url(repo, filename, source);

        // HEAD request para obtener metadata
        let response = self.client.head(&url).send().await?;
        let size_bytes = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(RemoteModelInfo {
            repo_id: repo.to_string(),
            filename: filename.to_string(),
            size_bytes,
            expected_sha256: None, // TODO: obtener de metadata del repo
            source: source.clone(),
            download_url: url,
            last_modified: None, // TODO: obtener de headers
        })
    }

    /// Calcular SHA-256 de un archivo
    fn compute_file_hash(&self, path: &Path) -> Result<String, std::io::Error> {
        let data = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Obtener path de cache para un modelo
    fn get_cache_path(&self, repo: &str, filename: &str) -> PathBuf {
        // Normalizar repo: "owner/repo" -> "owner_repo"
        let safe_repo = repo.replace('/', "_");
        self.config
            .cache_dir
            .join(format!("{}_{}", safe_repo, filename))
    }

    /// Listar modelos en cache
    pub fn list_cached_models(&self) -> Result<Vec<PathBuf>, SyncError> {
        let mut models = Vec::new();

        if !self.config.cache_dir.exists() {
            return Ok(models);
        }

        for entry in
            std::fs::read_dir(&self.config.cache_dir).map_err(|e| SyncError::Filesystem {
                path: self.config.cache_dir.to_string_lossy().to_string(),
                msg: e.to_string(),
            })?
        {
            let entry = entry.map_err(|e| SyncError::Filesystem {
                path: self.config.cache_dir.to_string_lossy().to_string(),
                msg: e.to_string(),
            })?;
            models.push(entry.path());
        }

        Ok(models)
    }

    /// Limpiar cache
    pub fn clear_cache(&self) -> Result<usize, SyncError> {
        if !self.config.cache_dir.exists() {
            return Ok(0);
        }

        let count = self.list_cached_models()?.len();
        std::fs::remove_dir_all(&self.config.cache_dir).map_err(|e| SyncError::Filesystem {
            path: self.config.cache_dir.to_string_lossy().to_string(),
            msg: e.to_string(),
        })?;

        info!(count, "Cache cleared");
        Ok(count)
    }

    /// Exportar dataset RLHF a formato Hugging Face
    ///
    /// Genera archivos JSONL compatibles con la librería `datasets` de HF.
    /// TODO: Phase 6 - Integrar con datasets library para push directo
    pub fn export_rlhf_dataset(
        &self,
        entries: &[serde_json::Value],
        output_path: &Path,
    ) -> Result<usize, SyncError> {
        std::fs::create_dir_all(output_path.parent().ok_or_else(|| SyncError::Filesystem {
            path: output_path.to_string_lossy().to_string(),
            msg: "No parent directory".to_string(),
        })?)
        .map_err(|e| SyncError::Filesystem {
            path: output_path.to_string_lossy().to_string(),
            msg: e.to_string(),
        })?;

        let mut file = std::fs::File::create(output_path).map_err(|e| SyncError::Filesystem {
            path: output_path.to_string_lossy().to_string(),
            msg: e.to_string(),
        })?;

        for entry in entries {
            let line = serde_json::to_string(entry)
                .map_err(|e| SyncError::Serialization(e.to_string()))?;
            writeln!(file, "{}", line).map_err(|e| SyncError::Filesystem {
                path: output_path.to_string_lossy().to_string(),
                msg: e.to_string(),
            })?;
        }

        info!(
            path = %output_path.display(),
            count = entries.len(),
            "RLHF dataset exported"
        );

        Ok(entries.len())
    }
}

impl Default for HfSyncManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.rate_limit_rpm, 30);
        assert!(config.verify_checksums);
    }

    #[test]
    fn test_build_download_url() {
        let manager = HfSyncManager::new();
        let url = manager.build_download_url(
            "Qwen-Scope/SAE-Res-Qwen3.5-27B",
            "model.safetensors",
            &ModelSource::HuggingFace,
        );
        assert!(url.contains("huggingface.co"));
        assert!(url.contains("Qwen-Scope/SAE-Res-Qwen3.5-27B"));
    }

    #[test]
    fn test_model_source_display() {
        assert_eq!(format!("{}", ModelSource::HuggingFace), "HuggingFace");
        assert_eq!(format!("{}", ModelSource::ModelScope), "ModelScope");
        assert_eq!(format!("{}", ModelSource::Local), "Local");
    }
}
