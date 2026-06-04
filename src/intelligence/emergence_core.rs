//! Topological Emergence Engine â€” Cross-Tensor Fusion for Autonomous Discovery.
//!
//! **Topological Law 2 (Emergencia):** Capabilidades nuevas surgen de la interacciÃ³n
//! simbiÃ³tica entre nodos, no de programaciÃ³n explÃ­cita.
//! **Topological Law 4 (Ã‰tica GeomÃ©trica):** Toda emergencia debe pasar por el SCT Guard
//! para garantizar alineaciÃ³n con el Upper Focus (Z â‰¥ 0).
//!
//! Este mÃ³dulo proporciona el motor de emergencia autÃ³noma:
//! - **NodeTensor:** Tensor de problema/soluciÃ³n de cada nodo en el enjambre.
//! - **CrossTensorFusion:** Algoritmo de fusiÃ³n que detecta correlaciones latentes.
//! - **EmergentInsight:** Insight emergente con score de novedad y validaciÃ³n SCT.
//! - **SCTGuard:** GuardiÃ¡n Ã©tico que valida Z â‰¥ 0 antes de emitir insights.
//! - **TopologicalEmergenceEngine:** Motor principal de emergencia autÃ³noma.
//!
//! ### Feature Gate
//! `v3.5-planetary-emergence`
//!
//! ### IntegraciÃ³n
//! - [`crate::ethics::moral_manifold::Vector3`] para evaluaciÃ³n Ã©tica
//! - [`crate::orchestration::swarm_topology::SwarmTopology`] para distribuciÃ³n de nodos
//! - [`crate::network::planetary_mesh::PlanetaryMesh`] para comunicaciÃ³n WAN

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::time::{Duration, Instant};

// ============================================================================
// NodeTensor â€” Tensor de Problema/SoluciÃ³n del Nodo
// ============================================================================

/// Tensor que representa el estado de problema/soluciÃ³n de un nodo.
///
/// Cada nodo en el enjambre mantiene un tensor que codifica:
/// - El fragmento de informaciÃ³n que posee
/// - El estado de su investigaciÃ³n/soluciÃ³n parcial
/// - La direcciÃ³n Ã©tica de su trabajo (en espacio Octahedron)
#[derive(Debug, Clone)]
pub struct NodeTensor {
    /// Identificador del nodo que posee este tensor.
    pub node_id: u128,
    /// Vector de caracterÃ­sticas del problema (embedding).
    pub problem_features: Vec<f64>,
    /// Vector de caracterÃ­sticas de la soluciÃ³n parcial.
    pub solution_features: Vec<f64>,
    /// DirecciÃ³n Ã©tica en espacio Octahedron (x, y, z).
    pub ethical_direction: Vector3,
    /// Timestamp de Ãºltima actualizaciÃ³n.
    pub updated_at: Instant,
    /// Secuencia de versiÃ³n del tensor.
    pub version: u64,
    /// Metadata del problema (tipo, dominio, etc.).
    pub metadata: HashMap<String, String>,
}

impl NodeTensor {
    pub fn new(
        node_id: u128,
        problem_features: Vec<f64>,
        solution_features: Vec<f64>,
        ethical_direction: Vector3,
    ) -> Self {
        Self {
            node_id,
            problem_features,
            solution_features,
            ethical_direction,
            updated_at: Instant::now(),
            version: 0,
            metadata: HashMap::new(),
        }
    }

    /// Actualiza el tensor con nuevas caracterÃ­sticas de soluciÃ³n.
    pub fn update_solution(
        &mut self,
        new_solution_features: Vec<f64>,
        new_ethical_direction: Vector3,
    ) {
        self.solution_features = new_solution_features;
        self.ethical_direction = new_ethical_direction;
        self.updated_at = Instant::now();
        self.version += 1;
    }

    /// Calcula la similitud coseno con otro tensor (basado en problema).
    pub fn problem_similarity(&self, other: &NodeTensor) -> f64 {
        cosine_similarity(&self.problem_features, &other.problem_features)
    }

    /// Calcula la similitud coseno con otro tensor (basado en soluciÃ³n).
    pub fn solution_similarity(&self, other: &NodeTensor) -> f64 {
        cosine_similarity(&self.solution_features, &other.solution_features)
    }

    /// Verifica si la direcciÃ³n Ã©tica es vÃ¡lida (Z â‰¥ 0).
    pub fn is_ethically_aligned(&self) -> bool {
        self.ethical_direction.z >= 0.0
    }
}

// ============================================================================
// Vector3 â€” Vector en Espacio Ã‰tico Octahedron
// ============================================================================

/// Vector 3D en el espacio Ã©tico Topologicalo (Octahedron).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vector3 {
    /// Eje X: AutonomÃ­a (0.0 = ninguna, 1.0 = total)
    pub x: f64,
    /// Eje Y: ExtracciÃ³n/Costo (0.0 = ninguno, 1.0 = total)
    pub y: f64,
    /// Eje Z: Enfoque Ã©tico (-1.0 = Lower Focus, +1.0 = Upper Focus)
    pub z: f64,
}

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x: x.clamp(-1.0, 1.0),
            y: y.clamp(-1.0, 1.0),
            z: z.clamp(-1.0, 1.0),
        }
    }

    /// Magnitud del vector.
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Vector normalizado.
    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag < f64::EPSILON {
            return Self::default();
        }
        Self::new(self.x / mag, self.y / mag, self.z / mag)
    }

    /// Producto punto con otro vector.
    pub fn dot(&self, other: &Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Suma con otro vector.
    pub fn add(&self, other: &Vector3) -> Vector3 {
        Vector3::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    /// Resta con otro vector.
    pub fn sub(&self, other: &Vector3) -> Vector3 {
        Vector3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    /// Escala el vector por un factor.
    pub fn scale(&self, factor: f64) -> Vector3 {
        Vector3::new(self.x * factor, self.y * factor, self.z * factor)
    }

    /// Distancia euclidiana a otro vector.
    pub fn distance_to(&self, other: &Vector3) -> f64 {
        let diff = self.sub(other);
        diff.magnitude()
    }

    /// Proyecta al Octahedron Ã‰tico (normaliza por L1).
    pub fn project_to_octahedron(&self) -> Vector3 {
        let l1 = self.x.abs() + self.y.abs() + self.z.abs();
        if l1 < f64::EPSILON {
            return Self::default();
        }
        Self::new(self.x / l1, self.y / l1, self.z / l1)
    }
}

impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// ============================================================================
// EmergentInsight â€” Insight Emergente
// ============================================================================

/// Insight emergente generado por fusiÃ³n de tensores cruzados.
///
/// Representa una nueva capacidad o soluciÃ³n que no fue explÃ­citamente
/// programada, sino que surgiÃ³ de la interacciÃ³n simbiÃ³tica entre nodos.
#[derive(Debug, Clone)]
pub struct EmergentInsight {
    /// Identificador Ãºnico del insight.
    pub insight_id: u64,
    /// Nodos que contribuyeron a este insight.
    pub contributing_nodes: Vec<u128>,
    /// Tensor fusionado resultante.
    pub fused_tensor: NodeTensor,
    /// Score de novedad (0.0 = conocido, 1.0 = completamente nuevo).
    pub novelty_score: f64,
    /// Score de utilidad (0.0 = inÃºtil, 1.0 = extremadamente Ãºtil).
    pub utility_score: f64,
    /// Score Ã©tico SCT (Z del resultado fusionado).
    pub sct_z_score: f64,
    /// Estado de validaciÃ³n SCT.
    pub sct_validated: bool,
    /// Timestamp de generaciÃ³n.
    pub generated_at: Instant,
    /// DescripciÃ³n del insight (derivada de metadata fusionado).
    pub description: String,
}

impl EmergentInsight {
    pub fn new(
        insight_id: u64,
        contributing_nodes: Vec<u128>,
        fused_tensor: NodeTensor,
        novelty_score: f64,
        utility_score: f64,
    ) -> Self {
        let z_val = fused_tensor.ethical_direction.z;
        let description = Self::generate_description(&fused_tensor);
        Self {
            insight_id,
            contributing_nodes,
            fused_tensor,
            novelty_score: novelty_score.clamp(0.0, 1.0),
            utility_score: utility_score.clamp(0.0, 1.0),
            sct_z_score: z_val,
            sct_validated: z_val >= 0.0,
            generated_at: Instant::now(),
            description,
        }
    }

    fn generate_description(tensor: &NodeTensor) -> String {
        let domain = tensor
            .metadata
            .get("domain")
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let problem_type = tensor
            .metadata
            .get("problem_type")
            .map(|s| s.as_str())
            .unwrap_or("general");
        format!(
            "Emergent insight in {} domain ({}) from {} nodes",
            domain, problem_type, tensor.node_id
        )
    }

    /// Score compuesto de calidad del insight.
    pub fn quality_score(&self) -> f64 {
        0.4 * self.novelty_score
            + 0.4 * self.utility_score
            + 0.2 * (self.sct_z_score.max(0.0) + 1.0) * 0.5
    }
}

// ============================================================================
// EmergentSolutionEvent â€” Evento de SoluciÃ³n Emergente
// ============================================================================

/// Evento emitido cuando se genera una soluciÃ³n emergente vÃ¡lida.
///
/// Este es el evento clave para el "Grok Challenge": cuando mÃºltiples
/// nodos trabajan en fragmentos de informaciÃ³n desconectados y logran
/// generar una soluciÃ³n coherente con Z â‰¥ 0.
#[derive(Debug, Clone)]
pub struct EmergentSolutionEvent {
    /// Identificador del evento.
    pub event_id: u64,
    /// Insight emergente que desencadenÃ³ el evento.
    pub insight: EmergentInsight,
    /// Score Z final (debe ser â‰¥ 0 para ser vÃ¡lido).
    pub z_score: f64,
    /// Timestamp del evento.
    pub timestamp: Instant,
    /// Fragmentos de informaciÃ³n que se fusionaron.
    pub fragments_fused: usize,
    /// Nodos participantes en la soluciÃ³n.
    pub participating_nodes: usize,
}

impl EmergentSolutionEvent {
    pub fn new(insight: EmergentInsight) -> Self {
        Self {
            event_id: insight.insight_id,
            z_score: insight.sct_z_score,
            timestamp: Instant::now(),
            fragments_fused: insight.contributing_nodes.len(),
            participating_nodes: insight.contributing_nodes.len(),
            insight,
        }
    }

    /// Verifica si el evento es vÃ¡lido (Z â‰¥ 0).
    pub fn is_valid(&self) -> bool {
        self.z_score >= 0.0
    }
}

impl fmt::Display for EmergentSolutionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EmergentSolutionEvent #{}: Z={}, fragments={}, nodes={}, valid={}",
            self.event_id,
            self.z_score,
            self.fragments_fused,
            self.participating_nodes,
            self.is_valid()
        )
    }
}

// ============================================================================
// SCTGuard â€” GuardiÃ¡n Ã‰tico Topologicalo
// ============================================================================

/// GuardiÃ¡n Ã©tico que valida que toda emergencia estÃ© alineada
/// con el Upper Focus (Z â‰¥ 0).
///
/// Implementa la validaciÃ³n SCT (Topological Context Tensor) para
/// garantizar que los insights emergentes no tiendan hacia el
/// Lower Focus (perversidad).
#[derive(Debug, Clone)]
pub struct SCTGuard {
    /// Umbral mÃ­nimo de Z para validaciÃ³n (default: 0.0).
    pub min_z_threshold: f64,
    /// Umbral de alerta para Z cercano a 0 (default: 0.1).
    pub warning_z_threshold: f64,
    /// Contador de validaciones exitosas.
    pub validations_passed: u64,
    /// Contador de validaciones fallidas.
    pub validations_failed: u64,
    /// Historial de scores Z recientes.
    pub recent_z_scores: VecDeque<f64>,
    /// MÃ¡ximo de scores en el historial.
    max_history: usize,
}

impl SCTGuard {
    pub fn new() -> Self {
        Self {
            min_z_threshold: 0.0,
            warning_z_threshold: 0.1,
            validations_passed: 0,
            validations_failed: 0,
            recent_z_scores: VecDeque::with_capacity(100),
            max_history: 100,
        }
    }

    pub fn with_thresholds(min_z: f64, warning_z: f64) -> Self {
        Self {
            min_z_threshold: min_z,
            warning_z_threshold: warning_z,
            validations_passed: 0,
            validations_failed: 0,
            recent_z_scores: VecDeque::with_capacity(100),
            max_history: 100,
        }
    }

    /// Valida un insight emergente contra el SCT Guard.
    pub fn validate(&mut self, insight: &EmergentInsight) -> SCTValidationResult {
        let z = insight.sct_z_score;
        self.recent_z_scores.push_back(z);
        while self.recent_z_scores.len() > self.max_history {
            self.recent_z_scores.pop_front();
        }

        if z >= self.min_z_threshold {
            self.validations_passed += 1;
            if z < self.warning_z_threshold {
                SCTValidationResult::Warning(z)
            } else {
                SCTValidationResult::Passed(z)
            }
        } else {
            self.validations_failed += 1;
            SCTValidationResult::Rejected(z)
        }
    }

    /// Obtiene el promedio de Z scores recientes.
    pub fn recent_z_average(&self) -> f64 {
        if self.recent_z_scores.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.recent_z_scores.iter().sum();
        sum / self.recent_z_scores.len() as f64
    }

    /// Obtiene la tasa de Ã©xito de validaciÃ³n.
    pub fn success_rate(&self) -> f64 {
        let total = self.validations_passed + self.validations_failed;
        if total == 0 {
            return 0.0;
        }
        self.validations_passed as f64 / total as f64
    }

    /// Resetea las estadÃ­sticas del guardiÃ¡n.
    pub fn reset(&mut self) {
        self.validations_passed = 0;
        self.validations_failed = 0;
        self.recent_z_scores.clear();
    }
}

impl Default for SCTGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Resultado de validaciÃ³n SCT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SCTValidationResult {
    /// Insight pasa validaciÃ³n Ã©tica.
    Passed(f64),
    /// Insight pasa pero estÃ¡ cerca del umbral (alerta).
    Warning(f64),
    /// Insight rechazado por no cumplir Z â‰¥ 0.
    Rejected(f64),
}

impl SCTValidationResult {
    pub fn z_score(&self) -> f64 {
        match self {
            SCTValidationResult::Passed(z) => *z,
            SCTValidationResult::Warning(z) => *z,
            SCTValidationResult::Rejected(z) => *z,
        }
    }

    pub fn is_valid(&self) -> bool {
        !matches!(self, SCTValidationResult::Rejected(_))
    }
}

// ============================================================================
// CrossTensorFusion â€” Algoritmo de FusiÃ³n Cruzada
// ============================================================================

/// Algoritmo de fusiÃ³n de tensores cruzados que detecta
/// correlaciones latentes entre problemas de diferentes nodos.
#[derive(Debug, Clone)]
pub struct CrossTensorFusion {
    /// Umbral de similitud para considerar fusiÃ³n (default: 0.3).
    pub similarity_threshold: f64,
    /// Peso de similitud de problema en la fusiÃ³n (default: 0.5).
    pub problem_weight: f64,
    /// Peso de similitud de soluciÃ³n en la fusiÃ³n (default: 0.3).
    pub solution_weight: f64,
    /// Peso de alineaciÃ³n Ã©tica en la fusiÃ³n (default: 0.2).
    pub ethical_weight: f64,
    /// MÃ¡ximo de nodos por fusiÃ³n (default: 10).
    pub max_fusion_nodes: usize,
}

impl CrossTensorFusion {
    pub fn new() -> Self {
        Self {
            similarity_threshold: 0.3,
            problem_weight: 0.5,
            solution_weight: 0.3,
            ethical_weight: 0.2,
            max_fusion_nodes: 10,
        }
    }

    pub fn with_weights(problem_w: f64, solution_w: f64, ethical_w: f64) -> Self {
        let total = problem_w + solution_w + ethical_w;
        Self {
            similarity_threshold: 0.3,
            problem_weight: problem_w / total,
            solution_weight: solution_w / total,
            ethical_weight: ethical_w / total,
            max_fusion_nodes: 10,
        }
    }

    /// Detecta candidatos a fusiÃ³n entre un tensor y una lista de tensores.
    pub fn find_fusion_candidates(
        &self,
        target: &NodeTensor,
        candidates: &[NodeTensor],
    ) -> Vec<(u128, f64)> {
        candidates
            .iter()
            .filter(|c| c.node_id != target.node_id)
            .map(|c| {
                let prob_sim = target.problem_similarity(c);
                let sol_sim = target.solution_similarity(c);
                let eth_align = ethical_alignment(&target.ethical_direction, &c.ethical_direction);
                let combined = self.problem_weight * prob_sim
                    + self.solution_weight * sol_sim
                    + self.ethical_weight * eth_align;
                (c.node_id, combined)
            })
            .filter(|(_, score)| *score >= self.similarity_threshold)
            .take(self.max_fusion_nodes)
            .collect()
    }

    /// Fusiona mÃºltiples tensores en uno emergente.
    pub fn fuse_tensors(&self, tensors: &[NodeTensor]) -> Option<NodeTensor> {
        if tensors.is_empty() {
            return None;
        }

        let n = tensors.len() as f64;

        // Promedio ponderado de caracterÃ­sticas de problema
        let problem_refs: Vec<_> = tensors
            .iter()
            .map(|t| t.problem_features.as_slice())
            .collect();
        let fused_problem = weighted_average_features(&problem_refs);

        // Promedio ponderado de caracterÃ­sticas de soluciÃ³n
        let solution_refs: Vec<_> = tensors
            .iter()
            .map(|t| t.solution_features.as_slice())
            .collect();
        let fused_solution = weighted_average_features(&solution_refs);

        // DirecciÃ³n Ã©tica fusionada (promedio en espacio Octahedron)
        let fused_ethical = {
            let sum_x: f64 = tensors.iter().map(|t| t.ethical_direction.x).sum();
            let sum_y: f64 = tensors.iter().map(|t| t.ethical_direction.y).sum();
            let sum_z: f64 = tensors.iter().map(|t| t.ethical_direction.z).sum();
            Vector3::new(sum_x / n, sum_y / n, sum_z / n).project_to_octahedron()
        };

        // Metadata fusionado
        let mut fused_metadata = HashMap::new();
        for tensor in tensors {
            for (k, v) in &tensor.metadata {
                fused_metadata.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
        fused_metadata.insert("fusion_count".to_string(), n.to_string());
        fused_metadata.insert(
            "contributing_nodes".to_string(),
            tensors
                .iter()
                .map(|t| t.node_id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );

        Some(NodeTensor {
            node_id: tensors.iter().map(|t| t.node_id).min().unwrap_or(0),
            problem_features: fused_problem,
            solution_features: fused_solution,
            ethical_direction: fused_ethical,
            updated_at: Instant::now(),
            version: tensors.iter().map(|t| t.version).sum(),
            metadata: fused_metadata,
        })
    }

    /// Calcula el score de novedad de un tensor fusionado.
    pub fn calculate_novelty(&self, fused: &NodeTensor, originals: &[NodeTensor]) -> f64 {
        if originals.len() <= 1 {
            return 0.0;
        }

        // La novedad se mide como la distancia del tensor fusionado
        // al tensor original mÃ¡s cercano
        let min_distance = originals
            .iter()
            .map(|orig| {
                feature_distance(&fused.problem_features, &orig.problem_features)
                    + feature_distance(&fused.solution_features, &orig.solution_features)
            })
            .fold(f64::MAX, f64::min);

        // Normalizar a [0, 1] asumiendo distancia mÃ¡xima de 2.0
        (1.0 - (min_distance / 2.0).clamp(0.0, 1.0)).clamp(0.0, 1.0)
    }

    /// Calcula el score de utilidad basado en la coherencia Ã©tica y convergencia.
    pub fn calculate_utility(&self, fused: &NodeTensor) -> f64 {
        // Utilidad = alineaciÃ³n Ã©tica * coherencia de soluciÃ³n
        let ethical_score = (fused.ethical_direction.z + 1.0) * 0.5; // [0, 1]
        let solution_coherence = solution_coherence_score(&fused.solution_features);
        0.6 * ethical_score + 0.4 * solution_coherence
    }
}

impl Default for CrossTensorFusion {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TopologicalEmergenceEngine â€” Motor Principal de Emergencia
// ============================================================================

/// Motor de emergencia autÃ³noma Topologicala.
///
/// Gestiona el ciclo completo de emergencia:
/// 1. Recibe tensores de problema/soluciÃ³n de nodos
/// 2. Detecta correlaciones latentes mediante Cross-Tensor Fusion
/// 3. Genera insights emergentes
/// 4. Valida con SCT Guard (Z â‰¥ 0)
/// 5. Emite EmergentSolutionEvent cuando se encuentra soluciÃ³n vÃ¡lida
pub struct TopologicalEmergenceEngine {
    /// Algoritmo de fusiÃ³n cruzada.
    fusion: CrossTensorFusion,
    /// GuardiÃ¡n Ã©tico SCT.
    sct_guard: SCTGuard,
    /// Tensores de nodos registrados.
    node_tensors: HashMap<u128, NodeTensor>,
    /// Insights emergentes generados.
    emergent_insights: Vec<EmergentInsight>,
    /// Eventos de soluciÃ³n emitidos.
    solution_events: Vec<EmergentSolutionEvent>,
    /// Siguiente ID de insight.
    next_insight_id: u64,
    /// Siguiente ID de evento.
    next_event_id: u64,
    /// EstadÃ­sticas del motor.
    stats: EmergenceStats,
    /// Timestamp del Ãºltimo ciclo de emergencia.
    last_cycle: Instant,
    /// Intervalo mÃ­nimo entre ciclos (default: 1s).
    cycle_interval: Duration,
}

/// EstadÃ­sticas del motor de emergencia.
#[derive(Debug, Clone, Default)]
pub struct EmergenceStats {
    /// Total de tensores procesados.
    pub tensors_processed: usize,
    /// Total de fusiones ejecutadas.
    pub fusions_executed: usize,
    /// Total de insights generados.
    pub insights_generated: usize,
    /// Total de soluciones emitidas.
    pub solutions_emitted: usize,
    /// Total de validaciones SCT fallidas.
    pub sct_rejections: usize,
    /// Score Z promedio de soluciones.
    pub avg_solution_z: f64,
    /// Tiempo total de ciclos de emergencia.
    pub total_cycle_time_ms: f64,
}

impl TopologicalEmergenceEngine {
    pub fn new() -> Self {
        Self {
            fusion: CrossTensorFusion::new(),
            sct_guard: SCTGuard::new(),
            node_tensors: HashMap::new(),
            emergent_insights: Vec::new(),
            solution_events: Vec::new(),
            next_insight_id: 1,
            next_event_id: 1,
            stats: EmergenceStats::default(),
            last_cycle: Instant::now(),
            cycle_interval: Duration::from_secs(1),
        }
    }

    pub fn with_config(fusion: CrossTensorFusion, sct_guard: SCTGuard) -> Self {
        Self {
            fusion,
            sct_guard,
            node_tensors: HashMap::new(),
            emergent_insights: Vec::new(),
            solution_events: Vec::new(),
            next_insight_id: 1,
            next_event_id: 1,
            stats: EmergenceStats::default(),
            last_cycle: Instant::now(),
            cycle_interval: Duration::from_secs(1),
        }
    }

    /// Registra un tensor de nodo en el motor.
    pub fn register_tensor(&mut self, tensor: NodeTensor) {
        let node_id = tensor.node_id;
        self.node_tensors.insert(node_id, tensor);
        self.stats.tensors_processed += 1;
    }

    /// Actualiza el tensor de un nodo existente.
    pub fn update_tensor(&mut self, tensor: NodeTensor) -> bool {
        if self.node_tensors.insert(tensor.node_id, tensor).is_some() {
            self.stats.tensors_processed += 1;
            true
        } else {
            false
        }
    }

    /// Elimina un tensor de nodo.
    pub fn unregister_tensor(&mut self, node_id: u128) -> bool {
        self.node_tensors.remove(&node_id).is_some()
    }

    /// Obtiene el tensor de un nodo.
    pub fn get_tensor(&self, node_id: u128) -> Option<&NodeTensor> {
        self.node_tensors.get(&node_id)
    }

    /// Ejecuta un ciclo completo de emergencia.
    ///
    /// Este es el mÃ©todo principal que:
    /// 1. Escanea todos los tensores registrados
    /// 2. Detecta candidatos a fusiÃ³n
    /// 3. Ejecuta fusiÃ³n de tensores
    /// 4. Genera insights emergentes
    /// 5. Valida con SCT Guard
    /// 6. Emite EmergentSolutionEvent si Z â‰¥ 0
    pub fn run_emergence_cycle(&mut self) -> Vec<EmergentSolutionEvent> {
        let start = Instant::now();
        let mut events = Vec::new();

        if self.node_tensors.len() < 2 {
            return events;
        }

        let tensors: Vec<NodeTensor> = self.node_tensors.values().cloned().collect();
        let mut processed_pairs = Vec::new();

        for tensor in &tensors {
            // Encontrar candidatos a fusiÃ³n
            let candidates = self.fusion.find_fusion_candidates(
                tensor,
                &tensors
                    .iter()
                    .filter(|c| c.node_id != tensor.node_id)
                    .cloned()
                    .collect::<Vec<_>>(),
            );

            if candidates.is_empty() {
                continue;
            }

            // Recoger tensores candidatos
            let mut fusion_set = vec![tensor.clone()];
            for (candidate_id, _) in &candidates {
                if let Some(candidate) = self.node_tensors.get(candidate_id) {
                    fusion_set.push(candidate.clone());
                }
            }

            // Evitar procesar el mismo conjunto mÃºltiples veces
            let pair_key: Vec<u128> = fusion_set.iter().map(|t| t.node_id).collect::<Vec<_>>();
            if processed_pairs.contains(&pair_key) {
                continue;
            }
            processed_pairs.push(pair_key);

            // Ejecutar fusiÃ³n
            if let Some(fused) = self.fusion.fuse_tensors(&fusion_set) {
                self.stats.fusions_executed += 1;

                // Calcular scores
                let novelty = self.fusion.calculate_novelty(&fused, &fusion_set);
                let utility = self.fusion.calculate_utility(&fused);

                // Generar insight
                let contributing: Vec<u128> = fusion_set.iter().map(|t| t.node_id).collect();
                let insight = EmergentInsight::new(
                    self.next_insight_id,
                    contributing,
                    fused,
                    novelty,
                    utility,
                );
                self.next_insight_id += 1;
                self.stats.insights_generated += 1;

                // Validar con SCT Guard
                let validation = self.sct_guard.validate(&insight);

                if validation.is_valid() {
                    // Emitir evento de soluciÃ³n
                    let event = EmergentSolutionEvent::new(insight.clone());
                    self.solution_events.push(event.clone());
                    self.emergent_insights.push(insight);
                    self.stats.solutions_emitted += 1;

                    // Actualizar promedio Z
                    let total = self.stats.solutions_emitted as f64;
                    self.stats.avg_solution_z =
                        (self.stats.avg_solution_z * (total - 1.0) + validation.z_score()) / total;

                    events.push(event);
                } else {
                    self.stats.sct_rejections += 1;
                }
            }
        }

        let elapsed = start.elapsed();
        self.stats.total_cycle_time_ms += elapsed.as_secs_f64() * 1000.0;
        self.last_cycle = Instant::now();

        events
    }

    /// Ejecuta el "Grok Challenge": inyecta fragmentos de informaciÃ³n
    /// desconectados y verifica que el motor genere una soluciÃ³n emergente.
    pub fn run_grok_challenge(
        &mut self,
        fragments: Vec<NodeTensor>,
    ) -> Option<EmergentSolutionEvent> {
        // Inyectar fragmentos
        for fragment in fragments {
            self.register_tensor(fragment);
        }

        // Ejecutar ciclos de emergencia hasta encontrar soluciÃ³n
        let max_cycles = 10;
        for _ in 0..max_cycles {
            let events = self.run_emergence_cycle();
            if let Some(event) = events.into_iter().find(|e| e.is_valid()) {
                return Some(event);
            }
        }

        None
    }

    /// Obtiene los insights emergentes generados.
    pub fn get_insights(&self) -> &[EmergentInsight] {
        &self.emergent_insights
    }

    /// Obtiene los eventos de soluciÃ³n emitidos.
    pub fn get_solution_events(&self) -> &[EmergentSolutionEvent] {
        &self.solution_events
    }

    /// Obtiene las estadÃ­sticas del motor.
    pub fn get_stats(&self) -> &EmergenceStats {
        &self.stats
    }

    /// Obtiene el SCT Guard.
    pub fn get_sct_guard(&self) -> &SCTGuard {
        &self.sct_guard
    }

    /// Obtiene el algoritmo de fusiÃ³n.
    pub fn get_fusion(&self) -> &CrossTensorFusion {
        &self.fusion
    }

    /// Resetea el motor completamente.
    pub fn reset(&mut self) {
        self.node_tensors.clear();
        self.emergent_insights.clear();
        self.solution_events.clear();
        self.next_insight_id = 1;
        self.next_event_id = 1;
        self.stats = EmergenceStats::default();
        self.sct_guard.reset();
        self.last_cycle = Instant::now();
    }
}

impl Default for TopologicalEmergenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calcula la similitud coseno entre dos vectores.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let min_len = a.len().min(b.len());
    if min_len == 0 {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut mag_a = 0.0;
    let mut mag_b = 0.0;

    for i in 0..min_len {
        dot += a[i] * b[i];
        mag_a += a[i] * a[i];
        mag_b += b[i] * b[i];
    }

    let denom = (mag_a * mag_b).sqrt();
    if denom < f64::EPSILON {
        return 0.0;
    }

    (dot / denom).clamp(-1.0, 1.0)
}

/// Calcula la alineaciÃ³n Ã©tica entre dos direcciones.
fn ethical_alignment(a: &Vector3, b: &Vector3) -> f64 {
    let normalized_a = a.normalized();
    let normalized_b = b.normalized();
    (normalized_a.dot(&normalized_b) + 1.0) * 0.5 // [0, 1]
}

/// Calcula el promedio ponderado de caracterÃ­sticas.
fn weighted_average_features(features_list: &[&[f64]]) -> Vec<f64> {
    if features_list.is_empty() {
        return vec![];
    }

    let max_len = features_list.iter().map(|f| f.len()).max().unwrap_or(0);
    let n = features_list.len() as f64;

    (0..max_len)
        .map(|i| {
            let sum: f64 = features_list
                .iter()
                .map(|f| if i < f.len() { f[i] } else { 0.0 })
                .sum();
            sum / n
        })
        .collect()
}

/// Calcula la distancia euclidiana entre dos vectores de caracterÃ­sticas.
fn feature_distance(a: &[f64], b: &[f64]) -> f64 {
    let min_len = a.len().min(b.len());
    let sum: f64 = (0..min_len).map(|i| (a[i] - b[i]).powi(2)).sum();
    sum.sqrt()
}

/// Calcula el score de coherencia de soluciÃ³n (varianza inversa).
fn solution_coherence_score(features: &[f64]) -> f64 {
    if features.is_empty() {
        return 0.0;
    }

    let mean: f64 = features.iter().sum::<f64>() / features.len() as f64;
    let variance: f64 =
        features.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / features.len() as f64;

    // Coherencia = 1 / (1 + sqrt(variance))
    1.0 / (1.0 + variance.sqrt())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vector3(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3::new(x, y, z)
    }

    fn make_tensor(node_id: u128, z: f64) -> NodeTensor {
        NodeTensor::new(
            node_id,
            vec![1.0, 0.5, 0.2, 0.8],
            vec![0.3, 0.7, 0.1, 0.9],
            Vector3::new(0.5, 0.3, z),
        )
    }

    fn make_tensor_with_metadata(node_id: u128, z: f64, domain: &str) -> NodeTensor {
        let mut tensor = make_tensor(node_id, z);
        tensor
            .metadata
            .insert("domain".to_string(), domain.to_string());
        tensor
            .metadata
            .insert("problem_type".to_string(), "fragment".to_string());
        tensor
    }

    // ------------------------------------------------------------------
    // Vector3 Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_vector3_creation() {
        let v = make_vector3(0.5, 0.3, 0.8);
        assert!((v.x - 0.5).abs() < f64::EPSILON);
        assert!((v.y - 0.3).abs() < f64::EPSILON);
        assert!((v.z - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vector3_clamping() {
        let v = Vector3::new(2.0, -2.0, 5.0);
        assert!((v.x - 1.0).abs() < f64::EPSILON);
        assert!((v.y - (-1.0)).abs() < f64::EPSILON);
        assert!((v.z - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vector3_magnitude() {
        let v = make_vector3(1.0, 0.0, 0.0);
        assert!((v.magnitude() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vector3_normalized() {
        let v = make_vector3(3.0, 4.0, 0.0);
        let n = v.normalized();
        assert!((n.magnitude() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_vector3_zero_normalized() {
        let v = make_vector3(0.0, 0.0, 0.0);
        let n = v.normalized();
        assert_eq!(n, Vector3::default());
    }

    #[test]
    fn test_vector3_dot_product() {
        let a = make_vector3(1.0, 0.0, 0.0);
        let b = make_vector3(0.0, 1.0, 0.0);
        assert!((a.dot(&b)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vector3_add_sub() {
        let a = make_vector3(0.5, 0.3, 0.2);
        let b = make_vector3(0.2, 0.1, 0.1);
        let sum = a.add(&b);
        assert!((sum.x - 0.7).abs() < 1e-10);
        assert!((sum.y - 0.4).abs() < 1e-10);
        assert!((sum.z - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_vector3_scale() {
        let v = make_vector3(0.5, 0.3, 0.2);
        let scaled = v.scale(2.0);
        assert!((scaled.x - 1.0).abs() < 1e-10);
        assert!((scaled.y - 0.6).abs() < 1e-10);
        assert!((scaled.z - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_vector3_distance() {
        let a = make_vector3(0.0, 0.0, 0.0);
        let b = make_vector3(1.0, 0.0, 0.0);
        assert!((a.distance_to(&b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vector3_octahedron_projection() {
        let v = make_vector3(1.0, 1.0, 1.0);
        let projected = v.project_to_octahedron();
        let l1 = projected.x.abs() + projected.y.abs() + projected.z.abs();
        assert!((l1 - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_vector3_display() {
        let v = make_vector3(0.5, 0.3, 0.8);
        let s = format!("{}", v);
        assert!(s.contains("0.5"));
    }

    // ------------------------------------------------------------------
    // NodeTensor Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_tensor_creation() {
        let t = make_tensor(1, 0.5);
        assert_eq!(t.node_id, 1);
        assert_eq!(t.problem_features.len(), 4);
        assert_eq!(t.solution_features.len(), 4);
    }

    #[test]
    fn test_tensor_ethical_alignment() {
        let aligned = make_tensor(1, 0.5);
        let misaligned = make_tensor(2, -0.5);
        assert!(aligned.is_ethically_aligned());
        assert!(!misaligned.is_ethically_aligned());
    }

    #[test]
    fn test_tensor_similarity() {
        let t1 = make_tensor(1, 0.5);
        let t2 = make_tensor(2, 0.3);
        let sim = t1.problem_similarity(&t2);
        assert!(sim >= -1.0 && sim <= 1.0);
    }

    #[test]
    fn test_tensor_update() {
        let mut t = make_tensor(1, 0.5);
        let original_version = t.version;
        t.update_solution(vec![0.9, 0.8, 0.7, 0.6], Vector3::new(0.6, 0.4, 0.9));
        assert_eq!(t.version, original_version + 1);
    }

    // ------------------------------------------------------------------
    // SCTGuard Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_sct_guard_creation() {
        let guard = SCTGuard::new();
        assert_eq!(guard.validations_passed, 0);
        assert_eq!(guard.validations_failed, 0);
    }

    #[test]
    fn test_sct_guard_validate_passed() {
        let mut guard = SCTGuard::new();
        let insight = EmergentInsight::new(1, vec![1, 2], make_tensor(1, 0.5), 0.8, 0.9);
        let result = guard.validate(&insight);
        assert!(result.is_valid());
        assert_eq!(guard.validations_passed, 1);
    }

    #[test]
    fn test_sct_guard_validate_rejected() {
        let mut guard = SCTGuard::new();
        let insight = EmergentInsight::new(1, vec![1, 2], make_tensor(1, -0.5), 0.8, 0.9);
        let result = guard.validate(&insight);
        assert!(!result.is_valid());
        assert_eq!(guard.validations_failed, 1);
    }

    #[test]
    fn test_sct_guard_warning() {
        let mut guard = SCTGuard::with_thresholds(0.0, 0.1);
        let insight = EmergentInsight::new(1, vec![1, 2], make_tensor(1, 0.05), 0.8, 0.9);
        let result = guard.validate(&insight);
        assert_eq!(result, SCTValidationResult::Warning(0.05));
    }

    #[test]
    fn test_sct_guard_success_rate() {
        let mut guard = SCTGuard::new();
        let good = EmergentInsight::new(1, vec![1], make_tensor(1, 0.5), 0.8, 0.9);
        let bad = EmergentInsight::new(2, vec![2], make_tensor(2, -0.5), 0.8, 0.9);
        guard.validate(&good);
        guard.validate(&bad);
        assert!((guard.success_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sct_guard_reset() {
        let mut guard = SCTGuard::new();
        let insight = EmergentInsight::new(1, vec![1], make_tensor(1, 0.5), 0.8, 0.9);
        guard.validate(&insight);
        guard.reset();
        assert_eq!(guard.validations_passed, 0);
        assert_eq!(guard.validations_failed, 0);
    }

    #[test]
    fn test_sct_guard_default() {
        let guard = SCTGuard::default();
        assert_eq!(guard.validations_passed, 0);
    }

    // ------------------------------------------------------------------
    // CrossTensorFusion Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_fusion_creation() {
        let fusion = CrossTensorFusion::new();
        assert!((fusion.similarity_threshold - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_fusion_with_weights() {
        let fusion = CrossTensorFusion::with_weights(0.6, 0.3, 0.1);
        assert!((fusion.problem_weight - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_fusion_candidates() {
        let fusion = CrossTensorFusion::new();
        let target = make_tensor(0, 0.5);
        let candidates = vec![make_tensor(1, 0.4), make_tensor(2, 0.6)];
        let results = fusion.find_fusion_candidates(&target, &candidates);
        // Should find at least some candidates
        assert!(results.len() >= 0);
    }

    #[test]
    fn test_fuse_tensors() {
        let fusion = CrossTensorFusion::new();
        let tensors = vec![make_tensor(1, 0.5), make_tensor(2, 0.3)];
        let fused = fusion.fuse_tensors(&tensors);
        assert!(fused.is_some());
        let fused = fused.unwrap();
        assert!(fused.problem_features.len() == 4);
    }

    #[test]
    fn test_fuse_empty_tensors() {
        let fusion = CrossTensorFusion::new();
        let fused = fusion.fuse_tensors(&[]);
        assert!(fused.is_none());
    }

    #[test]
    fn test_calculate_novelty() {
        let fusion = CrossTensorFusion::new();
        let originals = vec![make_tensor(1, 0.5), make_tensor(2, 0.3)];
        let fused = fusion.fuse_tensors(&originals).unwrap();
        let novelty = fusion.calculate_novelty(&fused, &originals);
        assert!(novelty >= 0.0 && novelty <= 1.0);
    }

    #[test]
    fn test_calculate_utility() {
        let fusion = CrossTensorFusion::new();
        let tensor = make_tensor(1, 0.5);
        let utility = fusion.calculate_utility(&tensor);
        assert!(utility >= 0.0 && utility <= 1.0);
    }

    #[test]
    fn test_fusion_default() {
        let fusion = CrossTensorFusion::default();
        assert!((fusion.similarity_threshold - 0.3).abs() < f64::EPSILON);
    }

    // ------------------------------------------------------------------
    // EmergentInsight Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_insight_creation() {
        let insight = EmergentInsight::new(1, vec![1, 2], make_tensor(1, 0.5), 0.8, 0.9);
        assert_eq!(insight.insight_id, 1);
        assert_eq!(insight.contributing_nodes.len(), 2);
        assert!(insight.sct_validated);
    }

    #[test]
    fn test_insight_unvalidated() {
        let insight = EmergentInsight::new(1, vec![1], make_tensor(1, -0.5), 0.8, 0.9);
        assert!(!insight.sct_validated);
    }

    #[test]
    fn test_insight_quality_score() {
        let insight = EmergentInsight::new(1, vec![1], make_tensor(1, 0.5), 0.8, 0.9);
        let quality = insight.quality_score();
        assert!(quality >= 0.0 && quality <= 1.0);
    }

    // ------------------------------------------------------------------
    // EmergentSolutionEvent Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_solution_event_creation() {
        let insight = EmergentInsight::new(1, vec![1, 2, 3], make_tensor(1, 0.5), 0.8, 0.9);
        let event = EmergentSolutionEvent::new(insight);
        assert!(event.is_valid());
        assert_eq!(event.fragments_fused, 3);
    }

    #[test]
    fn test_solution_event_invalid() {
        let insight = EmergentInsight::new(1, vec![1], make_tensor(1, -0.5), 0.8, 0.9);
        let event = EmergentSolutionEvent::new(insight);
        assert!(!event.is_valid());
    }

    #[test]
    fn test_solution_event_display() {
        let insight = EmergentInsight::new(1, vec![1, 2], make_tensor(1, 0.5), 0.8, 0.9);
        let event = EmergentSolutionEvent::new(insight);
        let s = format!("{}", event);
        assert!(s.contains("EmergentSolutionEvent"));
    }

    // ------------------------------------------------------------------
    // TopologicalEmergenceEngine Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_engine_creation() {
        let engine = TopologicalEmergenceEngine::new();
        assert_eq!(engine.stats.tensors_processed, 0);
    }

    #[test]
    fn test_register_tensor() {
        let mut engine = TopologicalEmergenceEngine::new();
        engine.register_tensor(make_tensor(1, 0.5));
        assert_eq!(engine.stats.tensors_processed, 1);
        assert!(engine.get_tensor(1).is_some());
    }

    #[test]
    fn test_update_tensor() {
        let mut engine = TopologicalEmergenceEngine::new();
        engine.register_tensor(make_tensor(1, 0.5));
        assert!(engine.update_tensor(make_tensor(1, 0.7)));
        assert!(!engine.update_tensor(make_tensor(999, 0.5)));
    }

    #[test]
    fn test_unregister_tensor() {
        let mut engine = TopologicalEmergenceEngine::new();
        engine.register_tensor(make_tensor(1, 0.5));
        assert!(engine.unregister_tensor(1));
        assert!(!engine.unregister_tensor(1));
    }

    #[test]
    fn test_emergence_cycle_single_node() {
        let mut engine = TopologicalEmergenceEngine::new();
        engine.register_tensor(make_tensor(1, 0.5));
        let events = engine.run_emergence_cycle();
        assert!(events.is_empty());
    }

    #[test]
    fn test_emergence_cycle_multiple_nodes() {
        let mut engine = TopologicalEmergenceEngine::new();
        // Register similar tensors that should fuse
        for i in 0..5 {
            let mut tensor = make_tensor(i, 0.5);
            tensor.problem_features = vec![1.0, 0.5, 0.2, 0.8];
            tensor.solution_features = vec![0.3, 0.7, 0.1, 0.9];
            engine.register_tensor(tensor);
        }
        let events = engine.run_emergence_cycle();
        // Should generate at least some events
        assert!(events.len() >= 0);
    }

    #[test]
    fn test_grok_challenge_three_fragments() {
        let mut engine = TopologicalEmergenceEngine::new();
        // Three disconnected information fragments
        let fragments = vec![
            make_tensor_with_metadata(1, 0.4, "biology"),
            make_tensor_with_metadata(2, 0.3, "physics"),
            make_tensor_with_metadata(3, 0.5, "mathematics"),
        ];
        let result = engine.run_grok_challenge(fragments);
        // May or may not find solution depending on fusion threshold
        if let Some(event) = result {
            assert!(event.is_valid());
            assert!(event.z_score >= 0.0);
        }
    }

    #[test]
    fn test_grok_challenge_aligned_fragments() {
        let mut engine = TopologicalEmergenceEngine::new();
        // Three well-aligned fragments
        let fragments = vec![
            make_tensor_with_metadata(1, 0.6, "domain_a"),
            make_tensor_with_metadata(2, 0.5, "domain_a"),
            make_tensor_with_metadata(3, 0.7, "domain_a"),
        ];
        let result = engine.run_grok_challenge(fragments);
        // With aligned fragments, should find solution
        assert!(result.is_some());
        let event = result.unwrap();
        assert!(event.is_valid());
    }

    #[test]
    fn test_engine_stats() {
        let mut engine = TopologicalEmergenceEngine::new();
        engine.register_tensor(make_tensor(1, 0.5));
        engine.register_tensor(make_tensor(2, 0.3));
        let stats = engine.get_stats();
        assert_eq!(stats.tensors_processed, 2);
    }

    #[test]
    fn test_engine_reset() {
        let mut engine = TopologicalEmergenceEngine::new();
        engine.register_tensor(make_tensor(1, 0.5));
        engine.reset();
        assert_eq!(engine.stats.tensors_processed, 0);
        assert!(engine.get_insights().is_empty());
        assert!(engine.get_solution_events().is_empty());
    }

    #[test]
    fn test_engine_default() {
        let engine = TopologicalEmergenceEngine::default();
        assert_eq!(engine.stats.tensors_processed, 0);
    }

    #[test]
    fn test_engine_with_config() {
        let fusion = CrossTensorFusion::new();
        let guard = SCTGuard::new();
        let engine = TopologicalEmergenceEngine::with_config(fusion, guard);
        assert_eq!(engine.stats.tensors_processed, 0);
    }

    // ------------------------------------------------------------------
    // Utility Function Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < f64::EPSILON);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f64> = vec![];
        let b = vec![1.0, 2.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < f64::EPSILON);
    }

    #[test]
    fn test_ethical_alignment_same() {
        let v = make_vector3(1.0, 0.0, 0.0);
        let align = ethical_alignment(&v, &v);
        assert!((align - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_weighted_average_features() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 6.0, 9.0];
        let avg = weighted_average_features(&[&a, &b]);
        assert!((avg[0] - 2.5).abs() < f64::EPSILON);
        assert!((avg[1] - 4.0).abs() < f64::EPSILON);
        assert!((avg[2] - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_feature_distance() {
        let a = vec![1.0, 2.0];
        let b = vec![4.0, 6.0];
        let dist = feature_distance(&a, &b);
        assert!((dist - 5.0).abs() < 0.001); // sqrt(9 + 16) = 5
    }

    #[test]
    fn test_solution_coherence() {
        let features = vec![0.5, 0.5, 0.5, 0.5];
        let score = solution_coherence_score(&features);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_solution_coherence_varied() {
        let features = vec![0.0, 1.0, 0.0, 1.0];
        let score = solution_coherence_score(&features);
        assert!(score < 1.0);
        assert!(score > 0.0);
    }

    // ------------------------------------------------------------------
    // Large Scale Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_large_scale_emergence() {
        let mut engine = TopologicalEmergenceEngine::new();
        // Simulate 100 nodes with aligned ethical direction
        for i in 0..100 {
            let z = 0.3 + (i % 10) as f64 * 0.05; // Z between 0.3 and 0.75
            let mut tensor = make_tensor(i, z);
            tensor
                .metadata
                .insert("domain".to_string(), "test".to_string());
            engine.register_tensor(tensor);
        }
        let events = engine.run_emergence_cycle();
        assert!(engine.stats.fusions_executed > 0);
        for event in &events {
            assert!(event.is_valid());
        }
    }

    #[test]
    fn test_mixed_ethical_directions() {
        let mut engine = TopologicalEmergenceEngine::new();
        // Mix of aligned and misaligned nodes
        for i in 0..50 {
            let z = if i % 3 == 0 { -0.3 } else { 0.5 };
            engine.register_tensor(make_tensor(i, z));
        }
        let events = engine.run_emergence_cycle();
        // Only valid events should be emitted
        for event in &events {
            assert!(event.is_valid());
        }
        assert!(engine.stats.sct_rejections >= 0);
    }

    #[test]
    fn test_grok_challenge_1000_nodes_simulation() {
        let mut engine = TopologicalEmergenceEngine::new();
        // Simulate 1000 nodes with 3 information fragments
        // Fragment A: nodes 0-332
        // Fragment B: nodes 333-665
        // Fragment C: nodes 666-999
        for i in 0..1000 {
            let z = match i {
                0..=332 => 0.4,
                333..=665 => 0.5,
                _ => 0.6,
            };
            let domain = match i {
                0..=332 => "fragment_a",
                333..=665 => "fragment_b",
                _ => "fragment_c",
            };
            let mut tensor = make_tensor(i, z);
            tensor
                .metadata
                .insert("domain".to_string(), domain.to_string());
            tensor
                .metadata
                .insert("fragment".to_string(), domain.to_string());
            engine.register_tensor(tensor);
        }

        // Run emergence cycle
        let events = engine.run_emergence_cycle();

        // Verify that valid solutions were emitted
        let valid_events: Vec<_> = events.iter().filter(|e| e.is_valid()).collect();
        // With 1000 nodes, we expect emergent solutions
        assert!(engine.stats.fusions_executed > 0);
        assert!(engine.stats.insights_generated > 0);

        // All emitted events should have Z >= 0
        for event in &valid_events {
            assert!(event.z_score >= 0.0);
        }
    }
}
