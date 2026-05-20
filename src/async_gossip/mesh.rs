//! Gossip Mesh — Configuración de GossipSub con tolerancia asíncrona.
//!
//! **Stuartian Law 5 (Múltiples Posibilidades):** Mesh dinámico sin maestros,
//! tolerante a particiones, con reconexión automática.

use std::fmt;

/// Error en la configuración o gestión del mesh GossipSub.
#[derive(Debug)]
pub enum GossipMeshError {
    /// Parámetro de mesh inválido.
    InvalidParameter(String),
    /// Error de conexión.
    ConnectionError(String),
    /// Topología inválida.
    InvalidTopology(String),
}

impl fmt::Display for GossipMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GossipMeshError::InvalidParameter(msg) => {
                write!(f, "Invalid mesh parameter: {}", msg)
            }
            GossipMeshError::ConnectionError(msg) => {
                write!(f, "Connection error: {}", msg)
            }
            GossipMeshError::InvalidTopology(msg) => {
                write!(f, "Invalid topology: {}", msg)
            }
        }
    }
}

impl std::error::Error for GossipMeshError {}

/// Configuración del mesh GossipSub.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Tamaño ideal del mesh (I).
    pub mesh_size: usize,
    /// Tamaño mínimo del mesh (I_min).
    pub mesh_min: usize,
    /// Tamaño máximo del mesh (I_max).
    pub mesh_max: usize,
    /// Grado de propagación (D).
    pub fanout: usize,
    /// Intervalo de heartbeat en milisegundos.
    pub heartbeat_interval_ms: u64,
}

impl MeshConfig {
    /// Crea configuración por defecto para ed2kIA.
    pub fn default_ed2kia() -> Self {
        Self {
            mesh_size: 6,
            mesh_min: 4,
            mesh_max: 12,
            fanout: 6,
            heartbeat_interval_ms: 700,
        }
    }

    /// Valida los parámetros del mesh.
    pub fn validate(&self) -> Result<(), GossipMeshError> {
        if self.mesh_min > self.mesh_size {
            return Err(GossipMeshError::InvalidParameter(
                "mesh_min cannot exceed mesh_size".into(),
            ));
        }
        if self.mesh_size > self.mesh_max {
            return Err(GossipMeshError::InvalidParameter(
                "mesh_size cannot exceed mesh_max".into(),
            ));
        }
        Ok(())
    }
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self::default_ed2kia()
    }
}

/// Mesh GossipSub con tolerancia asíncrona.
///
/// **Stuartian Law 5:** Sin puntos únicos de fallo.
/// La red se auto-organiza y tolera particiones.
pub struct GossipMesh {
    /// Configuración del mesh.
    pub config: MeshConfig,
}

impl GossipMesh {
    /// Crea un nuevo mesh con configuración especificada.
    pub fn new(config: MeshConfig) -> Result<Self, GossipMeshError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Inicializa el mesh con configuración por defecto.
    pub fn default_mesh() -> Self {
        Self {
            config: MeshConfig::default_ed2kia(),
        }
    }

    /// Añade un peer al mesh.
    pub fn add_peer(&mut self, _peer_id: String) {
        // TODO(Sprint16.4): Implement libp2p peer management.
    }

    /// Remueve un peer del mesh.
    pub fn remove_peer(&mut self, _peer_id: &str) {
        // TODO(Sprint16.4): Implement peer removal.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MeshConfig::default_ed2kia();
        assert_eq!(config.mesh_size, 6);
        assert_eq!(config.mesh_min, 4);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = MeshConfig::default_ed2kia();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_invalid() {
        let config = MeshConfig {
            mesh_size: 6,
            mesh_min: 8, // Invalid: min > size
            mesh_max: 12,
            fanout: 6,
            heartbeat_interval_ms: 700,
        };
        match config.validate() {
            Err(GossipMeshError::InvalidParameter(_)) => {} // Expected
            other => panic!("Expected InvalidParameter, got {:?}", other),
        }
    }

    #[test]
    fn test_mesh_creation() {
        let config = MeshConfig::default_ed2kia();
        let _ = GossipMesh::new(config).unwrap();
    }

    #[test]
    fn test_default_mesh() {
        let _ = GossipMesh::default_mesh();
    }

    #[test]
    fn test_error_display() {
        let err = GossipMeshError::ConnectionError("test".into());
        assert!(!format!("{}", err).is_empty());
    }
}
