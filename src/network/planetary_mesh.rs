//! Planetary Mesh — WAN-Scale Routing with Kademlia DHT, AutoNAT & Circuit Relay.
//!
//! **Stuartian Law 1 (Diversidad):** Descubrimiento orgánico de pares a escala global.
//! **Stuartian Law 5 (Múltiples Posibilidades):** Tolerancia a particiones WAN, convergencia eventual.
//!
//! Este módulo proporciona la capa de enrutamiento para conectividad planetaria:
//! - **Kademlia DHT:** Descubrimiento de pares y almacenamiento distribuido de rutas.
//! - **AutoNAT:** Detección automática de dirección pública para nodos detrás de NAT.
//! - **Circuit Relay v2 / DCutR:** Hole punching para conexiones directas entre nodos
//!   en firewalls estrictos y navegadores WASM.
//!
//! ### Feature Gate
//! `v3.5-planetary-emergence`

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Kademlia DHT — Distributed Hash Table for Peer Discovery
// ============================================================================

/// Distancia XOR entre dos identificadores de nodo (Kademlia metric).
/// La distancia Kademlia entre A y B es A XOR B interpretado como entero.
pub fn kademlia_distance(a: u128, b: u128) -> u128 {
    a ^ b
}

/// Bucket de Kademlia para un rango k-ary específico.
/// Cada bucket almacena hasta `k` pares ordenados por última actividad.
#[derive(Debug, Clone)]
pub struct KademliaBucket {
    /// Rango k-ary de este bucket (ej. 2^160..2^161 para alpha=160).
    pub k_range: u8,
    /// Pares en este bucket, ordenados por última actividad (primero = más reciente).
    pub peers: Vec<PeerEntry>,
    /// Capacidad máxima del bucket (típicamente k=20 en Kademlia estándar).
    pub max_size: usize,
}

impl KademliaBucket {
    pub fn new(k_range: u8, max_size: usize) -> Self {
        Self {
            k_range,
            peers: Vec::new(),
            max_size,
        }
    }

    /// Agrega o actualiza un par en el bucket.
    /// Si el bucket está lleno y el par no existe, se intenta split.
    pub fn add_or_update(&mut self, peer: PeerEntry) -> BucketAction {
        if let Some(pos) = self.peers.iter().position(|p| p.node_id == peer.node_id) {
            // Mover al frente (más reciente).
            self.peers.remove(pos);
            self.peers.insert(0, peer);
            BucketAction::Updated
        } else if self.peers.len() < self.max_size {
            self.peers.insert(0, peer);
            BucketAction::Added
        } else {
            BucketAction::Full
        }
    }

    /// Obtiene los `n` pares más cercanos a un target dentro de este bucket.
    pub fn closest(&self, target: u128, n: usize) -> Vec<&PeerEntry> {
        let mut scored: Vec<(&PeerEntry, u128)> = self
            .peers
            .iter()
            .map(|p| (p, kademlia_distance(p.node_id, target)))
            .collect();
        scored.sort_by_key(|(_, d)| *d);
        scored.into_iter().take(n).map(|(p, _)| p).collect()
    }

    /// Verifica si el bucket necesita ser dividido (split) por estar lleno.
    pub fn needs_split(&self) -> bool {
        self.peers.len() >= self.max_size
    }
}

/// Acción resultante de agregar un par al bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketAction {
    Added,
    Updated,
    Full,
}

/// Entrada de par en la tabla Kademlia.
#[derive(Debug, Clone)]
pub struct PeerEntry {
    pub node_id: u128,
    pub public_addr: String,
    pub last_seen: u64,
    pub capabilities: NodeCapabilities,
}

/// Capacidades de un nodo en la malla planetaria.
#[derive(Debug, Clone, Copy, Default)]
pub struct NodeCapabilities {
    /// Capacidad de cómputo (0 = ligero/WASM, 1 = estándar, 2 = GPU).
    pub compute_tier: u8,
    /// Soporte para relay de circuitos.
    pub can_relay: bool,
    /// Soporte para hole punching (DCUtR).
    pub can_hole_punch: bool,
    /// Ancho de banda estimado en Mbps.
    pub bandwidth_mbps: f64,
    /// Crédito de Existencia disponible para incentivos de enrutamiento.
    pub ce_balance: f64,
}

/// Tabla K (K-Bucket Tree) implementada como mapa de rangos a buckets.
#[derive(Debug)]
pub struct KTable {
    local_id: u128,
    buckets: HashMap<u8, KademliaBucket>,
    k_size: usize,
    alpha_bits: u8,
}

impl KTable {
    pub fn new(local_id: u128, alpha_bits: u8) -> Self {
        Self {
            local_id,
            buckets: HashMap::new(),
            k_size: 20, // Kademlia estándar k=20
            alpha_bits,
        }
    }

    /// Obtiene o crea el bucket para un nodo dado.
    fn get_bucket_range(&self, node_id: u128) -> u8 {
        let dist = kademlia_distance(self.local_id, node_id);
        if dist == 0 {
            return 0;
        }
        // Leading zeros determina el rango k-ary.
        (128 - dist.leading_zeros() as u8).min(self.alpha_bits)
    }

    /// Agrega o actualiza un par en la tabla K.
    pub fn add_peer(&mut self, peer: PeerEntry) -> BucketAction {
        let range = self.get_bucket_range(peer.node_id);
        let bucket = self
            .buckets
            .entry(range)
            .or_insert_with(|| KademliaBucket::new(range, self.k_size));
        bucket.add_or_update(peer)
    }

    /// Encuentra los `n` pares más cercanos a un target en toda la tabla.
    pub fn find_closest(&self, target: u128, n: usize) -> Vec<&PeerEntry> {
        let mut all_peers: Vec<(&PeerEntry, u128)> = self
            .buckets
            .values()
            .flat_map(|b| {
                b.peers
                    .iter()
                    .map(|p| (p, kademlia_distance(p.node_id, target)))
            })
            .collect();
        all_peers.sort_by_key(|(_, d)| *d);
        all_peers.into_iter().take(n).map(|(p, _)| p).collect()
    }

    /// Elimina pares inactivos por más de `timeout_ms`.
    pub fn prune_inactive(&mut self, now_ms: u64, timeout_ms: u64) -> usize {
        let mut pruned = 0;
        for bucket in self.buckets.values_mut() {
            let before = bucket.peers.len();
            bucket.peers.retain(|p| now_ms - p.last_seen < timeout_ms);
            pruned += bucket.peers.len() - before;
        }
        pruned
    }
}

// ============================================================================
// AutoNAT — Detección de Dirección Pública
// ============================================================================

/// Estado de detección AutoNAT para un nodo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoNatStatus {
    /// Aún no se ha determinado la dirección pública.
    Unknown,
    /// El nodo es alcanzable públicamente (puerto abierto).
    Public(String),
    /// El nodo está detrás de NAT y necesita relay.
    Private,
}

/// Motor de detección AutoNAT.
#[derive(Debug)]
pub struct AutoNatEngine {
    status: AutoNatStatus,
    public_addr_cache: Option<String>,
    dial_attempts: u32,
    max_attempts: u32,
}

impl AutoNatEngine {
    pub fn new() -> Self {
        Self {
            status: AutoNatStatus::Unknown,
            public_addr_cache: None,
            dial_attempts: 0,
            max_attempts: 5,
        }
    }

    /// Procesa la respuesta de un servidor AutoNAT.
    pub fn process_server_response(
        &mut self,
        success: bool,
        server_observed_addr: Option<String>,
    ) {
        self.dial_attempts += 1;
        if success {
            if let Some(addr) = server_observed_addr {
                self.status = AutoNatStatus::Public(addr.clone());
                self.public_addr_cache = Some(addr);
            } else {
                self.status = AutoNatStatus::Public("auto-detected".to_string());
            }
        } else if self.dial_attempts >= self.max_attempts {
            self.status = AutoNatStatus::Private;
        }
    }

    /// Simula un intento de dial desde un servidor remoto.
    pub fn simulate_server_dial(&mut self, can_reach: bool, observed_addr: &str) {
        self.process_server_response(can_reach, if can_reach { Some(observed_addr.to_string()) } else { None });
    }

    pub fn get_status(&self) -> AutoNatStatus {
        self.status.clone()
    }

    pub fn needs_relay(&self) -> bool {
        matches!(self.status, AutoNatStatus::Private | AutoNatStatus::Unknown)
    }
}

impl Default for AutoNatEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Circuit Relay v2 / DCutR — Hole Punching
// ============================================================================

/// Estado de un circuito de relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Pending,
    Established,
    HolePunched,
    Failed,
}

/// Circuito de relay entre dos nodos a través de un relay intermedio.
#[derive(Debug)]
pub struct RelayCircuit {
    pub circuit_id: u64,
    pub peer_a: u128,
    pub peer_b: u128,
    pub relay_node: u128,
    pub state: CircuitState,
    pub created_at: u64,
    pub expires_at: u64,
}

impl RelayCircuit {
    pub fn new(circuit_id: u64, peer_a: u128, peer_b: u128, relay_node: u128, ttl_ms: u64) -> Self {
        let now = Self::now_ms();
        Self {
            circuit_id,
            peer_a,
            peer_b,
            relay_node,
            state: CircuitState::Pending,
            created_at: now,
            expires_at: now + ttl_ms,
        }
    }

    /// Intenta hole punching (DCUtR) entre los dos pares.
    pub fn attempt_hole_punch(&mut self, success: bool) {
        if self.state == CircuitState::Established {
            self.state = if success {
                CircuitState::HolePunched
            } else {
                CircuitState::Failed
            };
        }
    }

    /// Verifica si el circuito expiró.
    pub fn is_expired(&self) -> bool {
        Self::now_ms() > self.expires_at
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ============================================================================
// Planetary Mesh Router
// ============================================================================

/// Error en operaciones de malla planetaria.
#[derive(Debug)]
pub enum MeshError {
    PeerNotFound(u128),
    CircuitFailed(u64),
    DhtFull,
    NoRelayAvailable,
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeshError::PeerNotFound(id) => write!(f, "Par no encontrado: {}", id),
            MeshError::CircuitFailed(id) => write!(f, "Circuito fallido: {}", id),
            MeshError::DhtFull => write!(f, "DHT lleno"),
            MeshError::NoRelayAvailable => write!(f, "Sin relay disponible"),
        }
    }
}

/// Estadísticas de la malla planetaria.
#[derive(Debug, Clone)]
pub struct MeshStats {
    pub total_peers: usize,
    pub public_peers: usize,
    pub private_peers: usize,
    pub active_circuits: usize,
    pub hole_punched_connections: usize,
    pub dht_lookups: u64,
    pub dht_hits: u64,
}

impl MeshStats {
    pub fn new() -> Self {
        Self {
            total_peers: 0,
            public_peers: 0,
            private_peers: 0,
            active_circuits: 0,
            hole_punched_connections: 0,
            dht_lookups: 0,
            dht_hits: 0,
        }
    }
}

impl Default for MeshStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuración de la malla planetaria.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    pub k_bucket_size: usize,
    pub alpha_bits: u8,
    pub autonat_max_attempts: u32,
    pub circuit_ttl_ms: u64,
    pub peer_timeout_ms: u64,
    pub bootstrap_nodes: Vec<(u128, String)>,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            k_bucket_size: 20,
            alpha_bits: 128,
            autonat_max_attempts: 5,
            circuit_ttl_ms: 300_000, // 5 minutos
            peer_timeout_ms: 600_000, // 10 minutos
            bootstrap_nodes: vec![
                (1, "/ip4/10.0.0.1/tcp/4001".to_string()),
                (2, "/ip4/10.0.0.2/tcp/4001".to_string()),
                (3, "/ip4/10.0.0.3/tcp/4001".to_string()),
            ],
        }
    }
}

/// Router de Malla Planetaria — Orquesta Kademlia DHT, AutoNAT y Circuit Relay.
pub struct PlanetaryMesh {
    local_id: u128,
    k_table: KTable,
    autonat: AutoNatEngine,
    circuits: HashMap<u64, RelayCircuit>,
    next_circuit_id: u64,
    config: MeshConfig,
    stats: MeshStats,
}

impl PlanetaryMesh {
    pub fn new(local_id: u128) -> Self {
        Self::with_config(local_id, MeshConfig::default())
    }

    pub fn with_config(local_id: u128, config: MeshConfig) -> Self {
        Self {
            local_id,
            k_table: KTable::new(local_id, config.alpha_bits),
            autonat: AutoNatEngine::new(),
            circuits: HashMap::new(),
            next_circuit_id: 1,
            config,
            stats: MeshStats::new(),
        }
    }

    // ------------------------------------------------------------------
    // Peer Discovery (Kademlia DHT)
    // ------------------------------------------------------------------

    /// Registra un par en la tabla K.
    pub fn add_peer(&mut self, peer: PeerEntry) -> BucketAction {
        let action = self.k_table.add_peer(peer.clone());
        self.stats.total_peers = self.k_table.buckets.values().map(|b| b.peers.len()).sum();
        if peer.capabilities.can_relay {
            self.stats.public_peers += 1;
        }
        action
    }

    /// Busca los `n` pares más cercanos a un target (Kademlia find_node).
    pub fn find_closest_peers(&mut self, target: u128, n: usize) -> Vec<&PeerEntry> {
        self.stats.dht_lookups += 1;
        let closest = self.k_table.find_closest(target, n);
        if !closest.is_empty() {
            self.stats.dht_hits += 1;
        }
        closest
    }

    /// Búsqueda iterativa de alpha (α) pares más cercanos.
    /// Simula α rondas de consulta Kademlia para convergencia.
    pub fn iterative_find_node(&mut self, target: u128, alpha: usize) -> Vec<&PeerEntry> {
        // En una implementación real con libp2p, esto haría α consultas concurrentes.
        // Aquí simulamos la convergencia local.
        let mut candidates = self.k_table.find_closest(target, alpha * 3);
        candidates.truncate(alpha);
        candidates
    }

    // ------------------------------------------------------------------
    // AutoNAT
    // ------------------------------------------------------------------

    /// Procesa respuesta de servidor AutoNAT.
    pub fn process_autonat_response(&mut self, success: bool, observed_addr: Option<String>) {
        self.autonat.process_server_response(success, observed_addr);
        match self.autonat.get_status() {
            AutoNatStatus::Public(_) => {
                self.stats.public_peers += 1;
            }
            AutoNatStatus::Private => {
                self.stats.private_peers += 1;
            }
            _ => {}
        }
    }

    /// Simula detección AutoNAT desde múltiples servidores.
    pub fn simulate_autonat_discovery(&mut self, servers: &[(bool, &str)]) {
        for (can_reach, addr) in servers {
            self.autonat.simulate_server_dial(*can_reach, addr);
            if !self.autonat.needs_relay() {
                break;
            }
        }
    }

    pub fn get_autonat_status(&self) -> AutoNatStatus {
        self.autonat.get_status()
    }

    pub fn needs_relay(&self) -> bool {
        self.autonat.needs_relay()
    }

    // ------------------------------------------------------------------
    // Circuit Relay / DCutR
    // ------------------------------------------------------------------

    /// Crea un circuito de relay entre dos pares privados.
    pub fn create_relay_circuit(&mut self, peer_a: u128, peer_b: u128, relay_node: u128) -> Result<u64, MeshError> {
        let circuit_id = self.next_circuit_id;
        self.next_circuit_id += 1;
        let circuit = RelayCircuit::new(
            circuit_id,
            peer_a,
            peer_b,
            relay_node,
            self.config.circuit_ttl_ms,
        );
        self.circuits.insert(circuit_id, circuit);
        self.stats.active_circuits += 1;
        Ok(circuit_id)
    }

    /// Establece un circuito y intenta hole punching.
    pub fn establish_and_punch(&mut self, circuit_id: u64, punch_success: bool) -> Result<CircuitState, MeshError> {
        let circuit = self.circuits.get_mut(&circuit_id).ok_or(MeshError::CircuitFailed(circuit_id))?;
        circuit.state = CircuitState::Established;
        circuit.attempt_hole_punch(punch_success);
        if circuit.state == CircuitState::HolePunched {
            self.stats.hole_punched_connections += 1;
        }
        Ok(circuit.state)
    }

    /// Limpia circuitos expirados.
    pub fn prune_expired_circuits(&mut self) -> usize {
        let before = self.circuits.len();
        self.circuits.retain(|_, c| !c.is_expired());
        let pruned = before - self.circuits.len();
        self.stats.active_circuits = self.circuits.len();
        pruned
    }

    /// Prune pares inactivos.
    pub fn prune_inactive_peers(&mut self) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.k_table.prune_inactive(now, self.config.peer_timeout_ms)
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    pub fn local_id(&self) -> u128 {
        self.local_id
    }

    pub fn get_stats(&self) -> &MeshStats {
        &self.stats
    }

    pub fn peer_count(&self) -> usize {
        self.k_table.buckets.values().map(|b| b.peers.len()).sum()
    }

    pub fn active_circuit_count(&self) -> usize {
        self.circuits.len()
    }

    /// Resetea el estado de la malla (para testing).
    pub fn reset(&mut self) {
        self.k_table = KTable::new(self.local_id, self.config.alpha_bits);
        self.autonat = AutoNatEngine::new();
        self.circuits.clear();
        self.stats = MeshStats::new();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(id: u128, addr: &str, compute_tier: u8) -> PeerEntry {
        PeerEntry {
            node_id: id,
            public_addr: addr.to_string(),
            last_seen: 1000,
            capabilities: NodeCapabilities {
                compute_tier,
                can_relay: compute_tier > 0,
                can_hole_punch: compute_tier > 0,
                bandwidth_mbps: 100.0,
                ce_balance: 50.0,
            },
        }
    }

    #[test]
    fn test_kademlia_distance() {
        assert_eq!(kademlia_distance(0, 0), 0);
        assert_eq!(kademlia_distance(1, 0), 1);
        assert_eq!(kademlia_distance(0xFF, 0), 0xFF);
        assert_eq!(kademlia_distance(0b1010, 0b0101), 0b1111);
    }

    #[test]
    fn test_bucket_add_and_update() {
        let mut bucket = KademliaBucket::new(0, 3);
        let p1 = make_peer(1, "addr1", 1);
        assert_eq!(bucket.add_or_update(p1), BucketAction::Added);
        assert_eq!(bucket.peers.len(), 1);

        let p1_updated = make_peer(1, "addr1_new", 2);
        assert_eq!(bucket.add_or_update(p1_updated), BucketAction::Updated);
        assert_eq!(bucket.peers.len(), 1);
        assert_eq!(bucket.peers[0].capabilities.compute_tier, 2);
    }

    #[test]
    fn test_bucket_full() {
        let mut bucket = KademliaBucket::new(0, 2);
        bucket.add_or_update(make_peer(1, "a", 1));
        bucket.add_or_update(make_peer(2, "b", 1));
        assert_eq!(bucket.add_or_update(make_peer(3, "c", 1)), BucketAction::Full);
        assert!(bucket.needs_split());
    }

    #[test]
    fn test_bucket_closest() {
        let mut bucket = KademliaBucket::new(0, 10);
        bucket.add_or_update(make_peer(100, "a", 1));
        bucket.add_or_update(make_peer(200, "b", 1));
        bucket.add_or_update(make_peer(150, "c", 1));

        let closest = bucket.closest(149, 2);
        assert_eq!(closest.len(), 2);
        assert_eq!(closest[0].node_id, 150);
        assert_eq!(closest[1].node_id, 100);
    }

    #[test]
    fn test_ktable_add_and_find() {
        let mut table = KTable::new(0, 128);
        table.add_peer(make_peer(10, "a", 1));
        table.add_peer(make_peer(20, "b", 2));
        table.add_peer(make_peer(15, "c", 1));

        let closest = table.find_closest(14, 2);
        assert_eq!(closest.len(), 2);
        assert_eq!(closest[0].node_id, 15);
    }

    #[test]
    fn test_ktable_prune_inactive() {
        let mut table = KTable::new(0, 128);
        table.add_peer(make_peer(1, "a", 1));
        // last_seen = 1000, now = 1000000, timeout = 500 -> debe eliminarse
        let pruned = table.prune_inactive(1_000_000, 500);
        assert_eq!(pruned, 1);
    }

    #[test]
    fn test_autonat_public_detection() {
        let mut engine = AutoNatEngine::new();
        engine.simulate_server_dial(true, "203.0.113.1:4001");
        assert_eq!(engine.get_status(), AutoNatStatus::Public("203.0.113.1:4001".to_string()));
        assert!(!engine.needs_relay());
    }

    #[test]
    fn test_autonat_private_detection() {
        let mut engine = AutoNatEngine::new();
        for _ in 0..5 {
            engine.simulate_server_dial(false, "");
        }
        assert_eq!(engine.get_status(), AutoNatStatus::Private);
        assert!(engine.needs_relay());
    }

    #[test]
    fn test_autonat_default() {
        let engine = AutoNatEngine::new();
        assert_eq!(engine.get_status(), AutoNatStatus::Unknown);
        assert!(engine.needs_relay());
    }

    #[test]
    fn test_relay_circuit_creation() {
        let circuit = RelayCircuit::new(1, 100, 200, 1, 300_000);
        assert_eq!(circuit.state, CircuitState::Pending);
        assert!(!circuit.is_expired());
    }

    #[test]
    fn test_hole_punch_success() {
        let mut circuit = RelayCircuit::new(1, 100, 200, 1, 300_000);
        circuit.state = CircuitState::Established;
        circuit.attempt_hole_punch(true);
        assert_eq!(circuit.state, CircuitState::HolePunched);
    }

    #[test]
    fn test_hole_punch_failure() {
        let mut circuit = RelayCircuit::new(1, 100, 200, 1, 300_000);
        circuit.state = CircuitState::Established;
        circuit.attempt_hole_punch(false);
        assert_eq!(circuit.state, CircuitState::Failed);
    }

    #[test]
    fn test_mesh_creation() {
        let mesh = PlanetaryMesh::new(42);
        assert_eq!(mesh.local_id(), 42);
        assert_eq!(mesh.peer_count(), 0);
    }

    #[test]
    fn test_mesh_add_peer() {
        let mut mesh = PlanetaryMesh::new(42);
        mesh.add_peer(make_peer(100, "addr", 1));
        assert_eq!(mesh.peer_count(), 1);
    }

    #[test]
    fn test_mesh_find_closest() {
        let mut mesh = PlanetaryMesh::new(42);
        mesh.add_peer(make_peer(100, "a", 1));
        mesh.add_peer(make_peer(200, "b", 1));
        mesh.add_peer(make_peer(150, "c", 1));

        let closest = mesh.find_closest_peers(149, 2);
        assert_eq!(closest.len(), 2);
        assert_eq!(closest[0].node_id, 150);
    }

    #[test]
    fn test_mesh_iterative_find() {
        let mut mesh = PlanetaryMesh::new(42);
        for i in 0..20 {
            mesh.add_peer(make_peer(i * 10, &format!("addr{}", i), 1));
        }
        let results = mesh.iterative_find_node(50, 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_mesh_relay_circuit() {
        let mut mesh = PlanetaryMesh::new(42);
        let cid = mesh.create_relay_circuit(100, 200, 1).unwrap();
        assert_eq!(mesh.active_circuit_count(), 1);

        let state = mesh.establish_and_punch(cid, true).unwrap();
        assert_eq!(state, CircuitState::HolePunched);
        assert_eq!(mesh.get_stats().hole_punched_connections, 1);
    }

    #[test]
    fn test_mesh_autonat_integration() {
        let mut mesh = PlanetaryMesh::new(42);
        mesh.process_autonat_response(true, Some("203.0.113.5:4001".to_string()));
        assert!(!mesh.needs_relay());
    }

    #[test]
    fn test_mesh_stats() {
        let mut mesh = PlanetaryMesh::new(42);
        mesh.add_peer(make_peer(1, "a", 1));
        mesh.add_peer(make_peer(2, "b", 2));
        let stats = mesh.get_stats();
        assert_eq!(stats.total_peers, 2);
    }

    #[test]
    fn test_mesh_reset() {
        let mut mesh = PlanetaryMesh::new(42);
        mesh.add_peer(make_peer(1, "a", 1));
        mesh.reset();
        assert_eq!(mesh.peer_count(), 0);
        assert_eq!(mesh.active_circuit_count(), 0);
    }

    #[test]
    fn test_mesh_config_default() {
        let config = MeshConfig::default();
        assert_eq!(config.k_bucket_size, 20);
        assert_eq!(config.alpha_bits, 128);
        assert!(!config.bootstrap_nodes.is_empty());
    }

    #[test]
    fn test_stats_default() {
        let stats = MeshStats::new();
        assert_eq!(stats.total_peers, 0);
        assert_eq!(stats.dht_lookups, 0);
    }

    #[test]
    fn test_error_display() {
        let err = MeshError::PeerNotFound(42);
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_bucket_action_equality() {
        assert_eq!(BucketAction::Added, BucketAction::Added);
        assert_eq!(BucketAction::Updated, BucketAction::Updated);
        assert_eq!(BucketAction::Full, BucketAction::Full);
    }

    #[test]
    fn test_capabilities_default() {
        let caps = NodeCapabilities::default();
        assert_eq!(caps.compute_tier, 0);
        assert!(!caps.can_relay);
    }

    #[test]
    fn test_large_scale_dht() {
        let mut mesh = PlanetaryMesh::new(0);
        for i in 1..=500 {
            mesh.add_peer(make_peer(i, &format!("addr{}", i), (i % 3) as u8));
        }
        assert_eq!(mesh.peer_count(), 500);
        let closest = mesh.find_closest_peers(250, 10);
        assert_eq!(closest.len(), 10);
    }
}
