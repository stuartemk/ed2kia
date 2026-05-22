//! Peer Manager - Gestión de conexiones, límites y scoring adaptativo
//!
//! Implementa scoring dinámico basado en latencia, throughput y reputación
//! criptográfica. Penaliza nodos con >5% de mensajes inválidos.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
// CLEANUP: removed unused import std::sync::Arc
use std::time::Instant;

use libp2p::PeerId;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

/// Límites de conexión por categoría de nodo
#[derive(Debug, Clone)]
pub struct ConnectionLimits {
    /// Máximo de conexiones entrantes totales
    pub max_inbound: usize,
    /// Máximo de conexiones salientes totales
    pub max_outbound: usize,
    /// Máximo de conexiones por IP
    pub max_per_ip: usize,
    /// Máximo de peers en mesh GossipSub
    pub mesh_n: usize,
    /// Máximo de peers en mesh GossipSub (límite superior)
    pub mesh_n_max: usize,
    /// Mínimo de peers en mesh GossipSub
    pub mesh_n_min: usize,
    /// Fanout TTL en segundos
    pub fanout_ttl: u32,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_inbound: 100,
            max_outbound: 50,
            max_per_ip: 3,
            mesh_n: 6,
            mesh_n_max: 12,
            mesh_n_min: 4,
            fanout_ttl: 12,
        }
    }
}

impl ConnectionLimits {
    /// Ajusta parámetros dinámicamente según tamaño de red
    pub fn adapt_for_network_size(&mut self, active_peers: usize) {
        if active_peers > 1000 {
            // Red masiva: más conexiones, mesh más grande
            self.max_inbound = 200;
            self.max_outbound = 100;
            self.mesh_n = 8;
            self.mesh_n_max = 16;
            self.mesh_n_min = 5;
            self.fanout_ttl = 15;
        } else if active_peers > 100 {
            // Red mediana
            self.max_inbound = 150;
            self.max_outbound = 75;
            self.mesh_n = 7;
            self.mesh_n_max = 14;
            self.mesh_n_min = 4;
            self.fanout_ttl = 12;
        } else if active_peers < 10 {
            // Red pequeña: menos overhead
            self.max_inbound = 50;
            self.max_outbound = 25;
            self.mesh_n = 4;
            self.mesh_n_max = 8;
            self.mesh_n_min = 2;
            self.fanout_ttl = 8;
        }
    }
}

/// Métricas de rendimiento por peer
#[derive(Debug, Clone)]
pub struct PeerMetrics {
    /// Latencia promedio en milisegundos
    pub avg_latency_ms: f64,
    /// Throughput en bytes/segundo
    pub throughput_bps: u64,
    /// Total de mensajes recibidos
    pub total_messages: u64,
    /// Mensajes inválidos recibidos
    pub invalid_messages: u64,
    /// Última actividad
    pub last_activity: Instant,
    /// Score de reputación criptográfica (0.0 - 1.0)
    pub crypto_reputation: f64,
}

impl Default for PeerMetrics {
    fn default() -> Self {
        Self {
            avg_latency_ms: 0.0,
            throughput_bps: 0,
            total_messages: 0,
            invalid_messages: 0,
            last_activity: Instant::now(),
            crypto_reputation: 0.5,
        }
    }
}

impl PeerMetrics {
    /// Calcula porcentaje de mensajes inválidos
    pub fn invalid_percentage(&self) -> f64 {
        if self.total_messages == 0 {
            return 0.0;
        }
        self.invalid_messages as f64 / self.total_messages as f64
    }

    /// Determina si el peer excede el umbral de mensajes inválidos (>5%)
    pub fn exceeds_invalid_threshold(&self, threshold: f64) -> bool {
        self.invalid_percentage() > threshold
    }

    /// Actualiza latencia con promedio exponencial
    pub fn update_latency(&mut self, new_latency_ms: f64, alpha: f64) {
        self.avg_latency_ms = alpha * new_latency_ms + (1.0 - alpha) * self.avg_latency_ms;
    }

    /// Registra mensaje recibido
    pub fn record_message(&mut self, is_valid: bool) {
        self.total_messages += 1;
        if !is_valid {
            self.invalid_messages += 1;
        }
        self.last_activity = Instant::now();
    }
}

/// Score compuesto para priorización de peers
// FIX: trait bound - f64 does not implement Eq or Ord due to NaN/Infinity
// Use total ordering via to_bits() for consistent comparison
#[derive(Debug, Clone, Copy)]
pub struct PeerScore(f64);

impl PartialEq for PeerScore {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for PeerScore {}

impl PartialOrd for PeerScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PeerScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Less)
    }
}

impl PeerScore {
    /// Calcula score basado en métricas
    pub fn from_metrics(metrics: &PeerMetrics, invalid_threshold: f64) -> Self {
        let mut score = 0.0;

        // Componente de latencia (menor = mejor, max 30 pts)
        let latency_score = if metrics.avg_latency_ms < 50.0 {
            30.0
        } else if metrics.avg_latency_ms < 200.0 {
            20.0
        } else if metrics.avg_latency_ms < 500.0 {
            10.0
        } else {
            5.0
        };
        score += latency_score;

        // Componente de throughput (mayor = mejor, max 20 pts)
        let throughput_score = if metrics.throughput_bps > 1_000_000 {
            20.0
        } else if metrics.throughput_bps > 100_000 {
            15.0
        } else if metrics.throughput_bps > 10_000 {
            10.0
        } else {
            5.0
        };
        score += throughput_score;

        // Componente de reputación criptográfica (max 30 pts)
        score += metrics.crypto_reputation * 30.0;

        // Penalización por mensajes inválidos (max -40 pts)
        let invalid_pct = metrics.invalid_percentage();
        if invalid_pct > invalid_threshold {
            // FIX: trait bound - use f64::min instead of std::cmp::min (f64 doesn't implement Ord)
            let penalty = f64::min(40.0, (invalid_pct / invalid_threshold) * 20.0);
            score -= penalty;
        }

        // Penalización por inactividad (max -20 pts)
        let idle_seconds = metrics.last_activity.elapsed().as_secs_f64();
        if idle_seconds > 300.0 {
            // FIX: trait bound - use f64::min instead of std::cmp::min (f64 doesn't implement Ord)
            score -= f64::min(20.0, (idle_seconds - 300.0) / 60.0);
        }

        PeerScore(score.clamp(0.0, 100.0))
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn is_good(&self) -> bool {
        self.0 >= 50.0
    }

    pub fn is_penalized(&self) -> bool {
        self.0 < 30.0
    }
}

/// Estado de conexión del peer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Connected,
    Disconnected,
    Banned,
    Pending,
}

/// Registro completo de un peer
#[derive(Debug, Clone)]
pub struct PeerRecord {
    pub peer_id: PeerId,
    pub state: PeerState,
    pub metrics: PeerMetrics,
    pub score: PeerScore,
    pub connected_at: Instant,
    pub address: Option<String>,
}

impl PeerRecord {
    pub fn new(peer_id: PeerId) -> Self {
        let metrics = PeerMetrics::default();
        Self {
            peer_id,
            state: PeerState::Pending,
            metrics,
            score: PeerScore(50.0),
            connected_at: Instant::now(),
            address: None,
        }
    }

    /// Actualiza score basado en métricas actuales
    pub fn recalculate_score(&mut self, invalid_threshold: f64) {
        self.score = PeerScore::from_metrics(&self.metrics, invalid_threshold);

        // Auto-ban si score es muy bajo
        if self.score.is_penalized() && self.metrics.invalid_percentage() > 0.10 {
            self.state = PeerState::Banned;
            warn!(
                peer = %self.peer_id,
                score = self.score.value(),
                invalid_pct = self.metrics.invalid_percentage(),
                "Peer banned due to low score and high invalid message rate"
            );
        }
    }
}

/// Manager central de peers
pub struct PeerManager {
    /// Registro de peers
    peers: RwLock<HashMap<PeerId, PeerRecord>>,
    /// Límites de conexión
    limits: RwLock<ConnectionLimits>,
    /// Umbral de mensajes inválidos (default 5%)
    invalid_threshold: f64,
    /// Total de conexiones activas
    active_connections: AtomicUsize,
    /// Total de bytes transferidos
    total_bytes: AtomicU64,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            limits: RwLock::new(ConnectionLimits::default()),
            invalid_threshold: 0.05, // 5%
            active_connections: AtomicUsize::new(0),
            total_bytes: AtomicU64::new(0),
        }
    }

    pub fn with_invalid_threshold(mut self, threshold: f64) -> Self {
        self.invalid_threshold = threshold;
        self
    }

    /// Registra un nuevo peer
    pub fn register_peer(&self, peer_id: PeerId, address: Option<String>) {
        let record = PeerRecord::new(peer_id);
        let record = PeerRecord { address, ..record };

        let mut peers = self.peers.write();
        if peers.insert(peer_id, record).is_none() {
            self.active_connections.fetch_add(1, Ordering::Relaxed);
            info!(peer = %peer_id, "New peer registered");
        }
    }

    /// Marca peer como conectado
    pub fn mark_connected(&self, peer_id: PeerId) {
        if let Some(record) = self.peers.write().get_mut(&peer_id) {
            record.state = PeerState::Connected;
            record.connected_at = Instant::now();
        }
    }

    /// Marca peer como desconectado
    pub fn mark_disconnected(&self, peer_id: PeerId) {
        if let Some(record) = self.peers.write().get_mut(&peer_id) {
            record.state = PeerState::Disconnected;
        }
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Registra mensaje recibido de un peer
    pub fn record_message(&self, peer_id: PeerId, is_valid: bool, bytes: usize) {
        if let Some(record) = self.peers.write().get_mut(&peer_id) {
            record.metrics.record_message(is_valid);
            record.metrics.throughput_bps += bytes as u64;
            self.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
            record.recalculate_score(self.invalid_threshold);
        }
    }

    /// Actualiza latencia de un peer
    pub fn update_latency(&self, peer_id: PeerId, latency_ms: f64) {
        if let Some(record) = self.peers.write().get_mut(&peer_id) {
            record.metrics.update_latency(latency_ms, 0.1);
            record.recalculate_score(self.invalid_threshold);
        }
    }

    /// Actualiza reputación criptográfica de un peer
    pub fn update_crypto_reputation(&self, peer_id: PeerId, reputation: f64) {
        if let Some(record) = self.peers.write().get_mut(&peer_id) {
            record.metrics.crypto_reputation = reputation.clamp(0.0, 1.0);
            record.recalculate_score(self.invalid_threshold);
        }
    }

    /// Obtiene score de un peer
    pub fn get_peer_score(&self, peer_id: &PeerId) -> Option<PeerScore> {
        self.peers.read().get(peer_id).map(|r| r.score)
    }

    /// Obtiene métricas de un peer
    pub fn get_peer_metrics(&self, peer_id: &PeerId) -> Option<PeerMetrics> {
        self.peers.read().get(peer_id).map(|r| r.metrics.clone())
    }

    /// Obtiene lista de peers ordenados por score (mejores primero)
    pub fn get_sorted_peers(&self) -> Vec<PeerRecord> {
        let mut peers: Vec<PeerRecord> = self.peers.read().values().cloned().collect();
        peers.sort_by(|a, b| b.score.cmp(&a.score));
        peers
    }

    /// Obtiene peers buenos para mesh
    pub fn get_mesh_peers(&self) -> Vec<PeerRecord> {
        self.get_sorted_peers()
            .into_iter()
            .filter(|p| p.state == PeerState::Connected && p.score.is_good())
            .take(self.limits.read().mesh_n_max)
            .collect()
    }

    /// Verifica si se puede aceptar nueva conexión entrante
    pub fn can_accept_inbound(&self) -> bool {
        let current = self.active_connections.load(Ordering::Relaxed);
        current < self.limits.read().max_inbound
    }

    /// Verifica si se puede crear nueva conexión saliente
    pub fn can_create_outbound(&self) -> bool {
        let current = self.active_connections.load(Ordering::Relaxed);
        current < self.limits.read().max_outbound
    }

    /// Ajusta límites según tamaño de red
    pub fn adapt_limits(&self) {
        let active = self.active_connections.load(Ordering::Relaxed);
        let mut limits = self.limits.write();
        limits.adapt_for_network_size(active);
        debug!(
            active_peers = active,
            max_inbound = limits.max_inbound,
            mesh_n = limits.mesh_n,
            "Connection limits adapted"
        );
    }

    /// Obtiene límites actuales
    pub fn get_limits(&self) -> ConnectionLimits {
        self.limits.read().clone()
    }

    /// Obtiene estadísticas generales
    pub fn stats(&self) -> PeerManagerStats {
        let peers = self.peers.read();
        let connected = peers
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .count();
        let banned = peers
            .values()
            .filter(|p| p.state == PeerState::Banned)
            .count();

        PeerManagerStats {
            total_registered: peers.len(),
            connected,
            banned,
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            limits: self.limits.read().clone(),
        }
    }

    /// Elimina peer del registro
    pub fn remove_peer(&self, peer_id: &PeerId) {
        if self.peers.write().remove(peer_id).is_some() {
            self.active_connections.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

/// Estadísticas del PeerManager
#[derive(Debug)]
pub struct PeerManagerStats {
    pub total_registered: usize,
    pub connected: usize,
    pub banned: usize,
    pub active_connections: usize,
    pub total_bytes: u64,
    pub limits: ConnectionLimits,
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_manager_creation() {
        let manager = PeerManager::new();
        let stats = manager.stats();
        assert_eq!(stats.total_registered, 0);
        assert_eq!(stats.connected, 0);
    }

    #[test]
    fn test_connection_limits_default() {
        let limits = ConnectionLimits::default();
        assert_eq!(limits.max_inbound, 100);
        assert_eq!(limits.mesh_n, 6);
    }

    #[test]
    fn test_invalid_percentage() {
        let mut metrics = PeerMetrics::default();
        assert_eq!(metrics.invalid_percentage(), 0.0);

        metrics.record_message(true);
        metrics.record_message(true);
        metrics.record_message(false);
        assert!((metrics.invalid_percentage() - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_peer_score_calculation() {
        let metrics = PeerMetrics::default();
        let score = PeerScore::from_metrics(&metrics, 0.05);
        assert!(score.value() >= 0.0);
        assert!(score.value() <= 100.0);
    }
}
