//! GGUF Loader â€” Parsing y validaciÃ³n de modelos base GGUF.
//!
//! **Topological Law 3 (Inteligencia HolÃ­stica):** Eficiencia termodinÃ¡mica.
//! Los modelos base se cargan una vez y se comparten inmutablemente.

use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use sha2::{Digest, Sha256};

#[cfg(feature = "v2.1-qlora-gguf")]
use memmap2::Mmap;

/// Error al cargar o validar un modelo GGUF.
#[derive(Debug)]
pub enum GgufLoaderError {
    /// Archivo no encontrado.
    FileNotFound(String),
    /// Formato GGUF invÃ¡lido.
    InvalidFormat(String),
    /// VersiÃ³n de GGUF no soportada.
    UnsupportedVersion(String),
    /// Checksum SHA256 no coincide.
    ChecksumMismatch(String),
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
            GgufLoaderError::ChecksumMismatch(msg) => {
                write!(f, "SHA256 checksum mismatch: {}", msg)
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
    /// VersiÃ³n del formato GGUF.
    pub version: u32,
    /// Arquitectura del modelo (e.g., "llama", "qwen2").
    pub architecture: String,
    /// NÃºmero de capas.
    pub num_layers: usize,
    /// DimensiÃ³n del embedding (d_model).
    pub embedding_dim: usize,
    /// TamaÃ±o en bytes.
    pub size_bytes: u64,
    /// Checksum SHA256 del archivo.
    pub sha256: String,
}

/// Modelo base GGUF inmutable con memoria mapeada.
///
/// **Topological Law 3:** Cero desperdicio. El modelo se mapea
/// memory-mapped para evitar copias innecesarias.
#[cfg(feature = "v2.1-qlora-gguf")]
pub struct GgufBaseModel {
    /// Metadata del modelo.
    pub info: GgufModelInfo,
    /// Memory-mapped file (read-only).
    pub mmap: Mmap,
}

#[cfg(feature = "v2.1-qlora-gguf")]
impl fmt::Debug for GgufBaseModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GgufBaseModel")
            .field("info", &self.info)
            .field("mmap_len", &self.mmap.len())
            .finish()
    }
}

/// Loader para modelos base GGUF.
///
/// Garantiza inmutabilidad del modelo base: una vez cargado,
/// el GGUF no se modifica. Solo se aplican adapters QLoRA encima.
pub struct GgufLoader {
    /// Checksum SHA256 esperado (opcional). Si se proporciona,
    /// se valida contra el archivo cargado.
    expected_sha256: Option<String>,
}

impl GgufLoader {
    /// Crea un nuevo GgufLoader sin validaciÃ³n de checksum.
    pub fn new() -> Self {
        GgufLoader {
            expected_sha256: None,
        }
    }

    /// Crea un GgufLoader con validaciÃ³n de checksum SHA256.
    ///
    /// # Arguments
    /// * `sha256_hex` - Checksum SHA256 esperado en formato hex (64 chars).
    pub fn with_sha256(mut self, sha256_hex: String) -> Self {
        self.expected_sha256 = Some(sha256_hex);
        self
    }

    /// Calcula el checksum SHA256 de un archivo.
    fn compute_sha256<P: AsRef<Path>>(path: P) -> Result<String, GgufLoaderError> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Valida el magic number GGUF en los primeros 4 bytes.
    ///
    /// GGUF files start with the magic bytes "GGUF" (0x47475546).
    fn validate_gguf_magic<P: AsRef<Path>>(path: P) -> Result<(), GgufLoaderError> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;

        if &magic != b"GGUF" {
            return Err(GgufLoaderError::InvalidFormat(
                "File does not start with GGUF magic bytes".into(),
            ));
        }

        Ok(())
    }

    /// Lee la versiÃ³n GGUF (4 bytes little-endian despuÃ©s del magic).
    fn read_gguf_version<P: AsRef<Path>>(path: P) -> Result<u32, GgufLoaderError> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(4))?; // Skip magic
        let mut version_bytes = [0u8; 4];
        file.read_exact(&mut version_bytes)?;
        Ok(u32::from_le_bytes(version_bytes))
    }

    /// Valida que el archivo existe y tiene formato GGUF vÃ¡lido.
    ///
    /// Retorna metadata del modelo si la validaciÃ³n pasa.
    pub fn validate<P: AsRef<Path>>(&self, path: P) -> Result<GgufModelInfo, GgufLoaderError> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(GgufLoaderError::FileNotFound(
                path_ref.display().to_string(),
            ));
        }

        // Validate GGUF magic
        Self::validate_gguf_magic(path_ref)?;

        // Read version
        let version = Self::read_gguf_version(path_ref)?;
        if version > 3 {
            return Err(GgufLoaderError::UnsupportedVersion(format!(
                "GGUF version {} (max supported: 3)",
                version
            )));
        }

        // Compute SHA256
        let sha256 = Self::compute_sha256(path_ref)?;

        // Validate expected checksum if provided
        if let Some(ref expected) = self.expected_sha256 {
            if &sha256 != expected {
                return Err(GgufLoaderError::ChecksumMismatch(format!(
                    "Expected {}, got {}",
                    expected, sha256
                )));
            }
        }

        // Get file size
        let size_bytes = path_ref.metadata()?.len();

        // Extract architecture from filename or metadata
        // (Full GGUF parsing would read the header fields)
        let architecture = Self::extract_architecture(path_ref)?;
        let (num_layers, embedding_dim) = Self::extract_dimensions(path_ref)?;

        Ok(GgufModelInfo {
            path: path_ref.display().to_string(),
            version,
            architecture,
            num_layers,
            embedding_dim,
            size_bytes,
            sha256,
        })
    }

    /// Extrae la arquitectura del modelo desde el header GGUF.
    fn extract_architecture<P: AsRef<Path>>(_path: P) -> Result<String, GgufLoaderError> {
        // TODO(Sprint16.2): Parse GGUF header fields for architecture.
        // For now, return a placeholder that indicates the model needs full parsing.
        Ok("unknown".into())
    }

    /// Extrae las dimensiones del modelo (n_layers, d_model).
    fn extract_dimensions<P: AsRef<Path>>(_path: P) -> Result<(usize, usize), GgufLoaderError> {
        // TODO(Sprint16.2): Parse GGUF tensor metadata for dimensions.
        // For now, return placeholder values.
        Ok((0, 0))
    }

    /// Carga el modelo GGUF en memoria (read-only, memory-mapped).
    ///
    /// **Topological Law 3:** Cero desperdicio. El modelo se mapea
    /// memory-mapped para evitar copias innecesarias.
    #[cfg(feature = "v2.1-qlora-gguf")]
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<GgufBaseModel, GgufLoaderError> {
        let info = self.validate(path.as_ref())?;

        // Memory-map the file for read-only access
        let file = File::open(&info.path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(GgufBaseModel { info, mmap })
    }

    /// Carga el modelo GGUF en memoria (read-only).
    ///
    /// VersiÃ³n sin feature gate para testing bÃ¡sico.
    #[cfg(not(feature = "v2.1-qlora-gguf"))]
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<GgufModelInfo, GgufLoaderError> {
        self.validate(path)
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
    fn test_loader_with_sha256() {
        let loader = GgufLoader::new().with_sha256("abc123".into());
        assert_eq!(loader.expected_sha256, Some("abc123".into()));
    }

    #[test]
    fn test_loader_default() {
        let _ = GgufLoader::default();
    }

    #[test]
    fn test_validate_nonexistent() {
        let loader = GgufLoader::new();
        match loader.validate("/nonexistent/model.gguf") {
            Err(GgufLoaderError::FileNotFound(_)) => {} // Expected
            other => panic!("Expected FileNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_invalid_magic() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(tmp.path(), b"NOT_GGUF_DATA").expect("write");

        let loader = GgufLoader::new();
        match loader.validate(tmp.path()) {
            Err(GgufLoaderError::InvalidFormat(_)) => {} // Expected
            other => panic!("Expected InvalidFormat, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_sha256() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(tmp.path(), b"test data").expect("write");

        let sha256 = GgufLoader::compute_sha256(tmp.path()).expect("sha256");
        assert_eq!(sha256.len(), 64); // SHA256 hex is always 64 chars
    }

    #[test]
    fn test_error_display() {
        let err = GgufLoaderError::FileNotFound("test.gguf".into());
        assert!(!format!("{}", err).is_empty());

        let err = GgufLoaderError::ChecksumMismatch("expected != got".into());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_model_info_creation() {
        let info = GgufModelInfo {
            path: "/path/to/model.gguf".into(),
            version: 2,
            architecture: "llama".into(),
            num_layers: 32,
            embedding_dim: 4096,
            size_bytes: 1_000_000_000,
            sha256: "abc123".into(),
        };
        assert_eq!(info.version, 2);
        assert_eq!(info.num_layers, 32);
        assert_eq!(info.embedding_dim, 4096);
    }
}
