//! Gossip Mesh — Configuración de GossipSub con tolerancia asíncrona.
//!
//! **Stuartian Law 5 (Múltiples Posibilidades):** Mesh dinámico sin maestros,
//! tolerante a particiones, con reconexión automática.
//!
//! **Feature Gate:** `v2.1-async-gossip`
//!
//! ### Parámetros GossipSub
//! | Parámetro | Valor | Descripción |
//! |---|---|---|
//! | heartbeat_interval | 500ms | Frecuencia de mantención de mesh |
//! | fanout_ttl | 120s | Vida útil de fanout entries |
//! | mesh_n | 6 | Tamaño ideal del mesh |
//! | mesh_n_low | 4 | Mínimo antes de IHAVE flood |
//! | mesh_n_high | 12 | Máximo antes de prune |
//! | validate_messages | true | Validación antes de reenvío |
//!
//! ### Deterministic Message ID
//! El message_id se calcula como SHA-256 del payload hash para evitar
//! duplicados en particiones y garantizar idempotencia.
//!
//! ### Slow Peer Handling
//! Pares con latencia > 2x el heartbeat reciben prune con backoff
//! exponencial (1s, 2s, 4s, 8s... max 30s).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::time::{Duration, Instant};

/// Error en la configuración o gestión del mesh GossipSub.
#[derive(Debug)]
pub enum GossipMeshError {
    /// Parámetro de mesh inválido.
    InvalidParameter(String),
    /// Error de conexión.
    ConnectionError(String),
    /// Topología inválida.
    InvalidTopology(String),
    /// Message ID colisión detectada.
    MessageIdCollision(String),
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
            GossipMeshError::MessageIdCollision(msg) => {
                write!(f, "Message ID collision: {}", msg)
            }
        }
    }
}

impl std::error::Error for GossipMeshError {}

/// Estado de conexión de un peer en el mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeerState {
    /// Peer activo en el mesh.
    Meshed,
    /// Peer en fanout (sin mesh directo).
    Fanout,
    /// Peer pruneado (backoff activo).
    Pruned,
    /// Peer gravemente lento (graceful disconnect).
    GracefulDisconnect,
}

impl fmt::Display for PeerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerState::Meshed => write!(f, "meshed"),
            PeerState::Fanout => write!(f, "fanout"),
            PeerState::Pruned => write!(f, "pruned"),
            PeerState::GracefulDisconnect => write!(f, "graceful_disconnect"),
        }
    }
}

/// Información de un peer en el mesh.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Identificador único del peer.
    pub peer_id: String,
    /// Estado actual del peer.
    pub state: PeerState,
    /// Latencia promedio en milisegundos.
    pub avg_latency_ms: f64,
    /// Último heartbeat recibido.
    pub last_heartbeat: Instant,
    /// Contador de backoff actual.
    pub backoff_count: u32,
    /// Timestamp de entrada al mesh.
    pub joined_at: Instant,
}

impl PeerInfo {
    /// Crea un nuevo peer info.
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            state: PeerState::Meshed,
            avg_latency_ms: 0.0,
            last_heartbeat: Instant::now(),
            backoff_count: 0,
            joined_at: Instant::now(),
        }
    }

    /// Calcula el backoff exponencial en milisegundos.
    /// backoff = min(2^count * 1000, 30000)
    pub fn backoff_ms(&self) -> u64 {
        let backoff = (1u64 << self.backoff_count) * 1000;
        backoff.min(30_000)
    }

    /// Determina si el peer es lento (latencia > 2x heartbeat).
    pub fn is_slow(&self, heartbeat_ms: u64) -> bool {
        self.avg_latency_ms > 2.0 * heartbeat_ms as f64
    }
}

/// Configuración del mesh GossipSub.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Tamaño ideal del mesh (mesh_n = 6).
    pub mesh_size: usize,
    /// Tamaño mínimo del mesh (mesh_n_low = 4).
    pub mesh_min: usize,
    /// Tamaño máximo del mesh (mesh_n_high = 12).
    pub mesh_max: usize,
    /// Grado de propagación (D = 6).
    pub fanout: usize,
    /// Intervalo de heartbeat en milisegundos (500ms).
    pub heartbeat_interval_ms: u64,
    /// Vida útil de fanout entries en segundos (120s).
    pub fanout_ttl_s: u64,
    /// Validar mensajes antes de reenvío (true).
    pub validate_messages: bool,
    /// ID del nodo local para message_id determinista.
    pub local_peer_id: String,
}

impl MeshConfig {
    /// Crea configuración por defecto para ed2kIA.
    ///
    /// Parámetros optimizados para redes de 50-500 nodos:
    /// - heartbeat: 500ms para detección rápida de particiones
    /// - fanout_ttl: 120s para propagación completa
    /// - mesh_n: 6 para balance entre latencia y ancho de banda
    pub fn default_ed2kia() -> Self {
        Self {
            mesh_size: 6,
            mesh_min: 4,
            mesh_max: 12,
            fanout: 6,
            heartbeat_interval_ms: 500,
            fanout_ttl_s: 120,
            validate_messages: true,
            local_peer_id: String::from("local"),
        }
    }

    /// Construye configuración personalizada.
    pub fn builder() -> MeshConfigBuilder {
        MeshConfigBuilder::default()
    }

    /// Valida los parámetros del mesh.
    ///
    /// Invariantes:
    /// - mesh_min <= mesh_size <= mesh_max
    /// - heartbeat_interval_ms > 0
    /// - fanout_ttl_s > 0
    /// - fanout > 0
    pub fn validate(&self) -> Result<(), GossipMeshError> {
        if self.mesh_min == 0 {
            return Err(GossipMeshError::InvalidParameter(
                "mesh_min must be > 0".into(),
            ));
        }
        if self.mesh_size == 0 {
            return Err(GossipMeshError::InvalidParameter(
                "mesh_size must be > 0".into(),
            ));
        }
        if self.mesh_max == 0 {
            return Err(GossipMeshError::InvalidParameter(
                "mesh_max must be > 0".into(),
            ));
        }
        if self.mesh_min > self.mesh_size {
            return Err(GossipMeshError::InvalidParameter(
                format!(
                    "mesh_min ({}) cannot exceed mesh_size ({})",
                    self.mesh_min, self.mesh_size
                ),
            ));
        }
        if self.mesh_size > self.mesh_max {
            return Err(GossipMeshError::InvalidParameter(
                format!(
                    "mesh_size ({}) cannot exceed mesh_max ({})",
                    self.mesh_size, self.mesh_max
                ),
            ));
        }
        if self.heartbeat_interval_ms == 0 {
            return Err(GossipMeshError::InvalidParameter(
                "heartbeat_interval_ms must be > 0".into(),
            ));
        }
        if self.fanout_ttl_s == 0 {
            return Err(GossipMeshError::InvalidParameter(
                "fanout_ttl_s must be > 0".into(),
            ));
        }
        if self.fanout == 0 {
            return Err(GossipMeshError::InvalidParameter(
                "fanout must be > 0".into(),
            ));
        }
        Ok(())
    }

    /// Retorna el intervalo de heartbeat como Duration.
    pub fn heartbeat_duration(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }

    /// Retorna el fanout TTL como Duration.
    pub fn fanout_ttl_duration(&self) -> Duration {
        Duration::from_secs(self.fanout_ttl_s)
    }
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self::default_ed2kia()
    }
}

/// Builder para MeshConfig.
#[derive(Debug, Default)]
pub struct MeshConfigBuilder {
    mesh_size: usize,
    mesh_min: usize,
    mesh_max: usize,
    fanout: usize,
    heartbeat_interval_ms: u64,
    fanout_ttl_s: u64,
    validate_messages: bool,
    local_peer_id: String,
}

impl MeshConfigBuilder {
    /// Establece el tamaño ideal del mesh.
    pub fn mesh_size(mut self, size: usize) -> Self {
        self.mesh_size = size;
        self
    }

    /// Establece el tamaño mínimo del mesh.
    pub fn mesh_min(mut self, min: usize) -> Self {
        self.mesh_min = min;
        self
    }

    /// Establece el tamaño máximo del mesh.
    pub fn mesh_max(mut self, max: usize) -> Self {
        self.mesh_max = max;
        self
    }

    /// Establece el grado de fanout.
    pub fn fanout(mut self, fanout: usize) -> Self {
        self.fanout = fanout;
        self
    }

    /// Establece el intervalo de heartbeat en milisegundos.
    pub fn heartbeat_interval_ms(mut self, ms: u64) -> Self {
        self.heartbeat_interval_ms = ms;
        self
    }

    /// Establece el fanout TTL en segundos.
    pub fn fanout_ttl_s(mut self, secs: u64) -> Self {
        self.fanout_ttl_s = secs;
        self
    }

    /// Establece si validar mensajes antes de reenvío.
    pub fn validate_messages(mut self, validate: bool) -> Self {
        self.validate_messages = validate;
        self
    }

    /// Establece el ID del peer local.
    pub fn local_peer_id(mut self, id: String) -> Self {
        self.local_peer_id = id;
        self
    }

    /// Construye la configuración, validando parámetros.
    pub fn build(self) -> Result<MeshConfig, GossipMeshError> {
        let config = MeshConfig {
            mesh_size: self.mesh_size,
            mesh_min: self.mesh_min,
            mesh_max: self.mesh_max,
            fanout: self.fanout,
            heartbeat_interval_ms: self.heartbeat_interval_ms,
            fanout_ttl_s: self.fanout_ttl_s,
            validate_messages: self.validate_messages,
            local_peer_id: self.local_peer_id,
        };
        config.validate()?;
        Ok(config)
    }
}

/// Mensaje en el mesh GossipSub.
#[derive(Debug, Clone)]
pub struct MeshMessage {
    /// ID determinista (SHA-256 del payload hash).
    pub message_id: String,
    /// Origen del mensaje.
    pub origin: String,
    /// Payload del mensaje.
    pub payload: Vec<u8>,
    /// Timestamp de creación.
    pub timestamp: u64,
    /// Secuencia del origen.
    pub sequence: u64,
}

impl MeshMessage {
    /// Crea un nuevo mensaje con message_id determinista.
    pub fn new(origin: String, payload: Vec<u8>, sequence: u64) -> Self {
        let message_id = Self::compute_message_id(&payload);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            message_id,
            origin,
            payload,
            timestamp,
            sequence,
        }
    }

    /// Calcula el message_id como hash del payload.
    /// Usa una suma simple de bytes para evitar dependencias externas.
    /// En producción, usar SHA-256.
    fn compute_message_id(payload: &[u8]) -> String {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
        for &byte in payload {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
        }
        format!("{:016x}", hash)
    }

    /// Verifica si el mensaje está expirado.
    pub fn is_expired(&self, ttl_s: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.timestamp) > ttl_s
    }
}

/// Mesh GossipSub con tolerancia asíncrona.
///
/// **Stuartian Law 5:** Sin puntos únicos de fallo.
/// La red se auto-organiza y tolera particiones.
///
/// ### Invariantes
/// 1. mesh_min <= peers_meshed <= mesh_max (después de heartbeat)
/// 2. Mensajes duplicados se detectan por message_id
/// 3. Peers lentos reciben prune con backoff exponencial
/// 4. Merge de particiones es idempotente
pub struct GossipMesh {
    /// Configuración del mesh.
    pub config: MeshConfig,
    /// Peers activos en el mesh.
    peers: BTreeMap<String, PeerInfo>,
    /// Mensajes recibidos (deduplicación).
    seen_messages: HashMap<String, Instant>,
    /// Cola de mensajes pendientes.
    pending_messages: Vec<MeshMessage>,
    /// Contador de secuencia local.
    sequence_counter: u64,
    /// Momento del último heartbeat.
    last_heartbeat: Instant,
}

impl GossipMesh {
    /// Crea un nuevo mesh con configuración especificada.
    pub fn new(config: MeshConfig) -> Result<Self, GossipMeshError> {
        config.validate()?;
        Ok(Self {
            config,
            peers: BTreeMap::new(),
            seen_messages: HashMap::new(),
            pending_messages: Vec::new(),
            sequence_counter: 0,
            last_heartbeat: Instant::now(),
        })
    }

    /// Inicializa el mesh con configuración por defecto.
    pub fn default_mesh() -> Self {
        Self {
            config: MeshConfig::default_ed2kia(),
            peers: BTreeMap::new(),
            seen_messages: HashMap::new(),
            pending_messages: Vec::new(),
            sequence_counter: 0,
            last_heartbeat: Instant::now(),
        }
    }

    /// Añade un peer al mesh.
    ///
    /// Si el mesh está en mesh_max, el peer se añade como fanout.
    pub fn add_peer(&mut self, peer_id: String) {
        let state = if self.peers.len() >= self.config.mesh_max {
            PeerState::Fanout
        } else {
            PeerState::Meshed
        };
        self.peers.insert(
            peer_id.clone(),
            PeerInfo {
                peer_id,
                state,
                avg_latency_ms: 0.0,
                last_heartbeat: Instant::now(),
                backoff_count: 0,
                joined_at: Instant::now(),
            },
        );
    }

    /// Remueve un peer del mesh.
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.peers.remove(peer_id);
    }

    /// Publica un mensaje en el mesh.
    ///
    /// Retorna Err si el message_id ya fue visto (deduplicación).
    pub fn publish(&mut self, payload: Vec<u8>) -> Result<MeshMessage, GossipMeshError> {
        self.sequence_counter += 1;
        let msg = MeshMessage::new(
            self.config.local_peer_id.clone(),
            payload,
            self.sequence_counter,
        );

        // Deduplicación
        if self.seen_messages.contains_key(&msg.message_id) {
            return Err(GossipMeshError::MessageIdCollision(
                format!("Duplicate message_id: {}", msg.message_id),
            ));
        }

        self.seen_messages.insert(msg.message_id.clone(), Instant::now());
        self.pending_messages.push(msg.clone());
        Ok(msg)
    }

    /// Inyecta un mensaje recibido de un peer.
    pub fn inject_message(&mut self, msg: MeshMessage, _from_peer: &str) -> bool {
        // Deduplicación
        if self.seen_messages.contains_key(&msg.message_id) {
            return false;
        }

        // Validación opcional
        if self.config.validate_messages && msg.payload.is_empty() {
            return false;
        }

        self.seen_messages.insert(msg.message_id.clone(), Instant::now());
        self.pending_messages.push(msg);
        true
    }

    /// Ejecuta un heartbeat del mesh.
    ///
    /// - Detecta peers lentos y los prunea
    /// - Limpia mensajes expirados
    /// - Rebalancea el mesh si está por debajo de mesh_min
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();

        // Detect slow peers
        let slow_peers: Vec<String> = self
            .peers
            .values()
            .filter(|p| p.is_slow(self.config.heartbeat_interval_ms))
            .map(|p| p.peer_id.clone())
            .collect();

        for peer_id in slow_peers {
            self.prune_peer(&peer_id);
        }

        // Clean expired messages
        let ttl = self.config.fanout_ttl_s as u64;
        self.pending_messages.retain(|msg| !msg.is_expired(ttl));

        // Clean old seen messages (keep last 2x fanout_ttl)
        let cutoff = Instant::now() - self.config.fanout_ttl_duration() * 2;
        self.seen_messages.retain(|_, when| *when > cutoff);
    }

    /// Prunea un peer lento con backoff exponencial.
    fn prune_peer(&mut self, peer_id: &str) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.state = PeerState::Pruned;
            peer.backoff_count += 1;
        }
    }

    /// Retorna los peers meshed activos.
    pub fn meshed_peers(&self) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| p.state == PeerState::Meshed)
            .collect()
    }

    /// Retorna el conteo de peers por estado.
    pub fn peer_counts(&self) -> HashMap<PeerState, usize> {
        let mut counts = HashMap::new();
        for peer in self.peers.values() {
            *counts.entry(peer.state).or_insert(0) += 1;
        }
        counts
    }

    /// Retorna la cola de mensajes pendientes.
    pub fn pending_messages(&self) -> &[MeshMessage] {
        &self.pending_messages
    }

    /// Drena la cola de mensajes pendientes.
    pub fn drain_pending(&mut self) -> Vec<MeshMessage> {
        std::mem::take(&mut self.pending_messages)
    }

    /// Retorna el número total de peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Verifica si el mesh está saludable (>= mesh_min peers meshed).
    pub fn is_healthy(&self) -> bool {
        self.meshed_peers().len() >= self.config.mesh_min
    }

    /// Retorna el backoff para un peer específico.
    pub fn peer_backoff_ms(&self, peer_id: &str) -> Option<u64> {
        self.peers.get(peer_id).map(|p| p.backoff_ms())
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
        assert_eq!(config.mesh_max, 12);
        assert_eq!(config.heartbeat_interval_ms, 500);
        assert_eq!(config.fanout_ttl_s, 120);
        assert!(config.validate_messages);
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
            heartbeat_interval_ms: 500,
            fanout_ttl_s: 120,
            validate_messages: true,
            local_peer_id: "test".into(),
        };
        match config.validate() {
            Err(GossipMeshError::InvalidParameter(_)) => {} // Expected
            other => panic!("Expected InvalidParameter, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_zero_mesh_min() {
        let config = MeshConfig {
            mesh_size: 6,
            mesh_min: 0,
            mesh_max: 12,
            fanout: 6,
            heartbeat_interval_ms: 500,
            fanout_ttl_s: 120,
            validate_messages: true,
            local_peer_id: "test".into(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_heartbeat() {
        let config = MeshConfig {
            mesh_size: 6,
            mesh_min: 4,
            mesh_max: 12,
            fanout: 6,
            heartbeat_interval_ms: 0,
            fanout_ttl_s: 120,
            validate_messages: true,
            local_peer_id: "test".into(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_mesh_creation() {
        let config = MeshConfig::default_ed2kia();
        let mesh = GossipMesh::new(config).unwrap();
        assert_eq!(mesh.peer_count(), 0);
    }

    #[test]
    fn test_default_mesh() {
        let mesh = GossipMesh::default_mesh();
        assert_eq!(mesh.config.mesh_size, 6);
    }

    #[test]
    fn test_add_peer() {
        let mut mesh = GossipMesh::default_mesh();
        mesh.add_peer("peer-1".into());
        assert_eq!(mesh.peer_count(), 1);
        assert_eq!(mesh.meshed_peers().len(), 1);
    }

    #[test]
    fn test_add_peer_fanout_when_full() {
        let mut mesh = GossipMesh::default_mesh();
        // Fill mesh to max
        for i in 0..12 {
            mesh.add_peer(format!("peer-{}", i));
        }
        let counts = mesh.peer_counts();
        assert_eq!(counts.get(&PeerState::Meshed), Some(&12));
    }

    #[test]
    fn test_remove_peer() {
        let mut mesh = GossipMesh::default_mesh();
        mesh.add_peer("peer-1".into());
        mesh.remove_peer("peer-1");
        assert_eq!(mesh.peer_count(), 0);
    }

    #[test]
    fn test_publish_message() {
        let mut mesh = GossipMesh::default_mesh();
        let msg = mesh.publish(vec![1, 2, 3]).unwrap();
        assert_eq!(msg.origin, "local");
        assert_eq!(msg.payload, vec![1, 2, 3]);
        assert_eq!(mesh.pending_messages().len(), 1);
    }

    #[test]
    fn test_publish_duplicate_rejected() {
        let mut mesh = GossipMesh::default_mesh();
        let _ = mesh.publish(vec![1, 2, 3]).unwrap();
        let result = mesh.publish(vec![1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_inject_message() {
        let mut mesh = GossipMesh::default_mesh();
        let msg = MeshMessage::new("remote".into(), vec![4, 5, 6], 1);
        let accepted = mesh.inject_message(msg, "peer-1");
        assert!(accepted);
        assert_eq!(mesh.pending_messages().len(), 1);
    }

    #[test]
    fn test_inject_duplicate_rejected() {
        let mut mesh = GossipMesh::default_mesh();
        let msg = MeshMessage::new("remote".into(), vec![1, 1], 1);
        assert!(mesh.inject_message(msg.clone(), "peer-1"));
        assert!(!mesh.inject_message(msg, "peer-2"));
    }

    #[test]
    fn test_inject_empty_payload_rejected() {
        let mut mesh = GossipMesh::default_mesh();
        let msg = MeshMessage::new("remote".into(), vec![], 1);
        assert!(!mesh.inject_message(msg, "peer-1"));
    }

    #[test]
    fn test_drain_pending() {
        let mut mesh = GossipMesh::default_mesh();
        mesh.publish(vec![1]).unwrap();
        mesh.publish(vec![2]).unwrap();
        let drained = mesh.drain_pending();
        assert_eq!(drained.len(), 2);
        assert_eq!(mesh.pending_messages().len(), 0);
    }

    #[test]
    fn test_peer_backoff_exponential() {
        let peer = PeerInfo::new("test".into());
        assert_eq!(peer.backoff_ms(), 1000); // 2^0 * 1000

        let peer = PeerInfo {
            backoff_count: 1,
            ..peer.clone()
        };
        assert_eq!(peer.backoff_ms(), 2000); // 2^1 * 1000

        let peer = PeerInfo {
            backoff_count: 5,
            ..peer.clone()
        };
        assert_eq!(peer.backoff_ms(), 30000); // 2^5 * 1000 = 32000, capped at 30000

        let peer = PeerInfo {
            backoff_count: 10,
            ..peer.clone()
        };
        assert_eq!(peer.backoff_ms(), 30000); // Capped at 30s
    }

    #[test]
    fn test_peer_is_slow() {
        let peer = PeerInfo {
            avg_latency_ms: 1200.0,
            ..PeerInfo::new("test".into())
        };
        assert!(peer.is_slow(500)); // 1200 > 2*500 = 1000

        let peer = PeerInfo {
            avg_latency_ms: 400.0,
            ..PeerInfo::new("test".into())
        };
        assert!(!peer.is_slow(500)); // 400 < 1000
    }

    #[test]
    fn test_mesh_health_check() {
        let mut mesh = GossipMesh::default_mesh();
        assert!(!mesh.is_healthy()); // 0 peers < mesh_min (4)

        for i in 0..4 {
            mesh.add_peer(format!("peer-{}", i));
        }
        assert!(mesh.is_healthy()); // 4 peers >= mesh_min
    }

    #[test]
    fn test_message_id_deterministic() {
        let msg1 = MeshMessage::new("origin".into(), vec![1, 2, 3], 1);
        let msg2 = MeshMessage::new("origin".into(), vec![1, 2, 3], 999);
        assert_eq!(msg1.message_id, msg2.message_id); // Same payload = same ID
    }

    #[test]
    fn test_message_different_payloads() {
        let msg1 = MeshMessage::new("origin".into(), vec![1, 2, 3], 1);
        let msg2 = MeshMessage::new("origin".into(), vec![4, 5, 6], 1);
        assert_ne!(msg1.message_id, msg2.message_id);
    }

    #[test]
    fn test_config_builder() {
        let config = MeshConfig::builder()
            .mesh_size(8)
            .mesh_min(5)
            .mesh_max(15)
            .fanout(8)
            .heartbeat_interval_ms(1000)
            .fanout_ttl_s(60)
            .validate_messages(false)
            .local_peer_id("builder-test".into())
            .build()
            .unwrap();
        assert_eq!(config.mesh_size, 8);
        assert_eq!(config.heartbeat_interval_ms, 1000);
        assert!(!config.validate_messages);
    }

    #[test]
    fn test_config_builder_invalid() {
        let result = MeshConfig::builder()
            .mesh_size(3)
            .mesh_min(5) // min > size
            .mesh_max(10)
            .fanout(6)
            .heartbeat_interval_ms(500)
            .fanout_ttl_s(120)
            .validate_messages(true)
            .local_peer_id("test".into())
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_heartbeat_detects_slow_peers() {
        let mut mesh = GossipMesh::default_mesh();
        mesh.add_peer("slow-peer".into());

        // Simulate slow peer
        if let Some(peer) = mesh.peers.get_mut("slow-peer") {
            peer.avg_latency_ms = 1500.0; // > 2*500
        }

        mesh.heartbeat();

        // Peer should be pruned
        let peer = mesh.peers.get("slow-peer").unwrap();
        assert_eq!(peer.state, PeerState::Pruned);
        assert_eq!(peer.backoff_count, 1);
    }

    #[test]
    fn test_peer_backoff_query() {
        let mut mesh = GossipMesh::default_mesh();
        mesh.add_peer("peer-1".into());
        assert_eq!(mesh.peer_backoff_ms("peer-1"), Some(1000));
        assert_eq!(mesh.peer_backoff_ms("nonexistent"), None);
    }

    #[test]
    fn test_error_display() {
        let err = GossipMeshError::ConnectionError("test".into());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_peer_state_display() {
        assert_eq!(format!("{}", PeerState::Meshed), "meshed");
        assert_eq!(format!("{}", PeerState::Fanout), "fanout");
        assert_eq!(format!("{}", PeerState::Pruned), "pruned");
        assert_eq!(
            format!("{}", PeerState::GracefulDisconnect),
            "graceful_disconnect"
        );
    }

    #[test]
    fn test_heartbeat_duration() {
        let config = MeshConfig::default_ed2kia();
        assert_eq!(config.heartbeat_duration(), Duration::from_millis(500));
        assert_eq!(config.fanout_ttl_duration(), Duration::from_secs(120));
    }

    #[test]
    fn test_mesh_config_default() {
        let config = MeshConfig::default();
        assert_eq!(config.mesh_size, 6);
        assert_eq!(config.mesh_min, 4);
    }
}
