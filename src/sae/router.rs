//! LayerRouter - Sharding Dinámico con Leases para capas SAE
//!
//! Gestiona la asignación de capas SAE entre nodos basado en:
//! - RAM disponible
//! - Núcleos de CPU
//! - Ancho de banda
//! - Latencia de red
//!
//! Usa un sistema de leases (5-10 min) con renovación automática.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tracing::{debug, info};

use crate::p2p::protocol::{LeaseRequest, LeaseResponse, NodeResources};

/// Duración default del lease (5 minutos)
const DEFAULT_LEASE_DURATION: Duration = Duration::from_secs(300);

/// Duración máxima del lease (10 minutos)
const MAX_LEASE_DURATION: Duration = Duration::from_secs(600);

/// Intervalo de renovación (30 segundos antes de expirar)
const RENEWAL_THRESHOLD: Duration = Duration::from_secs(30);

/// Score mínimo para asignar una capa a un nodo
const MIN_ASSIGNMENT_SCORE: f64 = 0.5;

// ============================================================================
// Lease Management
// ============================================================================

/// Lease activo para una capa SAE
#[derive(Debug, Clone, PartialEq)]
pub struct LayerLease {
    /// ID de la capa SAE
    pub layer_id: u32,
    /// Peer ID del nodo que posee el lease
    pub owner_peer_id: String,
    /// Timestamp de creación
    pub created_at: Instant,
    /// Timestamp de expiración
    pub expires_at: Instant,
    /// Duración del lease
    pub duration: Duration,
    /// Número de renovaciones
    pub renewal_count: u32,
}

impl LayerLease {
    /// Crear nuevo lease
    pub fn new(layer_id: u32, owner_peer_id: String, duration: Duration) -> Self {
        let now = Instant::now();
        Self {
            layer_id,
            owner_peer_id,
            created_at: now,
            expires_at: now + duration,
            duration,
            renewal_count: 0,
        }
    }

    /// Verificar si el lease ha expirado
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    /// Verificar si el lease necesita renovación pronto
    pub fn needs_renewal(&self) -> bool {
        let remaining = self.expires_at.saturating_duration_since(Instant::now());
        remaining <= RENEWAL_THRESHOLD
    }

    /// Renovar el lease
    pub fn renew(&mut self) {
        let now = Instant::now();
        self.expires_at = now + self.duration;
        self.renewal_count += 1;
        debug!(
            "Lease renovado para layer {}: renewals={}",
            self.layer_id, self.renewal_count
        );
    }

    /// Tiempo restante hasta expiración
    pub fn time_remaining(&self) -> Duration {
        self.expires_at
            .saturating_duration_since(Instant::now())
    }
}

// ============================================================================
// Node Scoring
// ============================================================================

/// Score de aptitud de un nodo para una capa SAE
#[derive(Debug, Clone)]
pub struct NodeScore {
    /// Peer ID del nodo
    pub peer_id: String,
    /// Score total (0.0 - 1.0)
    pub total_score: f64,
    /// Componentes del score
    pub ram_score: f64,
    pub cpu_score: f64,
    pub bandwidth_score: f64,
    pub latency_score: f64,
    pub gpu_bonus: f64,
}

impl NodeScore {
    /// Calcular score basado en recursos
    pub fn calculate(resources: &NodeResources, layer_memory_req_mb: f64) -> Self {
        // Score de RAM (peso: 30%)
        let ram_score = Self::score_ram(resources, layer_memory_req_mb);

        // Score de CPU (peso: 25%)
        let cpu_score = Self::score_cpu(resources);

        // Score de bandwidth (peso: 25%)
        let bandwidth_score = Self::score_bandwidth(resources);

        // Score de latencia (peso: 20%)
        let latency_score = Self::score_latency(resources);

        // Bonus por GPU (hasta +0.15)
        let gpu_bonus = if resources.has_gpu { 0.15 } else { 0.0 };

        let total_score =
            ram_score * 0.30 + cpu_score * 0.25 + bandwidth_score * 0.25 + latency_score * 0.20;
        let total_score = (total_score + gpu_bonus).min(1.0);

        Self {
            peer_id: String::new(), // Se establece externamente
            total_score,
            ram_score,
            cpu_score,
            bandwidth_score,
            latency_score,
            gpu_bonus,
        }
    }

    /// Score de RAM
    fn score_ram(resources: &NodeResources, layer_memory_req_mb: f64) -> f64 {
        let available_mb = resources.available_ram_gb * 1024.0;
        if available_mb < layer_memory_req_mb {
            return 0.0;
        }
        // Score basado en ratio de RAM disponible vs requerida
        let ratio = available_mb / layer_memory_req_mb;
        (ratio.min(4.0) / 4.0).clamp(0.0, 1.0)
    }

    /// Score de CPU
    fn score_cpu(resources: &NodeResources) -> f64 {
        // Más cores = mejor score
        let cores = resources.cpu_cores as f64;
        (cores.min(32.0) / 32.0).clamp(0.0, 1.0)
    }

    /// Score de bandwidth
    fn score_bandwidth(resources: &NodeResources) -> f64 {
        // Más bandwidth = mejor score
        let bw = resources.bandwidth_mbps;
        (bw.min(1000.0) / 1000.0).clamp(0.0, 1.0)
    }

    /// Score de latencia (menor latencia = mejor score)
    fn score_latency(resources: &NodeResources) -> f64 {
        // Menor latencia = mejor score
        let latency = resources.avg_latency_ms;
        if latency > 100.0 {
            return 0.1;
        }
        (1.0 - latency / 100.0).clamp(0.0, 1.0)
    }
}

impl Eq for NodeScore {}

impl PartialEq for NodeScore {
    fn eq(&self, other: &Self) -> bool {
        (self.total_score - other.total_score).abs() < f64::EPSILON
    }
}

impl Ord for NodeScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.total_score.partial_cmp(&other.total_score).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for NodeScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ============================================================================
// LayerRouter
// ============================================================================

/// Estado de una capa SAE
#[derive(Debug, Clone, PartialEq)]
pub enum LayerState {
    /// Capa no asignada
    Unassigned,
    /// Capa asignada a un nodo con lease activo
    Assigned {
        peer_id: String,
        lease: LayerLease,
    },
    /// Capa en proceso de reasignación
    Reassigning,
}

/// LayerRouter - Gestiona sharding dinámico de capas SAE
pub struct LayerRouter {
    /// Número total de capas SAE en el modelo
    total_layers: u32,
    /// Estado de cada capa
    layer_states: HashMap<u32, LayerState>,
    /// Leases activos por peer
    peer_leases: HashMap<String, Vec<LayerLease>>,
    /// Recursos de peers conocidos
    peer_resources: HashMap<String, NodeResources>,
    /// Duración default de leases
    default_lease_duration: Duration,
    /// Capas asignadas localmente a este nodo
    local_layers: HashSet<u32>,
}

impl LayerRouter {
    /// Crear nuevo LayerRouter
    pub fn new() -> Self {
        Self {
            total_layers: 32, // Default para Qwen2-7B
            layer_states: HashMap::new(),
            peer_leases: HashMap::new(),
            peer_resources: HashMap::new(),
            default_lease_duration: DEFAULT_LEASE_DURATION,
            local_layers: HashSet::new(),
        }
    }

    /// Configurar número total de capas
    pub fn with_total_layers(mut self, total_layers: u32) -> Self {
        self.total_layers = total_layers;
        for layer_id in 0..total_layers {
            self.layer_states.insert(layer_id, LayerState::Unassigned);
        }
        self
    }

    /// Configurar duración default de leases
    pub fn with_lease_duration(mut self, duration: Duration) -> Self {
        self.default_lease_duration = duration.clamp(
            Duration::from_secs(300),
            Duration::from_secs(600),
        );
        self
    }

    /// Registrar recursos de un peer
    pub fn register_peer_resources(&mut self, peer_id: String, resources: NodeResources) {
        info!(
            "Recursos registrados para peer {}: {} cores, {} GB RAM",
            peer_id, resources.cpu_cores, resources.available_ram_gb
        );
        self.peer_resources.insert(peer_id, resources);
    }

    /// Solicitar lease para capas SAE
    pub fn request_lease(&mut self, request: &LeaseRequest) -> LeaseResponse {
        info!(
            "LeaseRequest de {}: layers={:?}, duration={}s",
            request.requester_id, request.layers, request.duration_secs
        );

        let duration = Duration::from_secs(
            request
                .duration_secs
                .min(MAX_LEASE_DURATION.as_secs()),
        );

        let mut assigned_layers = Vec::new();
        let mut denial_reasons = Vec::new();

        for &layer_id in &request.layers {
            // Verificar que la capa existe
            if layer_id >= self.total_layers {
                denial_reasons.push(format!("Layer {} no existe", layer_id));
                continue;
            }

            // Verificar que la capa está disponible
            match &self.layer_states.get(&layer_id) {
                Some(LayerState::Unassigned) | None => {
                    // Calcular score del solicitante
                    let memory_req_mb = self.estimate_layer_memory(layer_id);
                    let score = NodeScore::calculate(&request.resources, memory_req_mb);

                    if score.total_score >= MIN_ASSIGNMENT_SCORE {
                        // Aprovar lease
                        let lease = LayerLease::new(
                            layer_id,
                            request.requester_id.clone(),
                            duration,
                        );

                        self.layer_states
                            .insert(layer_id, LayerState::Assigned {
                                peer_id: request.requester_id.clone(),
                                lease: lease.clone(),
                            });

                        self.peer_leases
                            .entry(request.requester_id.clone())
                            .or_default()
                            .push(lease);

                        assigned_layers.push(layer_id);
                        info!(
                            "Lease aprobado: layer={} -> peer={} (score={:.3})",
                            layer_id, request.requester_id, score.total_score
                        );
                    } else {
                        denial_reasons.push(format!(
                            "Score insuficiente para layer {}: {:.3} < {}",
                            layer_id, score.total_score, MIN_ASSIGNMENT_SCORE
                        ));
                    }
                }
                Some(LayerState::Assigned { peer_id, lease }) => {
                    if lease.owner_peer_id == request.requester_id {
                        // Renovación del mismo owner
                        let mut lease = lease.clone();
                        lease.renew();
                        self.layer_states.insert(
                            layer_id,
                            LayerState::Assigned {
                                peer_id: request.requester_id.clone(),
                                lease: lease.clone(),
                            },
                        );
                        assigned_layers.push(layer_id);
                    } else {
                        denial_reasons.push(format!(
                            "Layer {} ya asignada a {}",
                            layer_id, peer_id
                        ));
                    }
                }
                Some(LayerState::Reassigning) => {
                    denial_reasons.push(format!("Layer {} en reasignación", layer_id));
                }
            }
        }

        let expires_at = Some(
            std::time::SystemTime::now()
                .checked_add(duration)
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );

        LeaseResponse {
            granted: !assigned_layers.is_empty(),
            expires_at,
            assigned_layers,
            denial_reason: if !denial_reasons.is_empty() {
                Some(denial_reasons.join("; "))
            } else {
                None
            },
        }
    }

    /// Estimar memoria requerida para una capa (en MB)
    fn estimate_layer_memory(&self, _layer_id: u32) -> f64 {
        // TODO: Phase 2 - Calcular basado en config real del SAE
        // Estimación: input_dim * latent_dim * 4 bytes (f32) * 2 (enc + dec)
        // Para Qwen2-7B: 4096 * 16384 * 4 * 2 ≈ 512 MB por capa
        512.0
    }

    /// Verificar y renovar leases expirados
    pub fn check_and_renew_leases(&mut self) -> Vec<LayerLease> {
        let mut expired = Vec::new();

        for (layer_id, state) in &mut self.layer_states {
            if let LayerState::Assigned { peer_id, lease } = state {
                // MIGRATION: Clone lease before reassigning to avoid borrow conflict
                let lease_clone = lease.clone();
                if lease.is_expired() {
                    info!("Lease expirado: layer={} peer={}", layer_id, peer_id);
                    *state = LayerState::Unassigned;
                    expired.push(lease_clone);
                } else if lease.needs_renewal() {
                    debug!("Lease necesita renovación: layer={}", layer_id);
                    // TODO: Phase 2 - Enviar solicitud de renovación al peer
                }
            }
        }

        // Limpiar leases expirados de peer_leases
        let peer_ids: Vec<String> = self.peer_leases.keys().cloned().collect();
        for peer_id in peer_ids {
            if let Some(leases) = self.peer_leases.get_mut(&peer_id) {
                leases.retain(|l| !l.is_expired());
                if leases.is_empty() {
                    self.peer_leases.remove(&peer_id);
                }
            }
        }

        expired
    }

    /// Reasignar capas de un peer que se desconectó
    pub fn reassign_peer_layers(&mut self, peer_id: &str) -> Vec<u32> {
        info!(
            "Reasignando capas de peer desconectado: {}",
            peer_id
        );

        let mut layers_to_reassign = Vec::new();

        for (layer_id, state) in &mut self.layer_states {
            if let LayerState::Assigned {
                peer_id: owner, ..
            } = state
            {
                if owner == peer_id {
                    *state = LayerState::Reassigning;
                    layers_to_reassign.push(*layer_id);
                }
            }
        }

        // Marcar como unassigned para nueva asignación
        for layer_id in &layers_to_reassign {
            self.layer_states
                .insert(*layer_id, LayerState::Unassigned);
        }

        // TODO: Phase 2 - Iniciar proceso de reasignación automática
        // self.auto_reassign(layers_to_reassign);

        layers_to_reassign
    }

    /// Asignar capas localmente (para este nodo)
    pub fn assign_local_layers(&mut self, layers: Vec<u32>) {
        for layer_id in layers {
            self.local_layers.insert(layer_id);
            info!("Capa {} asignada localmente", layer_id);
        }
    }

    /// Verificar si una capa está asignada localmente
    pub fn is_local_layer(&self, layer_id: u32) -> bool {
        self.local_layers.contains(&layer_id)
    }

    /// Obtener capas que necesita este nodo procesar
    pub fn get_local_layers(&self) -> Vec<u32> {
        self.local_layers.iter().copied().collect()
    }

    /// Obtener estado de una capa
    pub fn get_layer_state(&self, layer_id: u32) -> Option<&LayerState> {
        self.layer_states.get(&layer_id)
    }

    /// Obtener número de leases activos
    pub fn active_lease_count(&self) -> usize {
        self.layer_states
            .values()
            .filter(|s| matches!(s, LayerState::Assigned { .. }))
            .count()
    }

    /// Obtener peers con leases activos
    pub fn peers_with_leases(&self) -> Vec<String> {
        self.peer_leases.keys().cloned().collect()
    }

    /// Calcular mejor peer para una capa dada
    pub fn find_best_peer_for_layer(&self, layer_id: u32) -> Option<NodeScore> {
        let memory_req = self.estimate_layer_memory(layer_id);
        let mut best_score: Option<NodeScore> = None;

        for (peer_id, resources) in &self.peer_resources {
            let mut score = NodeScore::calculate(resources, memory_req);
            score.peer_id = peer_id.clone();

            if best_score.as_ref().is_none_or(|b| score.total_score > b.total_score) {
                best_score = Some(score);
            }
        }

        best_score.filter(|s| s.total_score >= MIN_ASSIGNMENT_SCORE)
    }

    /// Estadísticas del router
    pub fn stats(&self) -> RouterStats {
        let mut assigned = 0;
        let mut unassigned = 0;
        let mut reassigning = 0;

        for state in self.layer_states.values() {
            match state {
                LayerState::Assigned { .. } => assigned += 1,
                LayerState::Unassigned => unassigned += 1,
                LayerState::Reassigning => reassigning += 1,
            }
        }

        RouterStats {
            total_layers: self.total_layers,
            assigned,
            unassigned,
            reassigning,
            active_peers: self.peer_resources.len(),
            total_leases: self
                .peer_leases
                .values()
                .map(|v| v.len())
                .sum(),
        }
    }
}

/// Estadísticas del LayerRouter
#[derive(Debug, Clone)]
pub struct RouterStats {
    pub total_layers: u32,
    pub assigned: usize,
    pub unassigned: usize,
    pub reassigning: usize,
    pub active_peers: usize,
    pub total_leases: usize,
}

impl Default for LayerRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_lease_creation() {
        let lease = LayerLease::new(0, "peer_1".to_string(), DEFAULT_LEASE_DURATION);
        assert_eq!(lease.layer_id, 0);
        assert_eq!(lease.owner_peer_id, "peer_1");
        assert!(!lease.is_expired());
    }

    #[test]
    fn test_node_score_calculation() {
        let resources = NodeResources {
            cpu_cores: 16,
            available_ram_gb: 32.0,
            bandwidth_mbps: 500.0,
            avg_latency_ms: 5.0,
            has_gpu: true,
            gpu_model: Some("RTX 4090".to_string()),
            vram_gb: Some(24.0),
        };

        let score = NodeScore::calculate(&resources, 512.0);
        assert!(score.total_score > MIN_ASSIGNMENT_SCORE);
        assert!(score.gpu_bonus > 0.0);
    }

    #[test]
    fn test_layer_router_lease_request() {
        let mut router = LayerRouter::new().with_total_layers(32);

        let request = LeaseRequest {
            requester_id: "peer_1".to_string(),
            layers: vec![0, 1, 2],
            duration_secs: 300,
            resources: NodeResources {
                cpu_cores: 8,
                available_ram_gb: 16.0,
                bandwidth_mbps: 100.0,
                avg_latency_ms: 10.0,
                has_gpu: false,
                gpu_model: None,
                vram_gb: None,
            },
        };

        let response = router.request_lease(&request);
        assert!(response.granted);
        assert!(!response.assigned_layers.is_empty());
    }

    #[test]
    fn test_router_stats() {
        let router = LayerRouter::new().with_total_layers(16);
        let stats = router.stats();
        assert_eq!(stats.total_layers, 16);
        assert_eq!(stats.unassigned, 16);
    }
}
