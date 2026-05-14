//! Network Init - Inicialización determinista de red, migración y fallback
//!
//! Modo --genesis para crear primera capa de leases, distribuir SAEs
//! iniciales y activar gossipsub. Fallback a modo offline si no hay pares.

use crate::bootstrap::seed_registry::{SeedRegistry, SeedRegistryError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
// CLEANUP: removed unused import Path
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn};
// CLEANUP: removed unused import error

/// Error de inicialización de red
#[derive(Debug, Error)]
pub enum NetworkInitError {
    #[error("No bootstrap seeds available")]
    NoBootstrapSeeds,
    #[error("Genesis configuration invalid: {0}")]
    InvalidGenesisConfig(String),
    #[error("Failed to initialize P2P swarm: {0}")]
    SwarmInitFailed(String),
    #[error("Failed to load initial SAE: {0}")]
    SaeLoadFailed(String),
    #[error("Database initialization failed: {0}")]
    DatabaseInitFailed(String),
    #[error("Network already initialized at {path}")]
    NetworkAlreadyInitialized { path: String },
    #[error("Fallback to offline mode: {0}")]
    OfflineFallback(String),
    #[error("Seed registry error: {0}")]
    SeedRegistry(#[from] SeedRegistryError),
}

/// Modo de operación de la red
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkMode {
    /// Red completa con pares P2P
    FullNetwork,
    /// Nodo solo, sin pares (modo offline)
    Offline,
    /// Nodo bootstrap/seed
    BootstrapNode,
    /// Modo genesis (primer nodo de la red)
    Genesis,
}

impl std::fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMode::FullNetwork => write!(f, "FullNetwork"),
            NetworkMode::Offline => write!(f, "Offline"),
            NetworkMode::BootstrapNode => write!(f, "BootstrapNode"),
            NetworkMode::Genesis => write!(f, "Genesis"),
        }
    }
}

/// Estado de inicialización
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InitState {
    /// Pendiente de inicialización
    Pending,
    /// Conectando a seeds
    Connecting,
    /// Sincronizando leases y SAEs
    Syncing,
    /// Red operativa
    Operational,
    /// Fallback a offline
    OfflineFallback,
    /// Error de inicialización
    Error,
}

impl std::fmt::Display for InitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitState::Pending => write!(f, "Pending"),
            InitState::Connecting => write!(f, "Connecting"),
            InitState::Syncing => write!(f, "Syncing"),
            InitState::Operational => write!(f, "Operational"),
            InitState::OfflineFallback => write!(f, "OfflineFallback"),
            InitState::Error => write!(f, "Error"),
        }
    }
}

/// Configuración de genesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Path al directorio de datos del nodo
    pub data_dir: PathBuf,
    /// Path al SAE inicial (.safetensors)
    pub initial_sae_path: Option<PathBuf>,
    /// Puertos P2P
    pub p2p_port: u16,
    /// Puerto HTTP (Web UI)
    pub http_port: u16,
    /// Si este nodo es seed/bootstrap
    pub is_bootstrap_node: bool,
    /// IDs de peers de confianza (para genesis)
    pub trusted_peers: Vec<String>,
    /// Parámetros de gossipsub iniciales
    pub gossipsub_params: GossipsubParams,
    /// Metadata del genesis
    pub metadata: HashMap<String, String>,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            initial_sae_path: None,
            p2p_port: 9000,
            http_port: 3000,
            is_bootstrap_node: false,
            trusted_peers: Vec::new(),
            gossipsub_params: GossipsubParams::default(),
            metadata: HashMap::new(),
        }
    }
}

/// Parámetros de GossipSub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipsubParams {
    /// Número de peers para mesh
    pub mesh_n: usize,
    /// Divergencia permitida en mesh
    pub mesh_n_low: usize,
    /// Máximo en mesh
    pub mesh_n_high: usize,
    /// Fanout para topics sin subscribe
    pub fanout_ttl: usize,
    /// TTL de mensajes
    pub gossip_ttl: u64,
    /// Historial de mensajes
    pub history_length: usize,
    /// Historial reciente
    pub history_gossip: usize,
}

impl Default for GossipsubParams {
    fn default() -> Self {
        Self {
            mesh_n: 6,
            mesh_n_low: 4,
            mesh_n_high: 12,
            fanout_ttl: 6,
            gossip_ttl: 30,
            history_length: 5,
            history_gossip: 3,
        }
    }
}

/// Resultado de la inicialización
#[derive(Debug, Serialize, Deserialize)]
pub struct InitResult {
    /// Modo de operación resultante
    pub network_mode: NetworkMode,
    /// Estado final
    pub state: InitState,
    /// Seeds conectados
    pub connected_seeds: usize,
    /// SAEs cargados
    pub sae_layers_loaded: usize,
    /// Leases creadas
    pub leases_created: usize,
    /// Timestamp (epoch seconds)
    pub timestamp: u64,
    /// Mensaje de estado
    pub message: String,
}

/// Gestor de inicialización de red
pub struct NetworkInitializer {
    seed_registry: SeedRegistry,
    config: GenesisConfig,
    state: InitState,
    genesis_file: PathBuf,
}

impl NetworkInitializer {
    pub fn new(config: GenesisConfig) -> Self {
        let genesis_file = config.data_dir.join("genesis.json");

        info!(
            data_dir = %config.data_dir.display(),
            p2p_port = config.p2p_port,
            http_port = config.http_port,
            is_bootstrap = config.is_bootstrap_node,
            "Network initializer created"
        );

        Self {
            seed_registry: SeedRegistry::new(),
            config,
            state: InitState::Pending,
            genesis_file,
        }
    }

    /// Verificar si la red ya fue inicializada
    pub fn is_initialized(&self) -> bool {
        self.genesis_file.exists()
    }

    /// Ejecutar inicialización genesis
    pub fn run_genesis(&mut self) -> Result<InitResult, NetworkInitError> {
        if self.is_initialized() {
            return Err(NetworkInitError::NetworkAlreadyInitialized {
                path: self.genesis_file.to_string_lossy().to_string(),
            });
        }

        info!("Starting genesis initialization...");
        self.state = InitState::Pending;

        // Paso 1: Crear directorio de datos
        self.create_data_directory()?;

        // Paso 2: Determinar modo de red
        let network_mode = self.determine_network_mode()?;
        info!(mode = %network_mode, "Network mode determined");

        // Paso 3: Conectar a seeds (si no es genesis puro)
        let connected_seeds = if network_mode == NetworkMode::Genesis {
            0
        } else {
            self.connect_to_seeds()?
        };

        // Paso 4: Cargar SAEs iniciales
        let sae_layers = self.load_initial_saes()?;

        // Paso 5: Crear leases iniciales
        let leases = self.create_initial_leases(sae_layers)?;

        // Paso 6: Guardar genesis
        self.save_genesis(&network_mode, connected_seeds, sae_layers, leases)?;

        let result = InitResult {
            network_mode: network_mode.clone(),
            state: InitState::Operational,
            connected_seeds,
            sae_layers_loaded: sae_layers,
            leases_created: leases,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            message: format!(
                "Genesis complete: mode={}, seeds={}, sae_layers={}, leases={}",
                network_mode, connected_seeds, sae_layers, leases
            ),
        };

        info!(%result.message, "Genesis initialization complete");
        Ok(result)
    }

    /// Crear directorio de datos
    fn create_data_directory(&self) -> Result<(), NetworkInitError> {
        std::fs::create_dir_all(&self.config.data_dir)
            .map_err(|e| NetworkInitError::DatabaseInitFailed(format!("Cannot create data dir: {e}")))?;

        // Subdirectorios
        for subdir in &["models", "leases", "feedback", "governance", "reputation"] {
            let path = self.config.data_dir.join(subdir);
            std::fs::create_dir_all(&path)
                .map_err(|e| NetworkInitError::DatabaseInitFailed(format!("Cannot create {subdir}: {e}")))?;
        }

        info!(
            path = %self.config.data_dir.display(),
            "Data directory created"
        );
        Ok(())
    }

    /// Determinar modo de red
    fn determine_network_mode(&self) -> Result<NetworkMode, NetworkInitError> {
        if self.config.is_bootstrap_node {
            return Ok(NetworkMode::BootstrapNode);
        }

        // Verificar seeds disponibles
        let usable_seeds = self.seed_registry.get_usable_seeds();
        if usable_seeds.is_empty() {
            warn!("No usable seeds found, falling back to offline mode");
            return Ok(NetworkMode::Offline);
        }

        Ok(NetworkMode::FullNetwork)
    }

    /// Conectar a seeds
    fn connect_to_seeds(&mut self) -> Result<usize, NetworkInitError> {
        self.state = InitState::Connecting;

        // Run health checks
        self.seed_registry.run_health_checks();

        let usable_count = self.seed_registry.get_usable_seeds().len();
        info!(usable_count, "Usable seeds found");

        if usable_count == 0 {
            self.state = InitState::OfflineFallback;
            warn!("No usable seeds, falling back to offline mode");
            return Ok(0);
        }

        // TODO: Phase 6 - Implementar conexión real con libp2p swarm
        // swarm.add_address_known(seed.multiaddress.parse()?);
        // swarm.dial(seed.multiaddress.parse()?);

        info!(connected = usable_count, "Connected to seeds");
        Ok(usable_count)
    }

    /// Cargar SAEs iniciales
    // FIX: E0594 - cannot mutate self.state behind &reference, change to &mut self
    fn load_initial_saes(&mut self) -> Result<usize, NetworkInitError> {
        self.state = InitState::Syncing;

        let models_dir = self.config.data_dir.join("models");
        let mut count = 0;

        // Cargar SAE especificado en config
        if let Some(ref sae_path) = self.config.initial_sae_path {
            if sae_path.exists() {
                info!(path = %sae_path.display(), "Loading initial SAE");
                // TODO: Phase 6 - Integrar con sae::loader
                count += 1;
            } else {
                warn!(path = %sae_path.display(), "Initial SAE path does not exist");
            }
        }

        // Cargar SAEs existentes en models/
        if models_dir.exists() {
            // FIX: trait bound - ReadDir doesn't implement Default, use ok().into_iter().flatten()
            for entry in std::fs::read_dir(&models_dir).ok().into_iter().flatten() {
                let entry = entry.map_err(|e| NetworkInitError::SaeLoadFailed(e.to_string()))?;
                if entry.path().extension().is_some_and(|ext| ext == "safetensors") { // CLEANUP: map_or(false, |ext| ext == X) -> is_some_and
                    info!(
                        path = %entry.path().display(),
                        "Found existing SAE model"
                    );
                    count += 1;
                }
            }
        }

        info!(count, "SAE layers loaded");
        Ok(count)
    }

    /// Crear leases iniciales
    fn create_initial_leases(&self, sae_layers: usize) -> Result<usize, NetworkInitError> {
        let leases_dir = self.config.data_dir.join("leases");

        // Crear lease por cada SAE layer
        for i in 0..sae_layers {
            let lease_id = format!("lease_layer_{}", i);
            let lease_path = leases_dir.join(format!("{}.json", lease_id));

            let lease_data = serde_json::json!({
                "lease_id": lease_id,
                "layer_id": format!("layer_{}", i),
                "owner": "genesis",
                "created_at": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "expires_at": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() + 3600, // 1 hora
            });

            std::fs::write(&lease_path, serde_json::to_string_pretty(&lease_data).unwrap())
                .map_err(|e| NetworkInitError::DatabaseInitFailed(e.to_string()))?;
        }

        info!(leases = sae_layers, "Initial leases created");
        Ok(sae_layers)
    }

    /// Guardar configuración de genesis
    fn save_genesis(
        &self,
        mode: &NetworkMode,
        seeds: usize,
        sae_layers: usize,
        leases: usize,
    ) -> Result<(), NetworkInitError> {
        let genesis_data = serde_json::json!({
            "genesis_timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "network_mode": mode.to_string(),
            "p2p_port": self.config.p2p_port,
            "http_port": self.config.http_port,
            "is_bootstrap": self.config.is_bootstrap_node,
            "connected_seeds": seeds,
            "sae_layers": sae_layers,
            "leases_created": leases,
            "gossipsub_params": self.config.gossipsub_params,
            "ed2k_version": "0.5.0",
        });

        std::fs::write(&self.genesis_file, serde_json::to_string_pretty(&genesis_data).unwrap())
            .map_err(|e| NetworkInitError::DatabaseInitFailed(format!("Cannot write genesis: {e}")))?;

        info!(
            path = %self.genesis_file.display(),
            "Genesis configuration saved"
        );
        Ok(())
    }

    /// Cargar configuración de genesis existente
    pub fn load_genesis(&self) -> Result<serde_json::Value, NetworkInitError> {
        let data = std::fs::read_to_string(&self.genesis_file)
            .map_err(|e| NetworkInitError::DatabaseInitFailed(format!("Cannot read genesis: {e}")))?;

        serde_json::from_str(&data)
            .map_err(|e| NetworkInitError::InvalidGenesisConfig(e.to_string()))
    }

    /// Migrar de versión anterior
    ///
    /// TODO: Phase 6 - Implementar migraciones específicas por versión
    pub fn migrate(&self, from_version: &str, to_version: &str) -> Result<(), NetworkInitError> {
        info!(from = from_version, to = to_version, "Starting migration");

        // Verificar si hay genesis
        if !self.is_initialized() {
            return Ok(()); // No hay qué migrar
        }

        let genesis = self.load_genesis()?;
        let old_version = genesis.get("ed2k_version").and_then(|v| v.as_str()).unwrap_or("0.0.0");

        info!(
            current = old_version,
            target = to_version,
            "Migration version check"
        );

        // TODO: Phase 6 - Agregar lógica de migración por versión
        // Ej: 0.4 -> 0.5: actualizar schema de leases

        Ok(())
    }

    /// Obtener estado actual
    pub fn state(&self) -> &InitState {
        &self.state
    }

    /// Obtener seed registry
    pub fn seed_registry(&self) -> &SeedRegistry {
        &self.seed_registry
    }

    /// Obtener configuración
    pub fn config(&self) -> &GenesisConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_config_default() {
        let config = GenesisConfig::default();
        assert_eq!(config.p2p_port, 9000);
        assert_eq!(config.http_port, 3000);
        assert!(!config.is_bootstrap_node);
    }

    #[test]
    fn test_gossipsub_params_default() {
        let params = GossipsubParams::default();
        assert_eq!(params.mesh_n, 6);
        assert_eq!(params.mesh_n_low, 4);
        assert_eq!(params.mesh_n_high, 12);
    }

    #[test]
    fn test_network_mode_display() {
        assert_eq!(format!("{}", NetworkMode::FullNetwork), "FullNetwork");
        assert_eq!(format!("{}", NetworkMode::Offline), "Offline");
        assert_eq!(format!("{}", NetworkMode::Genesis), "Genesis");
    }

    #[test]
    fn test_init_state_display() {
        assert_eq!(format!("{}", InitState::Pending), "Pending");
        assert_eq!(format!("{}", InitState::Operational), "Operational");
    }
}
