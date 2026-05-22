//! Multi-Chain Registry — Registro y gestión de múltiples redes blockchain
//!
//! Permite a los nodos ed2kIA participar en múltiples redes blockchain
//! simultáneamente, con gestión de estado de conexión, pooling y health checks.

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

// ============================================================================
// Error Types
// ============================================================================

/// Error types for multi-chain registry operations
#[derive(Debug, Clone, PartialEq)]
pub enum MultiChainError {
    /// Chain ID already registered
    ChainAlreadyRegistered(String),
    /// Chain ID not found
    ChainNotFound(String),
    /// Invalid chain configuration
    InvalidConfig(String),
    /// Connection pool exhausted
    ConnectionPoolExhausted,
    /// Health check failed
    HealthCheckFailed(String),
}

impl fmt::Display for MultiChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MultiChainError::ChainAlreadyRegistered(id) => {
                write!(f, "Chain '{}' already registered", id)
            }
            MultiChainError::ChainNotFound(id) => write!(f, "Chain '{}' not found", id),
            MultiChainError::InvalidConfig(msg) => write!(f, "Invalid chain config: {}", msg),
            MultiChainError::ConnectionPoolExhausted => {
                write!(f, "Connection pool exhausted")
            }
            MultiChainError::HealthCheckFailed(id) => {
                write!(f, "Health check failed for chain '{}'", id)
            }
        }
    }
}

impl std::error::Error for MultiChainError {}

// ============================================================================
// Chain Protocol
// ============================================================================

/// Supported blockchain protocols
#[derive(Debug, Clone, PartialEq)]
pub enum ChainProtocol {
    Ethereum,
    Solana,
    Polkadot,
    Custom(String),
}

impl fmt::Display for ChainProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainProtocol::Ethereum => write!(f, "Ethereum"),
            ChainProtocol::Solana => write!(f, "Solana"),
            ChainProtocol::Polkadot => write!(f, "Polkadot"),
            ChainProtocol::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

// ============================================================================
// Chain State
// ============================================================================

/// Connection state for a chain
#[derive(Debug, Clone, PartialEq)]
pub enum ChainState {
    Connected,
    Disconnected,
    Syncing,
    Error(String),
}

impl fmt::Display for ChainState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainState::Connected => write!(f, "Connected"),
            ChainState::Disconnected => write!(f, "Disconnected"),
            ChainState::Syncing => write!(f, "Syncing"),
            ChainState::Error(msg) => write!(f, "Error({})", msg),
        }
    }
}

// ============================================================================
// Chain Configuration
// ============================================================================

/// Configuration for a blockchain connection
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// Unique chain identifier
    pub chain_id: String,
    /// RPC/WS endpoint URL
    pub endpoint: String,
    /// Blockchain protocol type
    pub protocol: ChainProtocol,
    /// Additional protocol-specific parameters
    pub parameters: HashMap<String, String>,
}

impl ChainConfig {
    /// Create a new chain configuration
    pub fn new(chain_id: String, endpoint: String, protocol: ChainProtocol) -> Self {
        Self {
            chain_id,
            endpoint,
            protocol,
            parameters: HashMap::new(),
        }
    }

    /// Add a parameter to the configuration
    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), MultiChainError> {
        if self.chain_id.is_empty() {
            return Err(MultiChainError::InvalidConfig(
                "Chain ID cannot be empty".to_string(),
            ));
        }
        if self.endpoint.is_empty() {
            return Err(MultiChainError::InvalidConfig(
                "Endpoint cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Chain Entry
// ============================================================================

/// Entry in the registry for a registered chain
#[derive(Debug, Clone)]
pub struct ChainEntry {
    /// Chain configuration
    pub config: ChainConfig,
    /// Current connection state
    pub state: ChainState,
    /// Last successful heartbeat timestamp
    pub last_heartbeat: Instant,
    /// Number of connected nodes on this chain
    pub node_count: usize,
}

impl ChainEntry {
    /// Create a new chain entry
    pub fn new(config: ChainConfig) -> Self {
        Self {
            state: ChainState::Disconnected,
            last_heartbeat: Instant::now(),
            node_count: 0,
            config,
        }
    }

    /// Check if the chain is currently active (connected or syncing)
    pub fn is_active(&self) -> bool {
        matches!(self.state, ChainState::Connected | ChainState::Syncing)
    }

    /// Check if the chain heartbeat has expired
    pub fn is_heartbeat_expired(&self, timeout: std::time::Duration) -> bool {
        self.last_heartbeat.elapsed() > timeout
    }

    /// Update heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    /// Increment node count
    pub fn node_joined(&mut self) {
        self.node_count += 1;
    }

    /// Decrement node count
    pub fn node_left(&mut self) {
        self.node_count = self.node_count.saturating_sub(1);
    }
}

// ============================================================================
// Registry Configuration
// ============================================================================

/// Configuration for the multi-chain registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum number of chains allowed
    pub max_chains: usize,
    /// Heartbeat timeout duration
    pub heartbeat_timeout_ms: u64,
    /// Maximum connections per chain
    pub max_connections_per_chain: usize,
    /// Enable automatic health checks
    pub auto_health_check: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_chains: 64,
            heartbeat_timeout_ms: 30_000,
            max_connections_per_chain: 16,
            auto_health_check: true,
        }
    }
}

// ============================================================================
// Multi-Chain Registry
// ============================================================================

/// Registry for managing multiple blockchain network connections
pub struct MultiChainRegistry {
    /// Registered chain entries
    chains: HashMap<String, ChainEntry>,
    /// Registry configuration
    config: RegistryConfig,
}

impl MultiChainRegistry {
    /// Create a new registry with default configuration
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
            config: RegistryConfig::default(),
        }
    }

    /// Create a new registry with custom configuration
    pub fn with_config(config: RegistryConfig) -> Self {
        Self {
            chains: HashMap::new(),
            config,
        }
    }

    /// Register a new blockchain network
    pub fn register_chain(&mut self, config: ChainConfig) -> Result<(), MultiChainError> {
        // Validate config
        config.validate()?;

        // Check duplicate
        if self.chains.contains_key(&config.chain_id) {
            return Err(MultiChainError::ChainAlreadyRegistered(
                config.chain_id.clone(),
            ));
        }

        // Check capacity
        if self.chains.len() >= self.config.max_chains {
            return Err(MultiChainError::InvalidConfig(format!(
                "Registry full (max {} chains)",
                self.config.max_chains
            )));
        }

        let entry = ChainEntry::new(config);
        self.chains.insert(entry.config.chain_id.clone(), entry);
        Ok(())
    }

    /// Unregister a blockchain network
    pub fn unregister_chain(&mut self, chain_id: &str) -> Result<(), MultiChainError> {
        match self.chains.remove(chain_id) {
            Some(_) => Ok(()),
            None => Err(MultiChainError::ChainNotFound(chain_id.to_string())),
        }
    }

    /// Update the state of a registered chain
    pub fn update_state(&mut self, chain_id: &str, state: ChainState) {
        if let Some(entry) = self.chains.get_mut(chain_id) {
            entry.state = state;
            entry.update_heartbeat();
        }
    }

    /// Get a reference to a chain entry
    pub fn get_chain(&self, chain_id: &str) -> Option<&ChainEntry> {
        self.chains.get(chain_id)
    }

    /// Get a mutable reference to a chain entry
    pub fn get_chain_mut(&mut self, chain_id: &str) -> Option<&mut ChainEntry> {
        self.chains.get_mut(chain_id)
    }

    /// Get all active chains (connected or syncing)
    pub fn get_active_chains(&self) -> Vec<&ChainEntry> {
        self.chains
            .values()
            .filter(|entry| entry.is_active())
            .collect()
    }

    /// Get all registered chains
    pub fn get_all_chains(&self) -> Vec<&ChainEntry> {
        self.chains.values().collect()
    }

    /// Get the number of registered chains
    pub fn chain_count(&self) -> usize {
        self.chains.len()
    }

    /// Run health checks on all registered chains
    pub fn health_check(&mut self) -> HashMap<String, ChainState> {
        let timeout = std::time::Duration::from_millis(self.config.heartbeat_timeout_ms);
        let mut states = HashMap::new();

        for (chain_id, entry) in self.chains.iter_mut() {
            let state = if entry.is_heartbeat_expired(timeout) {
                let error_msg = format!(
                    "Heartbeat expired after {}ms",
                    self.config.heartbeat_timeout_ms
                );
                entry.state = ChainState::Error(error_msg.clone());
                ChainState::Error(error_msg)
            } else {
                entry.state.clone()
            };
            states.insert(chain_id.clone(), state);
        }

        states
    }

    /// Check if a chain is registered
    pub fn is_registered(&self, chain_id: &str) -> bool {
        self.chains.contains_key(chain_id)
    }

    /// Get chains by protocol type
    pub fn get_chains_by_protocol(&self, protocol: &ChainProtocol) -> Vec<&ChainEntry> {
        self.chains
            .values()
            .filter(|entry| &entry.config.protocol == protocol)
            .collect()
    }

    /// Get chains in error state
    pub fn get_error_chains(&self) -> Vec<&ChainEntry> {
        self.chains
            .values()
            .filter(|entry| matches!(entry.state, ChainState::Error(_)))
            .collect()
    }

    /// Get the registry configuration
    pub fn get_config(&self) -> &RegistryConfig {
        &self.config
    }

    /// Clear all registered chains
    pub fn clear(&mut self) {
        self.chains.clear();
    }
}

impl Default for MultiChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_eth_config(chain_id: &str) -> ChainConfig {
        ChainConfig::new(
            chain_id.to_string(),
            "https://eth.example.com".to_string(),
            ChainProtocol::Ethereum,
        )
    }

    fn make_solana_config(chain_id: &str) -> ChainConfig {
        ChainConfig::new(
            chain_id.to_string(),
            "https://solana.example.com".to_string(),
            ChainProtocol::Solana,
        )
    }

    #[test]
    fn test_registry_creation() {
        let registry = MultiChainRegistry::new();
        assert_eq!(registry.chain_count(), 0);
    }

    #[test]
    fn test_registry_with_config() {
        let config = RegistryConfig {
            max_chains: 10,
            ..Default::default()
        };
        let registry = MultiChainRegistry::with_config(config);
        assert_eq!(registry.chain_count(), 0);
        assert_eq!(registry.get_config().max_chains, 10);
    }

    #[test]
    fn test_register_chain() {
        let mut registry = MultiChainRegistry::new();
        let config = make_eth_config("eth-mainnet");
        assert!(registry.register_chain(config).is_ok());
        assert_eq!(registry.chain_count(), 1);
    }

    #[test]
    fn test_register_duplicate_chain() {
        let mut registry = MultiChainRegistry::new();
        let config1 = make_eth_config("eth-mainnet");
        let config2 = make_eth_config("eth-mainnet");
        assert!(registry.register_chain(config1).is_ok());
        match registry.register_chain(config2) {
            Err(MultiChainError::ChainAlreadyRegistered(id)) => {
                assert_eq!(id, "eth-mainnet");
            }
            other => panic!("Expected ChainAlreadyRegistered, got {:?}", other),
        }
    }

    #[test]
    fn test_unregister_chain() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        assert!(registry.unregister_chain("eth-mainnet").is_ok());
        assert_eq!(registry.chain_count(), 0);
    }

    #[test]
    fn test_unregister_nonexistent_chain() {
        let mut registry = MultiChainRegistry::new();
        match registry.unregister_chain("nonexistent") {
            Err(MultiChainError::ChainNotFound(id)) => {
                assert_eq!(id, "nonexistent");
            }
            other => panic!("Expected ChainNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_update_state() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        registry.update_state("eth-mainnet", ChainState::Connected);
        let entry = registry.get_chain("eth-mainnet").unwrap();
        assert_eq!(entry.state, ChainState::Connected);
    }

    #[test]
    fn test_get_chain() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        let entry = registry.get_chain("eth-mainnet").unwrap();
        assert_eq!(entry.config.chain_id, "eth-mainnet");
    }

    #[test]
    fn test_get_nonexistent_chain() {
        let registry = MultiChainRegistry::new();
        assert!(registry.get_chain("nonexistent").is_none());
    }

    #[test]
    fn test_get_active_chains() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        registry
            .register_chain(make_solana_config("solana-mainnet"))
            .unwrap();
        registry.update_state("eth-mainnet", ChainState::Connected);
        registry.update_state("solana-mainnet", ChainState::Disconnected);
        let active = registry.get_active_chains();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].config.chain_id, "eth-mainnet");
    }

    #[test]
    fn test_health_check() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        registry.update_state("eth-mainnet", ChainState::Connected);
        let states = registry.health_check();
        assert!(states.contains_key("eth-mainnet"));
    }

    #[test]
    fn test_is_registered() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        assert!(registry.is_registered("eth-mainnet"));
        assert!(!registry.is_registered("nonexistent"));
    }

    #[test]
    fn test_get_chains_by_protocol() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        registry
            .register_chain(make_eth_config("eth-goerli"))
            .unwrap();
        registry
            .register_chain(make_solana_config("solana-mainnet"))
            .unwrap();
        let eth_chains = registry.get_chains_by_protocol(&ChainProtocol::Ethereum);
        assert_eq!(eth_chains.len(), 2);
        let sol_chains = registry.get_chains_by_protocol(&ChainProtocol::Solana);
        assert_eq!(sol_chains.len(), 1);
    }

    #[test]
    fn test_get_error_chains() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        registry
            .register_chain(make_solana_config("solana-mainnet"))
            .unwrap();
        registry.update_state("eth-mainnet", ChainState::Error("timeout".to_string()));
        registry.update_state("solana-mainnet", ChainState::Connected);
        let errors = registry.get_error_chains();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].config.chain_id, "eth-mainnet");
    }

    #[test]
    fn test_clear_registry() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        registry
            .register_chain(make_solana_config("solana-mainnet"))
            .unwrap();
        registry.clear();
        assert_eq!(registry.chain_count(), 0);
    }

    #[test]
    fn test_chain_entry_creation() {
        let config = make_eth_config("eth-mainnet");
        let entry = ChainEntry::new(config);
        assert_eq!(entry.state, ChainState::Disconnected);
        assert_eq!(entry.node_count, 0);
        assert!(!entry.is_active());
    }

    #[test]
    fn test_chain_entry_is_active() {
        let mut entry = ChainEntry::new(make_eth_config("eth-mainnet"));
        assert!(!entry.is_active());
        entry.state = ChainState::Connected;
        assert!(entry.is_active());
        entry.state = ChainState::Syncing;
        assert!(entry.is_active());
        entry.state = ChainState::Disconnected;
        assert!(!entry.is_active());
    }

    #[test]
    fn test_chain_entry_node_count() {
        let mut entry = ChainEntry::new(make_eth_config("eth-mainnet"));
        entry.node_joined();
        entry.node_joined();
        assert_eq!(entry.node_count, 2);
        entry.node_left();
        assert_eq!(entry.node_count, 1);
        entry.node_left();
        assert_eq!(entry.node_count, 0);
    }

    #[test]
    fn test_chain_entry_heartbeat() {
        let entry = ChainEntry::new(make_eth_config("eth-mainnet"));
        assert!(!entry.is_heartbeat_expired(std::time::Duration::from_secs(1)));
    }

    #[test]
    fn test_chain_config_validation() {
        let config = ChainConfig::new(
            "eth-mainnet".to_string(),
            "https://eth.example.com".to_string(),
            ChainProtocol::Ethereum,
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_chain_config_empty_id() {
        let config = ChainConfig::new(
            "".to_string(),
            "https://eth.example.com".to_string(),
            ChainProtocol::Ethereum,
        );
        match config.validate() {
            Err(MultiChainError::InvalidConfig(msg)) => {
                assert!(msg.contains("Chain ID"));
            }
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_chain_config_empty_endpoint() {
        let config = ChainConfig::new(
            "eth-mainnet".to_string(),
            "".to_string(),
            ChainProtocol::Ethereum,
        );
        match config.validate() {
            Err(MultiChainError::InvalidConfig(msg)) => {
                assert!(msg.contains("Endpoint"));
            }
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_chain_config_with_parameters() {
        let config = ChainConfig::new(
            "eth-mainnet".to_string(),
            "https://eth.example.com".to_string(),
            ChainProtocol::Ethereum,
        )
        .with_parameter("gas_limit".to_string(), "30000000".to_string())
        .with_parameter("chain_type".to_string(), "mainnet".to_string());
        assert_eq!(config.parameters.len(), 2);
        assert_eq!(config.parameters.get("gas_limit").unwrap(), "30000000");
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(format!("{}", ChainProtocol::Ethereum), "Ethereum");
        assert_eq!(format!("{}", ChainProtocol::Solana), "Solana");
        assert_eq!(format!("{}", ChainProtocol::Polkadot), "Polkadot");
        assert_eq!(
            format!("{}", ChainProtocol::Custom("Avalanche".to_string())),
            "Custom(Avalanche)"
        );
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", ChainState::Connected), "Connected");
        assert_eq!(format!("{}", ChainState::Disconnected), "Disconnected");
        assert_eq!(format!("{}", ChainState::Syncing), "Syncing");
        assert_eq!(
            format!("{}", ChainState::Error("test".to_string())),
            "Error(test)"
        );
    }

    #[test]
    fn test_error_display() {
        let err = MultiChainError::ChainAlreadyRegistered("eth".to_string());
        assert!(format!("{}", err).contains("eth"));
        let err = MultiChainError::ChainNotFound("sol".to_string());
        assert!(format!("{}", err).contains("sol"));
    }

    #[test]
    fn test_max_chains_limit() {
        let config = RegistryConfig {
            max_chains: 2,
            ..Default::default()
        };
        let mut registry = MultiChainRegistry::with_config(config);
        registry.register_chain(make_eth_config("chain-1")).unwrap();
        registry.register_chain(make_eth_config("chain-2")).unwrap();
        match registry.register_chain(make_eth_config("chain-3")) {
            Err(MultiChainError::InvalidConfig(msg)) => {
                assert!(msg.contains("Registry full"));
            }
            other => panic!("Expected InvalidConfig, got {:?}", other),
        }
    }

    #[test]
    fn test_get_all_chains() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        registry
            .register_chain(make_solana_config("solana-mainnet"))
            .unwrap();
        assert_eq!(registry.get_all_chains().len(), 2);
    }

    #[test]
    fn test_get_chain_mut() {
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(make_eth_config("eth-mainnet"))
            .unwrap();
        if let Some(entry) = registry.get_chain_mut("eth-mainnet") {
            entry.node_joined();
            assert_eq!(entry.node_count, 1);
        }
    }

    #[test]
    fn test_registry_default() {
        let registry = MultiChainRegistry::default();
        assert_eq!(registry.chain_count(), 0);
    }

    #[test]
    fn test_config_default() {
        let config = RegistryConfig::default();
        assert_eq!(config.max_chains, 64);
        assert_eq!(config.heartbeat_timeout_ms, 30_000);
        assert_eq!(config.max_connections_per_chain, 16);
        assert!(config.auto_health_check);
    }
}
