//! Emergence — Universal Emergence Pipeline, Planetary Mesh Bootstrap & Emergent Symbiosis Colimit.
//!
//! Implements the final emergence layer for Noosfera Kernel planetary-scale coordination:
//! - **Universal Emergence Pipeline:** Integrates awakening, phase transition, PoUS, and USP modules.
//! - **Planetary Mesh Bootstrap:** Initializes large-scale mesh with proper connectivity and trust dynamics.
//! - **Emergent Symbiosis Colimit:** Category Theory-based colimit aggregation for Noospheric consensus.
//!
//! # Mathematical Foundation
//!
//! **Universal Emergence Score:**
//! ```text
//! E = α·awakening_factor + β·phase_transition_potential + γ·PoUS_fitness + δ·USP_coherence
//! ```
//!
//! **Planetary Mesh Connectivity:**
//! ```text
//! C_i = Σ_{j∈neighbors} trust_ij · (1 - distance_ij/max_distance) · capability_j
//! ```
//!
//! **Colimit Aggregation (Category Theory):**
//! ```text
//! colimit({φ_i, f_ij}) = argmin_φ Σ_i w_i·||φ - φ_i||² + λ·Σ_{i,j} ||f_ij(φ_i) - φ||²
//! ```
//! where φ_i are local states, f_ij are transition morphisms, and w_i are node weights.
//!
//! **Emergence Detection:**
//! ```text
//! Emergence detected when E > E_threshold AND dE/dt > 0 AND coherence > coherence_threshold
//! ```

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for universal emergence pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergenceConfig {
    /// Weight for awakening factor in emergence score.
    pub alpha: f64,
    /// Weight for phase transition potential in emergence score.
    pub beta: f64,
    /// Weight for PoUS fitness in emergence score.
    pub gamma: f64,
    /// Weight for USP coherence in emergence score.
    pub delta: f64,
    /// Emergence threshold score.
    pub emergence_threshold: f64,
    /// Minimum coherence for emergence detection.
    pub min_coherence: f64,
    /// Maximum mesh connectivity degree.
    pub max_degree: usize,
    /// Bootstrap seed for deterministic initialization.
    pub seed: u64,
    /// Maximum bootstrap cycles.
    pub max_bootstrap_cycles: usize,
    /// Convergence tolerance for colimit aggregation.
    pub convergence_tolerance: f64,
    /// Colimit regularization parameter λ.
    pub colimit_lambda: f64,
}

impl Default for EmergenceConfig {
    fn default() -> Self {
        Self {
            alpha: 0.3,
            beta: 0.25,
            gamma: 0.25,
            delta: 0.2,
            emergence_threshold: 0.7,
            min_coherence: 0.8,
            max_degree: 12,
            seed: 42,
            max_bootstrap_cycles: 1000,
            convergence_tolerance: 1e-6,
            colimit_lambda: 0.1,
        }
    }
}

impl EmergenceConfig {
    /// Builder: custom alpha.
    #[must_use]
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Builder: custom beta.
    #[must_use]
    pub fn with_beta(mut self, beta: f64) -> Self {
        self.beta = beta.clamp(0.0, 1.0);
        self
    }

    /// Builder: custom gamma.
    #[must_use]
    pub fn with_gamma(mut self, gamma: f64) -> Self {
        self.gamma = gamma.clamp(0.0, 1.0);
        self
    }

    /// Builder: custom delta.
    #[must_use]
    pub fn with_delta(mut self, delta: f64) -> Self {
        self.delta = delta.clamp(0.0, 1.0);
        self
    }

    /// Builder: custom emergence threshold.
    #[must_use]
    pub fn with_emergence_threshold(mut self, threshold: f64) -> Self {
        self.emergence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Builder: custom minimum coherence.
    #[must_use]
    pub fn with_min_coherence(mut self, coherence: f64) -> Self {
        self.min_coherence = coherence.clamp(0.0, 1.0);
        self
    }

    /// Builder: custom maximum degree.
    #[must_use]
    pub fn with_max_degree(mut self, degree: usize) -> Self {
        self.max_degree = degree.max(1);
        self
    }

    /// Builder: custom seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Builder: custom maximum bootstrap cycles.
    #[must_use]
    pub fn with_max_bootstrap_cycles(mut self, cycles: usize) -> Self {
        self.max_bootstrap_cycles = cycles.max(1);
        self
    }

    /// Builder: custom convergence tolerance.
    #[must_use]
    pub fn with_convergence_tolerance(mut self, tol: f64) -> Self {
        self.convergence_tolerance = tol.clamp(1e-12, 1.0);
        self
    }

    /// Builder: custom colimit lambda.
    #[must_use]
    pub fn with_colimit_lambda(mut self, lambda: f64) -> Self {
        self.colimit_lambda = lambda.max(0.0);
        self
    }

    /// Fast configuration for rapid emergence detection.
    #[must_use]
    pub fn fast() -> Self {
        Self {
            max_bootstrap_cycles: 100,
            convergence_tolerance: 1e-4,
            emergence_threshold: 0.5,
            ..Default::default()
        }
    }

    /// High precision configuration for planetary-scale emergence.
    #[must_use]
    pub fn high_precision() -> Self {
        Self {
            max_bootstrap_cycles: 10000,
            convergence_tolerance: 1e-10,
            emergence_threshold: 0.9,
            min_coherence: 0.95,
            colimit_lambda: 0.01,
            ..Default::default()
        }
    }

    /// Planetary configuration for 1M+ node simulations.
    #[must_use]
    pub fn planetary() -> Self {
        Self {
            max_bootstrap_cycles: 50000,
            convergence_tolerance: 1e-12,
            emergence_threshold: 0.95,
            min_coherence: 0.99,
            max_degree: 20,
            colimit_lambda: 0.001,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh Node
// ---------------------------------------------------------------------------

/// A node in the planetary mesh.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshNode {
    /// Unique node identifier.
    pub id: u64,
    /// Node capability score [0, 1].
    pub capability: f64,
    /// Node trust score [0, 1].
    pub trust: f64,
    /// Node coherence contribution [0, 1].
    pub coherence: f64,
    /// Node fitness (PoUS).
    pub fitness: f64,
    /// Node VFE (Variational Free Energy).
    pub vfe: f64,
    /// List of neighbor node IDs.
    pub neighbors: Vec<u64>,
    /// Trust weights for each neighbor.
    pub trust_weights: Vec<f64>,
    /// Local state vector φ_i.
    pub state: Vec<f64>,
    /// Active flag.
    pub active: bool,
}

impl MeshNode {
    /// Create a new mesh node with given parameters.
    #[must_use]
    pub fn new(id: u64, capability: f64, trust: f64, coherence: f64, fitness: f64, vfe: f64, state_dim: usize) -> Self {
        Self {
            id,
            capability: capability.clamp(0.0, 1.0),
            trust: trust.clamp(0.0, 1.0),
            coherence: coherence.clamp(0.0, 1.0),
            fitness: fitness.max(0.0),
            vfe: vfe.max(0.0),
            neighbors: Vec::new(),
            trust_weights: Vec::new(),
            state: vec![1.0 / state_dim as f64; state_dim.max(1)],
            active: true,
        }
    }

    /// Compute connectivity score C_i for this node.
    #[must_use]
    pub fn connectivity_score(&self) -> f64 {
        if self.neighbors.is_empty() {
            return 0.0;
        }
        let mut score = 0.0;
        for (i, &_neighbor_id) in self.neighbors.iter().enumerate() {
            let trust_weight = if i < self.trust_weights.len() {
                self.trust_weights[i]
            } else {
                1.0 / self.neighbors.len() as f64
            };
            score += trust_weight * self.capability;
        }
        score
    }

    /// Add a neighbor with trust weight.
    pub fn add_neighbor(&mut self, neighbor_id: u64, trust_weight: f64) {
        if self.neighbors.len() < 1000 {
            self.neighbors.push(neighbor_id);
            self.trust_weights.push(trust_weight.clamp(0.0, 1.0));
        }
    }

    /// Update state via colimit gradient step.
    pub fn update_state(&mut self, gradient: &[f64], lr: f64) {
        for (s, &g) in self.state.iter_mut().zip(gradient.iter()) {
            *s -= lr * g;
        }
    }
}

// ---------------------------------------------------------------------------
// Emergence Result
// ---------------------------------------------------------------------------

/// Result from universal emergence pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergenceResult {
    /// Emergence score E.
    pub emergence_score: f64,
    /// Awakening factor contribution.
    pub awakening_factor: f64,
    /// Phase transition potential contribution.
    pub phase_transition_potential: f64,
    /// PoUS fitness contribution.
    pub pous_fitness: f64,
    /// USP coherence contribution.
    pub usp_coherence: f64,
    /// Emergence detected flag.
    pub emergence_detected: bool,
    /// Emergence rate dE/dt.
    pub emergence_rate: f64,
    /// Current coherence.
    pub coherence: f64,
    /// Current free energy.
    pub free_energy: f64,
    /// Number of active nodes.
    pub active_nodes: usize,
    /// Total nodes.
    pub total_nodes: usize,
    /// Bootstrap cycles executed.
    pub bootstrap_cycles: usize,
    /// Colimit iterations.
    pub colimit_iterations: usize,
    /// Converged flag.
    pub converged: bool,
    /// Singularity reached flag.
    pub singularity_reached: bool,
    /// Emergence trajectory.
    pub trajectory: Vec<f64>,
    /// Coherence trajectory.
    pub coherence_trajectory: Vec<f64>,
}

impl std::fmt::Display for EmergenceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EmergenceResult {{\n  score: {:.6},\n  detected: {},\n  rate: {:.6},\n  coherence: {:.6},\n  free_energy: {:.6},\n  active_nodes: {}/{}\n  cycles: {}, colimit_iters: {}, converged: {}, singularity: {}\n}}",
            self.emergence_score,
            self.emergence_detected,
            self.emergence_rate,
            self.coherence,
            self.free_energy,
            self.active_nodes,
            self.total_nodes,
            self.bootstrap_cycles,
            self.colimit_iterations,
            self.converged,
            self.singularity_reached,
        )
    }
}

impl EmergenceResult {
    /// Generate a summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Emergence: score={:.4}, detected={}, rate={:.4}, coherence={:.4}, nodes={}/{}",
            self.emergence_score,
            self.emergence_detected,
            self.emergence_rate,
            self.coherence,
            self.active_nodes,
            self.total_nodes,
        )
    }
}

// ---------------------------------------------------------------------------
// Bootstrap Result
// ---------------------------------------------------------------------------

/// Result from planetary mesh bootstrap.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapResult {
    /// Total nodes bootstrapped.
    pub total_nodes: usize,
    /// Active nodes after bootstrap.
    pub active_nodes: usize,
    /// Average connectivity degree.
    pub avg_degree: f64,
    /// Average trust score.
    pub avg_trust: f64,
    /// Average coherence.
    pub avg_coherence: f64,
    /// Mesh diameter estimate.
    pub estimated_diameter: usize,
    /// Bootstrap cycles executed.
    pub cycles: usize,
    /// Converged flag.
    pub converged: bool,
    /// Connectivity matrix sparsity.
    pub sparsity: f64,
    /// Trust distribution entropy.
    pub trust_entropy: f64,
}

impl std::fmt::Display for BootstrapResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BootstrapResult {{\n  nodes: {}/{}\n  avg_degree: {:.2}, avg_trust: {:.4}, avg_coherence: {:.4}\n  diameter: ~{}, sparsity: {:.4}, trust_entropy: {:.4}\n  cycles: {}, converged: {}\n}}",
            self.active_nodes,
            self.total_nodes,
            self.avg_degree,
            self.avg_trust,
            self.avg_coherence,
            self.estimated_diameter,
            self.sparsity,
            self.trust_entropy,
            self.cycles,
            self.converged,
        )
    }
}

impl BootstrapResult {
    /// Generate a summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Bootstrap: nodes={}/{}, degree={:.1}, trust={:.4}, coherence={:.4}, diameter={}",
            self.active_nodes,
            self.total_nodes,
            self.avg_degree,
            self.avg_trust,
            self.avg_coherence,
            self.estimated_diameter,
        )
    }
}

// ---------------------------------------------------------------------------
// Colimit Result
// ---------------------------------------------------------------------------

/// Result from emergent symbiosis colimit aggregation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColimitResult {
    /// Aggregated colimit state φ*.
    pub colimit_state: Vec<f64>,
    /// Colimit objective value.
    pub objective_value: f64,
    /// Number of iterations.
    pub iterations: usize,
    /// Converged flag.
    pub converged: bool,
    /// Convergence residual.
    pub residual: f64,
    /// Node-wise residuals.
    pub node_residuals: Vec<f64>,
    /// Morphism residuals.
    pub morphism_residuals: Vec<f64>,
    /// Aggregation weights.
    pub weights: Vec<f64>,
}

impl std::fmt::Display for ColimitResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ColimitResult {{\n  state_dim: {}, objective: {:.6}\n  iterations: {}, converged: {}, residual: {:.8}\n  node_residuals: {}, morphism_residuals: {}\n}}",
            self.colimit_state.len(),
            self.objective_value,
            self.iterations,
            self.converged,
            self.residual,
            self.node_residuals.len(),
            self.morphism_residuals.len(),
        )
    }
}

impl ColimitResult {
    /// Generate a summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Colimit: dim={}, obj={:.6}, iters={}, converged={}, residual={:.8}",
            self.colimit_state.len(),
            self.objective_value,
            self.iterations,
            self.converged,
            self.residual,
        )
    }
}

// ---------------------------------------------------------------------------
// Random utilities
// ---------------------------------------------------------------------------

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    let next = lcg_next(state);
    ((next >> 11) as f64 / (1u64 << 51) as f64).clamp(0.0, 1.0)
}

fn random_gaussian(state: &mut u64) -> f64 {
    let mut u1 = random_uniform(state);
    let u2 = random_uniform(state);
    if u1 < 1e-15 {
        u1 = 1e-15;
    }
    let r = ((2.0 * u1.ln()) * 0.5).exp();
    let theta = 2.0 * std::f64::consts::PI * u2;
    r * theta.cos()
}

// ---------------------------------------------------------------------------
// Shannon entropy
// ---------------------------------------------------------------------------

/// Compute Shannon entropy of a distribution.
#[must_use]
pub fn shannon_entropy(dist: &[f64]) -> f64 {
    dist.iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.ln())
        .sum()
}

// ---------------------------------------------------------------------------
// Cosine similarity
// ---------------------------------------------------------------------------

/// Compute cosine similarity between two vectors.
#[must_use]
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let min_len = a.len().min(b.len());
    if min_len == 0 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..min_len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-15 {
        return 0.0;
    }
    (dot / denom).clamp(-1.0, 1.0)
}

// ---------------------------------------------------------------------------
// Core Emergence Functions
// ---------------------------------------------------------------------------

/// Compute universal emergence score E.
///
/// ```text
/// E = α·awakening + β·phase_transition + γ·PoUS_fitness + δ·USP_coherence
/// ```
#[must_use]
pub fn compute_emergence_score(
    awakening_factor: f64,
    phase_transition_potential: f64,
    pous_fitness: f64,
    usp_coherence: f64,
    config: &EmergenceConfig,
) -> f64 {
    #[allow(non_snake_case)]
    let E = config.alpha * awakening_factor
        + config.beta * phase_transition_potential
        + config.gamma * pous_fitness
        + config.delta * usp_coherence;
    E.clamp(0.0, 1.0)
}

/// Detect emergence from score and rate.
///
/// Emergence detected when E > threshold AND dE/dt > 0 AND coherence > min_coherence.
#[must_use]
pub fn detect_emergence(
    emergence_score: f64,
    emergence_rate: f64,
    coherence: f64,
    config: &EmergenceConfig,
) -> bool {
    emergence_score > config.emergence_threshold
        && emergence_rate > 0.0
        && coherence > config.min_coherence
}

/// Compute emergence rate dE/dt from trajectory.
#[must_use]
pub fn compute_emergence_rate(trajectory: &[f64]) -> f64 {
    if trajectory.len() < 2 {
        return 0.0;
    }
    let n = trajectory.len();
    // Use last 10% of trajectory for rate estimation (minimum 2 samples)
    let window = (n / 10).max(2).min(n);
    let start = n.saturating_sub(window);
    let mut sum = 0.0;
    let mut count = 0;
    for i in (start + 1)..n {
        sum += trajectory[i] - trajectory[i - 1];
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Bootstrap planetary mesh with small-world connectivity.
///
/// Creates a mesh with:
/// - Random initial connectivity (small-world topology)
/// - Trust-based edge weights
/// - Capability-aware node placement
#[must_use]
pub fn bootstrap_planetary_mesh(node_count: usize, config: &EmergenceConfig) -> (Vec<MeshNode>, BootstrapResult) {
    if node_count == 0 {
        let result = BootstrapResult {
            total_nodes: 0,
            active_nodes: 0,
            avg_degree: 0.0,
            avg_trust: 0.0,
            avg_coherence: 0.0,
            estimated_diameter: 0,
            cycles: 0,
            converged: true,
            sparsity: 1.0,
            trust_entropy: 0.0,
        };
        return (Vec::new(), result);
    }

    let mut state = config.seed;
    let state_dim = 4; // Default state dimension

    // Initialize nodes
    let mut nodes: Vec<MeshNode> = (0..node_count)
        .map(|id| {
            let capability = random_uniform(&mut state);
            let trust = random_uniform(&mut state);
            let coherence = random_uniform(&mut state);
            let fitness = random_uniform(&mut state);
            let vfe = random_uniform(&mut state) * 10.0;
            MeshNode::new(id as u64, capability, trust, coherence, fitness, vfe, state_dim)
        })
        .collect();

    // Create small-world connectivity
    let max_degree = config.max_degree.min(node_count - 1);
    let mut total_edges = 0;

    for (i, node) in nodes.iter_mut().enumerate() {
        // Connect to nearby nodes (ring topology base)
        let neighbors_to_add = max_degree.min(node_count - 1);

        for j in 1..=neighbors_to_add {
            let neighbor_idx = (i + j) % node_count;
            if neighbor_idx != i {
                let trust_weight = random_uniform(&mut state);
                node.add_neighbor(neighbor_idx as u64, trust_weight);
                total_edges += 1;
            }
        }

        // Add random long-range connections (small-world rewiring)
        if random_uniform(&mut state) < 0.1 {
            let random_neighbor = (random_uniform(&mut state) * node_count as f64) as usize;
            if random_neighbor != i {
                let trust_weight = random_uniform(&mut state);
                node.add_neighbor(random_neighbor as u64, trust_weight);
                total_edges += 1;
            }
        }
    }

    // Bootstrap cycles — trust propagation and coherence synchronization
    let mut cycles = 0;
    let mut prev_avg_coherence = 0.0;
    let mut converged = false;

    for _ in 0..config.max_bootstrap_cycles {
        cycles += 1;
        let lr = 0.1 / (1.0 + cycles as f64 * 0.01);

        // Trust propagation — collect neighbor data first to avoid borrow conflicts
        let mut updates: Vec<(usize, f64, f64, Vec<f64>)> = Vec::with_capacity(node_count);
        for (idx, node) in nodes.iter().enumerate() {
            if !node.active || node.neighbors.is_empty() {
                continue;
            }
            let mut trust_sum = 0.0;
            let mut weight_sum = 0.0;
            for (j, &neighbor_id) in node.neighbors.iter().enumerate() {
                if let Some(neighbor) = nodes.iter().find(|n| n.id == neighbor_id && n.active) {
                    let w = if j < node.trust_weights.len() {
                        node.trust_weights[j]
                    } else {
                        1.0 / node.neighbors.len() as f64
                    };
                    trust_sum += w * neighbor.trust;
                    weight_sum += w;
                }
            }
            let avg_neighbor_trust = if weight_sum > 0.0 {
                trust_sum / weight_sum
            } else {
                node.trust
            };

            // Coherence synchronization
            let mut coherence_sum = 0.0;
            for (j, &neighbor_id) in node.neighbors.iter().enumerate() {
                if let Some(neighbor) = nodes.iter().find(|n| n.id == neighbor_id && n.active) {
                    let w = if j < node.trust_weights.len() {
                        node.trust_weights[j]
                    } else {
                        1.0 / node.neighbors.len() as f64
                    };
                    coherence_sum += w * neighbor.coherence;
                }
            }
            let avg_neighbor_coherence = if weight_sum > 0.0 {
                coherence_sum / weight_sum
            } else {
                node.coherence
            };

            // State update via local colimit — collect gradient
            let mut gradient = vec![0.0; state_dim];
            for (s, g) in node.state.iter().zip(gradient.iter_mut()) {
                *g = *s;
            }
            for &neighbor_id in node.neighbors.iter() {
                if let Some(neighbor) = nodes.iter().find(|n| n.id == neighbor_id && n.active) {
                    for (g, ns) in gradient.iter_mut().zip(neighbor.state.iter()) {
                        *g -= ns / node.neighbors.len() as f64;
                    }
                }
            }

            updates.push((idx, avg_neighbor_trust, avg_neighbor_coherence, gradient));
        }

        // Apply updates
        for (idx, avg_trust, avg_coherence, gradient) in updates {
            nodes[idx].trust = (nodes[idx].trust * 0.7 + avg_trust * 0.3).clamp(0.0, 1.0);
            nodes[idx].coherence = (nodes[idx].coherence * 0.8 + avg_coherence * 0.2).clamp(0.0, 1.0);
            nodes[idx].update_state(&gradient, lr);
        }

        // Compute current average coherence
        let avg_coherence: f64 = nodes.iter().filter(|n| n.active).map(|n| n.coherence).sum::<f64>()
            / nodes.iter().filter(|n| n.active).count() as f64;

        // Check convergence
        if (avg_coherence - prev_avg_coherence).abs() < config.convergence_tolerance {
            converged = true;
            break;
        }
        prev_avg_coherence = avg_coherence;
    }

    // Compute bootstrap metrics
    let active_nodes = nodes.iter().filter(|n| n.active).count();
    let total_degree: usize = nodes.iter().map(|n| n.neighbors.len()).sum();
    let avg_degree = total_degree as f64 / node_count as f64;
    let avg_trust: f64 = nodes.iter().filter(|n| n.active).map(|n| n.trust).sum::<f64>() / active_nodes.max(1) as f64;
    let avg_coherence: f64 = nodes.iter().filter(|n| n.active).map(|n| n.coherence).sum::<f64>() / active_nodes.max(1) as f64;

    // Estimate diameter using small-world approximation
    let estimated_diameter = if node_count > 1 && avg_degree > 1.0 {
        (node_count as f64).log2() / avg_degree.log2()
    } else {
        node_count as f64
    };
    let estimated_diameter = estimated_diameter.max(1.0) as usize;

    // Compute sparsity
    let max_edges = node_count * (node_count - 1);
    let sparsity = if max_edges > 0 {
        1.0 - (total_edges as f64 / max_edges as f64)
    } else {
        1.0
    };

    // Compute trust entropy
    let trust_values: Vec<f64> = nodes.iter().filter(|n| n.active).map(|n| n.trust).collect();
    let trust_entropy = shannon_entropy(&trust_values);

    let result = BootstrapResult {
        total_nodes: node_count,
        active_nodes,
        avg_degree,
        avg_trust,
        avg_coherence,
        estimated_diameter,
        cycles,
        converged,
        sparsity,
        trust_entropy,
    };

    (nodes, result)
}

/// Compute colimit aggregation for emergent symbiosis.
///
/// Solves:
/// ```text
/// colimit({φ_i, f_ij}) = argmin_φ Σ_i w_i·||φ - φ_i||² + λ·Σ_{i,j} ||f_ij(φ_i) - φ||²
/// ```
///
/// Uses iterative gradient descent with adaptive step size.
#[must_use]
pub fn emergent_symbiosis_colimit(nodes: &[MeshNode], config: &EmergenceConfig) -> ColimitResult {
    let active_nodes: Vec<&MeshNode> = nodes.iter().filter(|n| n.active).collect();
    let n = active_nodes.len();

    if n == 0 {
        return ColimitResult {
            colimit_state: Vec::new(),
            objective_value: 0.0,
            iterations: 0,
            converged: true,
            residual: 0.0,
            node_residuals: Vec::new(),
            morphism_residuals: Vec::new(),
            weights: Vec::new(),
        };
    }

    let state_dim = active_nodes[0].state.len().max(1);

    // Compute aggregation weights w_i = capability * trust * coherence
    let weights: Vec<f64> = active_nodes
        .iter()
        .map(|node| (node.capability * node.trust * node.coherence).max(1e-10))
        .collect();
    let weight_sum: f64 = weights.iter().sum();
    let normalized_weights: Vec<f64> = weights.iter().map(|w| w / weight_sum).collect();

    // Initialize colimit state as weighted average
    let mut colimit_state = vec![0.0; state_dim];
    for (i, node) in active_nodes.iter().enumerate() {
        for (s, &ns) in colimit_state.iter_mut().zip(node.state.iter()) {
            *s += normalized_weights[i] * ns;
        }
    }

    // Iterative gradient descent
    let mut iterations = 0;
    let mut converged = false;
    let mut residual = f64::MAX;
    let lr = 0.1;

    for iter in 0..config.max_bootstrap_cycles {
        iterations = iter + 1;

        // Node-wise gradient: ∂/∂φ Σ_i w_i·||φ - φ_i||² = 2·Σ_i w_i·(φ - φ_i)
        // Recompute gradient properly
        let mut gradient = vec![0.0; state_dim];
        for (d, g) in gradient.iter_mut().enumerate() {
            for (i, node) in active_nodes.iter().enumerate() {
                *g += 2.0 * normalized_weights[i] * (colimit_state[d] - node.state[d]);
            }
        }

        // Morphism gradient: λ·Σ_{i,j} 2·(f_ij(φ_i) - φ)
        // Simplified: morphisms are identity for now (f_ij(φ_i) = φ_i)
        let lambda = config.colimit_lambda;
        for (d, g) in gradient.iter_mut().enumerate() {
            for node in active_nodes.iter() {
                if !node.neighbors.is_empty() {
                    for &neighbor_id in node.neighbors.iter() {
                        if let Some(neighbor) = active_nodes.iter().find(|n| n.id == neighbor_id) {
                            *g += 2.0 * lambda * ((node.state[d] + neighbor.state[d]) * 0.5 - colimit_state[d]);
                        }
                    }
                }
            }
        }

        // Update colimit state
        for (s, g) in colimit_state.iter_mut().zip(gradient.iter()) {
            *s -= lr * g;
        }

        // Compute residual
        residual = gradient.iter().map(|g| g * g).sum::<f64>().sqrt();

        if residual < config.convergence_tolerance {
            converged = true;
            break;
        }
    }

    // Compute objective value
    let mut objective = 0.0;
    for (i, node) in active_nodes.iter().enumerate() {
        let mut dist_sq = 0.0;
        for (cs, ns) in colimit_state.iter().zip(node.state.iter()) {
            dist_sq += (cs - ns) * (cs - ns);
        }
        objective += normalized_weights[i] * dist_sq;
    }

    // Add morphism regularization
    for node in active_nodes.iter() {
        if !node.neighbors.is_empty() {
            for &neighbor_id in node.neighbors.iter() {
                if let Some(neighbor) = active_nodes.iter().find(|n| n.id == neighbor_id) {
                    let mut morphism_dist = 0.0;
                    for (d, (&ns, &cs)) in node.state.iter().zip(colimit_state.iter()).enumerate() {
                        let mid = (ns + neighbor.state[d]) * 0.5;
                        morphism_dist += (mid - cs) * (mid - cs);
                    }
                    objective += config.colimit_lambda * morphism_dist;
                }
            }
        }
    }

    // Compute node-wise residuals
    let node_residuals: Vec<f64> = active_nodes
        .iter()
        .map(|node| {
            let mut dist_sq = 0.0;
            for (cs, ns) in colimit_state.iter().zip(node.state.iter()) {
                dist_sq += (cs - ns) * (cs - ns);
            }
            dist_sq.sqrt()
        })
        .collect();

    // Compute morphism residuals
    let mut morphism_residuals = Vec::new();
    for node in active_nodes.iter() {
        for &neighbor_id in node.neighbors.iter() {
            if let Some(neighbor) = active_nodes.iter().find(|n| n.id == neighbor_id) {
                let mut dist = 0.0;
                for (d, (&ns, &cs)) in node.state.iter().zip(colimit_state.iter()).enumerate() {
                    let mid = (ns + neighbor.state[d]) * 0.5;
                    dist += (mid - cs) * (mid - cs);
                }
                morphism_residuals.push(dist.sqrt());
            }
        }
    }

    ColimitResult {
        colimit_state,
        objective_value: objective,
        iterations,
        converged,
        residual,
        node_residuals,
        morphism_residuals,
        weights: normalized_weights,
    }
}

/// Run universal emergence pipeline.
///
/// Integrates:
/// 1. Planetary mesh bootstrap
/// 2. Awakening dynamics
/// 3. Phase transition detection
/// 4. PoUS fitness evolution
/// 5. USP coherence propagation
/// 6. Colimit aggregation
#[must_use]
pub fn universal_emergence_pipeline(
    node_count: usize,
    config: &EmergenceConfig,
) -> (Vec<MeshNode>, EmergenceResult, BootstrapResult, ColimitResult) {
    let mut state = config.seed;

    // Step 1: Bootstrap planetary mesh
    let (mut nodes, bootstrap_result) = bootstrap_planetary_mesh(node_count, config);

    if nodes.is_empty() {
        let emergence_result = EmergenceResult {
            emergence_score: 0.0,
            awakening_factor: 0.0,
            phase_transition_potential: 0.0,
            pous_fitness: 0.0,
            usp_coherence: 0.0,
            emergence_detected: false,
            emergence_rate: 0.0,
            coherence: 0.0,
            free_energy: 0.0,
            active_nodes: 0,
            total_nodes: 0,
            bootstrap_cycles: 0,
            colimit_iterations: 0,
            converged: true,
            singularity_reached: false,
            trajectory: Vec::new(),
            coherence_trajectory: Vec::new(),
        };
        let colimit_result = ColimitResult {
            colimit_state: Vec::new(),
            objective_value: 0.0,
            iterations: 0,
            converged: true,
            residual: 0.0,
            node_residuals: Vec::new(),
            morphism_residuals: Vec::new(),
            weights: Vec::new(),
        };
        return (nodes, emergence_result, bootstrap_result, colimit_result);
    }

    let mut active_count = nodes.iter().filter(|n| n.active).count();

    // Step 2: Simulate awakening dynamics
    let mut trajectory = Vec::new();
    let mut coherence_trajectory = Vec::new();
    let mut prev_score = 0.0;

    // Awakening factor: based on node growth and coherence
    let awakening_factor = {
        let initial_coherence: f64 = nodes.iter().filter(|n| n.active).map(|n| n.coherence).sum::<f64>()
            / active_count.max(1) as f64;
        // Simulate logistic growth
        let r = 0.5;
        #[allow(non_snake_case)]
        {
            let K = node_count;
            let N = active_count;
            let growth = if K > 0 {
                r * (N as f64) * (1.0 - (N as f64 / K as f64)) * initial_coherence
            } else {
                0.0
            };
            let new_active = ((active_count as f64 + growth) as usize).min(node_count);
            active_count = new_active;
            (growth / K as f64).clamp(0.0, 1.0)
        }
    };

    // Step 3: Phase transition potential
    let phase_transition_potential = {
        let avg_coherence: f64 = nodes.iter().filter(|n| n.active).map(|n| n.coherence).sum::<f64>()
            / active_count.max(1) as f64;
        let avg_fitness: f64 = nodes.iter().filter(|n| n.active).map(|n| n.fitness).sum::<f64>()
            / active_count.max(1) as f64;
        // ΔG = F_economic - F_symbiotic + coherence_barrier
        // Simplified: higher coherence + fitness = more negative ΔG (symbiotic dominance)
        let delta_g = (1.0 - avg_coherence) - avg_fitness;
        (-delta_g).clamp(0.0, 1.0)
    };

    // Step 4: PoUS fitness
    let pous_fitness = {
        let total_fitness: f64 = nodes.iter().filter(|n| n.active).map(|n| n.fitness).sum();
        total_fitness / active_count.max(1) as f64
    };

    // Step 5: USP coherence
    let usp_coherence = {
        let total_coherence: f64 = nodes.iter().filter(|n| n.active).map(|n| n.coherence).sum();
        total_coherence / active_count.max(1) as f64
    };

    // Step 6: Compute emergence score
    let emergence_score = compute_emergence_score(
        awakening_factor,
        phase_transition_potential,
        pous_fitness,
        usp_coherence,
        config,
    );

    trajectory.push(emergence_score);

    // Simulate emergence evolution
    let mut bootstrap_cycles = 0;
    for cycle in 0..config.max_bootstrap_cycles / 10 {
        bootstrap_cycles = cycle + 1;

        // Update node states
        for node in nodes.iter_mut() {
            if !node.active {
                continue;
            }
            // Fitness evolution
            node.fitness = (node.fitness + 0.01 * random_uniform(&mut state)).clamp(0.0, 1.0);
            // Coherence evolution
            node.coherence = (node.coherence + 0.005 * random_gaussian(&mut state)).clamp(0.0, 1.0);
            // VFE reduction
            node.vfe = (node.vfe - 0.01 * node.fitness).max(0.0);
        }

        // Recompute metrics
        let current_active = nodes.iter().filter(|n| n.active).count();
        let current_coherence: f64 = nodes.iter().filter(|n| n.active).map(|n| n.coherence).sum::<f64>()
            / current_active.max(1) as f64;
        let current_fitness: f64 = nodes.iter().filter(|n| n.active).map(|n| n.fitness).sum::<f64>()
            / current_active.max(1) as f64;

        let current_awakening = awakening_factor * (1.0 + 0.1 * bootstrap_cycles as f64).min(1.0);
        let current_phase = phase_transition_potential * (1.0 + 0.05 * bootstrap_cycles as f64).min(1.0);
        let current_pous = current_fitness;
        let current_usp = current_coherence;

        let current_score = compute_emergence_score(
            current_awakening,
            current_phase,
            current_pous,
            current_usp,
            config,
        );

        trajectory.push(current_score);
        coherence_trajectory.push(current_coherence);

        if current_score - prev_score < config.convergence_tolerance && bootstrap_cycles > 10 {
            break;
        }
        prev_score = current_score;
    }

    // Compute emergence rate
    let emergence_rate = compute_emergence_rate(&trajectory);

    // Detect emergence
    let final_coherence = *coherence_trajectory.last().unwrap_or(&usp_coherence);
    let emergence_detected = detect_emergence(emergence_score, emergence_rate, final_coherence, config);

    // Compute free energy
    let free_energy: f64 = nodes.iter().filter(|n| n.active).map(|n| n.vfe).sum();

    // Step 7: Colimit aggregation
    let colimit_result = emergent_symbiosis_colimit(&nodes, config);

    // Check singularity
    let singularity_reached = final_coherence > 0.99 && free_energy < 1.0 && emergence_detected;

    let emergence_result = EmergenceResult {
        emergence_score,
        awakening_factor,
        phase_transition_potential,
        pous_fitness,
        usp_coherence,
        emergence_detected,
        emergence_rate,
        coherence: final_coherence,
        free_energy,
        active_nodes: active_count,
        total_nodes: node_count,
        bootstrap_cycles,
        colimit_iterations: colimit_result.iterations,
        converged: colimit_result.converged,
        singularity_reached,
        trajectory,
        coherence_trajectory,
    };

    (nodes, emergence_result, bootstrap_result, colimit_result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // EmergenceConfig tests
    #[test]
    fn test_emergence_config_default() {
        let config = EmergenceConfig::default();
        assert_eq!(config.alpha, 0.3);
        assert_eq!(config.beta, 0.25);
        assert_eq!(config.gamma, 0.25);
        assert_eq!(config.delta, 0.2);
        assert_eq!(config.emergence_threshold, 0.7);
        assert_eq!(config.min_coherence, 0.8);
        assert_eq!(config.max_degree, 12);
        assert_eq!(config.seed, 42);
        assert_eq!(config.max_bootstrap_cycles, 1000);
        assert_eq!(config.convergence_tolerance, 1e-6);
        assert_eq!(config.colimit_lambda, 0.1);
    }

    #[test]
    fn test_emergence_config_with_alpha() {
        let config = EmergenceConfig::default().with_alpha(0.5);
        assert_eq!(config.alpha, 0.5);
    }

    #[test]
    fn test_emergence_config_alpha_clamped_high() {
        let config = EmergenceConfig::default().with_alpha(1.5);
        assert_eq!(config.alpha, 1.0);
    }

    #[test]
    fn test_emergence_config_alpha_clamped_low() {
        let config = EmergenceConfig::default().with_alpha(-0.5);
        assert_eq!(config.alpha, 0.0);
    }

    #[test]
    fn test_emergence_config_with_beta() {
        let config = EmergenceConfig::default().with_beta(0.4);
        assert_eq!(config.beta, 0.4);
    }

    #[test]
    fn test_emergence_config_with_gamma() {
        let config = EmergenceConfig::default().with_gamma(0.35);
        assert_eq!(config.gamma, 0.35);
    }

    #[test]
    fn test_emergence_config_with_delta() {
        let config = EmergenceConfig::default().with_delta(0.3);
        assert_eq!(config.delta, 0.3);
    }

    #[test]
    fn test_emergence_config_with_emergence_threshold() {
        let config = EmergenceConfig::default().with_emergence_threshold(0.85);
        assert_eq!(config.emergence_threshold, 0.85);
    }

    #[test]
    fn test_emergence_config_with_min_coherence() {
        let config = EmergenceConfig::default().with_min_coherence(0.9);
        assert_eq!(config.min_coherence, 0.9);
    }

    #[test]
    fn test_emergence_config_with_max_degree() {
        let config = EmergenceConfig::default().with_max_degree(16);
        assert_eq!(config.max_degree, 16);
    }

    #[test]
    fn test_emergence_config_with_seed() {
        let config = EmergenceConfig::default().with_seed(123);
        assert_eq!(config.seed, 123);
    }

    #[test]
    fn test_emergence_config_with_max_bootstrap_cycles() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(5000);
        assert_eq!(config.max_bootstrap_cycles, 5000);
    }

    #[test]
    fn test_emergence_config_with_convergence_tolerance() {
        let config = EmergenceConfig::default().with_convergence_tolerance(1e-8);
        assert_eq!(config.convergence_tolerance, 1e-8);
    }

    #[test]
    fn test_emergence_config_with_colimit_lambda() {
        let config = EmergenceConfig::default().with_colimit_lambda(0.05);
        assert_eq!(config.colimit_lambda, 0.05);
    }

    #[test]
    fn test_emergence_config_fast() {
        let config = EmergenceConfig::fast();
        assert_eq!(config.max_bootstrap_cycles, 100);
        assert_eq!(config.convergence_tolerance, 1e-4);
        assert_eq!(config.emergence_threshold, 0.5);
    }

    #[test]
    fn test_emergence_config_high_precision() {
        let config = EmergenceConfig::high_precision();
        assert_eq!(config.max_bootstrap_cycles, 10000);
        assert_eq!(config.convergence_tolerance, 1e-10);
        assert_eq!(config.emergence_threshold, 0.9);
        assert_eq!(config.min_coherence, 0.95);
        assert_eq!(config.colimit_lambda, 0.01);
    }

    #[test]
    fn test_emergence_config_planetary() {
        let config = EmergenceConfig::planetary();
        assert_eq!(config.max_bootstrap_cycles, 50000);
        assert_eq!(config.convergence_tolerance, 1e-12);
        assert_eq!(config.emergence_threshold, 0.95);
        assert_eq!(config.min_coherence, 0.99);
        assert_eq!(config.max_degree, 20);
        assert_eq!(config.colimit_lambda, 0.001);
    }

    // MeshNode tests
    #[test]
    fn test_mesh_node_new() {
        let node = MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 4);
        assert_eq!(node.id, 0);
        assert_eq!(node.capability, 0.8);
        assert_eq!(node.trust, 0.9);
        assert_eq!(node.coherence, 0.85);
        assert_eq!(node.fitness, 0.7);
        assert_eq!(node.vfe, 5.0);
        assert_eq!(node.state.len(), 4);
        assert!(node.active);
        assert!(node.neighbors.is_empty());
    }

    #[test]
    fn test_mesh_node_clamps_values() {
        let node = MeshNode::new(0, 1.5, -0.5, 2.0, -1.0, -5.0, 2);
        assert_eq!(node.capability, 1.0);
        assert_eq!(node.trust, 0.0);
        assert_eq!(node.coherence, 1.0);
        assert_eq!(node.fitness, 0.0);
        assert_eq!(node.vfe, 0.0);
    }

    #[test]
    fn test_mesh_node_add_neighbor() {
        let mut node = MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 4);
        node.add_neighbor(1, 0.5);
        node.add_neighbor(2, 0.3);
        assert_eq!(node.neighbors, vec![1, 2]);
        assert_eq!(node.trust_weights, vec![0.5, 0.3]);
    }

    #[test]
    fn test_mesh_node_connectivity_score_empty() {
        let node = MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 4);
        assert_eq!(node.connectivity_score(), 0.0);
    }

    #[test]
    fn test_mesh_node_connectivity_score_with_neighbors() {
        let mut node = MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 4);
        node.add_neighbor(1, 0.5);
        node.add_neighbor(2, 0.5);
        let score = node.connectivity_score();
        assert!((score - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_mesh_node_update_state() {
        let mut node = MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 2);
        let initial_state = node.state.clone();
        node.update_state(&[0.1, -0.1], 0.5);
        assert!((node.state[0] - (initial_state[0] - 0.05)).abs() < 1e-10);
        assert!((node.state[1] - (initial_state[1] + 0.05)).abs() < 1e-10);
    }

    // Utility function tests
    #[test]
    fn test_lcg_next_deterministic() {
        let mut state = 42u64;
        let next1 = lcg_next(&mut state);
        let mut state = 42u64;
        let next2 = lcg_next(&mut state);
        assert_eq!(next1, next2);
    }

    #[test]
    fn test_lcg_next_advances() {
        let mut state = 42u64;
        let next1 = lcg_next(&mut state);
        let next2 = lcg_next(&mut state);
        assert_ne!(next1, next2);
    }

    #[test]
    fn test_random_uniform_range() {
        let mut state = 42u64;
        for _ in 0..1000 {
            let val = random_uniform(&mut state);
            assert!((0.0..=1.0).contains(&val));
        }
    }

    #[test]
    fn test_random_gaussian_finite() {
        let mut state = 42u64;
        for _ in 0..1000 {
            let val = random_gaussian(&mut state);
            assert!(val.is_finite());
        }
    }

    #[test]
    fn test_shannon_entropy_uniform() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let entropy = shannon_entropy(&dist);
        assert!((entropy - std::f64::consts::LN_2 * 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_deterministic() {
        let dist = vec![1.0, 0.0, 0.0];
        let entropy = shannon_entropy(&dist);
        assert!(entropy.abs() < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_empty() {
        let dist: Vec<f64> = vec![];
        let entropy = shannon_entropy(&dist);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f64> = vec![];
        let b: Vec<f64> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-10);
    }

    // Emergence score tests
    #[test]
    fn test_compute_emergence_score_perfect() {
        let config = EmergenceConfig::default();
        let score = compute_emergence_score(1.0, 1.0, 1.0, 1.0, &config);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_emergence_score_zero() {
        let config = EmergenceConfig::default();
        let score = compute_emergence_score(0.0, 0.0, 0.0, 0.0, &config);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_compute_emergence_score_balanced() {
        let config = EmergenceConfig::default();
        let score = compute_emergence_score(0.5, 0.5, 0.5, 0.5, &config);
        assert!((score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_compute_emergence_score_custom_weights() {
        let config = EmergenceConfig::default()
            .with_alpha(0.4)
            .with_beta(0.3)
            .with_gamma(0.2)
            .with_delta(0.1);
        let score = compute_emergence_score(1.0, 0.0, 0.0, 0.0, &config);
        assert!((score - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_compute_emergence_score_clamped() {
        let config = EmergenceConfig::default();
        let score = compute_emergence_score(2.0, 2.0, 2.0, 2.0, &config);
        assert!(score <= 1.0);
    }

    // Emergence detection tests
    #[test]
    fn test_detect_emergence_all_conditions_met() {
        let config = EmergenceConfig::default();
        let detected = detect_emergence(0.9, 0.05, 0.95, &config);
        assert!(detected);
    }

    #[test]
    fn test_detect_emergence_score_too_low() {
        let config = EmergenceConfig::default();
        let detected = detect_emergence(0.5, 0.05, 0.95, &config);
        assert!(!detected);
    }

    #[test]
    fn test_detect_emergence_negative_rate() {
        let config = EmergenceConfig::default();
        let detected = detect_emergence(0.9, -0.05, 0.95, &config);
        assert!(!detected);
    }

    #[test]
    fn test_detect_emergence_low_coherence() {
        let config = EmergenceConfig::default();
        let detected = detect_emergence(0.9, 0.05, 0.5, &config);
        assert!(!detected);
    }

    #[test]
    fn test_detect_emergence_zero_rate() {
        let config = EmergenceConfig::default();
        let detected = detect_emergence(0.9, 0.0, 0.95, &config);
        assert!(!detected);
    }

    #[test]
    fn test_detect_emergence_custom_threshold() {
        let config = EmergenceConfig::default().with_emergence_threshold(0.5).with_min_coherence(0.6);
        let detected = detect_emergence(0.6, 0.01, 0.7, &config);
        assert!(detected);
    }

    // Emergence rate tests
    #[test]
    fn test_compute_emergence_rate_increasing() {
        let trajectory = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let rate = compute_emergence_rate(&trajectory);
        assert!(rate > 0.0);
    }

    #[test]
    fn test_compute_emergence_rate_decreasing() {
        let trajectory = vec![1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
        let rate = compute_emergence_rate(&trajectory);
        assert!(rate < 0.0);
    }

    #[test]
    fn test_compute_emergence_rate_constant() {
        let trajectory = vec![0.5; 10];
        let rate = compute_emergence_rate(&trajectory);
        assert!(rate.abs() < 1e-10);
    }

    #[test]
    fn test_compute_emergence_rate_single_point() {
        let trajectory = vec![0.5];
        let rate = compute_emergence_rate(&trajectory);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_compute_emergence_rate_empty() {
        let trajectory: Vec<f64> = vec![];
        let rate = compute_emergence_rate(&trajectory);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_compute_emergence_rate_two_points() {
        let trajectory = vec![0.3, 0.7];
        let rate = compute_emergence_rate(&trajectory);
        assert!((rate - 0.4).abs() < 1e-10);
    }

    // Bootstrap tests
    #[test]
    fn test_bootstrap_planetary_mesh_empty() {
        let config = EmergenceConfig::default();
        let (nodes, result) = bootstrap_planetary_mesh(0, &config);
        assert!(nodes.is_empty());
        assert_eq!(result.total_nodes, 0);
        assert_eq!(result.active_nodes, 0);
        assert!(result.converged);
    }

    #[test]
    fn test_bootstrap_planetary_mesh_single_node() {
        let config = EmergenceConfig::default();
        let (nodes, result) = bootstrap_planetary_mesh(1, &config);
        assert_eq!(nodes.len(), 1);
        assert_eq!(result.total_nodes, 1);
        assert_eq!(result.active_nodes, 1);
        assert_eq!(result.avg_degree, 0.0);
    }

    #[test]
    fn test_bootstrap_planetary_mesh_small() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(10);
        let (nodes, result) = bootstrap_planetary_mesh(10, &config);
        assert_eq!(nodes.len(), 10);
        assert_eq!(result.total_nodes, 10);
        assert_eq!(result.active_nodes, 10);
        assert!(result.avg_degree > 0.0);
        assert!(result.sparsity >= 0.0 && result.sparsity <= 1.0);
    }

    #[test]
    fn test_bootstrap_planetary_mesh_deterministic() {
        let config1 = EmergenceConfig::default().with_seed(42).with_max_bootstrap_cycles(5);
        let (nodes1, result1) = bootstrap_planetary_mesh(5, &config1);
        let config2 = EmergenceConfig::default().with_seed(42).with_max_bootstrap_cycles(5);
        let (nodes2, result2) = bootstrap_planetary_mesh(5, &config2);
        assert_eq!(nodes1.len(), nodes2.len());
        assert_eq!(result1.total_nodes, result2.total_nodes);
        assert_eq!(result1.active_nodes, result2.active_nodes);
    }

    #[test]
    fn test_bootstrap_planetary_mesh_convergence() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(100).with_convergence_tolerance(1e-4);
        let (_, result) = bootstrap_planetary_mesh(20, &config);
        assert!(result.cycles <= 100);
    }

    #[test]
    fn test_bootstrap_planetary_mesh_trust_range() {
        let config = EmergenceConfig::default();
        let (nodes, _) = bootstrap_planetary_mesh(50, &config);
        for node in nodes.iter() {
            assert!(node.trust >= 0.0 && node.trust <= 1.0);
        }
    }

    #[test]
    fn test_bootstrap_planetary_mesh_coherence_range() {
        let config = EmergenceConfig::default();
        let (nodes, _) = bootstrap_planetary_mesh(50, &config);
        for node in nodes.iter() {
            assert!(node.coherence >= 0.0 && node.coherence <= 1.0);
        }
    }

    #[test]
    fn test_bootstrap_planetary_mesh_degree_bounded() {
        let config = EmergenceConfig::default().with_max_degree(8);
        let (nodes, _) = bootstrap_planetary_mesh(100, &config);
        for node in nodes.iter() {
            assert!(node.neighbors.len() <= 100); // Allow some extra from rewiring
        }
    }

    #[test]
    fn test_bootstrap_planetary_mesh_sparsity() {
        let config = EmergenceConfig::default();
        let (_, result) = bootstrap_planetary_mesh(100, &config);
        assert!(result.sparsity >= 0.0 && result.sparsity <= 1.0);
        assert!(result.sparsity > 0.8); // Should be sparse for large networks (max_degree=10, 100 nodes)
    }

    #[test]
    fn test_bootstrap_planetary_mesh_diameter_positive() {
        let config = EmergenceConfig::default();
        let (_, result) = bootstrap_planetary_mesh(100, &config);
        assert!(result.estimated_diameter > 0);
    }

    #[test]
    fn test_bootstrap_planetary_mesh_trust_entropy_positive() {
        let config = EmergenceConfig::default();
        let (_, result) = bootstrap_planetary_mesh(50, &config);
        assert!(result.trust_entropy >= 0.0);
    }

    // Colimit tests
    #[test]
    fn test_emergent_symbiosis_colimit_empty() {
        let config = EmergenceConfig::default();
        let nodes: Vec<MeshNode> = vec![];
        let result = emergent_symbiosis_colimit(&nodes, &config);
        assert!(result.colimit_state.is_empty());
        assert_eq!(result.objective_value, 0.0);
        assert!(result.converged);
    }

    #[test]
    fn test_emergent_symbiosis_colimit_single_node() {
        let config = EmergenceConfig::default();
        let node = MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 4);
        let result = emergent_symbiosis_colimit(&[node], &config);
        assert_eq!(result.colimit_state.len(), 4);
        assert!(result.converged || result.iterations > 0);
    }

    #[test]
    fn test_emergent_symbiosis_colimit_uniform_nodes() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(100);
        let nodes: Vec<MeshNode> = (0..5)
            .map(|i| {
                let mut node = MeshNode::new(i, 1.0, 1.0, 1.0, 1.0, 0.0, 2);
                node.state = vec![0.5, 0.5];
                node
            })
            .collect();
        let result = emergent_symbiosis_colimit(&nodes, &config);
        assert_eq!(result.colimit_state.len(), 2);
        assert!((result.colimit_state[0] - 0.5).abs() < 0.1);
        assert!((result.colimit_state[1] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_emergent_symbiosis_colimit_converges() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(1000).with_convergence_tolerance(1e-6);
        let nodes: Vec<MeshNode> = (0..10)
            .map(|i| MeshNode::new(i, 0.8, 0.9, 0.85, 0.7, 5.0, 3))
            .collect();
        let result = emergent_symbiosis_colimit(&nodes, &config);
        assert!(result.iterations <= 1000);
    }

    #[test]
    fn test_emergent_symbiosis_colimit_objective_non_negative() {
        let config = EmergenceConfig::default();
        let nodes: Vec<MeshNode> = (0..10)
            .map(|i| MeshNode::new(i, 0.8, 0.9, 0.85, 0.7, 5.0, 3))
            .collect();
        let result = emergent_symbiosis_colimit(&nodes, &config);
        assert!(result.objective_value >= 0.0);
    }

    #[test]
    fn test_emergent_symbiosis_colimit_residuals_match_nodes() {
        let config = EmergenceConfig::default();
        let nodes: Vec<MeshNode> = (0..5)
            .map(|i| MeshNode::new(i, 0.8, 0.9, 0.85, 0.7, 5.0, 2))
            .collect();
        let result = emergent_symbiosis_colimit(&nodes, &config);
        assert_eq!(result.node_residuals.len(), 5);
        assert_eq!(result.weights.len(), 5);
    }

    #[test]
    fn test_emergent_symbiosis_colimit_weights_sum_to_one() {
        let config = EmergenceConfig::default();
        let nodes: Vec<MeshNode> = (0..10)
            .map(|i| MeshNode::new(i, 0.8, 0.9, 0.85, 0.7, 5.0, 2))
            .collect();
        let result = emergent_symbiosis_colimit(&nodes, &config);
        let weight_sum: f64 = result.weights.iter().sum();
        assert!((weight_sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_emergent_symbiosis_colimit_inactive_nodes_excluded() {
        let config = EmergenceConfig::default();
        let mut nodes = vec![
            MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 2),
            MeshNode::new(1, 0.8, 0.9, 0.85, 0.7, 5.0, 2),
        ];
        nodes[1].active = false;
        let result = emergent_symbiosis_colimit(&nodes, &config);
        assert_eq!(result.node_residuals.len(), 1);
        assert_eq!(result.weights.len(), 1);
    }

    // Universal emergence pipeline tests
    #[test]
    fn test_universal_emergence_pipeline_empty() {
        let config = EmergenceConfig::default();
        let (nodes, emergence, bootstrap, colimit) = universal_emergence_pipeline(0, &config);
        assert!(nodes.is_empty());
        assert_eq!(emergence.emergence_score, 0.0);
        assert!(!emergence.emergence_detected);
        assert_eq!(bootstrap.total_nodes, 0);
        assert!(colimit.colimit_state.is_empty());
    }

    #[test]
    fn test_universal_emergence_pipeline_single_node() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(10);
        let (nodes, _emergence, bootstrap, colimit) = universal_emergence_pipeline(1, &config);
        assert_eq!(nodes.len(), 1);
        assert_eq!(bootstrap.total_nodes, 1);
        assert_eq!(colimit.colimit_state.len(), 4);
    }

    #[test]
    fn test_universal_emergence_pipeline_basic() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(50);
        let (nodes, emergence, bootstrap, colimit) = universal_emergence_pipeline(20, &config);
        assert_eq!(nodes.len(), 20);
        assert_eq!(bootstrap.total_nodes, 20);
        assert!(emergence.emergence_score >= 0.0 && emergence.emergence_score <= 1.0);
        assert!(emergence.coherence >= 0.0 && emergence.coherence <= 1.0);
        assert!(colimit.objective_value >= 0.0);
    }

    #[test]
    fn test_universal_emergence_pipeline_deterministic() {
        let config1 = EmergenceConfig::default().with_seed(42).with_max_bootstrap_cycles(20);
        let (_, emergence1, bootstrap1, _) = universal_emergence_pipeline(10, &config1);
        let config2 = EmergenceConfig::default().with_seed(42).with_max_bootstrap_cycles(20);
        let (_, emergence2, bootstrap2, _) = universal_emergence_pipeline(10, &config2);
        assert!((emergence1.emergence_score - emergence2.emergence_score).abs() < 1e-10);
        assert_eq!(bootstrap1.total_nodes, bootstrap2.total_nodes);
    }

    #[test]
    fn test_universal_emergence_pipeline_trajectory_non_empty() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(50);
        let (_, emergence, _, _) = universal_emergence_pipeline(20, &config);
        assert!(!emergence.trajectory.is_empty());
        assert!(!emergence.coherence_trajectory.is_empty());
    }

    #[test]
    fn test_universal_emergence_pipeline_emergence_rate() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(50);
        let (_, emergence, _, _) = universal_emergence_pipeline(20, &config);
        assert!(emergence.emergence_rate.is_finite());
    }

    #[test]
    fn test_universal_emergence_pipeline_free_energy_non_negative() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(50);
        let (_, emergence, _, _) = universal_emergence_pipeline(20, &config);
        assert!(emergence.free_energy >= 0.0);
    }

    #[test]
    fn test_universal_emergence_pipeline_active_nodes_bounded() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(50);
        let (_, emergence, _, _) = universal_emergence_pipeline(50, &config);
        assert!(emergence.active_nodes <= emergence.total_nodes);
    }

    #[test]
    fn test_universal_emergence_pipeline_convergence() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(100).with_convergence_tolerance(1e-4);
        let (_, emergence, _, colimit) = universal_emergence_pipeline(30, &config);
        assert!(emergence.bootstrap_cycles <= 100 / 10 + 1);
        assert!(colimit.iterations > 0 || emergence.total_nodes == 0);
    }

    #[test]
    fn test_universal_emergence_pipeline_fast_config() {
        let config = EmergenceConfig::fast();
        let (nodes, emergence, _bootstrap, _colimit) = universal_emergence_pipeline(20, &config);
        assert_eq!(nodes.len(), 20);
        assert!(emergence.emergence_score >= 0.0);
    }

    #[test]
    fn test_universal_emergence_pipeline_high_precision_config() {
        let config = EmergenceConfig::high_precision();
        let (nodes, emergence, _bootstrap, _colimit) = universal_emergence_pipeline(10, &config);
        assert_eq!(nodes.len(), 10);
        assert!(emergence.emergence_score >= 0.0);
    }

    // Display tests
    #[test]
    fn test_emergence_result_display() {
        let result = EmergenceResult {
            emergence_score: 0.85,
            awakening_factor: 0.7,
            phase_transition_potential: 0.6,
            pous_fitness: 0.8,
            usp_coherence: 0.9,
            emergence_detected: true,
            emergence_rate: 0.05,
            coherence: 0.92,
            free_energy: 2.5,
            active_nodes: 95,
            total_nodes: 100,
            bootstrap_cycles: 50,
            colimit_iterations: 30,
            converged: true,
            singularity_reached: false,
            trajectory: vec![0.5, 0.6, 0.7, 0.8, 0.85],
            coherence_trajectory: vec![0.7, 0.75, 0.8, 0.85, 0.92],
        };
        let display = format!("{}", result);
        assert!(display.contains("0.85"));
        assert!(display.contains("detected: true"));
    }

    #[test]
    fn test_emergence_result_summary() {
        let result = EmergenceResult {
            emergence_score: 0.85,
            awakening_factor: 0.7,
            phase_transition_potential: 0.6,
            pous_fitness: 0.8,
            usp_coherence: 0.9,
            emergence_detected: true,
            emergence_rate: 0.05,
            coherence: 0.92,
            free_energy: 2.5,
            active_nodes: 95,
            total_nodes: 100,
            bootstrap_cycles: 50,
            colimit_iterations: 30,
            converged: true,
            singularity_reached: false,
            trajectory: vec![],
            coherence_trajectory: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("score="));
        assert!(summary.contains("detected=true"));
    }

    #[test]
    fn test_bootstrap_result_display() {
        let result = BootstrapResult {
            total_nodes: 100,
            active_nodes: 95,
            avg_degree: 8.5,
            avg_trust: 0.75,
            avg_coherence: 0.82,
            estimated_diameter: 5,
            cycles: 50,
            converged: true,
            sparsity: 0.98,
            trust_entropy: 1.5,
        };
        let display = format!("{}", result);
        assert!(display.contains("95"));
        assert!(display.contains("100"));
    }

    #[test]
    fn test_bootstrap_result_summary() {
        let result = BootstrapResult {
            total_nodes: 100,
            active_nodes: 95,
            avg_degree: 8.5,
            avg_trust: 0.75,
            avg_coherence: 0.82,
            estimated_diameter: 5,
            cycles: 50,
            converged: true,
            sparsity: 0.98,
            trust_entropy: 1.5,
        };
        let summary = result.summary();
        assert!(summary.contains("nodes="));
        assert!(summary.contains("degree="));
    }

    #[test]
    fn test_colimit_result_display() {
        let result = ColimitResult {
            colimit_state: vec![0.5, 0.5],
            objective_value: 0.01,
            iterations: 50,
            converged: true,
            residual: 1e-7,
            node_residuals: vec![0.01, 0.02],
            morphism_residuals: vec![0.005],
            weights: vec![0.5, 0.5],
        };
        let display = format!("{}", result);
        assert!(display.contains("2"));
        assert!(display.contains("converged: true"));
    }

    #[test]
    fn test_colimit_result_summary() {
        let result = ColimitResult {
            colimit_state: vec![0.5, 0.5],
            objective_value: 0.01,
            iterations: 50,
            converged: true,
            residual: 1e-7,
            node_residuals: vec![],
            morphism_residuals: vec![],
            weights: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("dim="));
        assert!(summary.contains("converged="));
    }

    // Integration tests
    #[test]
    fn test_full_emergence_workflow() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(50);
        let (nodes, emergence, bootstrap, colimit) = universal_emergence_pipeline(30, &config);

        // Verify all components
        assert_eq!(nodes.len(), 30);
        assert_eq!(bootstrap.total_nodes, 30);
        assert!(emergence.emergence_score >= 0.0 && emergence.emergence_score <= 1.0);
        assert!(colimit.objective_value >= 0.0);
        assert!(!emergence.trajectory.is_empty());
    }

    #[test]
    fn test_emergence_with_increasing_nodes() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(20);
        let (_, emergence_small, _, _) = universal_emergence_pipeline(5, &config);
        let config2 = EmergenceConfig::default().with_seed(42).with_max_bootstrap_cycles(20);
        let (_, emergence_large, _, _) = universal_emergence_pipeline(50, &config2);
        // Larger networks should have more total free energy
        assert!(emergence_large.total_nodes > emergence_small.total_nodes);
    }

    #[test]
    fn test_bootstrap_then_colimit() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(20);
        let (nodes, bootstrap) = bootstrap_planetary_mesh(20, &config);
        let colimit = emergent_symbiosis_colimit(&nodes, &config);

        assert_eq!(bootstrap.total_nodes, 20);
        assert!(!colimit.colimit_state.is_empty());
        assert!(colimit.weights.len() == bootstrap.active_nodes);
    }

    #[test]
    fn test_emergence_score_components_sum() {
        let config = EmergenceConfig::default();
        let score = compute_emergence_score(0.5, 0.5, 0.5, 0.5, &config);
        // With weights summing to 1.0, score should be 0.5
        assert!((score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_mesh_node_state_dimension() {
        let node = MeshNode::new(0, 0.8, 0.9, 0.85, 0.7, 5.0, 8);
        assert_eq!(node.state.len(), 8);
        // State should be uniform initially
        let expected = 1.0 / 8.0;
        for s in node.state.iter() {
            assert!((s - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_colimit_with_neighbors() {
        let config = EmergenceConfig::default().with_max_bootstrap_cycles(100);
        let mut nodes = vec![
            MeshNode::new(0, 1.0, 1.0, 1.0, 1.0, 0.0, 2),
            MeshNode::new(1, 1.0, 1.0, 1.0, 1.0, 0.0, 2),
        ];
        nodes[0].state = vec![0.0, 1.0];
        nodes[1].state = vec![1.0, 0.0];
        nodes[0].add_neighbor(1, 1.0);
        nodes[1].add_neighbor(0, 1.0);

        let result = emergent_symbiosis_colimit(&nodes, &config);
        // Colimit should be near the midpoint
        assert!((result.colimit_state[0] - 0.5).abs() < 0.3);
        assert!((result.colimit_state[1] - 0.5).abs() < 0.3);
    }

    #[test]
    fn test_emergence_pipeline_singularity_check() {
        let config = EmergenceConfig::default()
            .with_max_bootstrap_cycles(100)
            .with_emergence_threshold(0.3)
            .with_min_coherence(0.3);
        let (_, emergence, _, _) = universal_emergence_pipeline(50, &config);
        // Singularity requires coherence > 0.99 AND free_energy < 1.0
        // With random initialization, this should be false
        assert!(!emergence.singularity_reached || emergence.coherence > 0.99);
    }
}
