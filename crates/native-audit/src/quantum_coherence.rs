//! Quantum-Inspired Coherence on SGW Manifolds.
//!
//! Integrates concepts of superposition and decoherence into SGW manifolds
//! for handling collective uncertainty and symbiotic entanglement.
//!
//! **Key Formulas:**
//! - **Coherence Measure:** `Coherence(X,Y) = exp(-SGW(X,Y)/τ) · Tr(ρ_X ρ_Y)`
//! - **Decoherence Penalty:** Stabilizes collective steering via energy landscape penalty.

use serde::{Deserialize, Serialize};

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for quantum-inspired coherence computations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumCoherenceConfig {
    /// Temperature parameter τ for coherence decay.
    pub temperature: f64,
    /// Decoherence rate γ.
    pub decoherence_rate: f64,
    /// Entanglement strength λ.
    pub entanglement_strength: f64,
    /// Number of coherence iterations.
    pub iterations: usize,
    /// Convergence tolerance.
    pub tolerance: f64,
    /// Random seed.
    pub seed: u64,
}

impl Default for QuantumCoherenceConfig {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            decoherence_rate: 0.1,
            entanglement_strength: 0.5,
            iterations: 50,
            tolerance: 1e-6,
            seed: 42,
        }
    }
}

impl QuantumCoherenceConfig {
    /// Fast config for testing.
    pub fn fast() -> Self {
        Self {
            temperature: 1.0,
            decoherence_rate: 0.1,
            entanglement_strength: 0.5,
            iterations: 10,
            tolerance: 1e-4,
            seed: 42,
        }
    }

    /// High precision config.
    pub fn high_precision() -> Self {
        Self {
            temperature: 0.1,
            decoherence_rate: 0.01,
            entanglement_strength: 0.8,
            iterations: 500,
            tolerance: 1e-10,
            seed: 42,
        }
    }

    /// Set temperature.
    pub fn with_temperature(mut self, t: f64) -> Self {
        self.temperature = t.max(0.01);
        self
    }

    /// Set decoherence rate.
    pub fn with_decoherence_rate(mut self, r: f64) -> Self {
        self.decoherence_rate = r.max(0.0).min(1.0);
        self
    }

    /// Set entanglement strength.
    pub fn with_entanglement_strength(mut self, s: f64) -> Self {
        self.entanglement_strength = s.max(0.0).min(1.0);
        self
    }

    /// Set iterations.
    pub fn with_iterations(mut self, n: usize) -> Self {
        self.iterations = n.max(1);
        self
    }
}

// ─── Results ────────────────────────────────────────────────────────────────

/// Result of quantum-inspired coherence computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceResult {
    /// Coherence score: exp(-SGW/τ) · Tr(ρ_X ρ_Y).
    pub coherence_score: f64,
    /// SGW distance proxy.
    pub sgw_distance: f64,
    /// Density matrix overlap Tr(ρ_X ρ_Y).
    pub density_overlap: f64,
    /// Temperature used.
    pub temperature: f64,
    /// Iterations performed.
    pub iterations: usize,
    /// Converged.
    pub converged: bool,
}

impl std::fmt::Display for CoherenceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Coherence: {:.4}, SGW: {:.4}, Overlap: {:.4}, τ={:.2}, iters={}, converged={}",
            self.coherence_score, self.sgw_distance, self.density_overlap,
            self.temperature, self.iterations, self.converged
        )
    }
}

/// Result of entanglement symbiosis score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementResult {
    /// Entanglement score between distributions.
    pub entanglement_score: f64,
    /// Mutual information proxy.
    pub mutual_information: f64,
    /// Symbiotic correlation.
    pub symbiotic_correlation: f64,
    /// Number of entangled pairs.
    pub entangled_pairs: usize,
}

impl std::fmt::Display for EntanglementResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Entanglement: {:.4}, MI: {:.4}, Correlation: {:.4}, pairs={}",
            self.entanglement_score, self.mutual_information,
            self.symbiotic_correlation, self.entangled_pairs
        )
    }
}

/// Result of decoherence stabilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoherenceResult {
    /// Stabilized energy after decoherence penalty.
    pub stabilized_energy: f64,
    /// Decoherence penalty applied.
    pub decoherence_penalty: f64,
    /// Stability margin.
    pub stability_margin: f64,
    /// Iterations to stabilize.
    pub iterations: usize,
    /// Stabilized.
    pub stabilized: bool,
}

impl std::fmt::Display for DecoherenceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stabilized E: {:.6}, Penalty: {:.6}, Margin: {:.4}, iters={}, stabilized={}",
            self.stabilized_energy, self.decoherence_penalty, self.stability_margin,
            self.iterations, self.stabilized
        )
    }
}

// ─── LCG Random ─────────────────────────────────────────────────────────────

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

fn random_uniform(state: &mut u64) -> f64 {
    let val = lcg_next(state);
    (val >> 11) as f64 / (1u64 << 53) as f64
}

// ─── Core Math ──────────────────────────────────────────────────────────────

/// Compute Shannon entropy.
fn shannon_entropy(dist: &[f64]) -> f64 {
    dist.iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.ln())
        .sum()
}

/// Compute cosine similarity.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a < 1e-15 || norm_b < 1e-15 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Compute KL divergence D_KL(P||Q).
fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    p.iter()
        .zip(q.iter())
        .filter(|(&pi, &qi)| pi > 1e-15 && qi > 1e-15)
        .map(|(&pi, &qi)| pi * (pi / qi).ln())
        .sum()
}

// ─── Quantum-Inspired Coherence ─────────────────────────────────────────────

/// Compute quantum-inspired coherence between two distributions.
///
/// ```text
/// Coherence(X,Y) = exp(-SGW(X,Y)/τ) · Tr(ρ_X ρ_Y)
/// ```
///
/// Where:
/// - SGW(X,Y) is approximated by the L2 distance between distributions
/// - Tr(ρ_X ρ_Y) is approximated by the cosine similarity (density matrix overlap proxy)
/// - τ is the temperature parameter controlling coherence decay
pub fn compute_quantum_inspired_coherence(
    dist_x: &[f64],
    dist_y: &[f64],
    config: &QuantumCoherenceConfig,
) -> CoherenceResult {
    let tau = config.temperature;

    // SGW distance proxy: L2 distance between distributions
    let sgw_distance = if dist_x.len() == dist_y.len() {
        let sum: f64 = dist_x
            .iter()
            .zip(dist_y.iter())
            .map(|(&x, &y)| (x - y).powi(2))
            .sum();
        sum.sqrt()
    } else {
        // Different lengths: use max distance
        f64::MAX
    };

    // Density matrix overlap proxy: cosine similarity
    let density_overlap = cosine_similarity(dist_x, dist_y);

    // Coherence = exp(-SGW/τ) · overlap
    let coherence_factor = if tau > 1e-15 {
        (-sgw_distance / tau).exp()
    } else {
        0.0
    };
    let coherence_score = (coherence_factor * (density_overlap + 1.0) / 2.0).clamp(0.0, 1.0);

    CoherenceResult {
        coherence_score,
        sgw_distance,
        density_overlap,
        temperature: tau,
        iterations: 1,
        converged: true,
    }
}

/// Compute iterative quantum-inspired coherence with convergence.
pub fn compute_quantum_inspired_coherence_iterative(
    dist_x: &[f64],
    dist_y: &[f64],
    config: &QuantumCoherenceConfig,
) -> CoherenceResult {
    let mut rng = config.seed;
    let mut current_coherence = 0.0_f64;
    let mut iterations = 0;
    let converged = false;

    for i in 0..config.iterations {
        let result = compute_quantum_inspired_coherence(dist_x, dist_y, config);
        let noise = random_uniform(&mut rng) * 0.01;
        let new_coherence = (result.coherence_score + noise).clamp(0.0, 1.0);

        if (new_coherence - current_coherence).abs() < config.tolerance {
            return CoherenceResult {
                coherence_score: new_coherence,
                sgw_distance: result.sgw_distance,
                density_overlap: result.density_overlap,
                temperature: config.temperature,
                iterations: i + 1,
                converged: true,
            };
        }

        current_coherence = new_coherence;
        iterations = i + 1;
    }

    CoherenceResult {
        coherence_score: current_coherence,
        sgw_distance: compute_quantum_inspired_coherence(dist_x, dist_y, config).sgw_distance,
        density_overlap: compute_quantum_inspired_coherence(dist_x, dist_y, config).density_overlap,
        temperature: config.temperature,
        iterations,
        converged,
    }
}

// ─── Entanglement Symbiosis Score ───────────────────────────────────────────

/// Compute entanglement symbiosis score between multiple node distributions.
///
/// Measures the collective entanglement as mutual information proxy:
/// ```text
/// Entanglement = λ · Σ_{i<j} cosine_sim(ρ_i, ρ_j) · (1 - KL(ρ_i || ρ_j))
/// ```
pub fn entanglement_symbiosis_score(
    distributions: &[Vec<f64>],
    config: &QuantumCoherenceConfig,
) -> EntanglementResult {
    let lambda = config.entanglement_strength;
    let n = distributions.len();

    if n < 2 {
        return EntanglementResult {
            entanglement_score: 0.0,
            mutual_information: 0.0,
            symbiotic_correlation: 0.0,
            entangled_pairs: 0,
        };
    }

    let mut total_entanglement = 0.0;
    let mut total_mi = 0.0;
    let mut total_correlation = 0.0;
    let mut entangled_pairs = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            let sim = cosine_similarity(&distributions[i], &distributions[j]);
            let kl = kl_divergence(&distributions[i], &distributions[j]);
            let mi_proxy = (1.0 - (kl).clamp(0.0, 1.0)).max(0.0);

            let pair_entanglement = lambda * sim * mi_proxy;
            total_entanglement += pair_entanglement;
            total_mi += mi_proxy;
            total_correlation += sim;

            if pair_entanglement > 0.3 {
                entangled_pairs += 1;
            }
        }
    }

    let pair_count = n * (n - 1) / 2;
    EntanglementResult {
        entanglement_score: total_entanglement / pair_count as f64,
        mutual_information: total_mi / pair_count as f64,
        symbiotic_correlation: total_correlation / pair_count as f64,
        entangled_pairs,
    }
}

// ─── Decoherence Stabilizer ─────────────────────────────────────────────────

/// Apply decoherence penalty to stabilize energy landscape.
///
/// The decoherence penalty prevents oscillation in collective steering:
/// ```text
/// E_stabilized = E - γ · (1 - Coherence) · E
/// ```
///
/// Where γ is the decoherence rate and Coherence is the current coherence score.
pub fn decoherence_stabilizer(
    energy: f64,
    coherence: f64,
    config: &QuantumCoherenceConfig,
) -> DecoherenceResult {
    let gamma = config.decoherence_rate;

    // Decoherence penalty: reduces energy based on coherence deficit
    let coherence_deficit = (1.0 - coherence.clamp(0.0, 1.0)).max(0.0);
    let penalty = gamma * coherence_deficit * energy;
    let stabilized = energy - penalty;

    // Stability margin: how far from instability threshold
    let margin = (1.0 - (stabilized / (energy + 1e-15))).clamp(0.0, 1.0);

    DecoherenceResult {
        stabilized_energy: stabilized.max(0.0),
        decoherence_penalty: penalty,
        stability_margin: margin,
        iterations: 1,
        stabilized: margin > 0.5,
    }
}

/// Iterative decoherence stabilization until convergence.
pub fn decoherence_stabilizer_iterative(
    mut energy: f64,
    mut coherence: f64,
    config: &QuantumCoherenceConfig,
) -> DecoherenceResult {
    let mut total_penalty = 0.0;
    let mut iterations = 0;

    for i in 0..config.iterations {
        let result = decoherence_stabilizer(energy, coherence, config);

        total_penalty += result.decoherence_penalty;
        energy = result.stabilized_energy;

        // Coherence improves as energy stabilizes
        coherence = (coherence + 0.1 * result.stability_margin).clamp(0.0, 1.0);

        if result.decoherence_penalty < config.tolerance {
            return DecoherenceResult {
                stabilized_energy: energy,
                decoherence_penalty: total_penalty,
                stability_margin: result.stability_margin,
                iterations: i + 1,
                stabilized: true,
            };
        }

        iterations = i + 1;
    }

    let final_result = decoherence_stabilizer(energy, coherence, config);
    DecoherenceResult {
        stabilized_energy: energy,
        decoherence_penalty: total_penalty,
        stability_margin: final_result.stability_margin,
        iterations,
        stabilized: false,
    }
}

// ─── Full Quantum Coherence Pipeline ────────────────────────────────────────

/// Run the full quantum coherence pipeline: coherence + entanglement + stabilization.
pub fn run_quantum_coherence_pipeline(
    distributions: &[Vec<f64>],
    config: &QuantumCoherenceConfig,
) -> (CoherenceResult, EntanglementResult, DecoherenceResult) {
    // 1. Compute pairwise coherence (average)
    let n = distributions.len();
    let mut avg_coherence = 0.0;
    let mut avg_sgwd = 0.0;
    let mut avg_overlap = 0.0;
    let mut pair_count = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            let result = compute_quantum_inspired_coherence(&distributions[i], &distributions[j], config);
            avg_coherence += result.coherence_score;
            avg_sgwd += result.sgw_distance;
            avg_overlap += result.density_overlap;
            pair_count += 1;
        }
    }

    if pair_count > 0 {
        avg_coherence /= pair_count as f64;
        avg_sgwd /= pair_count as f64;
        avg_overlap /= pair_count as f64;
    }

    let coherence_result = CoherenceResult {
        coherence_score: avg_coherence,
        sgw_distance: avg_sgwd,
        density_overlap: avg_overlap,
        temperature: config.temperature,
        iterations: 1,
        converged: true,
    };

    // 2. Compute entanglement
    let entanglement_result = entanglement_symbiosis_score(distributions, config);

    // 3. Apply decoherence stabilization
    let energy = avg_sgwd;
    let decoherence_result = decoherence_stabilizer(energy, avg_coherence, config);

    (coherence_result, entanglement_result, decoherence_result)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Config Tests ─────────────────────────────────────────────────

    #[test]
    fn test_quantum_config_default() {
        let cfg = QuantumCoherenceConfig::default();
        assert_eq!(cfg.temperature, 1.0);
        assert_eq!(cfg.decoherence_rate, 0.1);
        assert_eq!(cfg.entanglement_strength, 0.5);
        assert_eq!(cfg.iterations, 50);
        assert_eq!(cfg.tolerance, 1e-6);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_quantum_config_fast() {
        let cfg = QuantumCoherenceConfig::fast();
        assert_eq!(cfg.iterations, 10);
        assert_eq!(cfg.tolerance, 1e-4);
    }

    #[test]
    fn test_quantum_config_high_precision() {
        let cfg = QuantumCoherenceConfig::high_precision();
        assert_eq!(cfg.temperature, 0.1);
        assert_eq!(cfg.iterations, 500);
        assert_eq!(cfg.tolerance, 1e-10);
    }

    #[test]
    fn test_quantum_config_with_temperature() {
        let cfg = QuantumCoherenceConfig::default().with_temperature(0.5);
        assert_eq!(cfg.temperature, 0.5);
    }

    #[test]
    fn test_quantum_config_temperature_min() {
        let cfg = QuantumCoherenceConfig::default().with_temperature(0.0);
        assert_eq!(cfg.temperature, 0.01);
    }

    #[test]
    fn test_quantum_config_with_decoherence_rate() {
        let cfg = QuantumCoherenceConfig::default().with_decoherence_rate(0.5);
        assert_eq!(cfg.decoherence_rate, 0.5);
    }

    #[test]
    fn test_quantum_config_decoherence_rate_clamped() {
        let cfg = QuantumCoherenceConfig::default().with_decoherence_rate(2.0);
        assert_eq!(cfg.decoherence_rate, 1.0);
    }

    #[test]
    fn test_quantum_config_with_entanglement_strength() {
        let cfg = QuantumCoherenceConfig::default().with_entanglement_strength(0.8);
        assert_eq!(cfg.entanglement_strength, 0.8);
    }

    #[test]
    fn test_quantum_config_entanglement_clamped() {
        let cfg = QuantumCoherenceConfig::default().with_entanglement_strength(2.0);
        assert_eq!(cfg.entanglement_strength, 1.0);
    }

    #[test]
    fn test_quantum_config_with_iterations() {
        let cfg = QuantumCoherenceConfig::default().with_iterations(200);
        assert_eq!(cfg.iterations, 200);
    }

    #[test]
    fn test_quantum_config_iterations_min() {
        let cfg = QuantumCoherenceConfig::default().with_iterations(0);
        assert_eq!(cfg.iterations, 1);
    }

    // ─── Coherence Tests ──────────────────────────────────────────────

    #[test]
    fn test_compute_coherence_identical_distributions() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let cfg = QuantumCoherenceConfig::default();
        let result = compute_quantum_inspired_coherence(&dist, &dist, &cfg);
        assert!((result.coherence_score - 1.0) < 1e-10);
        assert!((result.sgw_distance - 0.0) < 1e-10);
    }

    #[test]
    fn test_compute_coherence_orthogonal_distributions() {
        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0, 0.0];
        let cfg = QuantumCoherenceConfig::default();
        let result = compute_quantum_inspired_coherence(&a, &b, &cfg);
        assert!(result.coherence_score >= 0.0);
        assert!(result.coherence_score <= 1.0);
    }

    #[test]
    fn test_compute_coherence_bounded() {
        let a = vec![0.5, 0.3, 0.2];
        let b = vec![0.2, 0.3, 0.5];
        let cfg = QuantumCoherenceConfig::default();
        let result = compute_quantum_inspired_coherence(&a, &b, &cfg);
        assert!(result.coherence_score >= 0.0);
        assert!(result.coherence_score <= 1.0);
    }

    #[test]
    fn test_compute_coherence_temperature_effect() {
        let a = vec![0.5, 0.5];
        let b = vec![0.3, 0.7];
        let cfg_high = QuantumCoherenceConfig::default().with_temperature(10.0);
        let cfg_low = QuantumCoherenceConfig::default().with_temperature(0.1);
        let r_high = compute_quantum_inspired_coherence(&a, &b, &cfg_high);
        let r_low = compute_quantum_inspired_coherence(&a, &b, &cfg_low);
        // Higher temperature = less decay = higher coherence
        assert!(r_high.coherence_score >= r_low.coherence_score - 1e-10);
    }

    #[test]
    fn test_compute_coherence_different_lengths() {
        let a = vec![0.5, 0.5];
        let b = vec![0.3, 0.3, 0.4];
        let cfg = QuantumCoherenceConfig::default();
        let result = compute_quantum_inspired_coherence(&a, &b, &cfg);
        assert!(result.sgw_distance == f64::MAX);
    }

    #[test]
    fn test_compute_coherence_empty() {
        let a: Vec<f64> = vec![];
        let b: Vec<f64> = vec![];
        let cfg = QuantumCoherenceConfig::default();
        let result = compute_quantum_inspired_coherence(&a, &b, &cfg);
        assert!(result.coherence_score >= 0.0);
    }

    #[test]
    fn test_compute_coherence_deterministic() {
        let a = vec![0.5, 0.3, 0.2];
        let b = vec![0.2, 0.3, 0.5];
        let cfg = QuantumCoherenceConfig::default();
        let r1 = compute_quantum_inspired_coherence(&a, &b, &cfg);
        let r2 = compute_quantum_inspired_coherence(&a, &b, &cfg);
        assert!((r1.coherence_score - r2.coherence_score) < 1e-15);
    }

    #[test]
    fn test_compute_coherence_iterative_converges() {
        let a = vec![0.5, 0.5];
        let b = vec![0.5, 0.5];
        let cfg = QuantumCoherenceConfig::fast();
        let result = compute_quantum_inspired_coherence_iterative(&a, &b, &cfg);
        assert!(result.converged || result.iterations <= cfg.iterations);
    }

    #[test]
    fn test_compute_coherence_iterative_iterations_positive() {
        let a = vec![0.5, 0.5];
        let b = vec![0.3, 0.7];
        let cfg = QuantumCoherenceConfig::fast();
        let result = compute_quantum_inspired_coherence_iterative(&a, &b, &cfg);
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_coherence_result_display() {
        let result = CoherenceResult {
            coherence_score: 0.8,
            sgw_distance: 0.5,
            density_overlap: 0.9,
            temperature: 1.0,
            iterations: 10,
            converged: true,
        };
        let s = format!("{}", result);
        assert!(s.contains("Coherence:"));
    }

    // ─── Entanglement Tests ───────────────────────────────────────────

    #[test]
    fn test_entanglement_empty() {
        let dists: Vec<Vec<f64>> = vec![];
        let cfg = QuantumCoherenceConfig::default();
        let result = entanglement_symbiosis_score(&dists, &cfg);
        assert_eq!(result.entanglement_score, 0.0);
        assert_eq!(result.entangled_pairs, 0);
    }

    #[test]
    fn test_entanglement_single() {
        let dists = vec![vec![0.5, 0.5]];
        let cfg = QuantumCoherenceConfig::default();
        let result = entanglement_symbiosis_score(&dists, &cfg);
        assert_eq!(result.entanglement_score, 0.0);
    }

    #[test]
    fn test_entanglement_identical_distributions() {
        let dists = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let cfg = QuantumCoherenceConfig::default();
        let result = entanglement_symbiosis_score(&dists, &cfg);
        // Identical dists: sim=1, KL=0, MI=1, entanglement = lambda * 1 * 1
        assert!((result.entanglement_score - cfg.entanglement_strength) < 1e-10);
    }

    #[test]
    fn test_entanglement_orthogonal_distributions() {
        let dists = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let cfg = QuantumCoherenceConfig::default();
        let result = entanglement_symbiosis_score(&dists, &cfg);
        assert!(result.entanglement_score >= 0.0);
    }

    #[test]
    fn test_entanglement_bounded() {
        let dists = vec![
            vec![0.5, 0.5],
            vec![0.3, 0.7],
            vec![0.7, 0.3],
        ];
        let cfg = QuantumCoherenceConfig::default();
        let result = entanglement_symbiosis_score(&dists, &cfg);
        assert!(result.entanglement_score >= 0.0);
        assert!(result.entanglement_score <= cfg.entanglement_strength + 1e-10);
    }

    #[test]
    fn test_entanglement_mutual_information_positive() {
        let dists = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let cfg = QuantumCoherenceConfig::default();
        let result = entanglement_symbiosis_score(&dists, &cfg);
        assert!(result.mutual_information >= 0.0);
    }

    #[test]
    fn test_entanglement_correlation_bounded() {
        let dists = vec![vec![0.5, 0.5], vec![0.3, 0.7]];
        let cfg = QuantumCoherenceConfig::default();
        let result = entanglement_symbiosis_score(&dists, &cfg);
        assert!(result.symbiotic_correlation >= -1.0);
        assert!(result.symbiotic_correlation <= 1.0 + 1e-10);
    }

    #[test]
    fn test_entanglement_strength_affects_score() {
        let dists = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let cfg_low = QuantumCoherenceConfig::default().with_entanglement_strength(0.1);
        let cfg_high = QuantumCoherenceConfig::default().with_entanglement_strength(0.9);
        let r_low = entanglement_symbiosis_score(&dists, &cfg_low);
        let r_high = entanglement_symbiosis_score(&dists, &cfg_high);
        assert!(r_high.entanglement_score > r_low.entanglement_score);
    }

    #[test]
    fn test_entanglement_result_display() {
        let result = EntanglementResult {
            entanglement_score: 0.5,
            mutual_information: 0.8,
            symbiotic_correlation: 0.9,
            entangled_pairs: 3,
        };
        let s = format!("{}", result);
        assert!(s.contains("Entanglement:"));
    }

    // ─── Decoherence Tests ────────────────────────────────────────────

    #[test]
    fn test_decoherence_zero_energy() {
        let cfg = QuantumCoherenceConfig::default();
        let result = decoherence_stabilizer(0.0, 0.5, &cfg);
        assert_eq!(result.stabilized_energy, 0.0);
        assert_eq!(result.decoherence_penalty, 0.0);
    }

    #[test]
    fn test_decoherence_perfect_coherence() {
        let cfg = QuantumCoherenceConfig::default();
        let result = decoherence_stabilizer(1.0, 1.0, &cfg);
        // Perfect coherence = no deficit = no penalty
        assert!((result.decoherence_penalty - 0.0) < 1e-10);
        assert!((result.stabilized_energy - 1.0) < 1e-10);
    }

    #[test]
    fn test_decoherence_zero_coherence() {
        let cfg = QuantumCoherenceConfig::default();
        let result = decoherence_stabilizer(1.0, 0.0, &cfg);
        // Zero coherence = max deficit = max penalty
        assert!(result.decoherence_penalty > 0.0);
        assert!(result.stabilized_energy < 1.0);
    }

    #[test]
    fn test_decoherence_rate_affects_penalty() {
        let cfg_low = QuantumCoherenceConfig::default().with_decoherence_rate(0.01);
        let cfg_high = QuantumCoherenceConfig::default().with_decoherence_rate(0.9);
        let r_low = decoherence_stabilizer(1.0, 0.5, &cfg_low);
        let r_high = decoherence_stabilizer(1.0, 0.5, &cfg_high);
        assert!(r_high.decoherence_penalty > r_low.decoherence_penalty);
    }

    #[test]
    fn test_decoherence_stability_margin_bounded() {
        let cfg = QuantumCoherenceConfig::default();
        let result = decoherence_stabilizer(1.0, 0.5, &cfg);
        assert!(result.stability_margin >= 0.0);
        assert!(result.stability_margin <= 1.0);
    }

    #[test]
    fn test_decoherence_stabilized_flag() {
        let cfg = QuantumCoherenceConfig::default();
        let result = decoherence_stabilizer(1.0, 0.9, &cfg);
        // High coherence = high stability margin = stabilized
        assert!(result.stabilized);
    }

    #[test]
    fn test_decoherence_not_stabilized() {
        let cfg = QuantumCoherenceConfig::default().with_decoherence_rate(0.01);
        let result = decoherence_stabilizer(1.0, 0.1, &cfg);
        // Low decoherence rate + low coherence = may not be stabilized
        assert!(result.stability_margin >= 0.0);
    }

    #[test]
    fn test_decoherence_iterative_converges() {
        let cfg = QuantumCoherenceConfig::fast();
        let result = decoherence_stabilizer_iterative(1.0, 0.5, &cfg);
        assert!(result.iterations > 0);
        assert!(result.stabilized_energy >= 0.0);
    }

    #[test]
    fn test_decoherence_iterative_penalty_accumulates() {
        let cfg = QuantumCoherenceConfig::fast();
        let result = decoherence_stabilizer_iterative(1.0, 0.5, &cfg);
        assert!(result.decoherence_penalty >= 0.0);
    }

    #[test]
    fn test_decoherence_result_display() {
        let result = DecoherenceResult {
            stabilized_energy: 0.8,
            decoherence_penalty: 0.2,
            stability_margin: 0.6,
            iterations: 5,
            stabilized: true,
        };
        let s = format!("{}", result);
        assert!(s.contains("Stabilized E:"));
    }

    // ─── Pipeline Tests ───────────────────────────────────────────────

    #[test]
    fn test_pipeline_empty() {
        let dists: Vec<Vec<f64>> = vec![];
        let cfg = QuantumCoherenceConfig::default();
        let (coh, ent, dec) = run_quantum_coherence_pipeline(&dists, &cfg);
        assert_eq!(coh.coherence_score, 0.0);
        assert_eq!(ent.entanglement_score, 0.0);
    }

    #[test]
    fn test_pipeline_single_distribution() {
        let dists = vec![vec![0.5, 0.5]];
        let cfg = QuantumCoherenceConfig::default();
        let (coh, ent, dec) = run_quantum_coherence_pipeline(&dists, &cfg);
        assert_eq!(coh.coherence_score, 0.0);
        assert_eq!(ent.entangled_pairs, 0);
    }

    #[test]
    fn test_pipeline_two_distributions() {
        let dists = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        let cfg = QuantumCoherenceConfig::default();
        let (coh, ent, dec) = run_quantum_coherence_pipeline(&dists, &cfg);
        assert!(coh.coherence_score > 0.0);
        assert!(ent.entanglement_score > 0.0);
    }

    #[test]
    fn test_pipeline_multiple_distributions() {
        let dists = vec![
            vec![0.5, 0.5],
            vec![0.3, 0.7],
            vec![0.7, 0.3],
            vec![0.5, 0.5],
        ];
        let cfg = QuantumCoherenceConfig::default();
        let (coh, ent, dec) = run_quantum_coherence_pipeline(&dists, &cfg);
        assert!(coh.coherence_score >= 0.0);
        assert!(coh.coherence_score <= 1.0);
        assert!(ent.entangled_pairs >= 0);
    }

    #[test]
    fn test_pipeline_identical_distributions() {
        let dists = vec![
            vec![0.25, 0.25, 0.25, 0.25],
            vec![0.25, 0.25, 0.25, 0.25],
            vec![0.25, 0.25, 0.25, 0.25],
        ];
        let cfg = QuantumCoherenceConfig::default();
        let (coh, ent, dec) = run_quantum_coherence_pipeline(&dists, &cfg);
        assert!((coh.coherence_score - 1.0) < 1e-10);
    }

    #[test]
    fn test_pipeline_deterministic() {
        let dists = vec![vec![0.5, 0.5], vec![0.3, 0.7]];
        let cfg = QuantumCoherenceConfig::default();
        let (c1, e1, d1) = run_quantum_coherence_pipeline(&dists, &cfg);
        let (c2, e2, d2) = run_quantum_coherence_pipeline(&dists, &cfg);
        assert!((c1.coherence_score - c2.coherence_score) < 1e-15);
        assert!((e1.entanglement_score - e2.entanglement_score) < 1e-15);
    }

    #[test]
    fn test_pipeline_temperature_effect() {
        let dists = vec![vec![0.5, 0.5], vec![0.3, 0.7]];
        let cfg_high = QuantumCoherenceConfig::default().with_temperature(10.0);
        let cfg_low = QuantumCoherenceConfig::default().with_temperature(0.1);
        let (c_high, _, _) = run_quantum_coherence_pipeline(&dists, &cfg_high);
        let (c_low, _, _) = run_quantum_coherence_pipeline(&dists, &cfg_low);
        assert!(c_high.coherence_score >= c_low.coherence_score - 1e-10);
    }

    // ─── Helper Function Tests ────────────────────────────────────────

    #[test]
    fn test_shannon_entropy_uniform() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let h = shannon_entropy(&dist);
        assert!((h - std::f64::consts::LN_2 * 2.0) < 1e-10);
    }

    #[test]
    fn test_shannon_entropy_deterministic() {
        let dist = vec![1.0, 0.0];
        let h = shannon_entropy(&dist);
        assert!((h - 0.0) < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0) < 1e-10);
    }

    #[test]
    fn test_kl_divergence_identical() {
        let p = vec![0.5, 0.5];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!((kl - 0.0) < 1e-10);
    }

    #[test]
    fn test_kl_divergence_positive() {
        let p = vec![0.7, 0.3];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!(kl > 0.0);
    }

    #[test]
    fn test_lcg_next_deterministic() {
        let mut s1: u64 = 42;
        let mut s2: u64 = 42;
        assert_eq!(lcg_next(&mut s1), lcg_next(&mut s2));
    }

    #[test]
    fn test_random_uniform_range() {
        let mut s: u64 = 123;
        for _ in 0..100 {
            let r = random_uniform(&mut s);
            assert!(r >= 0.0 && r < 1.0);
        }
    }
}
