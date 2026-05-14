//! Model Registry - Registro local de versiones, checksums y rollback
//!
//! Mantiene un registro de todos los modelos SAE descargados o cargados
//! localmente, con versionado semántico, checksums SHA-256 y capacidad
//! de rollback a versiones anteriores.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
// CLEANUP: removed unused import Path
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn};
// CLEANUP: removed unused import error

/// Error del registro de modelos
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Model not found: {repo}/{version}")]
    ModelNotFound { repo: String, version: String },
    #[error("Version not found: {0}")]
    VersionNotFound(String),
    #[error("Checksum verification failed: {0}")]
    ChecksumFailed(String),
    #[error("Filesystem error: {path}: {msg}")]
    Filesystem { path: String, msg: String },
    #[error("Invalid semver: {0}")]
    InvalidSemver(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Active model must be downgraded before setting new one")]
    ActiveModelConflict,
}

/// Estado del modelo en el registro
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelStatus {
    /// Disponible y verificado
    Available,
    /// Modelo activo en uso
    Active,
    /// Descargado pero checksum falló
    Corrupted,
    /// Desinstalado (metadata retenida)
    Uninstalled,
}

impl std::fmt::Display for ModelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelStatus::Available => write!(f, "Available"),
            ModelStatus::Active => write!(f, "Active"),
            ModelStatus::Corrupted => write!(f, "Corrupted"),
            ModelStatus::Uninstalled => write!(f, "Uninstalled"),
        }
    }
}

/// Entrada del registro de modelos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Repositorio (ej: "Qwen-Scope/SAE-Res-Qwen3.5-27B")
    pub repo_id: String,
    /// Versión semántica
    pub version: String,
    /// Path local al archivo
    pub local_path: PathBuf,
    /// Checksum SHA-256
    pub sha256: String,
    /// Tamaño en bytes
    pub size_bytes: u64,
    /// Estado actual
    pub status: ModelStatus,
    /// Timestamp de registro (epoch seconds)
    pub registered_at: u64,
    /// Timestamp de última verificación
    pub last_verified_at: Option<u64>,
    /// Timestamp de última uso
    pub last_used_at: Option<u64>,
    /// Metadata adicional
    pub metadata: HashMap<String, String>,
}

impl ModelEntry {
    pub fn new(
        repo_id: String,
        version: String,
        local_path: PathBuf,
        sha256: String,
        size_bytes: u64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            repo_id,
            version,
            local_path,
            sha256,
            size_bytes,
            status: ModelStatus::Available,
            registered_at: now,
            last_verified_at: None,
            last_used_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Verificar checksum del archivo local
    pub fn verify_checksum(&self) -> Result<bool, RegistryError> {
        if !self.local_path.exists() {
            return Ok(false);
        }

        let data = std::fs::read(&self.local_path).map_err(|e| RegistryError::Filesystem {
            path: self.local_path.to_string_lossy().to_string(),
            msg: e.to_string(),
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let actual_hash = hex::encode(hasher.finalize());

        Ok(actual_hash == self.sha256)
    }

    /// Marcar como activo
    pub fn set_active(&mut self) {
        self.status = ModelStatus::Active;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_used_at = Some(now);
    }

    /// Marcar como disponible
    pub fn set_available(&mut self) {
        self.status = ModelStatus::Available;
    }
}

/// Registro de modelos
pub struct ModelRegistry {
    /// Entradas por repo_id
    entries: HashMap<String, Vec<ModelEntry>>,
    /// Modelo activo actual
    active_model: Option<String>, // repo_id
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            active_model: None,
        }
    }

    /// Registrar nuevo modelo
    pub fn register(&mut self, entry: ModelEntry) -> Result<(), RegistryError> {
        let repo_id = entry.repo_id.clone();
        let version = entry.version.clone();

        // Verificar si ya existe esta versión
        {
            let versions = self.entries.entry(repo_id.clone()).or_default(); // CLEANUP: or_insert_with -> or_default
            if versions.iter().any(|e| e.version == version) {
                warn!(repo_id, version, "Model version already registered, skipping");
                return Ok(());
            }
        }

        // Agregar entrada
        let versions = self.entries.get_mut(&repo_id).unwrap();
        versions.push(entry);

        // Ordenar por versión (más reciente primero)
        versions.sort_by(|a, b| b.version.cmp(&a.version));

        info!(
            repo_id,
            version,
            "Model registered in local registry"
        );

        Ok(())
    }

    /// Obtener modelo activo
    pub fn get_active(&self) -> Option<&ModelEntry> {
        let active_repo = self.active_model.as_ref()?;
        let versions = self.entries.get(active_repo)?;
        versions.iter().find(|e| e.status == ModelStatus::Active)
    }

    /// Establecer modelo activo por versión
    pub fn set_active(&mut self, repo_id: &str, version: &str) -> Result<(), RegistryError> {
        // Si hay un modelo activo, desactivarlo primero
        if let Some(ref active_repo) = self.active_model {
            if let Some(versions) = self.entries.get_mut(active_repo) {
                for entry in versions.iter_mut() {
                    if entry.status == ModelStatus::Active {
                        entry.set_available();
                        info!(
                            repo_id = %active_repo,
                            version = %entry.version,
                            "Model deactivated"
                        );
                    }
                }
            }
        }

        // Activar nuevo modelo
        let versions = self
            .entries
            .get_mut(repo_id)
            .ok_or_else(|| RegistryError::ModelNotFound {
                repo: repo_id.to_string(),
                version: version.to_string(),
            })?;

        let entry = versions
            .iter_mut()
            .find(|e| e.version == version)
            .ok_or_else(|| RegistryError::VersionNotFound(version.to_string()))?;

        entry.set_active();
        self.active_model = Some(repo_id.to_string());

        info!(
            repo_id,
            version,
            "Model set as active"
        );

        Ok(())
    }

    /// Rollback a versión anterior
    pub fn rollback(&mut self, repo_id: &str) -> Result<Option<String>, RegistryError> {
        let versions = self
            .entries
            .get(repo_id)
            .ok_or_else(|| RegistryError::ModelNotFound {
                repo: repo_id.to_string(),
                version: "unknown".to_string(),
            })?;

        // Encontrar versión activa
        let active_version = versions
            .iter()
            .find(|e| e.status == ModelStatus::Active)
            .map(|e| e.version.clone());

        // Encontrar versión anterior disponible
        let mut found_active = false;
        let mut previous_version = None;

        for entry in versions {
            if entry.status == ModelStatus::Active {
                found_active = true;
                continue;
            }
            if found_active && entry.status == ModelStatus::Available {
                previous_version = Some(entry.version.clone());
                break;
            }
        }

        let target_version = previous_version.ok_or_else(|| RegistryError::VersionNotFound(
            "No previous version available for rollback".to_string(),
        ))?;

        // Setear versión anterior como activa
        self.set_active(repo_id, &target_version)?;

        // FIX: borrow/move - Clone active_version before unwrap_or_else to avoid move | borrow/move
        let from_str = active_version.clone().unwrap_or_else(|| "none".to_string());
        info!(
            repo_id,
            from = from_str,
            to = target_version,
            "Rollback completed"
        );

        Ok(active_version)
    }

    /// Obtener todas las versiones de un repo
    pub fn get_versions(&self, repo_id: &str) -> Option<&[ModelEntry]> {
        self.entries.get(repo_id).map(|v| v.as_slice())
    }

    /// Obtener la versión más reciente de un repo
    pub fn get_latest(&self, repo_id: &str) -> Option<&ModelEntry> {
        self.entries.get(repo_id).and_then(|v| v.first())
    }

    /// Listar todos los repositorios registrados
    pub fn list_repos(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Verificar integridad de todos los modelos
    pub fn verify_all(&mut self) -> Result<Vec<(String, String, bool)>, RegistryError> {
        let mut results = Vec::new();

        for (repo_id, versions) in &self.entries {
            for entry in versions {
                let valid = entry.verify_checksum()?;
                results.push((
                    repo_id.clone(),
                    entry.version.clone(),
                    valid,
                ));

                if !valid && entry.status != ModelStatus::Corrupted {
                    warn!(
                        repo_id,
                        version = %entry.version,
                        "Checksum verification failed"
                    );
                }
            }
        }

        Ok(results)
    }

    /// Desinstalar modelo (retener metadata)
    pub fn uninstall(&mut self, repo_id: &str, version: &str) -> Result<(), RegistryError> {
        // No permitir desinstalar modelo activo
        if let Some(active) = self.get_active() {
            if active.repo_id == repo_id && active.version == version {
                return Err(RegistryError::ActiveModelConflict);
            }
        }

        let versions = self
            .entries
            .get_mut(repo_id)
            .ok_or_else(|| RegistryError::ModelNotFound {
                repo: repo_id.to_string(),
                version: version.to_string(),
            })?;

        let entry = versions
            .iter_mut()
            .find(|e| e.version == version)
            .ok_or_else(|| RegistryError::VersionNotFound(version.to_string()))?;

        // Eliminar archivo
        if entry.local_path.exists() {
            std::fs::remove_file(&entry.local_path).map_err(|e| RegistryError::Filesystem {
                path: entry.local_path.to_string_lossy().to_string(),
                msg: e.to_string(),
            })?;
        }

        entry.status = ModelStatus::Uninstalled;

        info!(
            repo_id,
            version,
            "Model uninstalled (metadata retained)"
        );

        Ok(())
    }

    /// Estadísticas del registro
    pub fn stats(&self) -> RegistryStats {
        let total_repos = self.entries.len();
        let total_versions: usize = self.entries.values().map(|v| v.len()).sum();
        let total_size: u64 = self
            .entries
            .values()
            .flat_map(|v| v.iter())
            .map(|e| e.size_bytes)
            .sum();

        let active_count = self
            .entries
            .values()
            .flat_map(|v| v.iter())
            .filter(|e| e.status == ModelStatus::Active)
            .count();

        RegistryStats {
            total_repos,
            total_versions,
            total_size_bytes: total_size,
            active_models: active_count,
        }
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Estadísticas del registro
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_repos: usize,
    pub total_versions: usize,
    pub total_size_bytes: u64,
    pub active_models: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ModelRegistry::new();
        assert_eq!(registry.list_repos().len(), 0);
    }

    #[test]
    fn test_model_registration() {
        let mut registry = ModelRegistry::new();
        let entry = ModelEntry::new(
            "test/model".to_string(),
            "1.0.0".to_string(),
            PathBuf::from("/tmp/test.safetensors"),
            "abc123".to_string(),
            1024,
        );

        registry.register(entry).unwrap();
        assert_eq!(registry.list_repos().len(), 1);
        assert_eq!(registry.get_versions("test/model").unwrap().len(), 1);
    }

    #[test]
    fn test_set_active_model() {
        let mut registry = ModelRegistry::new();
        registry.register(ModelEntry::new(
            "test/model".to_string(),
            "1.0.0".to_string(),
            PathBuf::from("/tmp/test.safetensors"),
            "abc123".to_string(),
            1024,
        ));

        registry.set_active("test/model", "1.0.0").unwrap();
        let active = registry.get_active().unwrap();
        assert_eq!(active.version, "1.0.0");
    }

    #[test]
    fn test_registry_stats() {
        let registry = ModelRegistry::new();
        let stats = registry.stats();
        assert_eq!(stats.total_repos, 0);
        assert_eq!(stats.total_versions, 0);
    }
}
