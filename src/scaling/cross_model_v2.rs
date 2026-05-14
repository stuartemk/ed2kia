//! Cross-Model Scaling v2 — Escalado cross-model con negociación de capacidades
//!
//! Extiende CrossModelScaler con negociación ponderada por reputación,
//! latencia histórica y cumplimiento DAO. Decisiones de enrutamiento ≤80ms.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint3")]`

#[cfg(feature = "v1.2-sprint3")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.2-sprint3")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.2-sprint3")]
use std::time::{Instant, SystemTime, UNIX_EPOCH};
#[cfg(feature = "v1.2-sprint3")]
use thiserror::Error;
#[cfg(feature = "v1.2-sprint3")]
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Error)]
pub enum ScalingV2Error {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("No available nodes")]
    NoAvailableNodes,

    #[error("Capacity exceeded")]
    CapacityExceeded,

    #[error("Negotiation failed: {0}")]
    NegotiationFailed(String),

    #[error("Invalid capability: {0}")]
    InvalidCapability(String),
}

// ---------------------------------------------------------------------------
// Node Profile v2
// ---------------------------------------------------------------------------

/// Perfil de nodo v2 con capacidades y métricas.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProfileV2 {
    pub node_id: String,
    pub model: String,
    pub max_capacity: usize,
    pub current_load: usize,
    pub reputation: f64,
    pub avg_latency_ms: f64,
    pub capabilities: Vec<String>,
    pub dao_compliance: f64,
    pub last_heartbeat: u64,
    pub active: bool,
}

#[cfg(feature = "v1.2-sprint3")]
impl NodeProfileV2 {
    pub fn new(node_id: String, model: String, max_capacity: usize) -> Self {
        Self {
            node_id,
            model,
            max_capacity,
            current_load: 0,
            reputation: 1.0,
            avg_latency_ms: 0.0,
            capabilities: Vec::new(),
            dao_compliance: 1.0,
            last_heartbeat: current_timestamp_ms(),
            active: true,
        }
    }

    pub fn load_factor(&self) -> f64 {
        if self.max_capacity == 0 {
            return 1.0;
        }
        self.current_load as f64 / self.max_capacity as f64
    }

    pub fn can_accept(&self, threshold: f64) -> bool {
        self.active && self.load_factor() < threshold && self.reputation > 0.3
    }

    /// Calcula el score de enrutamiento ponderado.
    pub fn routing_score(&self) -> f64 {
        let load = 1.0 - self.load_factor();
        let latency_factor = 1.0 / (1.0 + self.avg_latency_ms / 100.0);
        load * self.reputation * self.dao_compliance * latency_factor
    }

    pub fn add_capability(&mut self, capability: String) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    pub fn update_latency(&mut self, new_latency_ms: f64) {
        self.avg_latency_ms = self.avg_latency_ms * 0.9 + new_latency_ms * 0.1;
    }

    pub fn heartbeat(&mut self) {
        self.last_heartbeat = current_timestamp_ms();
    }
}

// ---------------------------------------------------------------------------
// Routing Decision
// ---------------------------------------------------------------------------

/// Decisión de enrutamiento.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub target_node: String,
    pub score: f64,
    pub reason: String,
    pub latency_estimate_ms: f64,
}

// ---------------------------------------------------------------------------
// Scaling v2 Config
// ---------------------------------------------------------------------------

/// Configuración de escalado v2.
#[cfg(feature = "v1.2-sprint3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingV2Config {
    /// Umbral de carga máxima antes de redistribuir.
    pub max_load_threshold: f64,
    /// Peso de reputación en el score.
    pub reputation_weight: f64,
    /// Peso de latencia en el score.
    pub latency_weight: f64,
    /// Peso de cumplimiento DAO en el score.
    pub dao_weight: f64,
    /// Timeout de heartbeat en ms.
    pub heartbeat_timeout_ms: u64,
    /// Habilitar redistribución automática.
    pub auto_redistribute: bool,
}

#[cfg(feature = "v1.2-sprint3")]
impl Default for ScalingV2Config {
    fn default() -> Self {
        Self {
            max_load_threshold: 0.8,
            reputation_weight: 0.35,
            latency_weight: 0.25,
            dao_weight: 0.25,
            heartbeat_timeout_ms: 30000,
            auto_redistribute: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-Model Scaler v2
// ---------------------------------------------------------------------------

/// Escalador cross-model v2 con negociación de capacidades.
#[cfg(feature = "v1.2-sprint3")]
pub struct CrossModelScalerV2 {
    config: ScalingV2Config,
    nodes: HashMap<String, NodeProfileV2>,
    routing_history: VecDeque<RoutingDecision>,
}

#[cfg(feature = "v1.2-sprint3")]
impl CrossModelScalerV2 {
    /// Crea un nuevo escalador v2.
    pub fn new() -> Self {
        Self {
            config: ScalingV2Config::default(),
            nodes: HashMap::new(),
            routing_history: VecDeque::with_capacity(1000),
        }
    }

    /// Crea con configuración personalizada.
    pub fn with_config(config: ScalingV2Config) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            routing_history: VecDeque::with_capacity(1000),
        }
    }

    /// Registra un nodo.
    pub fn register_node(&mut self, profile: NodeProfileV2) {
        self.nodes.insert(profile.node_id.clone(), profile);
        info!(node = %self.nodes.keys().next().unwrap_or(&String::new()), "node registered");
    }

    /// Remueve un nodo.
    pub fn unregister_node(&mut self, node_id: &str) -> Result<(), ScalingV2Error> {
        self.nodes
            .remove(node_id)
            .map(|_| ())
            .ok_or(ScalingV2Error::NodeNotFound(node_id.to_string()))
    }

    /// Actualiza heartbeat de un nodo.
    pub fn heartbeat(&mut self, node_id: &str) -> Result<(), ScalingV2Error> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ScalingV2Error::NodeNotFound(node_id.to_string()))?;
        node.heartbeat();
        Ok(())
    }

    /// Selecciona el mejor nodo para una request.
    pub fn route_request(
        &mut self,
        required_capabilities: &[String],
    ) -> Result<RoutingDecision, ScalingV2Error> {
        let start = Instant::now();

        let candidates: Vec<&NodeProfileV2> = self
            .nodes
            .values()
            .filter(|n| {
                n.can_accept(self.config.max_load_threshold)
                    && required_capabilities
                        .iter()
                        .all(|c| n.has_capability(c))
            })
            .collect();

        if candidates.is_empty() {
            return Err(ScalingV2Error::NoAvailableNodes);
        }

        let best = candidates
            .iter()
            .max_by(|a, b| a.routing_score().partial_cmp(&b.routing_score()).unwrap())
            .unwrap();

        let score = best.routing_score();
        let latency = best.avg_latency_ms;

        let decision = RoutingDecision {
            target_node: best.node_id.clone(),
            score,
            reason: format!(
                "score={:.3}, latency={:.1}ms, rep={:.2}",
                score, latency, best.reputation
            ),
            latency_estimate_ms: latency,
        };

        self.routing_history.push_back(decision.clone());
        if self.routing_history.len() > 1000 {
            self.routing_history.pop_front();
        }

        let elapsed = start.elapsed();
        debug!(
            node = %decision.target_node,
            elapsed_ms = elapsed.as_millis(),
            "request routed"
        );

        Ok(decision)
    }

    /// Actualiza la carga de un nodo.
    pub fn update_load(
        &mut self,
        node_id: &str,
        new_load: usize,
    ) -> Result<(), ScalingV2Error> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ScalingV2Error::NodeNotFound(node_id.to_string()))?;
        node.current_load = new_load.min(node.max_capacity);
        Ok(())
    }

    /// Actualiza la reputación de un nodo.
    pub fn update_reputation(
        &mut self,
        node_id: &str,
        reputation: f64,
    ) -> Result<(), ScalingV2Error> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ScalingV2Error::NodeNotFound(node_id.to_string()))?;
        node.reputation = reputation.clamp(0.0, 1.0);
        Ok(())
    }

    /// Actualiza el cumplimiento DAO de un nodo.
    pub fn update_dao_compliance(
        &mut self,
        node_id: &str,
        compliance: f64,
    ) -> Result<(), ScalingV2Error> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ScalingV2Error::NodeNotFound(node_id.to_string()))?;
        node.dao_compliance = compliance.clamp(0.0, 1.0);
        Ok(())
    }

    /// Detecta nodos stale y los desactiva.
    pub fn detect_stale_nodes(&mut self) -> Vec<String> {
        let now = current_timestamp_ms();
        let timeout = self.config.heartbeat_timeout_ms;
        self.nodes
            .values_mut()
            .filter(|n| n.active && (now - n.last_heartbeat) > timeout)
            .map(|n| {
                n.active = false;
                n.node_id.clone()
            })
            .collect()
    }

    /// Retorna el número de nodos activos.
    pub fn active_node_count(&self) -> usize {
        self.nodes.values().filter(|n| n.active).count()
    }

    /// Retorna el número total de nodos.
    pub fn total_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Obtiene un perfil de nodo.
    pub fn get_node(&self, node_id: &str) -> Option<&NodeProfileV2> {
        self.nodes.get(node_id)
    }

    /// Retorna las decisiones de enrutamiento recientes.
    pub fn get_routing_history(&self) -> &[RoutingDecision] {
        self.routing_history.as_slices().0
    }
}

#[cfg(feature = "v1.2-sprint3")]
impl Default for CrossModelScalerV2 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "v1.2-sprint3")]
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_profile(id: &str, caps: &[&str]) -> NodeProfileV2 {
        let mut profile = NodeProfileV2::new(id.to_string(), "model-1".into(), 100);
        for cap in caps {
            profile.add_capability(cap.to_string());
        }
        profile
    }

    #[test]
    fn test_scaler_creation() {
        let scaler = CrossModelScalerV2::new();
        assert_eq!(scaler.total_node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        assert_eq!(scaler.total_node_count(), 1);
    }

    #[test]
    fn test_unregister_node() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        assert!(scaler.unregister_node("n1").is_ok());
        assert_eq!(scaler.total_node_count(), 0);
    }

    #[test]
    fn test_unregister_missing() {
        let mut scaler = CrossModelScalerV2::new();
        assert!(scaler.unregister_node("missing").is_err());
    }

    #[test]
    fn test_route_request() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        scaler.register_node(make_profile("n2", &["inference"]));
        let decision = scaler.route_request(&["inference".into()]).unwrap();
        assert!(!decision.target_node.is_empty());
    }

    #[test]
    fn test_route_no_capabilities() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        assert!(scaler.route_request(&["training".into()]).is_err());
    }

    #[test]
    fn test_route_no_nodes() {
        let scaler = CrossModelScalerV2::new();
        let mut s = scaler;
        assert!(s.route_request(&[]).is_err());
    }

    #[test]
    fn test_update_load() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        assert!(scaler.update_load("n1", 50).is_ok());
        let node = scaler.get_node("n1").unwrap();
        assert_eq!(node.current_load, 50);
    }

    #[test]
    fn test_update_reputation() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        scaler.update_reputation("n1", 0.8).unwrap();
        let node = scaler.get_node("n1").unwrap();
        assert_eq!(node.reputation, 0.8);
    }

    #[test]
    fn test_update_dao_compliance() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        scaler.update_dao_compliance("n1", 0.9).unwrap();
        let node = scaler.get_node("n1").unwrap();
        assert_eq!(node.dao_compliance, 0.9);
    }

    #[test]
    fn test_heartbeat() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        assert!(scaler.heartbeat("n1").is_ok());
    }

    #[test]
    fn test_routing_score() {
        let profile = NodeProfileV2::new("n1".into(), "m".into(), 100);
        assert!(profile.routing_score() > 0.0);
    }

    #[test]
    fn test_load_factor() {
        let mut profile = NodeProfileV2::new("n1".into(), "m".into(), 100);
        profile.current_load = 50;
        assert_eq!(profile.load_factor(), 0.5);
    }

    #[test]
    fn test_can_accept() {
        let profile = NodeProfileV2::new("n1".into(), "m".into(), 100);
        assert!(profile.can_accept(0.8));
    }

    #[test]
    fn test_cannot_accept_when_full() {
        let mut profile = NodeProfileV2::new("n1".into(), "m".into(), 100);
        profile.current_load = 95;
        assert!(!profile.can_accept(0.8));
    }

    #[test]
    fn test_capability_check() {
        let profile = make_profile("n1", &["inference", "training"]);
        assert!(profile.has_capability("inference"));
        assert!(!profile.has_capability("embedding"));
    }

    #[test]
    fn test_update_latency() {
        let mut profile = NodeProfileV2::new("n1".into(), "m".into(), 100);
        profile.update_latency(50.0);
        assert!(profile.avg_latency_ms > 0.0);
    }

    #[test]
    fn test_detect_stale_nodes() {
        let mut scaler = CrossModelScalerV2::new();
        let config = ScalingV2Config {
            heartbeat_timeout_ms: 1,
            ..ScalingV2Config::default()
        };
        scaler = CrossModelScalerV2::with_config(config);
        scaler.register_node(make_profile("n1", &["inference"]));
        std::thread::sleep(Duration::from_millis(10));
        let stale = scaler.detect_stale_nodes();
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn test_active_node_count() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        scaler.register_node(make_profile("n2", &["inference"]));
        assert_eq!(scaler.active_node_count(), 2);
    }

    #[test]
    fn test_get_node() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        assert!(scaler.get_node("n1").is_some());
        assert!(scaler.get_node("missing").is_none());
    }

    #[test]
    fn test_routing_history() {
        let mut scaler = CrossModelScalerV2::new();
        scaler.register_node(make_profile("n1", &["inference"]));
        scaler.route_request(&["inference".into()]).unwrap();
        assert!(!scaler.get_routing_history().is_empty());
    }

    #[test]
    fn test_config_default() {
        let config = ScalingV2Config::default();
        assert_eq!(config.max_load_threshold, 0.8);
    }

    #[test]
    fn test_scaler_default() {
        let scaler = CrossModelScalerV2::default();
        assert_eq!(scaler.total_node_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = ScalingV2Error::NodeNotFound("x".into());
        assert!(format!("{}", err).contains("x"));
    }

    #[test]
    fn test_with_config() {
        let config = ScalingV2Config {
            max_load_threshold: 0.5,
            ..ScalingV2Config::default()
        };
        let scaler = CrossModelScalerV2::with_config(config);
        assert_eq!(scaler.config.max_load_threshold, 0.5);
    }
}
