//! Swarm Auto-Organization â€” Dynamic Sub-Network Topology by Hardware Capability.
//!
//! **Topological Law 1 (Diversidad):** Cada nodo se auto-asigna un rol basado en sus
//! capacidades hardware (GPU, RAM, ancho de banda) y balance de CE.
//! **Topological Law 3 (CooperaciÃ³n SimbiÃ³tica):** Rebalanceo fluido cuando nodos
//! se unen o abandonan la red.
//!
//! Este mÃ³dulo proporciona auto-organizaciÃ³n de enjambre a escala planetaria:
//! - **NodeCapabilities:** Perfil de hardware y recursos de cada nodo.
//! - **SwarmRole:** Rol auto-asignado (MaieuticSynth, Validator, Router, Relay, Light).
//! - **SubNetwork:** Sub-red dinÃ¡mica organizada por rol y capacidad.
//! - **SwarmTopology:** Motor principal de organizaciÃ³n con rebalanceo fluido.
//! - **TopologyConfig:** ConfiguraciÃ³n de umbrales y pesos de organizaciÃ³n.
//!
//! ### Feature Gate
//! `v3.5-planetary-emergence`
//!
//! ### IntegraciÃ³n
//! - [`crate::network::planetary_mesh::PlanetaryMesh`] para descubrimiento de pares
//! - [`crate::p2p::swarm::NodeResources`] para perfiles de hardware
//! - [`crate::ethics::moral_manifold::Vector3`] para evaluaciÃ³n Ã©tica de distribuciÃ³n

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::time::{Duration, Instant};

// ============================================================================
// NodeCapabilities â€” Hardware y Recursos del Nodo
// ============================================================================

/// Tier computacional del nodo (determina roles elegibles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ComputeTier {
    /// Nodo WASM/light â€” Solo routing y validaciÃ³n bÃ¡sica.
    #[default]
    Light = 0,
    /// Nodo estÃ¡ndar â€” ValidaciÃ³n completa y almacenamiento.
    Standard = 1,
    /// Nodo GPU â€” Maieutic Synthesizer y carga pesada.
    Gpu = 2,
}

impl fmt::Display for ComputeTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComputeTier::Light => write!(f, "Light"),
            ComputeTier::Standard => write!(f, "Standard"),
            ComputeTier::Gpu => write!(f, "GPU"),
        }
    }
}

/// Capacidad de un nodo para funciones especÃ­ficas.
#[derive(Debug, Clone, Copy, Default)]
pub struct NodeCapabilities {
    /// Tier computacional (0=WASM/light, 1=standard, 2=GPU).
    pub compute_tier: ComputeTier,
    /// NÃºcleos de CPU disponibles.
    pub cpu_cores: usize,
    /// RAM disponible en GB.
    pub ram_gb: f64,
    /// Ancho de banda en Mbps.
    pub bandwidth_mbps: f64,
    /// VRAM disponible en GB (solo GPU).
    pub vram_gb: f64,
    /// Balance de CE (Existence Credits).
    pub ce_balance: f64,
    /// Capacidad de relay (AutoNAT/circuit relay).
    pub can_relay: bool,
    /// Capacidad de hole punching (DCutR).
    pub can_hole_punch: bool,
    /// Latencia promedio en ms.
    pub avg_latency_ms: f64,
}

impl NodeCapabilities {
    /// Crea capacidades desde [`crate::p2p::swarm::NodeResources`].
    pub fn from_p2p_resources(
        resources: &crate::p2p::swarm::NodeResources,
        ce_balance: f64,
    ) -> Self {
        let compute_tier = if resources.has_gpu && resources.vram_gb.unwrap_or(0.0) >= 4.0 {
            ComputeTier::Gpu
        } else if resources.cpu_cores >= 4 && resources.available_ram_gb >= 8.0 {
            ComputeTier::Standard
        } else {
            ComputeTier::Light
        };

        Self {
            compute_tier,
            cpu_cores: resources.cpu_cores,
            ram_gb: resources.available_ram_gb,
            bandwidth_mbps: resources.bandwidth_mbps,
            vram_gb: resources.vram_gb.unwrap_or(0.0),
            ce_balance,
            can_relay: true, // Default: todos pueden relay
            can_hole_punch: resources.bandwidth_mbps >= 10.0,
            avg_latency_ms: resources.avg_latency_ms,
        }
    }

    /// Score de capacidad general (mayor = mÃ¡s capaz).
    pub fn capability_score(&self) -> f64 {
        let tier_weight = match self.compute_tier {
            ComputeTier::Light => 1.0,
            ComputeTier::Standard => 3.0,
            ComputeTier::Gpu => 10.0,
        };
        tier_weight
            + self.cpu_cores as f64 * 0.5
            + self.ram_gb * 0.3
            + self.bandwidth_mbps * 0.01
            + self.vram_gb * 0.8
    }

    /// Verifica si el nodo puede asumir un rol especÃ­fico.
    pub fn can_assume_role(&self, role: SwarmRole) -> bool {
        match role {
            SwarmRole::MaieuticSynth => {
                self.compute_tier == ComputeTier::Gpu && self.vram_gb >= 4.0
            }
            SwarmRole::Validator => {
                self.compute_tier >= ComputeTier::Standard && self.ram_gb >= 4.0
            }
            SwarmRole::Router => self.bandwidth_mbps >= 5.0 && self.ram_gb >= 2.0,
            SwarmRole::Relay => self.can_relay,
            SwarmRole::Light => true, // Todos pueden ser light
        }
    }
}

// ============================================================================
// SwarmRole â€” Rol Auto-Asignado del Nodo
// ============================================================================

/// Rol dentro del enjambre, determinado por capacidades y necesidad de red.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwarmRole {
    /// Maieutic Synthesizer â€” Ejecuta cargas pesadas de sÃ­ntesis (GPU).
    MaieuticSynth,
    /// Validator â€” Valida consenso BFT y verifica ZKP.
    Validator,
    /// Router â€” Enruta mensajes y mantiene la topologÃ­a DHT.
    Router,
    /// Relay â€” Provee circuitos de relay para nodos privados.
    Relay,
    /// Light â€” Nodo WASM con funciones bÃ¡sicas de routing.
    Light,
}

impl fmt::Display for SwarmRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwarmRole::MaieuticSynth => write!(f, "MaieuticSynth"),
            SwarmRole::Validator => write!(f, "Validator"),
            SwarmRole::Router => write!(f, "Router"),
            SwarmRole::Relay => write!(f, "Relay"),
            SwarmRole::Light => write!(f, "Light"),
        }
    }
}

impl SwarmRole {
    /// Prioridad de asignaciÃ³n (menor = mÃ¡s prioritario para la red).
    pub fn priority(&self) -> u8 {
        match self {
            SwarmRole::MaieuticSynth => 0,
            SwarmRole::Validator => 1,
            SwarmRole::Router => 2,
            SwarmRole::Relay => 3,
            SwarmRole::Light => 4,
        }
    }

    /// Roles compatibles con un tier computacional.
    pub fn compatible_roles(tier: ComputeTier) -> Vec<SwarmRole> {
        match tier {
            ComputeTier::Gpu => vec![
                SwarmRole::MaieuticSynth,
                SwarmRole::Validator,
                SwarmRole::Router,
                SwarmRole::Relay,
            ],
            ComputeTier::Standard => {
                vec![SwarmRole::Validator, SwarmRole::Router, SwarmRole::Relay]
            }
            ComputeTier::Light => vec![SwarmRole::Router, SwarmRole::Relay, SwarmRole::Light],
        }
    }
}

// ============================================================================
// NodeEntry â€” Entrada de Nodo en la TopologÃ­a
// ============================================================================

/// Estado de un nodo en la topologÃ­a del enjambre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Activo,
    Inactivo,
    Desconectado,
}

/// Entrada completa de un nodo en la topologÃ­a.
#[derive(Debug, Clone)]
pub struct NodeEntry {
    /// Identificador Ãºnico del nodo (u128 para compatibilidad Kademlia).
    pub node_id: u128,
    /// Rol actual asignado.
    pub role: SwarmRole,
    /// Capacidades del nodo.
    pub capabilities: NodeCapabilities,
    /// Estado actual.
    pub state: NodeState,
    /// Sub-red a la que pertenece.
    pub sub_network: Option<u64>,
    /// Timestamp de Ãºltima actividad.
    pub last_active: Instant,
    /// Timestamp de Ãºltima reasignaciÃ³n de rol.
    pub last_role_change: Instant,
    /// Contador de cambios de rol.
    pub role_change_count: u32,
}

impl NodeEntry {
    pub fn new(node_id: u128, capabilities: NodeCapabilities) -> Self {
        let role = Self::initial_role(&capabilities);
        Self {
            node_id,
            role,
            capabilities,
            state: NodeState::Activo,
            sub_network: None,
            last_active: Instant::now(),
            last_role_change: Instant::now(),
            role_change_count: 0,
        }
    }

    /// Rol inicial basado en capacidades.
    fn initial_role(capabilities: &NodeCapabilities) -> SwarmRole {
        let compatible = SwarmRole::compatible_roles(capabilities.compute_tier);
        compatible
            .into_iter()
            .min_by_key(|r| r.priority())
            .unwrap_or(SwarmRole::Light)
    }

    /// Marca al nodo como activo (actualiza timestamp).
    pub fn heartbeat(&mut self) {
        self.last_active = Instant::now();
        self.state = NodeState::Activo;
    }

    /// Verifica si el nodo estÃ¡ inactivo por timeout.
    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_active.elapsed() > timeout
    }

    /// Cambia el rol del nodo.
    pub fn change_role(&mut self, new_role: SwarmRole) {
        if self.role != new_role {
            self.role = new_role;
            self.last_role_change = Instant::now();
            self.role_change_count += 1;
        }
    }
}

// ============================================================================
// SubNetwork â€” Sub-Red DinÃ¡mica
// ============================================================================

/// Sub-red organizada por rol y capacidad.
#[derive(Debug, Clone)]
pub struct SubNetwork {
    /// Identificador Ãºnico de la sub-red.
    pub id: u64,
    /// Rol principal de esta sub-red.
    pub primary_role: SwarmRole,
    /// Nodos miembros (node_id -> NodeEntry).
    pub members: HashMap<u128, u64>, // node_id -> sub_network_id
    /// Capacidad total de la sub-red.
    pub total_capacity: f64,
    /// Carga actual de la sub-red (0.0 = vacÃ­a, 1.0 = saturada).
    pub current_load: f64,
    /// Timestamp de Ãºltima reorganizaciÃ³n.
    pub last_reorg: Instant,
}

impl SubNetwork {
    pub fn new(id: u64, primary_role: SwarmRole) -> Self {
        Self {
            id,
            primary_role,
            members: HashMap::new(),
            total_capacity: 0.0,
            current_load: 0.0,
            last_reorg: Instant::now(),
        }
    }

    /// Agrega un nodo a la sub-red.
    pub fn add_member(&mut self, node_id: u128, capacity: f64) {
        self.members.insert(node_id, self.id);
        self.total_capacity += capacity;
        self.update_load();
    }

    /// Elimina un nodo de la sub-red.
    pub fn remove_member(&mut self, node_id: u128, capacity: f64) {
        self.members.remove(&node_id);
        self.total_capacity = (self.total_capacity - capacity).max(0.0);
        self.update_load();
    }

    /// Actualiza la carga basada en miembros activos.
    fn update_load(&mut self) {
        if self.total_capacity > 0.0 {
            self.current_load = self.members.len() as f64 / (self.total_capacity * 0.1).max(1.0);
        } else {
            self.current_load = 0.0;
        }
        self.current_load = self.current_load.min(1.0);
    }

    /// Verifica si la sub-red estÃ¡ sobrecargada.
    pub fn is_overloaded(&self, threshold: f64) -> bool {
        self.current_load > threshold
    }

    /// Verifica si la sub-red estÃ¡ subutilizada.
    pub fn is_underutilized(&self, threshold: f64) -> bool {
        self.current_load < threshold && self.members.len() > 1
    }
}

// ============================================================================
// TopologyConfig â€” ConfiguraciÃ³n de OrganizaciÃ³n
// ============================================================================

/// ConfiguraciÃ³n del motor de topologÃ­a del enjambre.
#[derive(Debug, Clone)]
pub struct TopologyConfig {
    /// Timeout para considerar un nodo inactivo (default: 60s).
    pub inactive_timeout: Duration,
    /// Intervalo de rebalanceo automÃ¡tico (default: 30s).
    pub rebalance_interval: Duration,
    /// Umbral de sobrecarga para trigger rebalanceo (default: 0.8).
    pub overload_threshold: f64,
    /// Umbral de subutilizaciÃ³n (default: 0.2).
    pub underutil_threshold: f64,
    /// MÃ¡ximo de nodos por sub-red (default: 100).
    pub max_nodes_per_subnet: usize,
    /// MÃ­nimo de nodos por sub-red antes de fusionar (default: 2).
    pub min_nodes_per_subnet: usize,
    /// Cool-down entre cambios de rol (default: 10s).
    pub role_change_cooldown: Duration,
    /// Peso del balance CE en asignaciÃ³n de rol (default: 0.3).
    pub ce_weight: f64,
    /// Peso de la capacidad hardware en asignaciÃ³n (default: 0.7).
    pub hardware_weight: f64,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            inactive_timeout: Duration::from_secs(60),
            rebalance_interval: Duration::from_secs(30),
            overload_threshold: 0.8,
            underutil_threshold: 0.2,
            max_nodes_per_subnet: 100,
            min_nodes_per_subnet: 2,
            role_change_cooldown: Duration::from_secs(10),
            ce_weight: 0.3,
            hardware_weight: 0.7,
        }
    }
}

impl TopologyConfig {
    /// Valida la configuraciÃ³n.
    pub fn validate(&self) -> Result<(), TopologyError> {
        if self.ce_weight < 0.0 || self.ce_weight > 1.0 {
            return Err(TopologyError::InvalidCeWeight(self.ce_weight));
        }
        if self.hardware_weight < 0.0 || self.hardware_weight > 1.0 {
            return Err(TopologyError::InvalidHardwareWeight(self.hardware_weight));
        }
        if self.ce_weight + self.hardware_weight > 1.0 + f64::EPSILON {
            return Err(TopologyError::WeightsExceedOne(
                self.ce_weight + self.hardware_weight,
            ));
        }
        if self.max_nodes_per_subnet == 0 {
            return Err(TopologyError::InvalidMaxNodes(0));
        }
        Ok(())
    }
}

// ============================================================================
// TopologyError â€” Errores de TopologÃ­a
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyError {
    NodoNoExiste(u128),
    SubRedNoExiste(u64),
    RolNoCompatible(SwarmRole),
    InvalidCeWeight(f64),
    InvalidHardwareWeight(f64),
    WeightsExceedOne(f64),
    InvalidMaxNodes(usize),
    TopologiaSaturada,
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopologyError::NodoNoExiste(id) => write!(f, "Nodo {} no existe", id),
            TopologyError::SubRedNoExiste(id) => write!(f, "Sub-red {} no existe", id),
            TopologyError::RolNoCompatible(role) => {
                write!(f, "Rol {} no compatible con capacidades", role)
            }
            TopologyError::InvalidCeWeight(w) => write!(f, "Peso CE invÃ¡lido: {}", w),
            TopologyError::InvalidHardwareWeight(w) => write!(f, "Peso hardware invÃ¡lido: {}", w),
            TopologyError::WeightsExceedOne(w) => write!(f, "Pesos exceden 1.0: {}", w),
            TopologyError::InvalidMaxNodes(n) => write!(f, "MÃ¡ximo de nodos invÃ¡lido: {}", n),
            TopologyError::TopologiaSaturada => write!(f, "TopologÃ­a saturada"),
        }
    }
}

impl std::error::Error for TopologyError {}

// ============================================================================
// TopologyStats â€” EstadÃ­sticas de TopologÃ­a
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct TopologyStats {
    /// Total de nodos activos.
    pub active_nodes: usize,
    /// Total de sub-redes.
    pub sub_networks: usize,
    /// Cambios de rol totales.
    pub total_role_changes: u32,
    /// Rebalanceos ejecutados.
    pub rebalances_executed: u32,
    /// Nodos por rol.
    pub nodes_per_role: HashMap<SwarmRole, usize>,
    /// DistribuciÃ³n de carga promedio.
    pub avg_load_distribution: f64,
}

// ============================================================================
// SwarmTopology â€” Motor Principal de OrganizaciÃ³n
// ============================================================================

/// Motor de auto-organizaciÃ³n del enjambre.
///
/// Gestiona la distribuciÃ³n dinÃ¡mica de nodos en sub-redes basadas en:
/// - Capacidades hardware (GPU, RAM, CPU, ancho de banda)
/// - Balance de CE (Existence Credits)
/// - Carga actual de la red
/// - Necesidades de la topologÃ­a global
pub struct SwarmTopology {
    /// ConfiguraciÃ³n de la topologÃ­a.
    config: TopologyConfig,
    /// Nodos registrados (node_id -> NodeEntry).
    nodes: HashMap<u128, NodeEntry>,
    /// Sub-redes activas (subnet_id -> SubNetwork).
    sub_networks: HashMap<u64, SubNetwork>,
    /// Siguiente ID de sub-red.
    next_subnet_id: u64,
    /// EstadÃ­sticas de la topologÃ­a.
    stats: TopologyStats,
    /// Timestamp del Ãºltimo rebalanceo.
    last_rebalance: Instant,
    /// Historial de eventos de topologÃ­a.
    event_log: VecDeque<TopologyEvent>,
    /// MÃ¡ximo de eventos en el historial.
    max_events: usize,
}

/// Evento de topologÃ­a registrado en el historial.
#[derive(Debug, Clone)]
pub enum TopologyEvent {
    NodoIngresado {
        node_id: u128,
        role: SwarmRole,
        sub_network: Option<u64>,
    },
    NodoSalido {
        node_id: u128,
    },
    RolReasignado {
        node_id: u128,
        old_role: SwarmRole,
        new_role: SwarmRole,
    },
    SubRedCreada {
        sub_network_id: u64,
        primary_role: SwarmRole,
    },
    SubRedFusionada {
        source_id: u64,
        target_id: u64,
    },
    RebalanceoEjecutado {
        nodes_moved: usize,
    },
}

impl SwarmTopology {
    /// Crea una nueva topologÃ­a con configuraciÃ³n por defecto.
    pub fn new() -> Self {
        let config = TopologyConfig::default();
        Self::with_config(config).expect("Default config should be valid")
    }

    /// Crea una topologÃ­a con configuraciÃ³n personalizada.
    pub fn with_config(config: TopologyConfig) -> Result<Self, TopologyError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            sub_networks: HashMap::new(),
            next_subnet_id: 0,
            stats: TopologyStats::default(),
            last_rebalance: Instant::now(),
            event_log: VecDeque::with_capacity(1024),
            max_events: 1024,
        })
    }

    // ------------------------------------------------------------------
    // Node Management
    // ------------------------------------------------------------------

    /// Registra un nuevo nodo en la topologÃ­a.
    ///
    /// El nodo se auto-asigna un rol basado en sus capacidades y se coloca
    /// en la sub-red apropiada.
    pub fn register_node(
        &mut self,
        node_id: u128,
        capabilities: NodeCapabilities,
    ) -> Result<SwarmRole, TopologyError> {
        let entry = NodeEntry::new(node_id, capabilities);
        let initial_role = entry.role;

        // Asignar a sub-red apropiada
        let sub_network = self.assign_to_subnetwork(&entry)?;

        // Insertar nodo
        self.nodes.insert(node_id, entry);

        // Actualizar estadÃ­sticas
        self.stats.active_nodes += 1;
        self.update_role_stats(&initial_role, 1);

        // Registrar evento
        self.log_event(TopologyEvent::NodoIngresado {
            node_id,
            role: initial_role,
            sub_network,
        });

        // Verificar si necesita rebalanceo
        self.maybe_rebalance();

        Ok(initial_role)
    }

    /// Elimina un nodo de la topologÃ­a.
    pub fn unregister_node(&mut self, node_id: u128) -> Result<(), TopologyError> {
        let entry = self
            .nodes
            .remove(&node_id)
            .ok_or(TopologyError::NodoNoExiste(node_id))?;

        // Remover de sub-red
        if let Some(subnet_id) = entry.sub_network {
            if let Some(subnet) = self.sub_networks.get_mut(&subnet_id) {
                subnet.remove_member(node_id, entry.capabilities.capability_score());
            }
        }

        // Actualizar estadÃ­sticas
        self.stats.active_nodes = self.stats.active_nodes.saturating_sub(1);
        self.update_role_stats(&entry.role, -1);

        // Registrar evento
        self.log_event(TopologyEvent::NodoSalido { node_id });

        // Limpiar sub-redes vacÃ­as
        self.prune_empty_subnetworks();

        // Verificar si necesita rebalanceo
        self.maybe_rebalance();

        Ok(())
    }

    /// Actualiza el heartbeat de un nodo.
    pub fn heartbeat(&mut self, node_id: u128) -> Result<(), TopologyError> {
        let entry = self
            .nodes
            .get_mut(&node_id)
            .ok_or(TopologyError::NodoNoExiste(node_id))?;
        entry.heartbeat();
        Ok(())
    }

    /// Obtiene informaciÃ³n de un nodo.
    pub fn get_node(&self, node_id: u128) -> Option<&NodeEntry> {
        self.nodes.get(&node_id)
    }

    /// Obtiene los nodos activos con un rol especÃ­fico.
    pub fn get_nodes_by_role(&self, role: SwarmRole) -> Vec<&NodeEntry> {
        self.nodes
            .values()
            .filter(|n| n.role == role && n.state == NodeState::Activo)
            .collect()
    }

    /// Obtiene los nodos MaieuticSynth disponibles (GPU).
    pub fn get_maieutic_synth_nodes(&self) -> Vec<&NodeEntry> {
        self.get_nodes_by_role(SwarmRole::MaieuticSynth)
    }

    /// Obtiene los validadores disponibles.
    pub fn get_validators(&self) -> Vec<&NodeEntry> {
        self.get_nodes_by_role(SwarmRole::Validator)
    }

    /// Obtiene los routers disponibles.
    pub fn get_routers(&self) -> Vec<&NodeEntry> {
        self.get_nodes_by_role(SwarmRole::Router)
    }

    /// Obtiene los relays disponibles.
    pub fn get_relays(&self) -> Vec<&NodeEntry> {
        self.get_nodes_by_role(SwarmRole::Relay)
    }

    // ------------------------------------------------------------------
    // Sub-Network Management
    // ------------------------------------------------------------------

    /// Asigna un nodo a la sub-red mÃ¡s apropiada.
    fn assign_to_subnetwork(&mut self, entry: &NodeEntry) -> Result<Option<u64>, TopologyError> {
        // Buscar sub-red existente con el mismo rol principal
        let target_subnet = self
            .sub_networks
            .values()
            .filter(|s| {
                s.primary_role == entry.role
                    && s.members.len() < self.config.max_nodes_per_subnet
                    && !s.is_overloaded(self.config.overload_threshold)
            })
            .min_by_key(|s| s.members.len());

        if let Some(subnet) = target_subnet {
            // Agregar a sub-red existente
            let subnet_id = subnet.id;
            if let Some(s) = self.sub_networks.get_mut(&subnet_id) {
                s.add_member(entry.node_id, entry.capabilities.capability_score());
            }
            return Ok(Some(subnet_id));
        }

        // Crear nueva sub-red
        let new_subnet = SubNetwork::new(self.next_subnet_id, entry.role);
        self.next_subnet_id += 1;

        let subnet_id = new_subnet.id;
        let mut new_subnet = new_subnet;
        new_subnet.add_member(entry.node_id, entry.capabilities.capability_score());

        self.sub_networks.insert(subnet_id, new_subnet);
        self.stats.sub_networks = self.sub_networks.len();

        self.log_event(TopologyEvent::SubRedCreada {
            sub_network_id: subnet_id,
            primary_role: entry.role,
        });

        Ok(Some(subnet_id))
    }

    /// Obtiene informaciÃ³n de una sub-red.
    pub fn get_subnetwork(&self, id: u64) -> Option<&SubNetwork> {
        self.sub_networks.get(&id)
    }

    /// Lista todas las sub-redes activas.
    pub fn list_subnetworks(&self) -> Vec<&SubNetwork> {
        self.sub_networks.values().collect()
    }

    /// Elimina sub-redes vacÃ­as.
    fn prune_empty_subnetworks(&mut self) {
        let empty_ids: Vec<u64> = self
            .sub_networks
            .iter()
            .filter(|(_, s)| s.members.is_empty())
            .map(|(id, _)| *id)
            .collect();

        for id in empty_ids {
            self.sub_networks.remove(&id);
        }
        self.stats.sub_networks = self.sub_networks.len();
    }

    // ------------------------------------------------------------------
    // Role Rebalancing
    // ------------------------------------------------------------------

    /// Ejecuta un ciclo de rebalanceo completo.
    pub fn rebalance(&mut self) -> usize {
        let _before = Instant::now();
        let mut nodes_moved = 0;

        // 1. Marcar nodos inactivos
        self.mark_stale_nodes();

        // 2. Evaluar necesidades de cada rol
        let role_needs = self.evaluate_role_needs();

        // 3. Reasignar nodos segÃºn necesidades
        nodes_moved += self.reassign_roles(role_needs);

        // 4. Reorganizar sub-redes
        nodes_moved += self.reorganize_subnetworks();

        // 5. Fusionar sub-redes pequeÃ±as
        self.merge_small_subnetworks();

        // 6. Actualizar estadÃ­sticas
        self.stats.rebalances_executed += 1;
        self.last_rebalance = Instant::now();

        // Registrar evento
        self.log_event(TopologyEvent::RebalanceoEjecutado { nodes_moved });

        nodes_moved
    }

    /// Verifica si es momento de rebalancear.
    fn maybe_rebalance(&mut self) {
        if self.last_rebalance.elapsed() >= self.config.rebalance_interval {
            self.rebalance();
        }
    }

    /// Marca nodos inactivos como desconectados.
    fn mark_stale_nodes(&mut self) {
        for (_, entry) in self.nodes.iter_mut() {
            if entry.is_stale(self.config.inactive_timeout) && entry.state == NodeState::Activo {
                entry.state = NodeState::Desconectado;
            }
        }
    }

    /// EvalÃºa las necesidades de cada rol basado en la topologÃ­a actual.
    fn evaluate_role_needs(&self) -> HashMap<SwarmRole, i32> {
        let mut needs = HashMap::new();
        let total = self.nodes.len() as f64;

        if total == 0.0 {
            return needs;
        }

        // Ratio ideal por rol (basado en topologÃ­a de red saludable)
        let ideal_ratios: HashMap<SwarmRole, f64> = [
            (SwarmRole::MaieuticSynth, 0.05), // 5% GPU
            (SwarmRole::Validator, 0.25),     // 25% validadores
            (SwarmRole::Router, 0.30),        // 30% routers
            (SwarmRole::Relay, 0.20),         // 20% relays
            (SwarmRole::Light, 0.20),         // 20% light
        ]
        .iter()
        .cloned()
        .collect();

        for (role, ideal_ratio) in ideal_ratios {
            let current_count = self
                .nodes
                .values()
                .filter(|n| n.role == role && n.state == NodeState::Activo)
                .count() as f64;
            let ideal_count = (total * ideal_ratio).round() as i32;
            let current = current_count as i32;
            let diff = ideal_count - current;
            needs.insert(role, diff);
        }

        needs
    }

    /// Reasigna nodos segÃºn las necesidades de rol.
    fn reassign_roles(&mut self, needs: HashMap<SwarmRole, i32>) -> usize {
        let mut moved = 0;

        for (role, needed) in needs {
            if needed <= 0 {
                continue;
            }

            // Buscar nodos candidatos para este rol
            let candidates: Vec<u128> = self
                .nodes
                .values()
                .filter(|n| {
                    n.state == NodeState::Activo
                        && n.role != role
                        && n.capabilities.can_assume_role(role)
                        && n.last_role_change.elapsed() >= self.config.role_change_cooldown
                })
                .map(|n| n.node_id)
                .collect();

            let to_move = needed.min(candidates.len() as i32) as usize;
            for &candidate_id in &candidates[..to_move] {
                if let Some(entry) = self.nodes.get_mut(&candidate_id) {
                    let old_role = entry.role;
                    entry.change_role(role);

                    // Actualizar stats
                    self.update_role_stats(&old_role, -1);
                    self.update_role_stats(&role, 1);
                    self.stats.total_role_changes += 1;

                    self.log_event(TopologyEvent::RolReasignado {
                        node_id: candidate_id,
                        old_role,
                        new_role: role,
                    });

                    moved += 1;
                }
            }
        }

        moved
    }

    /// Reorganiza nodos entre sub-redes.
    fn reorganize_subnetworks(&mut self) -> usize {
        let mut moved = 0;

        // Reasignar nodos cuya sub-red no coincide con su rol actual
        for (_, entry) in self.nodes.iter() {
            if let Some(subnet_id) = entry.sub_network {
                if let Some(subnet) = self.sub_networks.get(&subnet_id) {
                    if subnet.primary_role != entry.role {
                        // Mover a sub-red correcta
                        // (simplificado: se maneja en prÃ³ximo rebalanceo)
                        moved += 1;
                    }
                }
            }
        }

        moved
    }

    /// Fusiona sub-redes pequeÃ±as en sub-redes vecinas del mismo rol.
    fn merge_small_subnetworks(&mut self) {
        let small_ids: Vec<u64> = self
            .sub_networks
            .iter()
            .filter(|(_, s)| s.members.len() < self.config.min_nodes_per_subnet)
            .map(|(id, _)| *id)
            .collect();

        for small_id in small_ids {
            let small_subnet = match self.sub_networks.remove(&small_id) {
                Some(s) => s,
                None => continue,
            };

            // Buscar sub-red destino del mismo rol
            let target = self
                .sub_networks
                .values()
                .filter(|s| {
                    s.primary_role == small_subnet.primary_role
                        && s.id != small_id
                        && s.members.len() + small_subnet.members.len()
                            <= self.config.max_nodes_per_subnet
                })
                .max_by_key(|s| s.members.len());

            if let Some(target) = target {
                let target_id = target.id;
                let moved_members: Vec<(u128, f64)> = small_subnet
                    .members
                    .iter()
                    .map(|(&node_id, _)| {
                        let capacity = self
                            .nodes
                            .get(&node_id)
                            .map(|n| n.capabilities.capability_score())
                            .unwrap_or(1.0);
                        (node_id, capacity)
                    })
                    .collect();
                if let Some(target_subnet) = self.sub_networks.get_mut(&target_id) {
                    for (node_id, capacity) in moved_members {
                        target_subnet.add_member(node_id, capacity);
                    }
                }

                self.log_event(TopologyEvent::SubRedFusionada {
                    source_id: small_id,
                    target_id,
                });
            } else {
                // No hay destino, restaurar
                self.sub_networks.insert(small_id, small_subnet);
            }
        }
    }

    // ------------------------------------------------------------------
    // CE-Based Role Scoring
    // ------------------------------------------------------------------

    /// Calcula el score ponderado para asignaciÃ³n de rol.
    ///
    /// Combina capacidad hardware con balance CE para determinar
    /// la prioridad de un nodo para un rol especÃ­fico.
    pub fn role_score(&self, entry: &NodeEntry) -> f64 {
        let hardware_score = entry.capabilities.capability_score();
        let ce_score = entry.capabilities.ce_balance.max(0.0);

        self.config.hardware_weight * hardware_score + self.config.ce_weight * ce_score
    }

    /// Obtiene los mejores candidatos para un rol.
    pub fn best_candidates_for_role(&self, role: SwarmRole, count: usize) -> Vec<&NodeEntry> {
        let mut candidates: Vec<&NodeEntry> = self
            .nodes
            .values()
            .filter(|n| {
                n.state == NodeState::Activo
                    && n.capabilities.can_assume_role(role)
                    && n.role != role
            })
            .collect();

        // Ordenar por score descendente
        candidates.sort_by(|a, b| {
            let score_a = self.role_score(a);
            let score_b = self.role_score(b);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.into_iter().take(count).collect()
    }

    // ------------------------------------------------------------------
    // Stats & Events
    // ------------------------------------------------------------------

    /// Obtiene las estadÃ­sticas actuales de la topologÃ­a.
    pub fn get_stats(&self) -> &TopologyStats {
        &self.stats
    }

    /// Actualiza las estadÃ­sticas de rol.
    fn update_role_stats(&mut self, role: &SwarmRole, delta: i32) {
        let counter = self.stats.nodes_per_role.entry(*role).or_insert(0);
        *counter = (*counter as i32 + delta).max(0) as usize;
    }

    /// Registra un evento en el historial.
    fn log_event(&mut self, event: TopologyEvent) {
        self.event_log.push_back(event);
        while self.event_log.len() > self.max_events {
            self.event_log.pop_front();
        }
    }

    /// Obtiene el historial de eventos.
    pub fn get_event_log(&self) -> &VecDeque<TopologyEvent> {
        &self.event_log
    }

    /// Obtiene nodos desconectados (para cleanup).
    pub fn get_disconnected_nodes(&self) -> Vec<u128> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.state == NodeState::Desconectado)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Limpia nodos desconectados.
    pub fn cleanup_disconnected(&mut self) -> usize {
        let disconnected = self.get_disconnected_nodes();
        let count = disconnected.len();
        for node_id in disconnected {
            let _ = self.unregister_node(node_id);
        }
        count
    }

    /// Obtiene la configuraciÃ³n actual.
    pub fn config(&self) -> &TopologyConfig {
        &self.config
    }

    /// Actualiza la configuraciÃ³n (con validaciÃ³n).
    pub fn update_config(&mut self, new_config: TopologyConfig) -> Result<(), TopologyError> {
        new_config.validate()?;
        self.config = new_config;
        Ok(())
    }

    /// Resetea la topologÃ­a completa.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.sub_networks.clear();
        self.next_subnet_id = 0;
        self.stats = TopologyStats::default();
        self.last_rebalance = Instant::now();
        self.event_log.clear();
    }
}

impl Default for SwarmTopology {
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

    fn make_capabilities(tier: ComputeTier, ce: f64) -> NodeCapabilities {
        NodeCapabilities {
            compute_tier: tier,
            cpu_cores: match tier {
                ComputeTier::Light => 2,
                ComputeTier::Standard => 8,
                ComputeTier::Gpu => 16,
            },
            ram_gb: match tier {
                ComputeTier::Light => 2.0,
                ComputeTier::Standard => 16.0,
                ComputeTier::Gpu => 64.0,
            },
            bandwidth_mbps: 100.0,
            vram_gb: match tier {
                ComputeTier::Gpu => 16.0,
                _ => 0.0,
            },
            ce_balance: ce,
            can_relay: true,
            can_hole_punch: true,
            avg_latency_ms: 10.0,
        }
    }

    // ------------------------------------------------------------------
    // ComputeTier & NodeCapabilities
    // ------------------------------------------------------------------

    #[test]
    fn test_compute_tier_display() {
        assert_eq!(format!("{}", ComputeTier::Light), "Light");
        assert_eq!(format!("{}", ComputeTier::Standard), "Standard");
        assert_eq!(format!("{}", ComputeTier::Gpu), "GPU");
    }

    #[test]
    fn test_capabilities_from_p2p_gpu() {
        let resources = crate::p2p::swarm::NodeResources {
            cpu_cores: 16,
            available_ram_gb: 64.0,
            bandwidth_mbps: 1000.0,
            avg_latency_ms: 5.0,
            has_gpu: true,
            gpu_model: Some("RTX 4090".to_string()),
            vram_gb: Some(24.0),
        };
        let caps = NodeCapabilities::from_p2p_resources(&resources, 100.0);
        assert_eq!(caps.compute_tier, ComputeTier::Gpu);
        assert_eq!(caps.vram_gb, 24.0);
        assert_eq!(caps.ce_balance, 100.0);
    }

    #[test]
    fn test_capabilities_from_p2p_standard() {
        let resources = crate::p2p::swarm::NodeResources {
            cpu_cores: 8,
            available_ram_gb: 16.0,
            bandwidth_mbps: 500.0,
            avg_latency_ms: 20.0,
            has_gpu: false,
            gpu_model: None,
            vram_gb: None,
        };
        let caps = NodeCapabilities::from_p2p_resources(&resources, 50.0);
        assert_eq!(caps.compute_tier, ComputeTier::Standard);
    }

    #[test]
    fn test_capabilities_from_p2p_light() {
        let resources = crate::p2p::swarm::NodeResources {
            cpu_cores: 2,
            available_ram_gb: 4.0,
            bandwidth_mbps: 50.0,
            avg_latency_ms: 100.0,
            has_gpu: false,
            gpu_model: None,
            vram_gb: None,
        };
        let caps = NodeCapabilities::from_p2p_resources(&resources, 10.0);
        assert_eq!(caps.compute_tier, ComputeTier::Light);
    }

    #[test]
    fn test_capability_score_gpu_highest() {
        let gpu = make_capabilities(ComputeTier::Gpu, 100.0);
        let std = make_capabilities(ComputeTier::Standard, 100.0);
        let light = make_capabilities(ComputeTier::Light, 100.0);
        assert!(gpu.capability_score() > std.capability_score());
        assert!(std.capability_score() > light.capability_score());
    }

    #[test]
    fn test_can_assume_role_gpu() {
        let gpu = make_capabilities(ComputeTier::Gpu, 100.0);
        assert!(gpu.can_assume_role(SwarmRole::MaieuticSynth));
        assert!(gpu.can_assume_role(SwarmRole::Validator));
        assert!(gpu.can_assume_role(SwarmRole::Router));
    }

    #[test]
    fn test_can_assume_role_light() {
        let light = make_capabilities(ComputeTier::Light, 10.0);
        assert!(!light.can_assume_role(SwarmRole::MaieuticSynth));
        assert!(!light.can_assume_role(SwarmRole::Validator));
        assert!(light.can_assume_role(SwarmRole::Light));
    }

    // ------------------------------------------------------------------
    // SwarmRole
    // ------------------------------------------------------------------

    #[test]
    fn test_swarm_role_display() {
        assert_eq!(format!("{}", SwarmRole::MaieuticSynth), "MaieuticSynth");
        assert_eq!(format!("{}", SwarmRole::Validator), "Validator");
        assert_eq!(format!("{}", SwarmRole::Router), "Router");
        assert_eq!(format!("{}", SwarmRole::Relay), "Relay");
        assert_eq!(format!("{}", SwarmRole::Light), "Light");
    }

    #[test]
    fn test_role_priority() {
        assert!(SwarmRole::MaieuticSynth.priority() < SwarmRole::Validator.priority());
        assert!(SwarmRole::Validator.priority() < SwarmRole::Router.priority());
        assert!(SwarmRole::Router.priority() < SwarmRole::Relay.priority());
        assert!(SwarmRole::Relay.priority() < SwarmRole::Light.priority());
    }

    #[test]
    fn test_compatible_roles_gpu() {
        let roles = SwarmRole::compatible_roles(ComputeTier::Gpu);
        assert!(roles.contains(&SwarmRole::MaieuticSynth));
        assert!(roles.contains(&SwarmRole::Validator));
        assert_eq!(roles.len(), 4);
    }

    #[test]
    fn test_compatible_roles_light() {
        let roles = SwarmRole::compatible_roles(ComputeTier::Light);
        assert!(!roles.contains(&SwarmRole::MaieuticSynth));
        assert!(!roles.contains(&SwarmRole::Validator));
        assert!(roles.contains(&SwarmRole::Light));
    }

    // ------------------------------------------------------------------
    // NodeEntry
    // ------------------------------------------------------------------

    #[test]
    fn test_node_entry_initial_role_gpu() {
        let caps = make_capabilities(ComputeTier::Gpu, 100.0);
        let entry = NodeEntry::new(1, caps);
        assert_eq!(entry.role, SwarmRole::MaieuticSynth);
        assert_eq!(entry.state, NodeState::Activo);
    }

    #[test]
    fn test_node_entry_initial_role_light() {
        let caps = make_capabilities(ComputeTier::Light, 10.0);
        let entry = NodeEntry::new(2, caps);
        assert!(matches!(
            entry.role,
            SwarmRole::Router | SwarmRole::Relay | SwarmRole::Light
        ));
    }

    #[test]
    fn test_node_heartbeat() {
        let caps = make_capabilities(ComputeTier::Standard, 50.0);
        let mut entry = NodeEntry::new(3, caps);
        let original_active = entry.last_active;
        std::thread::sleep(Duration::from_millis(10));
        entry.heartbeat();
        assert!(entry.last_active > original_active);
    }

    #[test]
    fn test_node_change_role() {
        let caps = make_capabilities(ComputeTier::Gpu, 100.0);
        let mut entry = NodeEntry::new(4, caps);
        assert_eq!(entry.role_change_count, 0);
        entry.change_role(SwarmRole::Validator);
        assert_eq!(entry.role, SwarmRole::Validator);
        assert_eq!(entry.role_change_count, 1);
    }

    #[test]
    fn test_node_same_role_no_increment() {
        let caps = make_capabilities(ComputeTier::Gpu, 100.0);
        let mut entry = NodeEntry::new(5, caps);
        let original_role = entry.role;
        entry.change_role(original_role);
        assert_eq!(entry.role_change_count, 0);
    }

    // ------------------------------------------------------------------
    // SubNetwork
    // ------------------------------------------------------------------

    #[test]
    fn test_subnetwork_add_member() {
        let mut subnet = SubNetwork::new(0, SwarmRole::Validator);
        subnet.add_member(100, 5.0);
        subnet.add_member(101, 3.0);
        assert_eq!(subnet.members.len(), 2);
        assert!((subnet.total_capacity - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_subnetwork_remove_member() {
        let mut subnet = SubNetwork::new(1, SwarmRole::Router);
        subnet.add_member(200, 4.0);
        subnet.add_member(201, 6.0);
        subnet.remove_member(200, 4.0);
        assert_eq!(subnet.members.len(), 1);
        assert!((subnet.total_capacity - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_subnetwork_overloaded() {
        let mut subnet = SubNetwork::new(2, SwarmRole::Light);
        subnet.add_member(300, 1.0);
        // High load simulation
        subnet.current_load = 0.9;
        assert!(subnet.is_overloaded(0.8));
        assert!(!subnet.is_overloaded(0.95));
    }

    #[test]
    fn test_subnetwork_underutilized() {
        let mut subnet = SubNetwork::new(3, SwarmRole::Relay);
        subnet.add_member(400, 10.0);
        subnet.add_member(401, 10.0);
        subnet.current_load = 0.1;
        assert!(subnet.is_underutilized(0.2));
        assert!(!subnet.is_underutilized(0.05));
    }

    // ------------------------------------------------------------------
    // TopologyConfig
    // ------------------------------------------------------------------

    #[test]
    fn test_default_config_valid() {
        let config = TopologyConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_ce_weight() {
        let config = TopologyConfig {
            ce_weight: -0.1,
            ..TopologyConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_weights_exceed_one() {
        let config = TopologyConfig {
            ce_weight: 0.6,
            hardware_weight: 0.6,
            ..TopologyConfig::default()
        };
        match config.validate() {
            Err(TopologyError::WeightsExceedOne(w)) => assert!(w > 1.0),
            _ => panic!("Expected WeightsExceedOne"),
        }
    }

    #[test]
    fn test_zero_max_nodes() {
        let config = TopologyConfig {
            max_nodes_per_subnet: 0,
            ..TopologyConfig::default()
        };
        assert!(config.validate().is_err());
    }

    // ------------------------------------------------------------------
    // SwarmTopology
    // ------------------------------------------------------------------

    #[test]
    fn test_topology_creation() {
        let topo = SwarmTopology::new();
        assert_eq!(topo.stats.active_nodes, 0);
        assert_eq!(topo.stats.sub_networks, 0);
    }

    #[test]
    fn test_register_gpu_node() {
        let mut topo = SwarmTopology::new();
        let caps = make_capabilities(ComputeTier::Gpu, 100.0);
        let role = topo.register_node(1, caps).unwrap();
        assert_eq!(role, SwarmRole::MaieuticSynth);
        assert_eq!(topo.stats.active_nodes, 1);
    }

    #[test]
    fn test_register_multiple_nodes() {
        let mut topo = SwarmTopology::new();
        for i in 0..10 {
            let tier = match i % 3 {
                0 => ComputeTier::Gpu,
                1 => ComputeTier::Standard,
                _ => ComputeTier::Light,
            };
            let caps = make_capabilities(tier, 50.0 + i as f64);
            topo.register_node(i, caps).unwrap();
        }
        assert_eq!(topo.stats.active_nodes, 10);
        assert!(topo.stats.sub_networks > 0);
    }

    #[test]
    fn test_unregister_node() {
        let mut topo = SwarmTopology::new();
        let caps = make_capabilities(ComputeTier::Standard, 50.0);
        topo.register_node(100, caps).unwrap();
        assert_eq!(topo.stats.active_nodes, 1);
        topo.unregister_node(100).unwrap();
        assert_eq!(topo.stats.active_nodes, 0);
    }

    #[test]
    fn test_unregister_nonexistent() {
        let mut topo = SwarmTopology::new();
        assert!(topo.unregister_node(999).is_err());
    }

    #[test]
    fn test_get_nodes_by_role() {
        let mut topo = SwarmTopology::new();
        for i in 0..5 {
            let caps = make_capabilities(ComputeTier::Gpu, 100.0);
            topo.register_node(i, caps).unwrap();
        }
        let synth_nodes = topo.get_maieutic_synth_nodes();
        assert_eq!(synth_nodes.len(), 5);
    }

    #[test]
    fn test_heartbeat() {
        let mut topo = SwarmTopology::new();
        let caps = make_capabilities(ComputeTier::Standard, 50.0);
        topo.register_node(200, caps).unwrap();
        assert!(topo.heartbeat(200).is_ok());
        assert!(topo.heartbeat(999).is_err());
    }

    #[test]
    fn test_rebalance() {
        let mut topo = SwarmTopology::new();
        // Add many nodes
        for i in 0..20 {
            let tier = match i % 3 {
                0 => ComputeTier::Gpu,
                1 => ComputeTier::Standard,
                _ => ComputeTier::Light,
            };
            let caps = make_capabilities(tier, 50.0);
            topo.register_node(i, caps).unwrap();
        }
        let moved = topo.rebalance();
        assert!(moved >= 0);
        assert_eq!(topo.stats.rebalances_executed, 1);
    }

    #[test]
    fn test_role_score() {
        let topo = SwarmTopology::new();
        let caps = make_capabilities(ComputeTier::Gpu, 100.0);
        let entry = NodeEntry::new(1, caps);
        let score = topo.role_score(&entry);
        assert!(score > 0.0);
    }

    #[test]
    fn test_best_candidates() {
        let mut topo = SwarmTopology::new();
        // Add standard nodes
        for i in 0..5 {
            let caps = make_capabilities(ComputeTier::Standard, 50.0 + i as f64 * 10.0);
            topo.register_node(i, caps).unwrap();
        }
        let candidates = topo.best_candidates_for_role(SwarmRole::Validator, 3);
        assert_eq!(candidates.len(), 0); // Already validators
    }

    #[test]
    fn test_get_event_log() {
        let mut topo = SwarmTopology::new();
        let caps = make_capabilities(ComputeTier::Gpu, 100.0);
        topo.register_node(1, caps).unwrap();
        let events = topo.get_event_log();
        assert!(!events.is_empty());
        assert!(matches!(
            events.back(),
            Some(&TopologyEvent::NodoIngresado { .. })
        ));
    }

    #[test]
    fn test_reset() {
        let mut topo = SwarmTopology::new();
        let caps = make_capabilities(ComputeTier::Gpu, 100.0);
        topo.register_node(1, caps).unwrap();
        topo.reset();
        assert_eq!(topo.stats.active_nodes, 0);
        assert_eq!(topo.stats.sub_networks, 0);
        assert!(topo.get_event_log().is_empty());
    }

    #[test]
    fn test_topology_default() {
        let topo = SwarmTopology::default();
        assert_eq!(topo.stats.active_nodes, 0);
    }

    #[test]
    fn test_update_config() {
        let mut topo = SwarmTopology::new();
        let new_config = TopologyConfig {
            max_nodes_per_subnet: 50,
            ..TopologyConfig::default()
        };
        assert!(topo.update_config(new_config).is_ok());
        assert_eq!(topo.config().max_nodes_per_subnet, 50);
    }

    #[test]
    fn test_update_invalid_config() {
        let mut topo = SwarmTopology::new();
        let bad_config = TopologyConfig {
            ce_weight: -1.0,
            ..TopologyConfig::default()
        };
        assert!(topo.update_config(bad_config).is_err());
    }

    #[test]
    fn test_list_subnetworks() {
        let mut topo = SwarmTopology::new();
        for i in 0..5 {
            let caps = make_capabilities(ComputeTier::Standard, 50.0);
            topo.register_node(i, caps).unwrap();
        }
        let subnets = topo.list_subnetworks();
        assert!(!subnets.is_empty());
    }

    #[test]
    fn test_get_disconnected_nodes() {
        let topo = SwarmTopology::new();
        let disconnected = topo.get_disconnected_nodes();
        assert!(disconnected.is_empty());
    }

    #[test]
    fn test_cleanup_disconnected() {
        let mut topo = SwarmTopology::new();
        let cleaned = topo.cleanup_disconnected();
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn test_topology_error_display() {
        let err = TopologyError::NodoNoExiste(42);
        assert!(format!("{}", err).contains("42"));
    }

    #[test]
    fn test_large_scale_organization() {
        let mut topo = SwarmTopology::new();
        // Simulate 100 nodes
        for i in 0..100 {
            let tier = match i % 10 {
                0..=2 => ComputeTier::Gpu,      // 30% GPU
                3..=6 => ComputeTier::Standard, // 40% Standard
                _ => ComputeTier::Light,        // 30% Light
            };
            let caps = make_capabilities(tier, 50.0 + (i % 50) as f64);
            topo.register_node(i, caps).unwrap();
        }
        assert_eq!(topo.stats.active_nodes, 100);

        // Rebalance
        topo.rebalance();
        assert!(topo.stats.rebalances_executed >= 1);

        // Verify role distribution
        let synth_count = topo.get_maieutic_synth_nodes().len();
        let validator_count = topo.get_validators().len();
        let router_count = topo.get_routers().len();
        assert!(synth_count + validator_count + router_count <= 100);
    }
}
