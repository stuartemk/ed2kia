//! Capability Negotiator — Negociación de capacidades entre nodos cross-model
//!
//! Motor de negociación basado en registro de capacidades, puntuación ponderada
//! y detección automática de nodos incompatibles.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Error del negociador de capacidades
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum CapabilityError {
    #[error("Capacidad no registrada: {0}")]
    CapabilityNotRegistered(String),
    #[error("Nodo no encontrado: {0}")]
    NodeNotFound(String),
    #[error("Incompatibilidad de capacidades: {0}")]
    CapabilityIncompatible(String),
    #[error("Score por debajo del umbral: {0}")]
    ScoreBelowThreshold(f64),
    #[error("Error de negociación: {0}")]
    NegotiationFailed(String),
}

/// Capacidad registrada en el sistema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEntry {
    /// Identificador único de la capacidad
    pub capability_id: String,
    /// Descripción de la capacidad
    pub description: String,
    /// Versión de la capacidad
    pub version: String,
    /// Nodos que soportan esta capacidad
    pub supporting_nodes: Vec<String>,
    /// Peso base para scoring
    pub base_weight: f64,
}

impl CapabilityEntry {
    pub fn new(
        capability_id: String,
        description: String,
        version: String,
        base_weight: f64,
    ) -> Self {
        Self {
            capability_id,
            description,
            version,
            supporting_nodes: Vec::new(),
            base_weight,
        }
    }

    pub fn add_node(&mut self, node_id: String) {
        if !self.supporting_nodes.contains(&node_id) {
            self.supporting_nodes.push(node_id);
        }
    }

    pub fn supports_node(&self, node_id: &str) -> bool {
        self.supporting_nodes.contains(&node_id.to_string())
    }
}

/// Perfil de capacidad de un nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilityProfile {
    /// Identificador del nodo
    pub node_id: String,
    /// Capacidades soportadas
    pub capabilities: Vec<String>,
    /// Puntuación de reputación del nodo
    pub reputation_score: f64,
    /// Latencia promedio en ms
    pub avg_latency_ms: f64,
    /// Factor de cumplimiento DAO
    pub dao_compliance: f64,
    /// Timestamp del último heartbeat
    pub last_heartbeat_ms: u64,
}

impl NodeCapabilityProfile {
    pub fn new(node_id: String, capabilities: Vec<String>) -> Self {
        Self {
            node_id,
            capabilities,
            reputation_score: 1.0,
            avg_latency_ms: 100.0,
            dao_compliance: 1.0,
            last_heartbeat_ms: current_timestamp_ms(),
        }
    }

    /// Calcula el score de negociación para un conjunto de capacidades requeridas
    pub fn negotiation_score(&self, required_capabilities: &[String]) -> f64 {
        let matched = self
            .capabilities
            .iter()
            .filter(|c| required_capabilities.contains(c))
            .count() as f64;
        let total = required_capabilities.len() as f64;
        let capability_ratio = if total == 0.0 { 1.0 } else { matched / total };
        let reputation_factor = self.reputation_score.clamp(0.0, 1.0);
        let latency_factor = 1.0 / (1.0 + self.avg_latency_ms / 200.0);
        let dao_factor = self.dao_compliance.clamp(0.0, 1.0);
        (capability_ratio * reputation_factor * latency_factor * dao_factor).clamp(0.0, 1.0)
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat_ms = current_timestamp_ms();
    }

    pub fn is_stale(&self, max_stale_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.last_heartbeat_ms) > max_stale_ms
    }
}

/// Resultado de negociación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationResult {
    /// Identificador del nodo seleccionado
    pub selected_node: String,
    /// Score de negociación
    pub negotiation_score: f64,
    /// Capacidades coincidentes
    pub matched_capabilities: Vec<String>,
    /// Timestamp de la decisión
    pub timestamp_ms: u64,
}

impl NegotiationResult {
    pub fn new(
        selected_node: String,
        negotiation_score: f64,
        matched_capabilities: Vec<String>,
    ) -> Self {
        Self {
            selected_node,
            negotiation_score,
            matched_capabilities,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

/// Configuración del negociador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatorConfig {
    /// Umbral mínimo de score para aceptar negociación
    pub min_score_threshold: f64,
    /// Tiempo máximo de inactividad antes de considerar nodo stale (ms)
    pub max_stale_ms: u64,
    /// Número máximo de candidatos a considerar
    pub max_candidates: usize,
    /// Habilitar redistribución automática
    pub auto_redistribute: bool,
}

impl Default for NegotiatorConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 0.5,
            max_stale_ms: 30_000,
            max_candidates: 10,
            auto_redistribute: true,
        }
    }
}

/// Negociador de capacidades
pub struct CapabilityNegotiator {
    config: NegotiatorConfig,
    capabilities: Vec<CapabilityEntry>,
    node_profiles: Vec<NodeCapabilityProfile>,
    negotiation_history: Vec<NegotiationResult>,
}

impl CapabilityNegotiator {
    pub fn new() -> Self {
        Self::with_config(NegotiatorConfig::default())
    }

    pub fn with_config(config: NegotiatorConfig) -> Self {
        Self {
            config,
            capabilities: Vec::new(),
            node_profiles: Vec::new(),
            negotiation_history: Vec::new(),
        }
    }

    /// Registra una capacidad en el sistema
    pub fn register_capability(&mut self, capability: CapabilityEntry) {
        let id = capability.capability_id.clone();
        if !self
            .capabilities
            .iter()
            .any(|c| c.capability_id == id)
        {
            self.capabilities.push(capability);
            info!("Capacidad registrada: {}", id);
        }
    }

    /// Registra un perfil de nodo con sus capacidades
    pub fn register_node(&mut self, profile: NodeCapabilityProfile) {
        let node_id = profile.node_id.clone();
        if !self
            .node_profiles
            .iter()
            .any(|p| p.node_id == node_id)
        {
            self.node_profiles.push(profile);
            info!("Nodo registrado con perfil de capacidades: {}", node_id);
        }
    }

    /// Realiza la negociación para encontrar el mejor nodo para un conjunto de capacidades
    pub fn negotiate(
        &mut self,
        required_capabilities: &[String],
    ) -> Result<NegotiationResult, CapabilityError> {
        // Filtrar nodos stale
        self.remove_stale_nodes();

        // Calcular scores para todos los nodos
        let mut candidates: Vec<(String, f64, Vec<String>)> = Vec::new();

        for profile in &self.node_profiles {
            let score = profile.negotiation_score(required_capabilities);
            let matched = profile
                .capabilities
                .iter()
                .filter(|c| required_capabilities.contains(c))
                .cloned()
                .collect();

            if score >= self.config.min_score_threshold {
                candidates.push((profile.node_id.clone(), score, matched));
            }
        }

        // Ordenar por score descendente
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limitar candidatos
        let top_candidates: Vec<_> = candidates
            .into_iter()
            .take(self.config.max_candidates)
            .collect();

        if top_candidates.is_empty() {
            return Err(CapabilityError::NegotiationFailed(
                "No hay candidatos con score suficiente".to_string(),
            ));
        }

        let (selected_node, score, matched) = top_candidates[0].clone();
        let result = NegotiationResult::new(selected_node.clone(), score, matched);

        info!(
            "Negociación completada: nodo={}, score={:.4}",
            result.selected_node, result.negotiation_score
        );

        self.negotiation_history.push(result.clone());
        Ok(result)
    }

    /// Verifica si un nodo soporta un conjunto de capacidades
    pub fn verify_capabilities(
        &self,
        node_id: &str,
        required_capabilities: &[String],
    ) -> Result<bool, CapabilityError> {
        let profile = self
            .node_profiles
            .iter()
            .find(|p| p.node_id == node_id)
            .ok_or(CapabilityError::NodeNotFound(node_id.to_string()))?;

        let supports = required_capabilities
            .iter()
            .all(|c| profile.capabilities.contains(c));

        if !supports {
            return Err(CapabilityError::CapabilityIncompatible(
                format!(
                    "Nodo {} no soporta todas las capacidades requeridas",
                    node_id
                ),
            ));
        }

        Ok(true)
    }

    /// Obtiene los candidatos disponibles para un conjunto de capacidades
    pub fn get_candidates(
        &self,
        required_capabilities: &[String],
    ) -> Vec<(String, f64)> {
        self.node_profiles
            .iter()
            .map(|p| {
                let score = p.negotiation_score(required_capabilities);
                (p.node_id.clone(), score)
            })
            .filter(|(_, score)| *score >= self.config.min_score_threshold)
            .collect()
    }

    /// Elimina nodos stale
    fn remove_stale_nodes(&mut self) {
        let before = self.node_profiles.len();
        self.node_profiles.retain(|p| !p.is_stale(self.config.max_stale_ms));
        let removed = before - self.node_profiles.len();
        if removed > 0 {
            warn!("Eliminados {} nodos stale", removed);
        }
    }

    /// Actualiza el heartbeat de un nodo
    pub fn heartbeat_node(&mut self, node_id: &str) -> Result<(), CapabilityError> {
        let profile = self
            .node_profiles
            .iter_mut()
            .find(|p| p.node_id == node_id)
            .ok_or(CapabilityError::NodeNotFound(node_id.to_string()))?;
        profile.update_heartbeat();
        debug!("Heartbeat actualizado para nodo {}", node_id);
        Ok(())
    }

    /// Obtiene el historial de negociaciones
    pub fn get_history(&self) -> &[NegotiationResult] {
        &self.negotiation_history
    }

    /// Obtiene las capacidades registradas
    pub fn get_capabilities(&self) -> &[CapabilityEntry] {
        &self.capabilities
    }

    /// Obtiene los perfiles de nodo activos
    pub fn get_active_nodes(&self) -> &[NodeCapabilityProfile] {
        &self.node_profiles
    }

    /// Limpia el historial de negociaciones
    pub fn clear_history(&mut self) {
        self.negotiation_history.clear();
    }
}

impl Default for CapabilityNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile(node_id: &str, capabilities: &[&str]) -> NodeCapabilityProfile {
        let caps: Vec<String> = capabilities.iter().map(|s| s.to_string()).collect();
        NodeCapabilityProfile::new(node_id.to_string(), caps)
    }

    fn make_capability(id: &str, weight: f64) -> CapabilityEntry {
        CapabilityEntry::new(
            id.to_string(),
            format!("Capacidad {}", id),
            "1.0".to_string(),
            weight,
        )
    }

    #[test]
    fn test_negotiator_creation() {
        let negotiator = CapabilityNegotiator::new();
        assert_eq!(negotiator.get_capabilities().len(), 0);
        assert_eq!(negotiator.get_active_nodes().len(), 0);
    }

    #[test]
    fn test_register_capability() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_capability(make_capability("ml-inference", 1.0));
        assert_eq!(negotiator.get_capabilities().len(), 1);
    }

    #[test]
    fn test_register_duplicate_capability() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_capability(make_capability("ml-inference", 1.0));
        negotiator.register_capability(make_capability("ml-inference", 0.8));
        assert_eq!(negotiator.get_capabilities().len(), 1);
    }

    #[test]
    fn test_register_node() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference", "zkp"]));
        assert_eq!(negotiator.get_active_nodes().len(), 1);
    }

    #[test]
    fn test_negotiate_selects_best_node() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference"]));
        negotiator.register_node(make_profile("node-2", &["ml-inference", "zkp"]));

        let required = vec!["ml-inference".to_string()];
        let result = negotiator.negotiate(&required).unwrap();

        assert_eq!(result.matched_capabilities.len(), 1);
        assert!(result.negotiation_score > 0.0);
    }

    #[test]
    fn test_negotiate_no_candidates() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.config.min_score_threshold = 1.0;
        let required = vec!["ml-inference".to_string()];
        let result = negotiator.negotiate(&required);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_capabilities_success() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference", "zkp"]));

        let required = vec!["ml-inference".to_string()];
        assert!(negotiator.verify_capabilities("node-1", &required).is_ok());
    }

    #[test]
    fn test_verify_capabilities_failure() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference"]));

        let required = vec!["zkp".to_string()];
        assert!(negotiator.verify_capabilities("node-1", &required).is_err());
    }

    #[test]
    fn test_get_candidates() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference"]));
        negotiator.register_node(make_profile("node-2", &["ml-inference"]));

        let required = vec!["ml-inference".to_string()];
        let candidates = negotiator.get_candidates(&required);
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn test_heartbeat_node() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference"]));
        assert!(negotiator.heartbeat_node("node-1").is_ok());
    }

    #[test]
    fn test_heartbeat_nonexistent_node() {
        let mut negotiator = CapabilityNegotiator::new();
        assert!(negotiator.heartbeat_node("nonexistent").is_err());
    }

    #[test]
    fn test_clear_history() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference"]));
        let required = vec!["ml-inference".to_string()];
        negotiator.negotiate(&required).unwrap();
        assert_eq!(negotiator.get_history().len(), 1);
        negotiator.clear_history();
        assert_eq!(negotiator.get_history().len(), 0);
    }

    #[test]
    fn test_negotiation_score_calculation() {
        let profile = make_profile("node-1", &["ml-inference", "zkp"]);
        let required = vec!["ml-inference".to_string(), "zkp".to_string()];
        let score = profile.negotiation_score(&required);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_negotiation_score_partial_match() {
        let profile = make_profile("node-1", &["ml-inference"]);
        let required = vec!["ml-inference".to_string(), "zkp".to_string()];
        let score = profile.negotiation_score(&required);
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

    #[test]
    fn test_negotiation_score_no_match() {
        let profile = make_profile("node-1", &["ml-inference"]);
        let required = vec!["zkp".to_string()];
        let score = profile.negotiation_score(&required);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_config_default() {
        let config = NegotiatorConfig::default();
        assert_eq!(config.min_score_threshold, 0.5);
        assert_eq!(config.max_stale_ms, 30_000);
        assert_eq!(config.max_candidates, 10);
        assert!(config.auto_redistribute);
    }

    #[test]
    fn test_negotiator_default() {
        let negotiator = CapabilityNegotiator::default();
        assert_eq!(negotiator.get_capabilities().len(), 0);
    }

    #[test]
    fn test_capability_entry_add_node() {
        let mut cap = make_capability("test", 1.0);
        cap.add_node("node-1".to_string());
        assert!(cap.supports_node("node-1"));
        assert!(!cap.supports_node("node-2"));
    }

    #[test]
    fn test_node_profile_update_heartbeat() {
        let mut profile = make_profile("node-1", &["ml-inference"]);
        let old_heartbeat = profile.last_heartbeat_ms;
        profile.update_heartbeat();
        assert!(profile.last_heartbeat_ms >= old_heartbeat);
    }

    #[test]
    fn test_multiple_negotiations_record_history() {
        let mut negotiator = CapabilityNegotiator::new();
        negotiator.register_node(make_profile("node-1", &["ml-inference"]));
        let required = vec!["ml-inference".to_string()];
        negotiator.negotiate(&required).unwrap();
        negotiator.negotiate(&required).unwrap();
        assert_eq!(negotiator.get_history().len(), 2);
    }
}
