//! Symbiotic Utility Function — Energy-Aware Scheduler for Edge Nodes.
//!
//! **Sprint 138:** Formalizes the utility equation for the Edge Scheduler,
//! integrating Variational Free Energy (VFE), Mutual Information, Energy Cost,
//! and KL Divergence into a single objective function.
//!
//! **Symbiotic Utility:**
//! ```math
//! U_i = w_1(-ΔVFE) + w_2(Σ MI) + w_3(-energy) + w_4(-KL)
//! ```
//!
//! Where:
//! - `ΔVFE`: Change in Variational Free Energy (negative = improvement)
//! - `MI`: Mutual Information with the network
//! - `energy`: Energy cost (battery / FLOPs)
//! - `KL`: KL divergence from global distribution

/// Weights for the symbiotic utility function.
#[derive(Debug, Clone)]
pub struct SymbioticWeights {
    /// Weight for VFE reduction (w₁). Higher = prioritize free energy minimization.
    pub w_vfe: f32,
    /// Weight for mutual information (w₂). Higher = prioritize information sharing.
    pub w_mutual_info: f32,
    /// Weight for energy cost (w₃). Higher = prioritize energy efficiency.
    pub w_energy: f32,
    /// Weight for KL divergence (w₄). Higher = prioritize distribution alignment.
    pub w_kl: f32,
}

impl Default for SymbioticWeights {
    fn default() -> Self {
        Self {
            w_vfe: 1.0,
            w_mutual_info: 0.5,
            w_energy: 0.3,
            w_kl: 0.2,
        }
    }
}

impl SymbioticWeights {
    /// Equal weights for all terms.
    pub fn equal() -> Self {
        Self {
            w_vfe: 1.0,
            w_mutual_info: 1.0,
            w_energy: 1.0,
            w_kl: 1.0,
        }
    }

    /// Energy-first configuration — prioritize battery life.
    pub fn energy_first() -> Self {
        Self {
            w_vfe: 0.5,
            w_mutual_info: 0.3,
            w_energy: 2.0,
            w_kl: 0.2,
        }
    }

    /// VFE-first configuration — prioritize free energy minimization.
    pub fn vfe_first() -> Self {
        Self {
            w_vfe: 2.0,
            w_mutual_info: 0.5,
            w_energy: 0.3,
            w_kl: 0.2,
        }
    }

    /// Create custom weights.
    pub fn new(w_vfe: f32, w_mutual_info: f32, w_energy: f32, w_kl: f32) -> Self {
        Self {
            w_vfe: w_vfe.max(0.0),
            w_mutual_info: w_mutual_info.max(0.0),
            w_energy: w_energy.max(0.0),
            w_kl: w_kl.max(0.0),
        }
    }
}

/// Compute the symbiotic utility for a node.
///
/// ```math
/// U_i = w_1(-ΔVFE) + w_2(Σ MI) + w_3(-energy) + w_4(-KL)
/// ```
///
/// # Arguments
/// * `delta_vfe` — Change in Variational Free Energy (negative = improvement)
/// * `mutual_info` — Mutual information with the network
/// * `energy_cost` — Energy cost (battery / FLOPs)
/// * `kl_divergence` — KL(p_local || p_global)
/// * `weights` — Weight tuple (w₁, w₂, w₃, w₄)
///
/// # Returns
/// Symbiotic utility score U_i
pub fn compute_symbiotic_utility(
    delta_vfe: f32,
    mutual_info: f32,
    energy_cost: f32,
    kl_divergence: f32,
    weights: (f32, f32, f32, f32),
) -> f32 {
    let (w1, w2, w3, w4) = weights;
    // U_i = w1*(-ΔVFE) + w2*(MI) + w3*(-energy) + w4*(-KL)
    (w1 * -delta_vfe) + (w2 * mutual_info) + (w3 * -energy_cost) + (w4 * -kl_divergence)
}

/// Compute symbiotic utility using SymbioticWeights struct.
///
/// # Arguments
/// * `delta_vfe` — Change in Variational Free Energy
/// * `mutual_info` — Mutual information with the network
/// * `energy_cost` — Energy cost
/// * `kl_divergence` — KL divergence
/// * `weights` — SymbioticWeights configuration
///
/// # Returns
/// Symbiotic utility score U_i
pub fn compute_symbiotic_utility_weighted(
    delta_vfe: f32,
    mutual_info: f32,
    energy_cost: f32,
    kl_divergence: f32,
    weights: &SymbioticWeights,
) -> f32 {
    compute_symbiotic_utility(
        delta_vfe,
        mutual_info,
        energy_cost,
        kl_divergence,
        (
            weights.w_vfe,
            weights.w_mutual_info,
            weights.w_energy,
            weights.w_kl,
        ),
    )
}

/// Energy-Aware Scheduler decision.
///
/// Selects the action with the highest symbiotic utility from a set of candidates.
///
/// # Arguments
/// * `candidates` — Slice of (delta_vfe, mutual_info, energy_cost, kl_divergence) tuples
/// * `weights` — SymbioticWeights configuration
///
/// # Returns
/// Index of the best candidate, or None if empty
pub fn select_best_action(
    candidates: &[(f32, f32, f32, f32)],
    weights: &SymbioticWeights,
) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }

    let mut best_idx = 0;
    let mut best_util = f32::NEG_INFINITY;

    for (i, &(delta_vfe, mutual_info, energy_cost, kl_divergence)) in candidates.iter().enumerate()
    {
        let util = compute_symbiotic_utility_weighted(
            delta_vfe,
            mutual_info,
            energy_cost,
            kl_divergence,
            weights,
        );
        if util > best_util {
            best_util = util;
            best_idx = i;
        }
    }

    Some(best_idx)
}

/// Compute KL divergence: KL(p || q) = Σ p_i · ln(p_i / q_i).
pub fn kl_divergence(p: &[f32], q: &[f32]) -> f32 {
    p.iter()
        .zip(q.iter())
        .filter_map(|(&pi, &qi)| {
            if pi > 1e-10 && qi > 1e-10 {
                Some(pi * (pi / qi).ln())
            } else {
                None
            }
        })
        .sum()
}

/// Compute approximate mutual information from correlation coefficient.
/// MI ≈ -½ ln(1 - ρ²)
pub fn mutual_info_from_correlation(rho: f32) -> f32 {
    let rho_sq = rho * rho;
    if rho_sq >= 1.0 - 1e-10 {
        f32::INFINITY
    } else {
        -0.5 * (1.0 - rho_sq).ln()
    }
}

// ─── Unit Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_symbiotic_utility_positive() {
        // delta_vfe = -0.5 (improvement), mutual_info = 0.8, energy = 0.3, kl = 0.1
        let util = compute_symbiotic_utility(-0.5, 0.8, 0.3, 0.1, (1.0, 1.0, 1.0, 1.0));
        // U = 1.0*0.5 + 1.0*0.8 + 1.0*(-0.3) + 1.0*(-0.1) = 0.5 + 0.8 - 0.3 - 0.1 = 0.9
        assert!((util - 0.9).abs() < 0.01, "Expected 0.9, got {}", util);
        assert!(util > 0.0, "Utility should be positive for good candidate");
    }

    #[test]
    fn test_compute_symbiotic_utility_negative() {
        // delta_vfe = 0.5 (worse), mutual_info = 0.0, energy = 1.0, kl = 0.5
        let util = compute_symbiotic_utility(0.5, 0.0, 1.0, 0.5, (1.0, 1.0, 1.0, 1.0));
        // U = 1.0*(-0.5) + 0 + 1.0*(-1.0) + 1.0*(-0.5) = -0.5 - 1.0 - 0.5 = -2.0
        assert!((util - (-2.0)).abs() < 0.01, "Expected -2.0, got {}", util);
        assert!(util < 0.0, "Utility should be negative for bad candidate");
    }

    #[test]
    fn test_compute_symbiotic_utility_zero() {
        let util = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, (1.0, 1.0, 1.0, 1.0));
        assert!((util - 0.0).abs() < 1e-6, "Expected 0.0, got {}", util);
    }

    #[test]
    fn test_compute_symbiotic_utility_custom_weights() {
        let util = compute_symbiotic_utility(-1.0, 1.0, 1.0, 1.0, (2.0, 1.0, 0.5, 0.5));
        // U = 2.0*1.0 + 1.0*1.0 + 0.5*(-1.0) + 0.5*(-1.0) = 2 + 1 - 0.5 - 0.5 = 2.0
        assert!((util - 2.0).abs() < 0.01, "Expected 2.0, got {}", util);
    }

    #[test]
    fn test_compute_symbiotic_utility_weighted() {
        let weights = SymbioticWeights::default();
        let util = compute_symbiotic_utility_weighted(-0.5, 0.8, 0.3, 0.1, &weights);
        assert!(util.is_finite());
    }

    #[test]
    fn test_symbiotic_weights_default() {
        let w = SymbioticWeights::default();
        assert!((w.w_vfe - 1.0).abs() < 1e-6);
        assert!((w.w_mutual_info - 0.5).abs() < 1e-6);
        assert!((w.w_energy - 0.3).abs() < 1e-6);
        assert!((w.w_kl - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_symbiotic_weights_equal() {
        let w = SymbioticWeights::equal();
        assert!((w.w_vfe - 1.0).abs() < 1e-6);
        assert!((w.w_mutual_info - 1.0).abs() < 1e-6);
        assert!((w.w_energy - 1.0).abs() < 1e-6);
        assert!((w.w_kl - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_symbiotic_weights_energy_first() {
        let w = SymbioticWeights::energy_first();
        assert!(w.w_energy > w.w_vfe, "Energy weight should be highest");
    }

    #[test]
    fn test_symbiotic_weights_vfe_first() {
        let w = SymbioticWeights::vfe_first();
        assert!(w.w_vfe > w.w_energy, "VFE weight should be highest");
    }

    #[test]
    fn test_symbiotic_weights_new() {
        let w = SymbioticWeights::new(0.5, 0.3, 0.2, 0.1);
        assert!((w.w_vfe - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_symbiotic_weights_new_clamps_negative() {
        let w = SymbioticWeights::new(-1.0, -0.5, 0.0, 0.0);
        assert!(w.w_vfe >= 0.0);
        assert!(w.w_mutual_info >= 0.0);
    }

    #[test]
    fn test_select_best_action_basic() {
        let candidates = [
            (-0.5, 0.8, 0.3, 0.1), // Good: U = 0.9
            (0.5, 0.0, 1.0, 0.5),  // Bad: U = -2.0
            (-0.3, 0.5, 0.2, 0.1), // Medium: U = 0.3 + 0.5 - 0.2 - 0.1 = 0.5
        ];
        let weights = SymbioticWeights::equal();
        let best = select_best_action(&candidates, &weights);
        assert_eq!(best, Some(0), "First candidate should be best");
    }

    #[test]
    fn test_select_best_action_empty() {
        let candidates: [(f32, f32, f32, f32); 0] = [];
        let weights = SymbioticWeights::default();
        assert_eq!(select_best_action(&candidates, &weights), None);
    }

    #[test]
    fn test_select_best_action_single() {
        let candidates = [(0.0, 0.0, 0.0, 0.0)];
        let weights = SymbioticWeights::default();
        assert_eq!(select_best_action(&candidates, &weights), Some(0));
    }

    #[test]
    fn test_select_best_action_energy_first() {
        let candidates = [
            (-1.0, 1.0, 0.1, 0.1), // Low energy, good VFE
            (-0.5, 0.5, 0.8, 0.5), // High energy, worse VFE
        ];
        let weights = SymbioticWeights::energy_first();
        let best = select_best_action(&candidates, &weights);
        assert_eq!(best, Some(0), "Energy-first should pick low energy");
    }

    #[test]
    fn test_kl_divergence_identical() {
        let p = [0.3, 0.4, 0.3];
        let kl = kl_divergence(&p, &p);
        assert!(kl < 1e-6, "KL(p||p) should be 0");
    }

    #[test]
    fn test_kl_divergence_positive() {
        let p = [0.7, 0.3];
        let q = [0.3, 0.7];
        let kl = kl_divergence(&p, &q);
        assert!(kl > 0.0, "KL divergence should be positive for different distributions");
    }

    #[test]
    fn test_kl_divergence_asymmetric() {
        let p = [0.7, 0.3];
        let q = [0.3, 0.7];
        let kl_pq = kl_divergence(&p, &q);
        let kl_qp = kl_divergence(&q, &p);
        assert!((kl_pq - kl_qp).abs() < 1e-6, "KL is symmetric for binary with swapped probs");
    }

    #[test]
    fn test_mutual_info_from_correlation_zero() {
        let mi = mutual_info_from_correlation(0.0);
        assert!((mi - 0.0).abs() < 1e-6, "MI should be 0 for zero correlation");
    }

    #[test]
    fn test_mutual_info_from_correlation_positive() {
        let mi = mutual_info_from_correlation(0.5);
        assert!(mi > 0.0, "MI should be positive for positive correlation");
    }

    #[test]
    fn test_mutual_info_from_correlation_near_one() {
        let mi = mutual_info_from_correlation(0.99);
        assert!(mi > 1.0, "MI should be large for near-perfect correlation");
    }

    #[test]
    fn test_mutual_info_from_correlation_one() {
        let mi = mutual_info_from_correlation(1.0);
        assert!(mi.is_infinite(), "MI should be infinite for perfect correlation");
    }

    #[test]
    fn test_full_scheduler_pipeline() {
        let candidates = [
            (-0.8, 0.9, 0.2, 0.05), // Excellent candidate
            (-0.3, 0.4, 0.5, 0.3),  // Mediocre
            (0.2, 0.1, 0.9, 0.8),   // Bad candidate
            (-0.6, 0.7, 0.3, 0.1),  // Good candidate
        ];

        let weights = SymbioticWeights::default();
        let best = select_best_action(&candidates, &weights).unwrap();

        // Compute utilities for verification
        let utils: Vec<f32> = candidates
            .iter()
            .map(|&(dv, mi, ec, kl)| {
                compute_symbiotic_utility_weighted(dv, mi, ec, kl, &weights)
            })
            .collect();

        let max_util = *utils.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        assert!((utils[best] - max_util).abs() < 1e-6, "Selected should have max utility");
    }
}
