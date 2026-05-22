//! Bootstrap Discovery - Registro de nodos bootstrap + descubrimiento global
//!
//! Soporte para nodos bootstrap hardcoded + DNS discovery.
//! Integra con `libp2p::autonat` y `identify` para versionado de protocolo.

use std::collections::HashMap;
// CLEANUP: removed unused import std::sync::Arc
use std::time::{Duration, Instant};

use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;
use tracing::{debug, info, warn};

/// Versión del protocolo ed2kIA
pub const ED2K_PROTOCOL_VERSION: &str = "ed2k/0.4.0";

/// Prefix de protocolo para identify
pub const ED2K_PROTOCOL_PREFIX: &str = "/ed2k";

/// Puertos por defecto para nodos bootstrap
const DEFAULT_BOOTSTRAP_PORTS: &[u16] = &[9001, 9002, 9003];

/// Entrada de nodo bootstrap
#[derive(Debug, Clone)]
pub struct BootstrapNode {
    /// Multiaddress completo del nodo
    pub address: Multiaddr,
    /// Peer ID extraído del address
    pub peer_id: PeerId,
    /// Tipo de origen (hardcoded, dns, config)
    pub source: BootstrapSource,
    /// Última vez que se verificó como activo
    pub last_checked: Option<Instant>,
    /// Estado actual del nodo
    pub status: BootstrapStatus,
    /// Latencia promedio en ms
    pub avg_latency_ms: f64,
}

impl BootstrapNode {
    pub fn new(address: Multiaddr, source: BootstrapSource) -> Self {
        let peer_id = address
            .iter()
            .find_map(|p| match p {
                Protocol::P2p(pid) => Some(pid),
                _ => None,
            })
            .unwrap_or_else(|| {
                // Generar PeerId aleatorio si no está en el address
                PeerId::random()
            });

        Self {
            address,
            peer_id,
            source,
            last_checked: None,
            status: BootstrapStatus::Unknown,
            avg_latency_ms: 0.0,
        }
    }

    /// Marca nodo como activo después de verificación
    pub fn mark_active(&mut self, latency_ms: f64) {
        self.status = BootstrapStatus::Active;
        self.last_checked = Some(Instant::now());
        // Promedio exponencial
        if self.avg_latency_ms > 0.0 {
            self.avg_latency_ms = 0.9 * self.avg_latency_ms + 0.1 * latency_ms;
        } else {
            self.avg_latency_ms = latency_ms;
        }
    }

    /// Marca nodo como inactivo
    pub fn mark_inactive(&mut self) {
        self.status = BootstrapStatus::Inactive;
        self.last_checked = Some(Instant::now());
    }

    /// Verifica si el nodo fue verificado recientemente (últimos 5 min)
    pub fn is_recently_checked(&self) -> bool {
        self.last_checked
            .map(|t| t.elapsed() < Duration::from_secs(300))
            .unwrap_or(false)
    }
}

/// Origen del nodo bootstrap
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapSource {
    Hardcoded,
    Dns,
    Config,
    PeerDiscovery,
}

impl std::fmt::Display for BootstrapSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapSource::Hardcoded => write!(f, "hardcoded"),
            BootstrapSource::Dns => write!(f, "dns"),
            BootstrapSource::Config => write!(f, "config"),
            BootstrapSource::PeerDiscovery => write!(f, "peer-discovery"),
        }
    }
}

/// Estado del nodo bootstrap
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapStatus {
    Unknown,
    Active,
    Inactive,
    Unreachable,
}

impl std::fmt::Display for BootstrapStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapStatus::Unknown => write!(f, "unknown"),
            BootstrapStatus::Active => write!(f, "active"),
            BootstrapStatus::Inactive => write!(f, "inactive"),
            BootstrapStatus::Unreachable => write!(f, "unreachable"),
        }
    }
}

/// Configuración de descubrimiento DNS
#[derive(Debug, Clone)]
pub struct DnsDiscoveryConfig {
    /// Records DNS para consultar (ej: "_ed2k._tcp.network.example.com")
    pub dns_records: Vec<String>,
    /// Intervalo de consulta en segundos
    pub query_interval_secs: u64,
    /// Timeout por consulta en segundos
    pub query_timeout_secs: u64,
}

impl Default for DnsDiscoveryConfig {
    fn default() -> Self {
        Self {
            dns_records: vec![],
            query_interval_secs: 300,
            query_timeout_secs: 10,
        }
    }
}

/// Configuración de AutoNAT
#[derive(Debug, Clone)]
pub struct AutoNatConfig {
    /// Habilitar AutoNAT
    pub enabled: bool,
    /// Intervalo de verificación NAT en segundos
    pub throttle_interval_secs: u64,
    /// Usar peers de la red como servidores AutoNAT
    pub use_peers_as_servers: bool,
}

impl Default for AutoNatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            throttle_interval_secs: 60,
            use_peers_as_servers: true,
        }
    }
}

/// Manager de nodos bootstrap
pub struct BootstrapManager {
    /// Nodos bootstrap registrados
    nodes: RwLock<HashMap<PeerId, BootstrapNode>>,
    /// Configuración DNS
    dns_config: RwLock<DnsDiscoveryConfig>,
    /// Configuración AutoNAT
    autonat_config: RwLock<AutoNatConfig>,
    /// Versión del protocolo
    protocol_version: String,
    /// Intervalo de verificación de nodos
    check_interval: Duration,
}

impl BootstrapManager {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();

        // Cargar nodos bootstrap por defecto con PeerId generados dinámicamente
        for &port in DEFAULT_BOOTSTRAP_PORTS {
            let peer_id = PeerId::random();
            let address: Multiaddr = [
                Protocol::Ip4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                Protocol::Tcp(port),
                Protocol::P2p(peer_id),
            ]
            .into_iter()
            .collect();
            let node = BootstrapNode::new(address, BootstrapSource::Hardcoded);
            nodes.insert(node.peer_id, node);
        }

        Self {
            nodes: RwLock::new(nodes),
            dns_config: RwLock::new(DnsDiscoveryConfig::default()),
            autonat_config: RwLock::new(AutoNatConfig::default()),
            protocol_version: ED2K_PROTOCOL_VERSION.to_string(),
            check_interval: Duration::from_secs(300),
        }
    }

    /// Crea manager con nodos bootstrap personalizados
    pub fn with_bootstrap_nodes(addresses: Vec<String>) -> Self {
        let manager = Self::new();

        for addr_str in addresses {
            if let Ok(address) = addr_str.parse::<Multiaddr>() {
                let node = BootstrapNode::new(address, BootstrapSource::Config);
                manager.nodes.write().insert(node.peer_id, node);
            }
        }

        manager
    }

    /// Configura descubrimiento DNS
    pub fn set_dns_discovery(&self, config: DnsDiscoveryConfig) {
        *self.dns_config.write() = config;
    }

    /// Configura AutoNAT
    pub fn set_autonat_config(&self, config: AutoNatConfig) {
        *self.autonat_config.write() = config;
    }

    /// Agrega nodo bootstrap desde string address
    pub fn add_bootstrap_node(
        &self,
        address_str: &str,
        source: BootstrapSource,
    ) -> Result<(), String> {
        let address = address_str
            .parse::<Multiaddr>()
            .map_err(|e| format!("Invalid multiaddr: {}", e))?;

        let node = BootstrapNode::new(address, source);
        // FIX: borrow/move - Clone node before inserting into map
        self.nodes.write().insert(node.peer_id, node.clone());
        info!(
            peer = %node.peer_id,
            source = %source,
            "Bootstrap node added"
        );
        Ok(())
    }

    /// Obtiene nodos activos para conexión inicial
    pub fn get_active_nodes(&self) -> Vec<BootstrapNode> {
        self.nodes
            .read()
            .values()
            .filter(|n| n.status == BootstrapStatus::Active)
            .cloned()
            .collect()
    }

    /// Obtiene todos los nodos (activos e inactivos)
    pub fn get_all_nodes(&self) -> Vec<BootstrapNode> {
        self.nodes.read().values().cloned().collect()
    }

    /// Obtiene nodos que necesitan verificación
    pub fn get_nodes_to_check(&self) -> Vec<BootstrapNode> {
        self.nodes
            .read()
            .values()
            .filter(|n| !n.is_recently_checked())
            .cloned()
            .collect()
    }

    /// Marca nodo como activo
    pub fn mark_node_active(&self, peer_id: &PeerId, latency_ms: f64) {
        if let Some(node) = self.nodes.write().get_mut(peer_id) {
            node.mark_active(latency_ms);
            debug!(
                peer = %peer_id,
                latency_ms,
                "Bootstrap node marked active"
            );
        }
    }

    /// Marca nodo como inactivo
    pub fn mark_node_inactive(&self, peer_id: &PeerId) {
        if let Some(node) = self.nodes.write().get_mut(peer_id) {
            node.mark_inactive();
            warn!(peer = %peer_id, "Bootstrap node marked inactive");
        }
    }

    /// Simula descubrimiento DNS (placeholder para integración real)
    pub async fn discover_dns_peers(&self) -> Vec<BootstrapNode> {
        let dns_config = self.dns_config.read();
        let discovered = Vec::new();

        for record in &dns_config.dns_records {
            debug!(record, "Querying DNS for bootstrap peers");
            // TODO: Phase 5 - Integración real con DNS-SD / mDNS
            // Por ahora, placeholder que no rompe la compilación
        }

        discovered
    }

    /// Obtiene configuración de protocolo para `identify`
    pub fn get_protocol_info(&self) -> ProtocolInfo {
        ProtocolInfo {
            protocol_version: self.protocol_version.clone(),
            protocol_prefix: ED2K_PROTOCOL_PREFIX.to_string(),
            supported_protocols: vec![
                format!("{}/tensor-request/1.0.0", ED2K_PROTOCOL_PREFIX),
                format!("{}/gossipsub/1.1.0", ED2K_PROTOCOL_PREFIX),
                format!("{}/consensus/1.0.0", ED2K_PROTOCOL_PREFIX),
                format!("{}/feedback/1.0.0", ED2K_PROTOCOL_PREFIX),
            ],
        }
    }

    /// Obtiene estadísticas
    pub fn stats(&self) -> BootstrapStats {
        let nodes = self.nodes.read();
        let total = nodes.len();
        let active = nodes
            .values()
            .filter(|n| n.status == BootstrapStatus::Active)
            .count();
        let inactive = nodes
            .values()
            .filter(|n| n.status == BootstrapStatus::Inactive)
            .count();
        let unknown = nodes
            .values()
            .filter(|n| n.status == BootstrapStatus::Unknown)
            .count();

        BootstrapStats {
            total,
            active,
            inactive,
            unknown,
            protocol_version: self.protocol_version.clone(),
            autonat_enabled: self.autonat_config.read().enabled,
            dns_records: self.dns_config.read().dns_records.len(),
        }
    }
}

/// Información del protocolo para `identify`
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    pub protocol_version: String,
    pub protocol_prefix: String,
    pub supported_protocols: Vec<String>,
}

/// Estadísticas del BootstrapManager
#[derive(Debug)]
pub struct BootstrapStats {
    pub total: usize,
    pub active: usize,
    pub inactive: usize,
    pub unknown: usize,
    pub protocol_version: String,
    pub autonat_enabled: bool,
    pub dns_records: usize,
}

impl Default for BootstrapManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_manager_creation() {
        let manager = BootstrapManager::new();
        let stats = manager.stats();
        assert!(stats.total > 0);
        assert_eq!(stats.protocol_version, ED2K_PROTOCOL_VERSION);
    }

    #[test]
    fn test_add_bootstrap_node() {
        let manager = BootstrapManager::new();
        let peer_id = PeerId::random();
        let addr = format!("/ip4/192.168.1.1/tcp/9000/p2p/{}", peer_id);
        let result = manager.add_bootstrap_node(&addr, BootstrapSource::Config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_protocol_info() {
        let manager = BootstrapManager::new();
        let info = manager.get_protocol_info();
        assert_eq!(info.protocol_version, ED2K_PROTOCOL_VERSION);
        assert!(!info.supported_protocols.is_empty());
    }

    #[test]
    fn test_bootstrap_node_status() {
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/9000".parse().unwrap();
        let mut node = BootstrapNode::new(addr, BootstrapSource::Hardcoded);
        assert_eq!(node.status, BootstrapStatus::Unknown);

        node.mark_active(50.0);
        assert_eq!(node.status, BootstrapStatus::Active);
        assert!(node.is_recently_checked());

        node.mark_inactive();
        assert_eq!(node.status, BootstrapStatus::Inactive);
    }
}
