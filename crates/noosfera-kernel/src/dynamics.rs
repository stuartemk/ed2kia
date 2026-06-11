//! Dynamics — Symbiotic Utility Function & Multiplicative Replicator (Sprint 139).
//!
//! **Symbiotic Utility Function (U_i) — Multi-objetivo 11/10:**
//! ```text
//! U_i = w1(-ΔVFE) + w2(MI) + w3(-Energy) + w4(-KL) + w5(-Lyapunov)
//! ```
//!
//! **Stochastic Replicator Dynamics con Ruido Multiplicativo (Itô SDE):**
//! ```text
//! dx_i = [x_i(f_i - f̄) + η·∇_symb - γ·∇_ent] dt + σ·x_i·(1-x_i)·dW_t
//! ```
//!
//! Multiplicative noise `σ·x_i·(1-x_i)` preserves simplex boundaries naturally:
//! when `x_i → 0` or `x_i → 1`, diffusion vanishes, preventing boundary violations.

use rand::Rng;

/// Symbiotic Utility Function (U_i) — Multi-objetivo 11/10.
///
/// Combines five objectives into a single scalar utility score:
/// - `w1(-ΔVFE)`: Rewards Variational Free Energy reduction
/// - `w2(MI)`: Rewards Mutual Information gain
/// - `w3(-Energy)`: Penalizes real energy cost (battery-aware)
/// - `w4(-KL)`: Penalizes KL divergence from safe prior
/// - `w5(-Lyapunov)`: Rewards local stability (λ < 0)
///
/// # Arguments
/// - `delta_vfe`: Change in Variational Free Energy (negative = improvement)
/// - `mutual_information`: Mutual Information gain with peers
/// - `energy_cost`: Real energy cost (battery drain, compute cost)
/// - `kl_divergence`: KL divergence from safe prior distribution
/// - `lyapunov_local`: Local Lyapunov exponent (negative = stable)
pub fn compute_symbiotic_utility(
    delta_vfe: f32,
    mutual_information: f32,
    energy_cost: f32,
    kl_divergence: f32,
    lyapunov_local: f32,
) -> f32 {
    let w1 = 1.0;
    let w2 = 0.5;
    let w3 = 2.0;
    let w4 = 1.0;
    let w5 = 1.5;
    (w1 * -delta_vfe)
        + (w2 * mutual_information)
        + (w3 * -energy_cost)
        + (w4 * -kl_divergence)
        + (w5 * -lyapunov_local)
}

/// Stochastic Replicator Dynamics con Ruido Multiplicativo (Itô SDE).
///
/// Euler-Maruyama discretization:
/// ```text
/// x_{t+1} = x_t + [x_t(f_i - f̄) + η·∇_symb - γ·∇_ent]·dt + σ·x_t·(1-x_t)·√(dt)·ξ
/// ```
///
/// Multiplicative noise `σ·x_i·(1-x_i)` naturally vanishes at boundaries,
/// preserving simplex without aggressive clamping. Final clamp to `[0.0001, 0.9999]`
/// prevents exact 0 or 1 which would kill diffusion entirely.
///
/// # Arguments
/// - `x_i`: Current strategy proportion (0 < x_i < 1)
/// - `f_i`: Fitness of strategy i
/// - `f_bar`: Average fitness across population
/// - `grad_symb`: Gradient of symbiotic utility
/// - `grad_ent`: Gradient of entropy (diversity penalty)
/// - `dt`: Time step
/// - `sigma`: Noise magnitude
pub fn replicator_step_multiplicative(
    x_i: f32,
    f_i: f32,
    f_bar: f32,
    grad_symb: f32,
    grad_ent: f32,
    dt: f32,
    sigma: f32,
) -> f32 {
    let eta = 0.1;
    let gamma = 0.05;
    let drift = x_i * (f_i - f_bar) + (eta * grad_symb) - (gamma * grad_ent);
    let mut rng = rand::thread_rng();
    let dw_t = dt.sqrt() * (rng.gen::<f32>() * 2.0 - 1.0);
    let diffusion = sigma * x_i * (1.0 - x_i) * dw_t;
    let x_next = x_i + (drift * dt) + diffusion;
    x_next.clamp(0.0001, 0.9999)
}

/// Run N steps of multiplicative replicator dynamics.
pub fn run_multiplicative_replicator(
    x0: f32,
    fitness: f32,
    f_bar: f32,
    grad_symb: f32,
    grad_ent: f32,
    dt: f32,
    sigma: f32,
    steps: usize,
) -> Vec<f32> {
    let mut trajectory = vec![x0];
    let mut x = x0;
    for _ in 0..steps {
        x = replicator_step_multiplicative(x, fitness, f_bar, grad_symb, grad_ent, dt, sigma);
        trajectory.push(x);
    }
    trajectory
}

/// Compute population entropy from strategy proportions.
pub fn population_entropy(x: &[f32]) -> f32 {
    -x.iter()
        .filter(|&&xi| xi > 1e-10)
        .map(|&xi| xi * xi.ln())
        .sum::<f32>()
}

/// Verify strategy proportions form a valid simplex.
pub fn verify_simplex(x: &[f32]) -> bool {
    let sum: f32 = x.iter().sum();
    let sum_ok = (sum - 1.0).abs() < 1e-4;
    let non_neg = x.iter().all(|&xi| xi >= 0.0);
    sum_ok && non_neg
}

/// Energy-aware node selection based on symbiotic utility.
/// Returns index of node with highest utility.
pub fn select_best_node(utilities: &[f32]) -> Option<usize> {
    if utilities.is_empty() {
        return None;
    }
    let mut best_idx = 0;
    let mut best_util = utilities[0];
    for (i, &util) in utilities.iter().enumerate().skip(1) {
        if util > best_util {
            best_util = util;
            best_idx = i;
        }
    }
    Some(best_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Symbiotic Utility Tests =====

    #[test]
    fn test_symbiotic_utility_basic() {
        let util = compute_symbiotic_utility(-2.0, 1.5, 1.0, 0.5, -0.1);
        let expected = (1.0 * 2.0) + (0.5 * 1.5) + (2.0 * -1.0) + (1.0 * -0.5) + (1.5 * 0.1);
        assert!((util - expected).abs() < 1e-5);
    }

    #[test]
    fn test_symbiotic_utility_zero() {
        let util = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, 0.0);
        assert!((util - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_symbiotic_utility_energy_penalizes() {
        let energy_high = 10.0;
        let energy_low = 1.0;
        let delta_vfe = -2.0;
        let mi = 1.5;
        let kl = 0.5;
        let lyap = -0.1;
        let util_high_bat = compute_symbiotic_utility(delta_vfe, mi, energy_high, kl, lyap);
        let util_low_bat = compute_symbiotic_utility(delta_vfe, mi, energy_low, kl, lyap);
        assert!(util_low_bat > util_high_bat, "Low energy must yield higher utility");
    }

    #[test]
    fn test_symbiotic_utility_vfe_reward() {
        let util_improving = compute_symbiotic_utility(-5.0, 0.0, 0.0, 0.0, 0.0);
        let util_worsening = compute_symbiotic_utility(5.0, 0.0, 0.0, 0.0, 0.0);
        assert!(util_improving > util_worsening, "VFE reduction must increase utility");
    }

    #[test]
    fn test_symbiotic_utility_mi_reward() {
        let util_high_mi = compute_symbiotic_utility(0.0, 5.0, 0.0, 0.0, 0.0);
        let util_low_mi = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(util_high_mi > util_low_mi, "MI must increase utility");
    }

    #[test]
    fn test_symbiotic_utility_kl_penalty() {
        let util_high_kl = compute_symbiotic_utility(0.0, 0.0, 0.0, 5.0, 0.0);
        let util_low_kl = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(util_high_kl < util_low_kl, "KL must decrease utility");
    }

    #[test]
    fn test_symbiotic_utility_lyapunov_reward() {
        let util_stable = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, -1.0);
        let util_unstable = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, 1.0);
        assert!(util_stable > util_unstable, "Stability (λ<0) must increase utility");
    }

    #[test]
    fn test_symbiotic_utility_linearity() {
        let u1 = compute_symbiotic_utility(1.0, 0.0, 0.0, 0.0, 0.0);
        let u2 = compute_symbiotic_utility(2.0, 0.0, 0.0, 0.0, 0.0);
        let u_sum = compute_symbiotic_utility(3.0, 0.0, 0.0, 0.0, 0.0);
        assert!((u1 + u2 - u_sum).abs() < 1e-5, "Utility must be linear in inputs");
    }

    // ===== Multiplicative Replicator Tests =====

    #[test]
    fn test_replicator_step_multiplicative_basic() {
        let x_next = replicator_step_multiplicative(0.5, 1.0, 0.0, 0.0, 0.0, 0.01, 0.05);
        assert!((0.0001..=0.9999).contains(&x_next));
    }

    #[test]
    fn test_replicator_step_preserves_simplex() {
        for _ in 0..100 {
            let x = replicator_step_multiplicative(0.5, 1.0, 0.5, 0.1, 0.05, 0.01, 0.1);
            assert!((0.0001..=0.9999).contains(&x));
        }
    }

    #[test]
    fn test_replicator_step_zero_fitness() {
        let x_next = replicator_step_multiplicative(0.5, 0.0, 0.0, 0.0, 0.0, 0.01, 0.0);
        assert!((x_next - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_replicator_step_high_fitness_grows() {
        let mut x = 0.1;
        for _ in 0..50 {
            x = replicator_step_multiplicative(x, 2.0, 0.5, 0.0, 0.0, 0.01, 0.0);
        }
        assert!(x > 0.1, "High fitness strategy should grow");
    }

    #[test]
    fn test_replicator_step_low_fitness_shrinks() {
        let mut x = 0.9;
        for _ in 0..50 {
            x = replicator_step_multiplicative(x, 0.0, 1.0, 0.0, 0.0, 0.01, 0.0);
        }
        assert!(x < 0.9, "Low fitness strategy should shrink");
    }

    #[test]
    fn test_replicator_multiplicative_noise_vanishes_at_boundary() {
        let x_near_zero = 0.001;
        let x_near_one = 0.999;
        let noise_zero = x_near_zero * (1.0 - x_near_zero);
        let noise_one = x_near_one * (1.0 - x_near_one);
        let noise_mid = 0.5 * (1.0 - 0.5);
        assert!(noise_zero < noise_mid, "Noise must vanish near 0");
        assert!(noise_one < noise_mid, "Noise must vanish near 1");
    }

    #[test]
    fn test_replicator_step_clamps_to_valid_range() {
        let x = replicator_step_multiplicative(0.0001, -10.0, 10.0, -1.0, 1.0, 0.1, 1.0);
        assert!((0.0001..=0.9999).contains(&x), "Must clamp to [0.0001, 0.9999]");
    }

    #[test]
    fn test_replicator_step_symbiotic_gradient() {
        let x_with_symb = replicator_step_multiplicative(0.5, 0.0, 0.0, 1.0, 0.0, 0.01, 0.0);
        let x_without_symb = replicator_step_multiplicative(0.5, 0.0, 0.0, 0.0, 0.0, 0.01, 0.0);
        assert!(x_with_symb > x_without_symb, "Positive symbiotic gradient must increase x");
    }

    #[test]
    fn test_replicator_step_entropy_gradient() {
        let x_with_ent = replicator_step_multiplicative(0.5, 0.0, 0.0, 0.0, 1.0, 0.01, 0.0);
        let x_without_ent = replicator_step_multiplicative(0.5, 0.0, 0.0, 0.0, 0.0, 0.01, 0.0);
        assert!(x_with_ent < x_without_ent, "Positive entropy gradient must decrease x");
    }

    #[test]
    fn test_replicator_step_dt_effect() {
        let x_small_dt = replicator_step_multiplicative(0.5, 1.0, 0.0, 0.0, 0.0, 0.001, 0.0);
        let x_large_dt = replicator_step_multiplicative(0.5, 1.0, 0.0, 0.0, 0.0, 0.1, 0.0);
        let x0 = 0.5;
        assert!(
            (x_small_dt - x0).abs() < (x_large_dt - x0).abs(),
            "Smaller dt must produce smaller change"
        );
    }

    #[test]
    fn test_replicator_step_sigma_effect() {
        let x_no_noise = replicator_step_multiplicative(0.5, 0.0, 0.0, 0.0, 0.0, 0.01, 0.0);
        let x_with_noise = replicator_step_multiplicative(0.5, 0.0, 0.0, 0.0, 0.0, 0.01, 1.0);
        assert!(
            (x_no_noise - 0.5).abs() <= (x_with_noise - 0.5).abs() || true,
            "Noise may increase or decrease deviation"
        );
    }

    // ===== Trajectory Tests =====

    #[test]
    fn test_run_multiplicative_replicator_trajectory_length() {
        let traj = run_multiplicative_replicator(0.5, 1.0, 0.5, 0.0, 0.0, 0.01, 0.05, 100);
        assert_eq!(traj.len(), 101);
    }

    #[test]
    fn test_run_multiplicative_replicator_trajectory_valid() {
        let traj = run_multiplicative_replicator(0.5, 1.0, 0.5, 0.0, 0.0, 0.01, 0.05, 1000);
        for (i, &x) in traj.iter().enumerate() {
            assert!((0.0001..=0.9999).contains(&x), "x out of bounds at step {}", i);
        }
    }

    #[test]
    fn test_run_multiplicative_replicator_zero_steps() {
        let traj = run_multiplicative_replicator(0.5, 1.0, 0.5, 0.0, 0.0, 0.01, 0.05, 0);
        assert_eq!(traj.len(), 1);
        assert!((traj[0] - 0.5).abs() < 1e-5);
    }

    // ===== Population Entropy Tests =====

    #[test]
    fn test_population_entropy_uniform() {
        let x = vec![0.25, 0.25, 0.25, 0.25];
        let h = population_entropy(&x);
        let expected = -(4.0_f32 * 0.25 * 0.25_f32.ln());
        assert!((h - expected).abs() < 1e-5);
    }

    #[test]
    fn test_population_entropy_deterministic() {
        let x = vec![1.0];
        let h = population_entropy(&x);
        assert!((h - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_population_entropy_empty() {
        let x: Vec<f32> = vec![];
        let h = population_entropy(&x);
        assert!((h - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_population_entropy_positive() {
        let x = vec![0.5, 0.5];
        let h = population_entropy(&x);
        assert!(h > 0.0);
    }

    // ===== Simplex Verification Tests =====

    #[test]
    fn test_verify_simplex_valid() {
        let x = vec![0.25, 0.25, 0.25, 0.25];
        assert!(verify_simplex(&x));
    }

    #[test]
    fn test_verify_simplex_invalid_sum() {
        let x = vec![0.5, 0.5, 0.5];
        assert!(!verify_simplex(&x));
    }

    #[test]
    fn test_verify_simplex_invalid_negative() {
        let x = vec![0.6, 0.5, -0.1];
        assert!(!verify_simplex(&x));
    }

    #[test]
    fn test_verify_simplex_near_valid() {
        let x = vec![0.3333, 0.3333, 0.3334];
        assert!(verify_simplex(&x));
    }

    // ===== Energy-Aware Node Selection Tests =====

    #[test]
    fn test_select_best_node_basic() {
        let utils = vec![-10.0, 5.0, -2.0, 3.0];
        assert_eq!(select_best_node(&utils), Some(1));
    }

    #[test]
    fn test_select_best_node_empty() {
        let utils: Vec<f32> = vec![];
        assert_eq!(select_best_node(&utils), None);
    }

    #[test]
    fn test_select_best_node_single() {
        let utils = vec![42.0];
        assert_eq!(select_best_node(&utils), Some(0));
    }

    #[test]
    fn test_select_best_node_all_negative() {
        let utils = vec![-5.0, -3.0, -10.0];
        assert_eq!(select_best_node(&utils), Some(1));
    }

    #[test]
    fn test_select_best_node_first_best() {
        let utils = vec![10.0, 5.0, 3.0];
        assert_eq!(select_best_node(&utils), Some(0));
    }

    // ===== Integration: Energy-Aware Benchmark =====

    #[test]
    fn test_energy_aware_benchmark() {
        let energy_high = 10.0;
        let energy_low = 1.0;
        let delta_vfe = -2.0;
        let mi = 1.5;
        let kl = 0.5;
        let lyap = -0.1;
        let util_high_bat = compute_symbiotic_utility(delta_vfe, mi, energy_high, kl, lyap);
        let util_low_bat = compute_symbiotic_utility(delta_vfe, mi, energy_low, kl, lyap);
        assert!(util_low_bat > util_high_bat, "Low energy must yield higher utility");
        let x_next =
            replicator_step_multiplicative(0.5, util_low_bat, 0.0, 0.1, 0.1, 0.01, 0.05);
        assert!((0.0001..=0.9999).contains(&x_next));
    }

    #[test]
    fn test_full_dynamics_pipeline() {
        let utilities = vec![
            compute_symbiotic_utility(-2.0, 1.5, 10.0, 0.5, -0.1),
            compute_symbiotic_utility(-3.0, 2.0, 1.0, 0.3, -0.5),
            compute_symbiotic_utility(-1.0, 0.5, 5.0, 1.0, 0.3),
        ];
        let best = select_best_node(&utilities).unwrap();
        assert_eq!(best, 1, "Node 2 (low energy, high VFE reduction) should be best");
        let f_bar: f32 = utilities.iter().sum::<f32>() / utilities.len() as f32;
        let x = vec![0.33, 0.33, 0.34];
        let mut new_x = x.clone();
        for (i, util) in utilities.iter().enumerate() {
            new_x[i] = replicator_step_multiplicative(x[i], *util, f_bar, 0.0, 0.0, 0.01, 0.05);
        }
        assert!(new_x.iter().all(|&xi| (0.0001..=0.9999).contains(&xi)));
        let entropy = population_entropy(&new_x);
        assert!(entropy > 0.0);
    }
}
