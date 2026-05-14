//! Adaptive Router v2 — Enrutamiento adaptativo cross-model con balanceo predictivo
//!
//! LP-37: Adaptive Routing v2
//! Proporciona enrutamiento de solicitudes ponderado por reputación, latencia
//! histórica y cumplimiento SLO, con fallback a enrutamiento estático Kademlia
//! cuando la confianza de predicción es baja.
//!
//! Características:
//! - Decisiones de enrutamiento ponderadas por reputación + latencia + SLO
//! - Integración con PredictiveBalancer para balanceo predictivo
//! - Fallback automático a Kademlia estático cuando prediction_confidence < 0.7
//! - Tracking histórico de latencia por nodo con ventanas deslizantes
//! - Detección de degradación con circuit breaker
//!
//! Protegido con `#[cfg(feature = "v1.1-sprint5")]`.

#[cfg(feature = "v1.1-sprint5")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint5")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint5")]
use std::time::Instant;
#[cfg(feature = "v1.1-sprint5")]
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Nodo no encontrado: {0}")]
    NodeNotFound(String),

    #[error("Sin nodos disponibles para {0}")]
    NoAvailableNodes(String),

    #[error("Confianza de predicción baja: {confidence:.3} < {threshold:.3}")]
    LowPredictionConfidence { confidence: f32, threshold: f32 },

    #[error("Circuit breaker activo para nodo {0}")]
    CircuitBreakerOpen(String),

    #[error("Error de cálculo: {0}")]
    CalculationError(String),
}

// ─── Routing Decision ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub target_node: String,
    pub strategy: RoutingStrategy,
    pub confidence: f32,
    pub estimated_latency_ms: f32,
    pub score: f32,
    pub fallback_used: bool,
    pub timestamp_ms: u64,
}

#[cfg(feature = "v1.1-sprint5")]
impl RoutingDecision {
    pub fn new(
        target_node: String,
        strategy: RoutingStrategy,
        confidence: f32,
        estimated_latency_ms: f32,
        score: f32,
    ) -> Self {
        let fallback_used = matches!(&strategy, RoutingStrategy::KademliaFallback);
        Self {
            target_node,
            strategy,
            confidence,
            estimated_latency_ms,
            score,
            fallback_used,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ─── Routing Strategy ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    Adaptive,
    KademliaFallback,
    Predictive,
}

#[cfg(feature = "v1.1-sprint5")]
impl std::fmt::Display for RoutingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingStrategy::Adaptive => write!(f, "adaptive"),
            RoutingStrategy::KademliaFallback => write!(f, "kademlia_fallback"),
            RoutingStrategy::Predictive => write!(f, "predictive"),
        }
    }
}

// ─── Node Profile ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProfile {
    pub node_id: String,
    pub model_type: String,
    pub reputation: f32,
    pub slo_compliance: f32,
    pub latencies: VecDeque<f32>,
    pub max_latency_history: usize,
    pub consecutive_failures: u32,
    pub circuit_breaker_open: bool,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_reset_ms: u64,
    pub last_failure_ms: u64,
    pub total_requests: u64,
    pub total_failures: u64,
    pub last_heartbeat_ms: u64,
}

#[cfg(feature = "v1.1-sprint5")]
impl NodeProfile {
    pub fn new(node_id: String, model_type: String) -> Self {
        Self {
            node_id,
            model_type,
            reputation: 1.0,
            slo_compliance: 1.0,
            latencies: VecDeque::with_capacity(100),
            max_latency_history: 100,
            consecutive_failures: 0,
            circuit_breaker_open: false,
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_ms: 30_000,
            last_failure_ms: 0,
            total_requests: 0,
            total_failures: 0,
            last_heartbeat_ms: current_timestamp_ms(),
        }
    }

    pub fn record_success(&mut self, latency_ms: f32) {
        self.total_requests += 1;
        self.consecutive_failures = 0;
        self.circuit_breaker_open = false;
        self.latencies.push_back(latency_ms);
        if self.latencies.len() > self.max_latency_history {
            self.latencies.pop_front();
        }
    }

    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.total_failures += 1;
        self.consecutive_failures += 1;
        self.last_failure_ms = current_timestamp_ms();
        if self.consecutive_failures >= self.circuit_breaker_threshold {
            self.circuit_breaker_open = true;
        }
    }

    pub fn avg_latency(&self) -> f32 {
        if self.latencies.is_empty() {
            return 100.0; // Default 100ms
        }
        let sum: f32 = self.latencies.iter().sum();
        sum / self.latencies.len() as f32
    }

    pub fn p95_latency(&self) -> f32 {
        if self.latencies.is_empty() {
            return 100.0;
        }
        let mut sorted: Vec<f32> = self.latencies.iter().cloned().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((sorted.len() as f32 * 0.95) as usize).min(sorted.len() - 1);
        sorted[idx]
    }

    pub fn composite_score(&self, weights: &RouterWeights) -> f32 {
        if self.circuit_breaker_open {
            return 0.0;
        }
        let latency_score = 1.0 - (self.avg_latency() / 500.0).min(1.0);
        weights.reputation * self.reputation
            + weights.latency * latency_score
            + weights.slo * self.slo_compliance
    }

    pub fn check_circuit_breaker(&mut self) -> bool {
        if !self.circuit_breaker_open {
            return false;
        }
        let now = current_timestamp_ms();
        if now.saturating_sub(self.last_failure_ms) > self.circuit_breaker_reset_ms {
            self.circuit_breaker_open = false;
            self.consecutive_failures = 0;
            return false;
        }
        true
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat_ms = current_timestamp_ms();
    }
}

// ─── Router Weights ───────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterWeights {
    pub reputation: f32,
    pub latency: f32,
    pub slo: f32,
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for RouterWeights {
    fn default() -> Self {
        Self {
            reputation: 0.3,
            latency: 0.4,
            slo: 0.3,
        }
    }
}

// ─── Router Config ────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveRouterConfig {
    pub weights: RouterWeights,
    pub prediction_confidence_threshold: f32,
    pub max_nodes: usize,
    pub health_check_interval_ms: u64,
    pub enable_predictive: bool,
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for AdaptiveRouterConfig {
    fn default() -> Self {
        Self {
            weights: RouterWeights::default(),
            prediction_confidence_threshold: 0.7,
            max_nodes: 1000,
            health_check_interval_ms: 10_000,
            enable_predictive: true,
        }
    }
}

// ─── Router Stats ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStats {
    pub total_decisions: u64,
    pub adaptive_decisions: u64,
    pub fallback_decisions: u64,
    pub predictive_decisions: u64,
    pub avg_decision_time_ms: f64,
    pub avg_routing_latency_ms: f32,
    pub circuit_breaker_trips: u64,
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for RouterStats {
    fn default() -> Self {
        Self {
            total_decisions: 0,
            adaptive_decisions: 0,
            fallback_decisions: 0,
            predictive_decisions: 0,
            avg_decision_time_ms: 0.0,
            avg_routing_latency_ms: 0.0,
            circuit_breaker_trips: 0,
        }
    }
}

// ─── Adaptive Router ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
pub struct AdaptiveRouter {
    config: AdaptiveRouterConfig,
    nodes: HashMap<String, NodeProfile>,
    stats: RouterStats,
    prediction_confidence: f32,
}

#[cfg(feature = "v1.1-sprint5")]
impl AdaptiveRouter {
    pub fn new() -> Self {
        Self::with_config(AdaptiveRouterConfig::default())
    }

    pub fn with_config(config: AdaptiveRouterConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            stats: RouterStats::default(),
            prediction_confidence: 1.0,
        }
    }

    // ─── Node Management ──────────────────────────────────────────────────────

    pub fn register_node(&mut self, node_id: String, model_type: String) {
        if self.nodes.len() < self.config.max_nodes {
            self.nodes.insert(node_id.clone(), NodeProfile::new(node_id, model_type));
        }
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.remove(node_id);
    }

    pub fn update_reputation(&mut self, node_id: &str, reputation: f32) -> Result<(), RouterError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| {
            RouterError::NodeNotFound(node_id.to_string())
        })?;
        node.reputation = reputation.clamp(0.0, 1.0);
        Ok(())
    }

    pub fn update_slo_compliance(
        &mut self,
        node_id: &str,
        compliance: f32,
    ) -> Result<(), RouterError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| {
            RouterError::NodeNotFound(node_id.to_string())
        })?;
        node.slo_compliance = compliance.clamp(0.0, 1.0);
        Ok(())
    }

    // ─── Routing ──────────────────────────────────────────────────────────────

    pub fn route(
        &mut self,
        model_type: &str,
        predictive_score: Option<f32>,
    ) -> Result<RoutingDecision, RouterError> {
        let start = Instant::now();

        // Check if predictive routing is available and confident enough
        if self.config.enable_predictive {
            if let Some(score) = predictive_score {
                if self.prediction_confidence >= self.config.prediction_confidence_threshold {
                    return self.route_predictive(model_type, score, start);
                }
            }
        }

        // Adaptive routing
        let decision = self.route_adaptive(model_type)?;

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.stats.total_decisions += 1;
        self.stats.adaptive_decisions += 1;
        self.stats.avg_decision_time_ms =
            (self.stats.avg_decision_time_ms * (self.stats.total_decisions - 1) as f64 + elapsed)
                / self.stats.total_decisions as f64;

        Ok(decision)
    }

    fn route_adaptive(&self, model_type: &str) -> Result<RoutingDecision, RouterError> {
        let candidates: Vec<&NodeProfile> = self
            .nodes
            .values()
            .filter(|n| {
                n.model_type == model_type && !n.circuit_breaker_open
            })
            .collect();

        if candidates.is_empty() {
            // Fallback to Kademlia
            return self.route_fallback(model_type);
        }

        let mut best = &candidates[0];
        for candidate in &candidates[1..] {
            let best_score = best.composite_score(&self.config.weights);
            let candidate_score = candidate.composite_score(&self.config.weights);
            if candidate_score > best_score {
                best = candidate;
            }
        }

        let score = best.composite_score(&self.config.weights);
        let confidence = (best.reputation * best.slo_compliance).sqrt();

        Ok(RoutingDecision::new(
            best.node_id.clone(),
            RoutingStrategy::Adaptive,
            confidence,
            best.avg_latency(),
            score,
        ))
    }

    fn route_predictive(
        &self,
        model_type: &str,
        predictive_score: f32,
        _start: Instant,
    ) -> Result<RoutingDecision, RouterError> {
        let candidates: Vec<&NodeProfile> = self
            .nodes
            .values()
            .filter(|n| {
                n.model_type == model_type && !n.circuit_breaker_open
            })
            .collect();

        if candidates.is_empty() {
            return self.route_fallback(model_type);
        }

        // Combine predictive score with composite score
        let mut best = &candidates[0];
        let mut best_combined = 0.0;

        for candidate in &candidates {
            let composite = candidate.composite_score(&self.config.weights);
            let combined = predictive_score * 0.5 + composite * 0.5;
            if combined > best_combined {
                best_combined = combined;
                best = candidate;
            }
        }

        let confidence = (best.reputation * best.slo_compliance * self.prediction_confidence).cbrt();

        Ok(RoutingDecision::new(
            best.node_id.clone(),
            RoutingStrategy::Predictive,
            confidence,
            best.avg_latency(),
            best_combined,
        ))
    }

    fn route_fallback(&self, model_type: &str) -> Result<RoutingDecision, RouterError> {
        // Static Kademlia fallback — pick first available node
        let candidate = self
            .nodes
            .values()
            .find(|n| n.model_type == model_type)
            .ok_or_else(|| RouterError::NoAvailableNodes(model_type.to_string()))?;

        Ok(RoutingDecision::new(
            candidate.node_id.clone(),
            RoutingStrategy::KademliaFallback,
            0.5, // Low confidence for fallback
            candidate.avg_latency(),
            0.0,
        ))
    }

    // ─── Result Recording ─────────────────────────────────────────────────────

    pub fn record_success(&mut self, node_id: &str, latency_ms: f32) -> Result<(), RouterError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| {
            RouterError::NodeNotFound(node_id.to_string())
        })?;
        node.record_success(latency_ms);
        Ok(())
    }

    pub fn record_failure(&mut self, node_id: &str) -> Result<(), RouterError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| {
            RouterError::NodeNotFound(node_id.to_string())
        })?;
        let was_closed = !node.circuit_breaker_open;
        node.record_failure();
        if node.circuit_breaker_open && was_closed {
            self.stats.circuit_breaker_trips += 1;
        }
        Ok(())
    }

    // ─── Prediction Confidence ────────────────────────────────────────────────

    pub fn update_prediction_confidence(&mut self, confidence: f32) {
        self.prediction_confidence = confidence.clamp(0.0, 1.0);
    }

    pub fn get_prediction_confidence(&self) -> f32 {
        self.prediction_confidence
    }

    // ─── Health Checks ────────────────────────────────────────────────────────

    pub fn health_check(&mut self) -> Vec<String> {
        let mut unhealthy = Vec::new();
        let now = current_timestamp_ms();

        for (id, node) in &mut self.nodes {
            node.check_circuit_breaker();
            if now.saturating_sub(node.last_heartbeat_ms) > self.config.health_check_interval_ms * 3
            {
                unhealthy.push(id.clone());
            }
        }

        unhealthy
    }

    // ─── Stats ────────────────────────────────────────────────────────────────

    pub fn get_stats(&self) -> RouterStats {
        self.stats.clone()
    }

    pub fn reset_stats(&mut self) {
        self.stats = RouterStats::default();
    }

    pub fn get_node_profile(&self, node_id: &str) -> Option<&NodeProfile> {
        self.nodes.get(node_id)
    }

    pub fn get_available_nodes(&self, model_type: &str) -> Vec<&NodeProfile> {
        self.nodes
            .values()
            .filter(|n| n.model_type == model_type && !n.circuit_breaker_open)
            .collect()
    }
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for AdaptiveRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_router_creation() {
        let router = AdaptiveRouter::new();
        assert_eq!(router.nodes.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_router_with_config() {
        let config = AdaptiveRouterConfig {
            prediction_confidence_threshold: 0.8,
            ..AdaptiveRouterConfig::default()
        };
        let router = AdaptiveRouter::with_config(config);
        assert_eq!(router.config.prediction_confidence_threshold, 0.8);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_register_node() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        assert_eq!(router.nodes.len(), 1);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_route_adaptive() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.register_node("node-2".into(), "gpt".into());
        let decision = router.route("gpt", None).unwrap();
        assert_eq!(decision.strategy, RoutingStrategy::Adaptive);
        assert!(!decision.fallback_used);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_route_fallback_no_nodes() {
        let mut router = AdaptiveRouter::new();
        let result = router.route("unknown", None);
        assert!(result.is_err());
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_route_predictive() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.update_prediction_confidence(0.9);
        let decision = router.route("gpt", Some(0.8)).unwrap();
        assert_eq!(decision.strategy, RoutingStrategy::Predictive);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_predictive_falls_back_on_low_confidence() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.update_prediction_confidence(0.5); // Below threshold
        let decision = router.route("gpt", Some(0.9)).unwrap();
        assert_eq!(decision.strategy, RoutingStrategy::Adaptive);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_record_success() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.record_success("node-1", 50.0).unwrap();
        let node = router.get_node_profile("node-1").unwrap();
        assert_eq!(node.total_requests, 1);
        assert_eq!(node.consecutive_failures, 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_record_failure() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.record_failure("node-1").unwrap();
        let node = router.get_node_profile("node-1").unwrap();
        assert_eq!(node.total_failures, 1);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_circuit_breaker_opens() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        for _ in 0..5 {
            router.record_failure("node-1").unwrap();
        }
        let node = router.get_node_profile("node-1").unwrap();
        assert!(node.circuit_breaker_open);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_circuit_breaker_excludes_from_routing() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.register_node("node-2".into(), "gpt".into());
        // Trip circuit breaker on node-1
        for _ in 0..5 {
            router.record_failure("node-1").unwrap();
        }
        let decision = router.route("gpt", None).unwrap();
        assert_eq!(decision.target_node, "node-2");
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_update_reputation() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.update_reputation("node-1", 0.8).unwrap();
        let node = router.get_node_profile("node-1").unwrap();
        assert_eq!(node.reputation, 0.8);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_update_slo_compliance() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.update_slo_compliance("node-1", 0.95).unwrap();
        let node = router.get_node_profile("node-1").unwrap();
        assert_eq!(node.slo_compliance, 0.95);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_health_check() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        let unhealthy = router.health_check();
        assert_eq!(unhealthy.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_stats_tracking() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.route("gpt", None).unwrap();
        let stats = router.get_stats();
        assert_eq!(stats.total_decisions, 1);
        assert_eq!(stats.adaptive_decisions, 1);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_reset_stats() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.route("gpt", None).unwrap();
        router.reset_stats();
        let stats = router.get_stats();
        assert_eq!(stats.total_decisions, 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_remove_node() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.remove_node("node-1");
        assert_eq!(router.nodes.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_node_profile_avg_latency() {
        let mut profile = NodeProfile::new("n1".into(), "gpt".into());
        profile.record_success(100.0);
        profile.record_success(200.0);
        assert!((profile.avg_latency() - 150.0).abs() < 0.01);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_node_profile_p95_latency() {
        let mut profile = NodeProfile::new("n1".into(), "gpt".into());
        for i in 0..20 {
            profile.record_success(i as f32 * 10.0);
        }
        let p95 = profile.p95_latency();
        assert!(p95 >= 170.0 && p95 <= 190.0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_composite_score() {
        let mut profile = NodeProfile::new("n1".into(), "gpt".into());
        profile.reputation = 0.8;
        profile.slo_compliance = 0.9;
        profile.record_success(50.0);
        let weights = RouterWeights::default();
        let score = profile.composite_score(&weights);
        assert!(score > 0.0 && score <= 1.0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_strategy_display() {
        assert_eq!(RoutingStrategy::Adaptive.to_string(), "adaptive");
        assert_eq!(
            RoutingStrategy::KademliaFallback.to_string(),
            "kademlia_fallback"
        );
        assert_eq!(RoutingStrategy::Predictive.to_string(), "predictive");
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_config_default() {
        let config = AdaptiveRouterConfig::default();
        assert_eq!(config.prediction_confidence_threshold, 0.7);
        assert_eq!(config.enable_predictive, true);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_weights_default() {
        let weights = RouterWeights::default();
        assert!((weights.reputation - 0.3).abs() < 0.01);
        assert!((weights.latency - 0.4).abs() < 0.01);
        assert!((weights.slo - 0.3).abs() < 0.01);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_stats_default() {
        let stats = RouterStats::default();
        assert_eq!(stats.total_decisions, 0);
        assert_eq!(stats.circuit_breaker_trips, 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_router_default() {
        let router = AdaptiveRouter::default();
        assert_eq!(router.nodes.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_get_available_nodes() {
        let mut router = AdaptiveRouter::new();
        router.register_node("node-1".into(), "gpt".into());
        router.register_node("node-2".into(), "llama".into());
        let available = router.get_available_nodes("gpt");
        assert_eq!(available.len(), 1);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_prediction_confidence_update() {
        let mut router = AdaptiveRouter::new();
        router.update_prediction_confidence(0.85);
        assert!((router.get_prediction_confidence() - 0.85).abs() < 0.01);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_prediction_confidence_clamped() {
        let mut router = AdaptiveRouter::new();
        router.update_prediction_confidence(1.5);
        assert!((router.get_prediction_confidence() - 1.0).abs() < 0.01);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_routing_decision_creation() {
        let decision = RoutingDecision::new(
            "node-1".into(),
            RoutingStrategy::Adaptive,
            0.8,
            50.0,
            0.75,
        );
        assert_eq!(decision.target_node, "node-1");
        assert!(!decision.fallback_used);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_fallback_decision_marks_fallback() {
        let decision = RoutingDecision::new(
            "node-1".into(),
            RoutingStrategy::KademliaFallback,
            0.5,
            100.0,
            0.0,
        );
        assert!(decision.fallback_used);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_node_heartbeat() {
        let mut profile = NodeProfile::new("n1".into(), "gpt".into());
        let old = profile.last_heartbeat_ms;
        std::thread::sleep(std::time::Duration::from_millis(10));
        profile.update_heartbeat();
        assert!(profile.last_heartbeat_ms > old);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_max_nodes_limit() {
        let config = AdaptiveRouterConfig {
            max_nodes: 2,
            ..AdaptiveRouterConfig::default()
        };
        let mut router = AdaptiveRouter::with_config(config);
        router.register_node("node-1".into(), "gpt".into());
        router.register_node("node-2".into(), "gpt".into());
        router.register_node("node-3".into(), "gpt".into());
        assert_eq!(router.nodes.len(), 2);
    }
}
