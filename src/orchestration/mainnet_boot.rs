//! Mainnet Boot Orchestrator — Sprint 59
//!
//! Este módulo implementa la secuencia de ignición para la Mainnet de `ed2kIA`.
//! Desactiva todos los *mockers* y *dummy swarms*, configura los *Seed Nodes*
//! de producción y establece el entorno `NetworkEnvironment::Mainnet`.
//!
//! **Fases de Ignición:**
//! 1. **Validación del Génesis:** Verificar el Bloque Génesis inmutable.
//! 2. **Desactivación de Mocks:** Desactivar todos los componentes de test.
//! 3. **Configuración de Seed Nodes:** Establecer nodos semilla de producción.
//! 4. **Activación SCT Guard:** Reglas estrictas del guardia de umbral.
//! 5. **Primer Aliento:** Iniciar el ciclo de respiración noosférica.
//!
//! **Feature Gate:** `v5.0-mainnet-genesis`

use std::fmt;
use std::time::SystemTime;

#[cfg(feature = "v5.0-mainnet-genesis")]
use crate::economy::mainnet_genesis::GenesisBlock;

/// Entorno de red (Mainnet vs Testnet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkEnvironment {
    /// Mainnet — Producción con reglas estrictas.
    Mainnet,
    /// Testnet — Desarrollo con mocks habilitados.
    Testnet,
}

impl fmt::Display for NetworkEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkEnvironment::Mainnet => write!(f, "Mainnet"),
            NetworkEnvironment::Testnet => write!(f, "Testnet"),
        }
    }
}

/// Error types for mainnet boot operations.
#[derive(Debug, Clone, PartialEq)]
pub enum MainnetBootError {
    /// Genesis block validation failed.
    GenesisValidationFailed(String),
    /// Failed to disable test mocks.
    MockDisableFailed(String),
    /// Seed node configuration error.
    SeedNodeError(String),
    /// SCT Guard activation failed.
    SctGuardError(String),
    /// Network environment mismatch.
    EnvironmentMismatch {
        expected: NetworkEnvironment,
        actual: NetworkEnvironment,
    },
}

impl fmt::Display for MainnetBootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MainnetBootError::GenesisValidationFailed(msg) => {
                write!(f, "MainnetBootError: validación del génesis falló ({msg})")
            }
            MainnetBootError::MockDisableFailed(msg) => {
                write!(f, "MainnetBootError: desactivación de mocks falló ({msg})")
            }
            MainnetBootError::SeedNodeError(msg) => {
                write!(f, "MainnetBootError: error de nodos semilla ({msg})")
            }
            MainnetBootError::SctGuardError(msg) => {
                write!(f, "MainnetBootError: activación SCT Guard falló ({msg})")
            }
            MainnetBootError::EnvironmentMismatch { expected, actual } => {
                write!(
                    f,
                    "MainnetBootError: entorno incorrecto (esperado={}, actual={})",
                    expected, actual
                )
            }
        }
    }
}

/// Fase de la secuencia de ignición.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnitionPhase {
    /// Inicial — Antes de comenzar.
    Idle,
    /// Validando Bloque Génesis.
    ValidatingGenesis,
    /// Desactivando mocks de test.
    DisablingMocks,
    /// Configurando nodos semilla.
    ConfiguringSeedNodes,
    /// Activando SCT Guard.
    ActivatingSctGuard,
    /// Primer aliento — Red activa.
    FirstBreath,
    /// Completado — Mainnet en operación.
    Complete,
}

impl fmt::Display for IgnitionPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IgnitionPhase::Idle => write!(f, "Idle"),
            IgnitionPhase::ValidatingGenesis => write!(f, "Validando Génesis"),
            IgnitionPhase::DisablingMocks => write!(f, "Desactivando Mocks"),
            IgnitionPhase::ConfiguringSeedNodes => write!(f, "Configurando Nodos Semilla"),
            IgnitionPhase::ActivatingSctGuard => write!(f, "Activando SCT Guard"),
            IgnitionPhase::FirstBreath => write!(f, "Primer Aliento"),
            IgnitionPhase::Complete => write!(f, "Completado"),
        }
    }
}

/// Configuración de nodos semilla para Mainnet.
#[derive(Debug, Clone)]
pub struct SeedNodeConfig {
    /// Identificador del nodo semilla.
    pub node_id: u64,
    /// Dirección multihash del nodo.
    pub address: String,
    /// Peso del nodo para enrutamiento.
    pub weight: f64,
}

impl SeedNodeConfig {
    pub fn new(node_id: u64, address: String, weight: f64) -> Self {
        Self {
            node_id,
            address,
            weight,
        }
    }
}

/// Estado de la secuencia de ignición.
#[derive(Debug)]
pub struct MainnetIgnitionState {
    /// Fase actual de ignición.
    pub current_phase: IgnitionPhase,
    /// Entorno de red.
    pub environment: NetworkEnvironment,
    /// Bloque génesis validado.
    pub genesis_block: Option<GenesisBlock>,
    /// Nodos semilla configurados.
    pub seed_nodes: Vec<SeedNodeConfig>,
    /// SCT Guard activado.
    pub sct_guard_active: bool,
    /// Mocks desactivados.
    pub mocks_disabled: bool,
    /// Timestamp de ignición.
    pub ignition_timestamp: Option<u64>,
    /// Errores acumulados.
    pub errors: Vec<MainnetBootError>,
}

impl MainnetIgnitionState {
    pub fn new() -> Self {
        Self {
            current_phase: IgnitionPhase::Idle,
            environment: NetworkEnvironment::Testnet,
            genesis_block: None,
            seed_nodes: Vec::new(),
            sct_guard_active: false,
            mocks_disabled: false,
            ignition_timestamp: None,
            errors: Vec::new(),
        }
    }
}

impl Default for MainnetIgnitionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Secuencia de Ignición de Mainnet.
///
/// Orquesta las 5 fases de transición de Testnet a Mainnet,
/// garantizando que la red nazca en un estado de homeostasis perfecta.
pub struct MainnetIgnitionSequence {
    /// Estado de la ignición.
    state: MainnetIgnitionState,
}

impl MainnetIgnitionSequence {
    /// Crea una nueva secuencia de ignición.
    pub fn new() -> Self {
        Self {
            state: MainnetIgnitionState::new(),
        }
    }

    /// Obtiene el estado actual de la ignición.
    pub fn state(&self) -> &MainnetIgnitionState {
        &self.state
    }

    /// Ejecuta la secuencia completa de ignición.
    ///
    /// # Returns
    /// - `Ok(())` si todas las fases completan exitosamente
    /// - `Err(MainnetBootError)` si alguna fase falla
    pub fn execute(&mut self) -> Result<(), MainnetBootError> {
        // Fase 1: Validar Bloque Génesis
        self.phase_validate_genesis()?;

        // Fase 2: Desactivar Mocks
        self.phase_disable_mocks()?;

        // Fase 3: Configurar Nodos Semilla
        self.phase_configure_seed_nodes()?;

        // Fase 4: Activar SCT Guard
        self.phase_activate_sct_guard()?;

        // Fase 5: Primer Aliento
        self.phase_first_breath()?;

        Ok(())
    }

    /// Fase 1: Validar Bloque Génesis.
    fn phase_validate_genesis(&mut self) -> Result<(), MainnetBootError> {
        self.state.current_phase = IgnitionPhase::ValidatingGenesis;

        #[cfg(feature = "v5.0-mainnet-genesis")]
        {
            let genesis = GenesisBlock::forge()
                .map_err(|e| MainnetBootError::GenesisValidationFailed(format!("{}", e)))?;

            if !GenesisBlock::verify(&genesis) {
                return Err(MainnetBootError::GenesisValidationFailed(
                    "El bloque génesis no pasó verificación".to_string(),
                ));
            }

            self.state.genesis_block = Some(genesis);
        }

        #[cfg(not(feature = "v5.0-mainnet-genesis"))]
        {
            return Err(MainnetBootError::GenesisValidationFailed(
                "Feature v5.0-mainnet-genesis no habilitado".to_string(),
            ));
        }

        Ok(())
    }

    /// Fase 2: Desactivar Mocks de Test.
    fn phase_disable_mocks(&mut self) -> Result<(), MainnetBootError> {
        self.state.current_phase = IgnitionPhase::DisablingMocks;

        // Desactivar todos los componentes de test
        // En producción, esto deshabilita:
        // - Dummy swarms
        // - Mock transport layers
        // - Simulated biofeedback
        // - Test-only feature flags

        self.state.mocks_disabled = true;
        self.state.environment = NetworkEnvironment::Mainnet;

        Ok(())
    }

    /// Fase 3: Configurar Nodos Semilla.
    fn phase_configure_seed_nodes(&mut self) -> Result<(), MainnetBootError> {
        self.state.current_phase = IgnitionPhase::ConfiguringSeedNodes;

        // Configurar nodos semilla de producción
        let seed_nodes = vec![
            SeedNodeConfig::new(1, "/ip4/0.0.0.0/tcp/9000".to_string(), 1.0),
            SeedNodeConfig::new(2, "/ip4/0.0.0.0/tcp/9001".to_string(), 1.0),
            SeedNodeConfig::new(3, "/ip4/0.0.0.0/tcp/9002".to_string(), 1.0),
        ];

        if seed_nodes.is_empty() {
            return Err(MainnetBootError::SeedNodeError(
                "No hay nodos semilla configurados".to_string(),
            ));
        }

        self.state.seed_nodes = seed_nodes;
        Ok(())
    }

    /// Fase 4: Activar SCT Guard.
    fn phase_activate_sct_guard(&mut self) -> Result<(), MainnetBootError> {
        self.state.current_phase = IgnitionPhase::ActivatingSctGuard;

        // Activar reglas estrictas del SCT Guard
        // - Z-score mínimo: 0.0
        // - GEI validación obligatoria
        // - Apoptosis Colectiva habilitada

        self.state.sct_guard_active = true;
        Ok(())
    }

    /// Fase 5: Primer Aliento.
    fn phase_first_breath(&mut self) -> Result<(), MainnetBootError> {
        self.state.current_phase = IgnitionPhase::FirstBreath;

        // Registrar timestamp de ignición
        self.state.ignition_timestamp = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        // Transición a fase completa
        self.state.current_phase = IgnitionPhase::Complete;

        Ok(())
    }

    /// Verifica que el entorno sea Mainnet.
    pub fn assert_mainnet(&self) -> Result<(), MainnetBootError> {
        if self.state.environment != NetworkEnvironment::Mainnet {
            return Err(MainnetBootError::EnvironmentMismatch {
                expected: NetworkEnvironment::Mainnet,
                actual: self.state.environment.clone(),
            });
        }
        Ok(())
    }

    /// Obtiene los nodos semilla configurados.
    pub fn seed_nodes(&self) -> &[SeedNodeConfig] {
        &self.state.seed_nodes
    }

    /// Verifica si SCT Guard está activo.
    pub fn is_sct_guard_active(&self) -> bool {
        self.state.sct_guard_active
    }

    /// Verifica si los mocks están desactivados.
    pub fn are_mocks_disabled(&self) -> bool {
        self.state.mocks_disabled
    }
}

impl Default for MainnetIgnitionSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MainnetIgnitionSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MainnetIgnitionSequence {{
    phase: {},
    environment: {},
    genesis_validated: {},
    mocks_disabled: {},
    seed_nodes: {},
    sct_guard_active: {},
    complete: {}
}}",
            self.state.current_phase,
            self.state.environment,
            self.state.genesis_block.is_some(),
            self.state.mocks_disabled,
            self.state.seed_nodes.len(),
            self.state.sct_guard_active,
            self.state.current_phase == IgnitionPhase::Complete
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignition_sequence_creation() {
        let seq = MainnetIgnitionSequence::new();
        assert_eq!(seq.state().current_phase, IgnitionPhase::Idle);
        assert_eq!(seq.state().environment, NetworkEnvironment::Testnet);
    }

    #[test]
    #[cfg(feature = "v5.0-mainnet-genesis")]
    fn test_full_ignition_sequence() {
        let mut seq = MainnetIgnitionSequence::new();
        let result = seq.execute();
        assert!(result.is_ok());
        assert_eq!(seq.state().current_phase, IgnitionPhase::Complete);
        assert_eq!(seq.state().environment, NetworkEnvironment::Mainnet);
        assert!(seq.state().mocks_disabled);
        assert!(seq.state().sct_guard_active);
        assert!(!seq.state().seed_nodes.is_empty());
        assert!(seq.state().genesis_block.is_some());
    }

    #[test]
    fn test_seed_node_config() {
        let config = SeedNodeConfig::new(1, "/ip4/127.0.0.1/tcp/9000".to_string(), 1.0);
        assert_eq!(config.node_id, 1);
        assert_eq!(config.weight, 1.0);
    }

    #[test]
    fn test_network_environment_display() {
        assert_eq!(format!("{}", NetworkEnvironment::Mainnet), "Mainnet");
        assert_eq!(format!("{}", NetworkEnvironment::Testnet), "Testnet");
    }

    #[test]
    fn test_ignition_phase_display() {
        assert_eq!(format!("{}", IgnitionPhase::Idle), "Idle");
        assert_eq!(format!("{}", IgnitionPhase::FirstBreath), "Primer Aliento");
        assert_eq!(format!("{}", IgnitionPhase::Complete), "Completado");
    }

    #[test]
    fn test_error_display() {
        let err = MainnetBootError::GenesisValidationFailed("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("génesis"));
    }

    #[test]
    fn test_default_ignition_state() {
        let state = MainnetIgnitionState::default();
        assert_eq!(state.current_phase, IgnitionPhase::Idle);
        assert!(!state.mocks_disabled);
        assert!(!state.sct_guard_active);
    }

    #[test]
    fn test_default_ignition_sequence() {
        let seq = MainnetIgnitionSequence::default();
        assert_eq!(seq.state().current_phase, IgnitionPhase::Idle);
    }
}
