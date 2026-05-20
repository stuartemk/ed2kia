//! GGUF Loader — Parsing y validación de modelos base GGUF.
//!
//! **Stuartian Law 3 (Inteligencia Holística):** Eficiencia termodinámica.
//! Los modelos base se cargan una vez y se comparten inmutablemente.

use std::fmt;
use std::path::Path;

/// Error al cargar o validar un modelo GGUF.
#[derive(Debug)]
pub enum GgufLoaderError {
    /// Archivo no encontrado.
    FileNotFound(String),
    /// Formato GGUF inválido.
    InvalidFormat(String),
    /// Versión de GGUF no soportada.
    UnsupportedVersion(String),
    /// Error de I/O.
    Io(std::io::Error),
}

impl fmt::Display for GgufLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GgufLoaderError::FileNotFound(path) => write!(f, "GGUF file not found: {}", path),
            GgufLoaderError::InvalidFormat(msg) => write!(f, "Invalid GGUF format: {}", msg),
            GgufLoaderError::UnsupportedVersion(ver) => {
                write!(f, "Unsupported GGUF version: {}", ver)
            }
            GgufLoaderError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for GgufLoaderError {}

impl From<std::io::Error> for GgufLoaderError {
    fn from(err: std::io::Error) -> Self {
        GgufLoaderError::Io(err)
    }
}

/// Metadata de un modelo GGUF cargado.
#[derive(Debug, Clone)]
pub struct GgufModelInfo {
    /// Ruta del archivo GGUF.
    pub path: String,
    /// Versión del formato GGUF.
    pub version: u32,
    /// Arquitectura del modelo (e.g., "llama", "qwen2").
    pub architecture: String,
    /// Número de capas.
    pub num_layers: usize,
    /// Dimensión del embedding.
    pub embedding_dim: usize,
    /// Tamaño en bytes.
    pub size_bytes: u64,
}

/// Loader para modelos base GGUF.
///
/// Garantiza inmutabilidad del modelo base: una vez cargado,
/// el GGUF no se modifica. Solo se aplican adapters QLoRA encima.
pub struct GgufLoader;

impl GgufLoader {
    /// Crea un nuevo GgufLoader.
    pub fn new() -> Self {
        GgufLoader
    }

    /// Valida que el archivo existe y tiene formato GGUF válido.
    ///
    /// Retorna metadata del modelo si la validación pasa.
    pub fn validate<P: AsRef<Path>>(&self, _path: P) -> Result<GgufModelInfo, GgufLoaderError> {
        // TODO(Sprint16.1): Implement GGUF parsing with gguf crate.
        // For now, return scaffold structure.
        Err(GgufLoaderError::InvalidFormat(
            "GGUF parsing not yet implemented".into(),
        ))
    }

    /// Carga el modelo GGUF en memoria (read-only).
    ///
    /// **Stuartian Law 3:** Cero desperdicio. El modelo se mapea
    /// memory-mapped para evitar copias innecesarias.
    pub fn load<P: AsRef<Path>>(&self, _path: P) -> Result<GgufModelInfo, GgufLoaderError> {
        // TODO(Sprint16.1): Implement memory-mapped GGUF loading.
        Err(GgufLoaderError::InvalidFormat(
            "GGUF loading not yet implemented".into(),
        ))
    }
}

impl Default for GgufLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = GgufLoader::new();
        let _ = loader;
    }

    #[test]
    fn test_loader_default() {
        let _ = GgufLoader::default();
    }

    #[test]
    fn test_validate_nonexistent() {
        let loader = GgufLoader::new();
        match loader.validate("/nonexistent/model.gguf") {
            Err(GgufLoaderError::InvalidFormat(_)) => {} // Expected
            other => panic!("Expected InvalidFormat, got {:?}", other),
        }
    }

    #[test]
    fn test_error_display() {
        let err = GgufLoaderError::FileNotFound("test.gguf".into());
        assert!(!format!("{}", err).is_empty());
    }
}
