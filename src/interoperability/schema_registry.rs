//! Schema Registry - Registro versionado de esquemas con compatibilidad backward
//!
//! Implementa `SchemaRegistry` para gestión de esquemas de interoperabilidad:
//! 1. Registro de esquemas con versiones semánticas
//! 2. Validación de compatibilidad backward/forward
//! 3. Checksums SHA-256 para integridad
//! 4. Matrices de compatibilidad entre versiones
//! 5. Deprecación controlada con migración
//!
//! **Feature:** `phase7-sprint2`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn};

// ============================================================================
// Errors
// ============================================================================

/// Error específico del Schema Registry
#[derive(Debug, Error)]
pub enum SchemaRegistryError {
    #[error("Schema not found: {version}")]
    SchemaNotFound { version: String },

    #[error("Schema already registered: {version}")]
    SchemaAlreadyRegistered { version: String },

    #[error("Backward compatibility broken: {source_version} -> {target_version}")]
    BackwardCompatibilityBroken { source_version: String, target_version: String },

    #[error("Invalid semantic version: {version}")]
    InvalidSemanticVersion { version: String },

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Schema deprecated: {version} (use {migration} instead)")]
    SchemaDeprecated { version: String, migration: String },

    #[error("Breaking change without --allow-breaking flag")]
    BreakingChangeNotAllowed,
}

// ============================================================================
// Schema Definition
// ============================================================================

/// Definición de esquema registrado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    /// Versión semántica (ej: "7.1.0")
    pub version: String,
    /// Nombre del esquema (ej: "qwen-scope", "llama-3")
    pub name: String,
    /// Checksum SHA-256 del esquema (hex)
    pub checksum: String,
    /// Dimensiones del tensor esperado
    pub dimensions: Vec<usize>,
    /// Tipo de dato (ej: "f32", "f16", "i8")
    pub dtype: String,
    /// Versión anterior compatible (backward)
    pub backward_compatible_with: Option<String>,
    /// Versiones futuras compatibles (forward)
    pub forward_compatible_with: Vec<String>,
    /// Si el esquema está deprecado
    pub deprecated: bool,
    /// Versión de migración (si deprecado)
    pub migration_target: Option<String>,
    /// Timestamp de registro (epoch ms)
    pub registered_at_ms: u64,
    /// Metadata adicional
    pub metadata: HashMap<String, String>,
}

impl SchemaDefinition {
    /// Crea nueva definición de esquema
    pub fn new(version: String, name: String, dimensions: Vec<usize>, dtype: String) -> Self {
        let checksum = Self::compute_checksum(&version, &name, &dimensions, &dtype);
        Self {
            version,
            name,
            checksum,
            dimensions,
            dtype,
            backward_compatible_with: None,
            forward_compatible_with: Vec::new(),
            deprecated: false,
            migration_target: None,
            registered_at_ms: current_timestamp_ms(),
            metadata: HashMap::new(),
        }
    }

    /// Calcula checksum SHA-256 del esquema
    fn compute_checksum(version: &str, name: &str, dimensions: &[usize], dtype: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(version.as_bytes());
        hasher.update(name.as_bytes());
        hasher.update(dimensions.iter().flat_map(|d| d.to_le_bytes()).collect::<Vec<u8>>());
        hasher.update(dtype.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verifica checksum
    pub fn verify_checksum(&self) -> bool {
        let expected = Self::compute_checksum(&self.version, &self.name, &self.dimensions, &self.dtype);
        self.checksum == expected
    }

    /// Agrega metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

// ============================================================================
// Schema Result
// ============================================================================

/// Resultado de validación de esquema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaResult {
    /// Versión del esquema
    pub version: String,
    /// Si es compatible
    pub compatible: bool,
    /// Ruta de migración (si aplica)
    pub migration_path: Option<Vec<String>>,
    /// Mensaje de error (si no compatible)
    pub error_message: Option<String>,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

// ============================================================================
// Compatibility Matrix
// ============================================================================

/// Matriz de compatibilidad entre versiones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityMatrix {
    /// Versión fuente
    pub source: String,
    /// Versión destino
    pub target: String,
    /// Tipo de compatibilidad
    pub compatibility: CompatibilityType,
    /// Requiere migración
    pub requires_migration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompatibilityType {
    /// Fully backward compatible
    Backward,
    /// Fully forward compatible
    Forward,
    /// Fully compatible (both directions)
    Full,
    /// No compatibility (breaking change)
    None,
}

impl std::fmt::Display for CompatibilityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompatibilityType::Backward => write!(f, "backward"),
            CompatibilityType::Forward => write!(f, "forward"),
            CompatibilityType::Full => write!(f, "full"),
            CompatibilityType::None => write!(f, "none"),
        }
    }
}

// ============================================================================
// Schema Registry Config
// ============================================================================

/// Configuración del Schema Registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryConfig {
    /// Permitir cambios breaking (requiere flag explícito)
    pub allow_breaking: bool,
    /// Máximo de versiones registradas
    pub max_schemas: usize,
    /// Retención de esquemas deprecados (días)
    pub deprecated_retention_days: u64,
}

impl Default for SchemaRegistryConfig {
    fn default() -> Self {
        Self {
            allow_breaking: false,
            max_schemas: 100,
            deprecated_retention_days: 90,
        }
    }
}

// ============================================================================
// Schema Registry
// ============================================================================

/// Registro versionado de esquemas
pub struct SchemaRegistry {
    /// Configuración
    config: SchemaRegistryConfig,
    /// Esquemas registrados por versión
    schemas: HashMap<String, SchemaDefinition>,
    /// Matriz de compatibilidad
    compatibility_matrix: Vec<CompatibilityMatrix>,
    /// Esquema actual (default)
    current_version: Option<String>,
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaRegistry {
    /// Crea nuevo SchemaRegistry con configuración por defecto
    pub fn new() -> Self {
        Self::with_config(SchemaRegistryConfig::default())
    }

    /// Crea nuevo SchemaRegistry con configuración personalizada
    pub fn with_config(config: SchemaRegistryConfig) -> Self {
        Self {
            config,
            schemas: HashMap::new(),
            compatibility_matrix: Vec::new(),
            current_version: None,
        }
    }

    /// Registra un nuevo esquema
    ///
    /// Valida versión semántica, verifica compatibilidad backward,
    /// y rechaza cambios breaking sin flag `--allow-breaking`.
    ///
    /// # Arguments
    ///
    /// * `definition` - Definición del esquema
    ///
    /// # Returns
    ///
    /// `Ok(())` si el esquema fue registrado exitosamente
    pub fn register(&mut self, mut definition: SchemaDefinition) -> Result<(), SchemaRegistryError> {
        // Validar versión semántica
        Self::validate_semver(&definition.version)?;

        // Verificar duplicado
        if self.schemas.contains_key(&definition.version) {
            return Err(SchemaRegistryError::SchemaAlreadyRegistered {
                version: definition.version.clone(),
            });
        }

        // Verificar checksum
        if !definition.verify_checksum() {
            return Err(SchemaRegistryError::ChecksumMismatch {
                expected: definition.checksum.clone(),
                actual: "invalid".to_string(),
            });
        }

        // Verificar compatibilidad backward si existe versión anterior
        if let Some(prev_version) = &self.current_version {
            let prev_schema = self.schemas.get(prev_version)
                .ok_or_else(|| SchemaRegistryError::SchemaNotFound {
                    version: prev_version.clone(),
                })?;

            let compatible = self.check_backward_compatibility(prev_schema, &definition);

            if !compatible && !self.config.allow_breaking {
                return Err(SchemaRegistryError::BreakingChangeNotAllowed);
            }

            if compatible {
                definition.backward_compatible_with = Some(prev_version.clone());
            }

            // Agregar a matriz de compatibilidad
            self.compatibility_matrix.push(CompatibilityMatrix {
                source: prev_version.clone(),
                target: definition.version.clone(),
                compatibility: if compatible {
                    CompatibilityType::Backward
                } else {
                    CompatibilityType::None
                },
                requires_migration: !compatible,
            });
        }

        self.schemas.insert(definition.version.clone(), definition);
        // Update current version to newest
        self.current_version = Some(self.schemas.keys().next().cloned().unwrap_or_default());

        info!(version = %self.current_version.clone().unwrap_or_default(), "Schema registered");
        Ok(())
    }

    /// Valida un esquema contra el registro
    ///
    /// Verifica existencia, compatibilidad, y estado de deprecación.
    ///
    /// # Arguments
    ///
    /// * `version` - Versión del esquema a validar
    /// * `target_version` - Versión destino (para compatibilidad)
    ///
    /// # Returns
    ///
    /// `Ok(SchemaResult)` con el resultado de validación
    pub fn validate(
        &self,
        version: &str,
        target_version: Option<&str>,
    ) -> Result<SchemaResult, SchemaRegistryError> {
        // Verificar existencia
        let schema = self.schemas.get(version)
            .ok_or_else(|| SchemaRegistryError::SchemaNotFound {
                version: version.to_string(),
            })?;

        // Verificar deprecación
        if schema.deprecated {
            return Ok(SchemaResult {
                version: version.to_string(),
                compatible: false,
                migration_path: schema.migration_target.as_ref().map(|t| vec![t.clone()]),
                error_message: Some(format!("Schema deprecated, migrate to {}", schema.migration_target.as_ref().unwrap())),
                timestamp_ms: current_timestamp_ms(),
            });
        }

        // Verificar compatibilidad con target
        if let Some(target) = target_version {
            let compatible = self.is_compatible(version, target);
            if !compatible {
                return Ok(SchemaResult {
                    version: version.to_string(),
                    compatible: false,
                    migration_path: None,
                    error_message: Some(format!("Incompatible with {}", target)),
                    timestamp_ms: current_timestamp_ms(),
                });
            }
        }

        Ok(SchemaResult {
            version: version.to_string(),
            compatible: true,
            migration_path: None,
            error_message: None,
            timestamp_ms: current_timestamp_ms(),
        })
    }

    /// Obtiene esquemas compatibles con una versión
    ///
    /// # Arguments
    ///
    /// * `version` - Versión de referencia
    ///
    /// # Returns
    ///
    /// `Vec<String>` con las versiones compatibles
    pub fn get_compatible(&self, version: &str) -> Vec<String> {
        let mut compatible = Vec::new();

        if let Some(schema) = self.schemas.get(version) {
            // Backward compatible
            if let Some(ref backward) = schema.backward_compatible_with {
                compatible.push(backward.clone());
            }

            // Forward compatible
            for forward in &schema.forward_compatible_with {
                compatible.push(forward.clone());
            }
        }

        compatible
    }

    /// Deprecar un esquema
    ///
    /// Marca el esquema como deprecado y establece target de migración.
    ///
    /// # Arguments
    ///
    /// * `version` - Versión a deprecuar
    /// * `migration_target` - Versión de migración recomendada
    ///
    /// # Returns
    ///
    /// `Ok(())` si la deprecación fue exitosa
    pub fn deprecate(
        &mut self,
        version: &str,
        migration_target: &str,
    ) -> Result<(), SchemaRegistryError> {
        let schema = self.schemas.get_mut(version)
            .ok_or_else(|| SchemaRegistryError::SchemaNotFound {
                version: version.to_string(),
            })?;

        schema.deprecated = true;
        schema.migration_target = Some(migration_target.to_string());

        warn!(version = %version, migration = %migration_target, "Schema deprecated");
        Ok(())
    }

    /// Obtiene un esquema por versión
    pub fn get_schema(&self, version: &str) -> Option<&SchemaDefinition> {
        self.schemas.get(version)
    }

    /// Obtiene la versión actual (default)
    pub fn current_version(&self) -> Option<&str> {
        self.current_version.as_deref()
    }

    /// Establece la versión actual
    pub fn set_current_version(&mut self, version: &str) -> Result<(), SchemaRegistryError> {
        if !self.schemas.contains_key(version) {
            return Err(SchemaRegistryError::SchemaNotFound {
                version: version.to_string(),
            });
        }
        self.current_version = Some(version.to_string());
        Ok(())
    }

    /// Obtiene todos los esquemas registrados
    pub fn get_all_schemas(&self) -> Vec<&SchemaDefinition> {
        self.schemas.values().collect()
    }

    /// Obtiene la matriz de compatibilidad
    pub fn get_compatibility_matrix(&self) -> &[CompatibilityMatrix] {
        &self.compatibility_matrix
    }

    /// Obtiene la configuración actual
    pub fn config(&self) -> &SchemaRegistryConfig {
        &self.config
    }

    /// Obtiene estadísticas
    pub fn stats(&self) -> SchemaStats {
        let total = self.schemas.len();
        let deprecated = self.schemas.values().filter(|s| s.deprecated).count();
        let active = total - deprecated;

        SchemaStats {
            total_schemas: total,
            active_schemas: active,
            deprecated_schemas: deprecated,
            compatibility_entries: self.compatibility_matrix.len(),
        }
    }

    /// Limpia esquemas deprecados antiguos
    pub fn cleanup_old_deprecated(&mut self, current_time_ms: u64) -> usize {
        let retention_ms = self.config.deprecated_retention_days * 24 * 60 * 60 * 1000;
        let mut removed = 0;

        self.schemas.retain(|_, schema| {
            if schema.deprecated {
                let age = current_time_ms.saturating_sub(schema.registered_at_ms);
                if age > retention_ms {
                    removed += 1;
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });

        if removed > 0 {
            info!(removed, "Cleaned up old deprecated schemas");
        }
        removed
    }

    // ------------------------------------------------------------------------
    // Private Helpers
    // ------------------------------------------------------------------------

    /// Valida formato de versión semántica
    fn validate_semver(version: &str) -> Result<(), SchemaRegistryError> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(SchemaRegistryError::InvalidSemanticVersion {
                version: version.to_string(),
            });
        }

        for part in parts {
            if part.parse::<u32>().is_err() {
                return Err(SchemaRegistryError::InvalidSemanticVersion {
                    version: version.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Verifica compatibilidad backward entre esquemas
    fn check_backward_compatibility(&self, old: &SchemaDefinition, new: &SchemaDefinition) -> bool {
        // Same name required
        if old.name != new.name {
            return false;
        }

        // Dimensions must be compatible (same or expanded)
        if new.dimensions.len() != old.dimensions.len() {
            return false;
        }

        for (old_dim, new_dim) in old.dimensions.iter().zip(new.dimensions.iter()) {
            if *new_dim < *old_dim {
                return false; // Shrinking dimension = breaking
            }
        }

        // dtype must be compatible
        Self::are_dtypes_compatible(&old.dtype, &new.dtype)
    }

    /// Verifica compatibilidad entre tipos de dato
    fn are_dtypes_compatible(old: &str, new: &str) -> bool {
        // Same type = compatible
        if old == new {
            return true;
        }

        // f32 → f16 = compatible (precision loss but structurally OK)
        if old == "f32" && new == "f16" {
            return true;
        }

        // f16 → f32 = compatible (precision gain)
        if old == "f16" && new == "f32" {
            return true;
        }

        false
    }

    /// Verifica compatibilidad entre dos versiones
    fn is_compatible(&self, source: &str, target: &str) -> bool {
        // Same version = compatible
        if source == target {
            return true;
        }

        // Check compatibility matrix
        for entry in &self.compatibility_matrix {
            if (entry.source == source && entry.target == target)
                || (entry.source == target && entry.target == source)
            {
                return entry.compatibility != CompatibilityType::None;
            }
        }

        // Check schema definitions
        if let Some(schema) = self.schemas.get(source) {
            if schema.forward_compatible_with.contains(&target.to_string()) {
                return true;
            }
            if schema.backward_compatible_with.as_deref() == Some(target) {
                return true;
            }
        }

        false
    }
}

/// Obtiene timestamp actual en milisegundos epoch
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// Schema Stats
// ============================================================================

/// Estadísticas del Schema Registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaStats {
    /// Total de esquemas registrados
    pub total_schemas: usize,
    /// Esquemas activos
    pub active_schemas: usize,
    /// Esquemas deprecados
    pub deprecated_schemas: usize,
    /// Entradas en matriz de compatibilidad
    pub compatibility_entries: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_schema(version: &str, name: &str) -> SchemaDefinition {
        SchemaDefinition::new(version.to_string(), name.to_string(), vec![256], "f32".to_string())
    }

    #[test]
    fn test_registry_creation() {
        let registry = SchemaRegistry::new();
        assert_eq!(registry.stats().total_schemas, 0);
    }

    #[test]
    fn test_register_schema() {
        let mut registry = SchemaRegistry::new();
        let schema = make_schema("1.0.0", "qwen-scope");
        assert!(registry.register(schema).is_ok());
        assert_eq!(registry.stats().total_schemas, 1);
    }

    #[test]
    fn test_register_duplicate() {
        let mut registry = SchemaRegistry::new();
        let schema = make_schema("1.0.0", "qwen-scope");
        registry.register(schema).unwrap();

        let schema2 = make_schema("1.0.0", "qwen-scope");
        let result = registry.register(schema2);
        assert!(matches!(result, Err(SchemaRegistryError::SchemaAlreadyRegistered { .. })));
    }

    #[test]
    fn test_invalid_semver() {
        let mut registry = SchemaRegistry::new();
        let mut schema = make_schema("1.0", "qwen-scope"); // Missing patch
        schema.checksum = SchemaDefinition::compute_checksum("1.0", "qwen-scope", &[256], "f32");
        let result = registry.register(schema);
        assert!(matches!(result, Err(SchemaRegistryError::InvalidSemanticVersion { .. })));
    }

    #[test]
    fn test_validate_compatible() {
        let mut registry = SchemaRegistry::new();
        registry.register(make_schema("1.0.0", "qwen-scope")).unwrap();

        let result = registry.validate("1.0.0", None);
        assert!(result.is_ok());
        assert!(result.unwrap().compatible);
    }

    #[test]
    fn test_validate_not_found() {
        let registry = SchemaRegistry::new();
        let result = registry.validate("9.9.9", None);
        assert!(matches!(result, Err(SchemaRegistryError::SchemaNotFound { .. })));
    }

    #[test]
    fn test_deprecate_schema() {
        let mut registry = SchemaRegistry::new();
        registry.register(make_schema("1.0.0", "qwen-scope")).unwrap();
        registry.register(make_schema("2.0.0", "qwen-scope")).unwrap();

        assert!(registry.deprecate("1.0.0", "2.0.0").is_ok());

        let schema = registry.get_schema("1.0.0").unwrap();
        assert!(schema.deprecated);
        assert_eq!(schema.migration_target.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn test_validate_deprecated_schema() {
        let mut registry = SchemaRegistry::new();
        registry.register(make_schema("1.0.0", "qwen-scope")).unwrap();
        registry.register(make_schema("2.0.0", "qwen-scope")).unwrap();
        registry.deprecate("1.0.0", "2.0.0").unwrap();

        let result = registry.validate("1.0.0", None).unwrap();
        assert!(!result.compatible);
        assert!(result.migration_path.is_some());
    }

    #[test]
    fn test_get_compatible() {
        let mut registry = SchemaRegistry::new();
        registry.register(make_schema("1.0.0", "qwen-scope")).unwrap();
        registry.register(make_schema("2.0.0", "qwen-scope")).unwrap();

        let compatible = registry.get_compatible("2.0.0");
        assert!(!compatible.is_empty()); // Should contain 1.0.0
    }

    #[test]
    fn test_stats() {
        let mut registry = SchemaRegistry::new();
        registry.register(make_schema("1.0.0", "qwen-scope")).unwrap();
        registry.register(make_schema("2.0.0", "qwen-scope")).unwrap();
        registry.deprecate("1.0.0", "2.0.0").unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total_schemas, 2);
        assert_eq!(stats.active_schemas, 1);
        assert_eq!(stats.deprecated_schemas, 1);
    }

    #[test]
    fn test_checksum_verification() {
        let schema = make_schema("1.0.0", "qwen-scope");
        assert!(schema.verify_checksum());
    }

    #[test]
    fn test_dtype_compatibility() {
        assert!(SchemaRegistry::are_dtypes_compatible("f32", "f32"));
        assert!(SchemaRegistry::are_dtypes_compatible("f32", "f16"));
        assert!(SchemaRegistry::are_dtypes_compatible("f16", "f32"));
        assert!(!SchemaRegistry::are_dtypes_compatible("f32", "i8"));
    }

    #[test]
    fn test_backward_compatibility_check() {
        let registry = SchemaRegistry::new();
        let old = make_schema("1.0.0", "qwen-scope");
        let new = SchemaDefinition::new("2.0.0".to_string(), "qwen-scope".to_string(), vec![512], "f32".to_string());
        assert!(registry.check_backward_compatibility(&old, &new)); // Expanded dim = OK
    }

    #[test]
    fn test_backward_compatibility_breaking() {
        let registry = SchemaRegistry::new();
        let old = make_schema("1.0.0", "qwen-scope");
        let new = SchemaDefinition::new("2.0.0".to_string(), "qwen-scope".to_string(), vec![128], "f32".to_string());
        assert!(!registry.check_backward_compatibility(&old, &new)); // Shrinking dim = breaking
    }

    #[test]
    fn test_config_default() {
        let config = SchemaRegistryConfig::default();
        assert!(!config.allow_breaking);
        assert_eq!(config.max_schemas, 100);
        assert_eq!(config.deprecated_retention_days, 90);
    }

    #[test]
    fn test_schema_result_structure() {
        let result = SchemaResult {
            version: "1.0.0".to_string(),
            compatible: true,
            migration_path: None,
            error_message: None,
            timestamp_ms: 1000,
        };
        assert!(result.compatible);
        assert!(result.migration_path.is_none());
    }

    #[test]
    fn test_compatibility_type_display() {
        assert_eq!(format!("{}", CompatibilityType::Backward), "backward");
        assert_eq!(format!("{}", CompatibilityType::Forward), "forward");
        assert_eq!(format!("{}", CompatibilityType::Full), "full");
        assert_eq!(format!("{}", CompatibilityType::None), "none");
    }

    #[test]
    fn test_set_current_version() {
        let mut registry = SchemaRegistry::new();
        registry.register(make_schema("1.0.0", "qwen-scope")).unwrap();
        assert!(registry.set_current_version("1.0.0").is_ok());
        assert_eq!(registry.current_version(), Some("1.0.0"));
    }

    #[test]
    fn test_metadata() {
        let schema = SchemaDefinition::new("1.0.0".to_string(), "test".to_string(), vec![256], "f32".to_string())
            .with_metadata("author".to_string(), "ed2kIA".to_string());
        assert_eq!(schema.metadata.get("author").unwrap(), "ed2kIA");
    }
}
