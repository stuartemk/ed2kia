//! Cross-Model Scaling — Load balancing cross-model, routing dinámico, fallback por capacidad
//!
//! Implementa `CrossModelScaler` para routing inteligente de requests entre modelos
//! distribuidos, considerando capacidad del nodo, latencia histórica, reputación y
//! compatibilidad de esquema. Incluye fallback seguro a `core-only` mode cuando los
//! nodos superan umbrales de carga.
//!
//! **Feature:** `phase8-sprint2`

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn, debug};

// ============================================================================
// Errors
// ============================================================================

/// Error específico del escalado cross-model
#[derive(Debug, Error)]
pub enum ScalingError {
    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: String },

    #[error("No available nodes for routing")]
    NoAvailableNodes,

    #[error("Schema incompatible: source={_source}, target={_target}")]
    SchemaIncompatible { _source: String, _target: String },

    #[error("Capacity exceeded: current={current}, max={max}")]
    CapacityExceeded { current: usize, max: usize },

    #[error("Fallback triggered: {reason}")]
    FallbackTriggered { reason: String },

    #[error("Sybil resistance: node {node_id} flagged")]
    SybilFlagged { node_id: String },
}

// ============================================================================
// Node Capacity
// ============================================================================

/// Información de capacidad y estado de un nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    /// ID único del nodo
    pub node_id: String,
    /// Modelo asociado (e.g., "qwen-scope-7b", "llama-3-8b")
    pub model: String,
    /// Capacidad máxima de requests concurrentes
    pub max_capacity: usize,
    /// Requests activos actuales
    pub current_load: usize,
    /// Latencia histórica promedio (ms)
    pub avg_latency_ms: u64,
    /// Puntuación de reputación [0.0, 1.0]
    pub reputation: f32,
    /// Esquema soportado (versión semver)
    pub schema_version: String,
    /// Timestamp del último heartbeat (epoch ms)
    pub last_heartbeat_ms: u64,
    /// Flag de estado activo
    pub active: bool,
}

impl NodeCapacity {
    /// Crea nueva entrada de capacidad de nodo
    pub fn new(node_id: String, model: String, max_capacity: usize) -> Self {
        Self {
            node_id,
            model,
            max_capacity,
            current_load: 0,
            avg_latency_ms: 0,
            reputation: 1.0,
            schema_version: "1.0.0".into(),
            last_heartbeat_ms: current_timestamp_ms(),
            active: true,
        }
    }

    /// Calcula el factor de carga (0.0 = vacío, 1.0 = saturado)
    pub fn load_factor(&self) -> f32 {
        if self.max_capacity == 0 {
            return 1.0;
        }
        self.current_load as f32 / self.max_capacity as f32
    }

    /// Verifica si el nodo puede aceptar más requests
    pub fn can_accept(&self, threshold: f32) -> bool {
        self.active && self.load_factor() < threshold && self.reputation > 0.3
    }

    /// Actualiza la latencia histórica con un nuevo valor
    pub fn update_latency(&mut self, new_latency_ms: u64) {
        let current = self.avg_latency_ms as f64;
        let new = new_latency_ms as f64;
        self.avg_latency_ms = (current * 0.9 + new * 0.1) as u64;
    }

    /// Incrementa la carga actual
    pub fn increment_load(&mut self) {
        self.current_load = self.current_load.saturating_add(1);
    }

    /// Decrementa la carga actual
    pub fn decrement_load(&mut self) {
        self.current_load = self.current_load.saturating_sub(1);
    }

    /// Actualiza el heartbeat
    pub fn heartbeat(&mut self) {
        self.last_heartbeat_ms = current_timestamp_ms();
    }
}

// ============================================================================
// Scale Result
// ============================================================================

/// Resultado de una operación de routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleResult {
    /// Nodo seleccionado para routing
    pub routed_to: String,
    /// Factor de carga del nodo seleccionado
    pub load_factor: f32,
    /// Si se activó fallback
    pub fallback_triggered: bool,
    /// Latencia estimada (ms)
    pub latency_ms: u64,
}

impl ScaleResult {
    /// Crea resultado exitoso
    pub fn success(node_id: String, load_factor: f32, latency_ms: u64) -> Self {
        Self {
            routed_to: node_id,
            load_factor,
            fallback_triggered: false,
            latency_ms,
        }
    }

    /// Crea resultado con fallback
    pub fn fallback(node_id: String, load_factor: f32, reason: &str) -> Self {
        debug!("Fallback triggered: {}", reason);
        Self {
            routed_to: node_id,
            load_factor,
            fallback_triggered: true,
            latency_ms: 0,
        }
    }
}

// ============================================================================
// Routing Request
// ============================================================================

/// Request de routing cross-model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRequest {
    /// Modelo fuente
    pub source_model: String,
    /// Capa SAE destino
    pub layer_id: u32,
    /// Esquema requerido
    pub required_schema: String,
    /// Prioridad (0 = baja, 10 = crítica)
    pub priority: u8,
}

impl RoutingRequest {
    /// Crea nuevo request de routing
    pub fn new(source_model: String, layer_id: u32) -> Self {
        Self {
            source_model,
            layer_id,
            required_schema: "1.0.0".into(),
            priority: 5,
        }
    }
}

// ============================================================================
// Cross Model Scaler
// ============================================================================

/// Escalador cross-model con routing dinámico y fallback seguro
pub struct CrossModelScaler {
    /// Registro de nodos por ID
    nodes: BTreeMap<String, NodeCapacity>,
    /// Umbral de carga para activar fallback
    load_threshold: f32,
    /// Umbral mínimo de reputación
    min_reputation: f32,
    /// Esquemas compatibles
    compatible_schemas: Vec<String>,
    /// Historial de routing (para auditoría)
    routing_history: VecDeque<ScaleResult>,
    /// Modo core-only activo
    core_only_mode: bool,
}

impl CrossModelScaler {
    /// Crea nuevo escalador con configuración por defecto
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            load_threshold: 0.8,
            min_reputation: 0.5,
            compatible_schemas: vec!["1.0.0".into()],
            routing_history: VecDeque::with_capacity(256),
            core_only_mode: false,
        }
    }

    /// Crea escalador con umbrales personalizados
    pub fn with_thresholds(load_threshold: f32, min_reputation: f32) -> Self {
        Self {
            load_threshold,
            min_reputation,
            ..Self::new()
        }
    }

    /// Registra un nodo en el pool de routing
    pub fn register_node(&mut self, capacity: NodeCapacity) {
        let node_id = capacity.node_id.clone();
        self.nodes.insert(node_id.clone(), capacity);
        info!("Node registered: {}", node_id);
    }

    /// Elimina un nodo del pool
    pub fn unregister_node(&mut self, node_id: &str) -> bool {
        let removed = self.nodes.remove(node_id).is_some();
        if removed {
            info!("Node unregistered: {}", node_id);
        }
        removed
    }

    /// Rutea un request al nodo óptimo basado en carga, latencia y reputación
    pub fn route_request(
        &mut self,
        request: &RoutingRequest,
    ) -> Result<ScaleResult, ScalingError> {
        // Verificar compatibilidad de esquema
        self.validate_compatibility(&request.required_schema)?;

        // Filtrar nodos elegibles
        let eligible: Vec<&NodeCapacity> = self.nodes.values().filter(|node| {
            node.can_accept(self.load_threshold)
                && node.reputation >= self.min_reputation
        }).collect();

        if eligible.is_empty() {
            // Intentar fallback
            return self.fallback_to_capacity(request);
        }

        // Seleccionar nodo con mejor score (menor carga + menor latencia + mayor reputación)
        let best = eligible
            .iter()
            .cloned()
            .min_by_key(|node| {
                ((node.load_factor() * 1000.0) as u64 + node.avg_latency_ms)
                    .saturating_sub((node.reputation * 1000.0) as u64)
            })
            .unwrap();

        let node_id = best.node_id.clone();
        let load_factor = best.load_factor();
        let latency_ms = best.avg_latency_ms;

        // Incrementar carga del nodo
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.increment_load();
            node.heartbeat();
        }

        let result = ScaleResult::success(node_id.clone(), load_factor, latency_ms);
        self.record_routing(&result);
        info!(
            "Routed to {} (load={:.2}, latency={}ms)",
            node_id, load_factor, latency_ms
        );
        Ok(result)
    }

    /// Balancea la carga redistribuyendo requests entre nodos
    pub fn balance_load(&mut self) -> usize {
        let mut balanced = 0;
        let overloaded: Vec<(String, usize, usize)> = self
            .nodes
            .values()
            .filter(|n| n.load_factor() >= self.load_threshold)
            .map(|n| (n.node_id.clone(), n.current_load, n.max_capacity))
            .collect();

        for (_node_id, current_load, max_capacity) in overloaded {
            let excess = current_load.saturating_sub(
                (max_capacity as f32 * self.load_threshold) as usize
            );
            // Redistribuir exceso a nodos con menor carga
            for _ in 0..excess {
                if self.redistribute_one(&_node_id) {
                    balanced += 1;
                }
            }
        }

        if balanced > 0 {
            info!("Balanced {} requests across nodes", balanced);
        }
        balanced
    }

    /// Fallback seguro: redirige a nodo con menor carga o activa core-only
    pub fn fallback_to_capacity(
        &mut self,
        request: &RoutingRequest,
    ) -> Result<ScaleResult, ScalingError> {
        // Buscar cualquier nodo activo
        let fallback = self
            .nodes
            .values()
            .filter(|n| n.active)
            .min_by_key(|n| n.current_load);

        match fallback {
            Some(node) => {
                let node_id = node.node_id.clone();
                let load_factor = node.load_factor();
                warn!(
                    "Fallback to {} (load={:.2}) for request layer={}",
                    node_id, load_factor, request.layer_id
                );
                Ok(ScaleResult::fallback(
                    node_id,
                    load_factor,
                    "No eligible nodes, using fallback",
                ))
            }
            None => {
                // Activar core-only mode
                self.core_only_mode = true;
                warn!("Core-only mode activated: no nodes available");
                Err(ScalingError::FallbackTriggered {
                    reason: "All nodes unavailable, core-only mode activated".into(),
                })
            }
        }
    }

    /// Valida compatibilidad de esquema
    pub fn validate_compatibility(&self, schema_version: &str) -> Result<(), ScalingError> {
        if self.compatible_schemas.is_empty() {
            return Ok(());
        }

        // Verificar compatibilidad semver básica (mayor versión igual)
        let compatible = self.compatible_schemas.iter().any(|s| {
            let s_major = s.split('.').next().unwrap_or("0").parse::<u32>().unwrap_or(0);
            let req_major = schema_version
                .split('.')
                .next()
                .unwrap_or("0")
                .parse::<u32>()
                .unwrap_or(0);
            s_major == req_major
        });

        if compatible {
            Ok(())
        } else {
            Err(ScalingError::SchemaIncompatible {
                _source: schema_version.into(),
                _target: self.compatible_schemas[0].clone(),
            })
        }
    }

    /// Agrega un esquema compatible
    pub fn add_compatible_schema(&mut self, schema: String) {
        if !self.compatible_schemas.contains(&schema) {
            self.compatible_schemas.push(schema);
        }
    }

    /// Verifica resistencia Sybil: penaliza nodos con reputación muy baja
    pub fn check_sybil_resistance(&self, node_id: &str) -> Result<(), ScalingError> {
        if let Some(node) = self.nodes.get(node_id) {
            if node.reputation < 0.2 {
                return Err(ScalingError::SybilFlagged {
                    node_id: node_id.into(),
                });
            }
        }
        Ok(())
    }

    /// Actualiza la reputación de un nodo
    pub fn update_reputation(&mut self, node_id: &str, reputation: f32) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.reputation = reputation.clamp(0.0, 1.0);
        }
    }

    /// Activa/desactiva modo core-only
    pub fn set_core_only_mode(&mut self, active: bool) {
        self.core_only_mode = active;
    }

    /// Verifica si core-only mode está activo
    pub fn is_core_only(&self) -> bool {
        self.core_only_mode
    }

    /// Retorna la cantidad de nodos registrados
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Retorna los nodos activos
    pub fn active_nodes(&self) -> Vec<&NodeCapacity> {
        self.nodes.values().filter(|n| n.active).collect()
    }

    /// Retorna el historial de routing
    pub fn routing_history(&self) -> &[ScaleResult] {
        self.routing_history.as_slices().0
    }

    /// Calcula estadísticas globales
    pub fn stats(&self) -> ScalingStats {
        let total_capacity: usize = self.nodes.values().map(|n| n.max_capacity).sum();
        let total_load: usize = self.nodes.values().map(|n| n.current_load).sum();
        let avg_reputation: f32 = if self.nodes.is_empty() {
            0.0
        } else {
            self.nodes.values().map(|n| n.reputation).sum::<f32>() / self.nodes.len() as f32
        };

        ScalingStats {
            total_nodes: self.nodes.len(),
            active_nodes: self.active_nodes().len(),
            total_capacity,
            total_load,
            avg_reputation,
            core_only_mode: self.core_only_mode,
        }
    }

    // ---- Internal helpers ----

    /// Intenta redistribuir un request desde un nodo sobrecargado
    fn redistribute_one(&mut self, source_id: &str) -> bool {
        let target = self
            .nodes
            .values()
            .filter(|n| n.node_id != source_id && n.can_accept(0.6))
            .min_by_key(|n| n.current_load);

        if let Some(target) = target {
            let target_id = target.node_id.clone();
            if let Some(src) = self.nodes.get_mut(source_id) {
                src.decrement_load();
            }
            if let Some(tgt) = self.nodes.get_mut(&target_id) {
                tgt.increment_load();
            }
            return true;
        }
        false
    }

    /// Registra un resultado de routing en el historial
    fn record_routing(&mut self, result: &ScaleResult) {
        self.routing_history.push_back(result.clone());
        if self.routing_history.len() > 256 {
            self.routing_history.pop_front();
        }
    }
}

// ============================================================================
// Scaling Stats
// ============================================================================

/// Estadísticas globales de escalado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingStats {
    /// Total de nodos registrados
    pub total_nodes: usize,
    /// Nodos activos
    pub active_nodes: usize,
    /// Capacidad total
    pub total_capacity: usize,
    /// Carga total actual
    pub total_load: usize,
    /// Reputación promedio
    pub avg_reputation: f32,
    /// Modo core-only activo
    pub core_only_mode: bool,
}

impl Default for CrossModelScaler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, capacity: usize) -> NodeCapacity {
        NodeCapacity::new(id.into(), "qwen-scope-7b".into(), capacity)
    }

    fn make_request(layer: u32) -> RoutingRequest {
        RoutingRequest::new("qwen-scope-7b".into(), layer)
    }

    #[test]
    fn test_node_creation() {
        let node = make_node("node-1", 100);
        assert_eq!(node.node_id, "node-1");
        assert_eq!(node.max_capacity, 100);
        assert_eq!(node.current_load, 0);
        assert!(node.active);
    }

    #[test]
    fn test_load_factor() {
        let mut node = make_node("node-1", 100);
        assert!((node.load_factor() - 0.0) < f32::EPSILON);

        node.current_load = 50;
        assert!((node.load_factor() - 0.5) < 0.01);

        node.current_load = 100;
        assert!((node.load_factor() - 1.0) < 0.01);
    }

    #[test]
    fn test_can_accept() {
        let node = make_node("node-1", 100);
        assert!(node.can_accept(0.8));

        let mut full_node = make_node("node-2", 10);
        full_node.current_load = 9;
        assert!(!full_node.can_accept(0.8));
    }

    #[test]
    fn test_register_node() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_node(make_node("node-1", 100));
        assert_eq!(scaler.node_count(), 1);
    }

    #[test]
    fn test_unregister_node() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_node(make_node("node-1", 100));
        assert!(scaler.unregister_node("node-1"));
        assert_eq!(scaler.node_count(), 0);
    }

    #[test]
    fn test_route_request() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_node(make_node("node-1", 100));
        scaler.register_node(make_node("node-2", 200));

        let result = scaler.route_request(&make_request(0)).unwrap();
        assert!(!result.fallback_triggered);
        assert!(result.routed_to == "node-1" || result.routed_to == "node-2");
    }

    #[test]
    fn test_route_selects_lowest_load() {
        let mut scaler = CrossModelScaler::new();
        let mut node_a = make_node("heavy", 100);
        node_a.current_load = 80;
        let mut node_b = make_node("light", 100);
        node_b.current_load = 10;

        scaler.register_node(node_a);
        scaler.register_node(node_b);

        let result = scaler.route_request(&make_request(0)).unwrap();
        assert_eq!(result.routed_to, "light");
    }

    #[test]
    fn test_fallback_when_all_full() {
        let mut scaler = CrossModelScaler::with_thresholds(0.1, 0.0);
        let mut node = make_node("full", 10);
        node.current_load = 10;
        scaler.register_node(node);

        let result = scaler.fallback_to_capacity(&make_request(0)).unwrap();
        assert!(result.fallback_triggered);
    }

    #[test]
    fn test_fallback_error_when_empty() {
        let mut scaler = CrossModelScaler::new();
        let result = scaler.fallback_to_capacity(&make_request(0));
        assert!(result.is_err());
        assert!(scaler.is_core_only());
    }

    #[test]
    fn test_schema_compatibility() {
        let mut scaler = CrossModelScaler::new();
        scaler.add_compatible_schema("1.0.0".into());
        assert!(scaler.validate_compatibility("1.2.3").is_ok());
        assert!(scaler.validate_compatibility("2.0.0").is_err());
    }

    #[test]
    fn test_sybil_resistance() {
        let mut scaler = CrossModelScaler::new();
        let mut node = make_node("bad-actor", 100);
        node.reputation = 0.1;
        scaler.register_node(node);

        assert!(scaler.check_sybil_resistance("bad-actor").is_err());
    }

    #[test]
    fn test_balance_load() {
        let mut scaler = CrossModelScaler::with_thresholds(0.5, 0.0);
        let mut heavy = make_node("heavy", 100);
        heavy.current_load = 90;
        let light = make_node("light", 100);

        scaler.register_node(heavy);
        scaler.register_node(light);

        let balanced = scaler.balance_load();
        assert!(balanced > 0);
    }

    #[test]
    fn test_stats() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_node(make_node("node-1", 100));
        scaler.register_node(make_node("node-2", 200));

        let stats = scaler.stats();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.total_capacity, 300);
        assert!(!stats.core_only_mode);
    }

    #[test]
    fn test_routing_history() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_node(make_node("node-1", 100));

        for _ in 0..5 {
            let _ = scaler.route_request(&make_request(0));
        }

        assert_eq!(scaler.routing_history().len(), 5);
    }

    #[test]
    fn test_core_only_mode() {
        let mut scaler = CrossModelScaler::new();
        assert!(!scaler.is_core_only());
        scaler.set_core_only_mode(true);
        assert!(scaler.is_core_only());
    }

    #[test]
    fn test_update_reputation() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_node(make_node("node-1", 100));
        scaler.update_reputation("node-1", 0.7);

        let node = scaler.nodes.get("node-1").unwrap();
        assert!((node.reputation - 0.7) < 0.01);
    }

    #[test]
    fn test_scale_result_success() {
        let result = ScaleResult::success("node-1".into(), 0.5, 42);
        assert_eq!(result.routed_to, "node-1");
        assert!(!result.fallback_triggered);
        assert_eq!(result.latency_ms, 42);
    }

    #[test]
    fn test_scale_result_fallback() {
        let result = ScaleResult::fallback("node-2".into(), 0.9, "test reason");
        assert!(result.fallback_triggered);
    }

    #[test]
    fn test_latency_update() {
        let mut node = make_node("node-1", 100);
        node.avg_latency_ms = 100;
        node.update_latency(50);
        // Exponential moving average: 100 * 0.9 + 50 * 0.1 = 95
        assert_eq!(node.avg_latency_ms, 95);
    }

    #[test]
    fn test_active_nodes() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_node(make_node("node-1", 100));
        let mut node2 = make_node("node-2", 100);
        node2.active = false;
        scaler.register_node(node2);

        assert_eq!(scaler.active_nodes().len(), 1);
    }

    #[test]
    fn test_default() {
        let scaler = CrossModelScaler::default();
        assert_eq!(scaler.node_count(), 0);
    }
}
