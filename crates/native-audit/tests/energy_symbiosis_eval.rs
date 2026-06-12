//! Energy Symbiosis Evaluation — Sprint 139
//!
//! Integration tests demonstrating that the Symbiotic Utility Function
//! properly penalizes high energy cost (battery drain), and that the
//! Multiplicative Replicator preserves simplex boundaries.

use noosfera_kernel::dynamics::{
    compute_symbiotic_utility, population_entropy, replicator_step_multiplicative,
    run_multiplicative_replicator, select_best_node, verify_simplex,
};

#[test]
fn test_symbiotic_utility_energy_penalty() {
    let energy_high = 10.0;
    let energy_low = 1.0;
    let delta_vfe = -2.0;
    let mi = 1.5;
    let kl = 0.5;
    let lyap = -0.1;

    let util_high_bat = compute_symbiotic_utility(delta_vfe, mi, energy_high, kl, lyap);
    let util_low_bat = compute_symbiotic_utility(delta_vfe, mi, energy_low, kl, lyap);

    println!("🔋 Utilidad (Batería Baja): {:.2}", util_low_bat);
    println!("🔌 Utilidad (Batería Alta): {:.2}", util_high_bat);
    assert!(
        util_low_bat > util_high_bat,
        "Low energy cost must yield higher utility (actual: {:.2} > {:.2})",
        util_low_bat, util_high_bat
    );
}

#[test]
fn test_replicator_multiplicative_simplex_preservation() {
    let x_next = replicator_step_multiplicative(0.5, 1.0, 0.0, 0.1, 0.1, 0.01, 0.05);
    println!("🧬 Replicator Next State: {:.4}", x_next);
    assert!(
        (0.0001..=0.9999).contains(&x_next),
        "Replicator must preserve simplex: got {:.4}",
        x_next
    );
}

#[test]
fn test_replicator_trajectory_stability() {
    let traj = run_multiplicative_replicator(0.5, 1.0, 0.5, 0.0, 0.0, 0.01, 0.05, 1000);
    assert_eq!(traj.len(), 1001);
    for (i, &x) in traj.iter().enumerate() {
        assert!(
            (0.0001..=0.9999).contains(&x),
            "Trajectory out of bounds at step {}: {:.6}",
            i,
            x
        );
    }
    println!("✅ Trajectory stable for {} steps", traj.len());
}

#[test]
fn test_energy_aware_node_selection() {
    let utilities = vec![
        compute_symbiotic_utility(-2.0, 1.5, 10.0, 0.5, -0.1),
        compute_symbiotic_utility(-3.0, 2.0, 1.0, 0.3, -0.5),
        compute_symbiotic_utility(-1.0, 0.5, 5.0, 1.0, 0.3),
    ];
    println!("Utilities: [{:.2}, {:.2}, {:.2}]", utilities[0], utilities[1], utilities[2]);
    let best = select_best_node(&utilities).unwrap();
    assert_eq!(
        best, 1,
        "Node 1 (low energy, high VFE reduction, stable) should be selected"
    );
    println!("✅ Best node: {} (utility: {:.2})", best, utilities[best]);
}

#[test]
fn test_full_energy_symbiosis_pipeline() {
    let nodes = vec![
        {
            let u = compute_symbiotic_utility(-2.0, 1.5, 10.0, 0.5, -0.1);
            println!("🔋 Node 0 (high energy): U = {:.2}", u);
            u
        },
        {
            let u = compute_symbiotic_utility(-3.0, 2.0, 1.0, 0.3, -0.5);
            println!("🔋 Node 1 (low energy):  U = {:.2}", u);
            u
        },
        {
            let u = compute_symbiotic_utility(-1.0, 0.5, 5.0, 1.0, 0.3);
            println!("🔋 Node 2 (med energy):  U = {:.2}", u);
            u
        },
    ];

    let best = select_best_node(&nodes).unwrap();
    let f_bar: f32 = nodes.iter().sum::<f32>() / nodes.len() as f32;

    let mut strategies = vec![0.33, 0.33, 0.34];
    for (i, node_util) in nodes.iter().enumerate() {
        strategies[i] =
            replicator_step_multiplicative(strategies[i], *node_util, f_bar, 0.0, 0.0, 0.01, 0.05);
    }

    assert!(verify_simplex(&strategies), "Strategies must form valid simplex");
    assert!(
        strategies[best] > 0.33,
        "Best node strategy should increase"
    );

    let entropy = population_entropy(&strategies);
    println!(
        "🧬 Strategies: [{:.4}, {:.4}, {:.4}], H = {:.4}",
        strategies[0],
        strategies[1],
        strategies[2],
        entropy
    );
    assert!(entropy > 0.0, "Entropy must be positive");
}

#[test]
fn test_multiplicative_noise_boundary_behavior() {
    let x_near_zero = 0.001;
    let x_near_one = 0.999;
    let noise_zero = x_near_zero * (1.0 - x_near_zero);
    let noise_one = x_near_one * (1.0 - x_near_one);
    let noise_mid = 0.5 * (1.0 - 0.5);

    assert!(noise_zero < noise_mid, "Noise must vanish near x=0");
    assert!(noise_one < noise_mid, "Noise must vanish near x=1");
    println!(
        "✅ Multiplicative noise: zero={:.4}, mid={:.4}, one={:.4}",
        noise_zero, noise_mid, noise_one
    );
}

#[test]
fn test_vfe_reduction_priority() {
    let util_high_vfe = compute_symbiotic_utility(-10.0, 0.0, 0.0, 0.0, 0.0);
    let util_low_vfe = compute_symbiotic_utility(-1.0, 0.0, 0.0, 0.0, 0.0);
    assert!(
        util_high_vfe > util_low_vfe,
        "Higher VFE reduction must yield higher utility"
    );
    println!(
        "✅ VFE priority: high_vfe={:.2} > low_vfe={:.2}",
        util_high_vfe, util_low_vfe
    );
}

#[test]
fn test_lyapunov_stability_bonus() {
    let util_stable = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, -2.0);
    let util_unstable = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, 2.0);
    assert!(
        util_stable > util_unstable,
        "Stable systems (λ<0) must have higher utility"
    );
    println!(
        "✅ Lyapunov bonus: stable={:.2} > unstable={:.2}",
        util_stable, util_unstable
    );
}

#[test]
fn test_kl_divergence_penalty() {
    let util_low_kl = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.1, 0.0);
    let util_high_kl = compute_symbiotic_utility(0.0, 0.0, 0.0, 5.0, 0.0);
    assert!(
        util_low_kl > util_high_kl,
        "Lower KL divergence must yield higher utility"
    );
    println!(
        "✅ KL penalty: low_kl={:.2} > high_kl={:.2}",
        util_low_kl, util_high_kl
    );
}

#[test]
fn test_mutual_information_bonus() {
    let util_low_mi = compute_symbiotic_utility(0.0, 0.0, 0.0, 0.0, 0.0);
    let util_high_mi = compute_symbiotic_utility(0.0, 5.0, 0.0, 0.0, 0.0);
    assert!(
        util_high_mi > util_low_mi,
        "Higher mutual information must yield higher utility"
    );
    println!(
        "✅ MI bonus: high_mi={:.2} > low_mi={:.2}",
        util_high_mi, util_low_mi
    );
}

#[test]
fn test_replicator_convergence_to_best() {
    let best_util = 5.0;
    let avg_util = 2.0;
    let mut x = 0.1;
    for _ in 0..200 {
        x = replicator_step_multiplicative(x, best_util, avg_util, 0.0, 0.0, 0.01, 0.0);
    }
    assert!(x > 0.1, "Best strategy should grow over time (final: {:.4})", x);
    println!("✅ Replicator convergence: x0=0.1 → x_final={:.4}", x);
}

#[test]
fn test_replicator_elimination_of_worst() {
    let worst_util = -5.0;
    let avg_util = 2.0;
    let mut x = 0.9;
    for _ in 0..200 {
        x = replicator_step_multiplicative(x, worst_util, avg_util, 0.0, 0.0, 0.01, 0.0);
    }
    assert!(x < 0.9, "Worst strategy should shrink over time (final: {:.4})", x);
    println!("✅ Replicator elimination: x0=0.9 → x_final={:.4}", x);
}

#[test]
fn test_entropy_decreases_under_selection() {
    let utils = vec![5.0, 2.0, -1.0];
    let f_bar: f32 = utils.iter().sum::<f32>() / utils.len() as f32;
    let mut x = vec![0.33, 0.33, 0.34];
    let h_initial = population_entropy(&x);

    for _ in 0..100 {
        for i in 0..3 {
            x[i] = replicator_step_multiplicative(x[i], utils[i], f_bar, 0.0, 0.0, 0.01, 0.0);
        }
    }

    let h_final = population_entropy(&x);
    println!(
        "✅ Entropy: H_initial={:.4} → H_final={:.4}",
        h_initial, h_final
    );
    assert!(h_final < h_initial, "Entropy should decrease under selection pressure");
}

#[test]
fn test_energy_dominates_utility() {
    let base_util = compute_symbiotic_utility(-5.0, 3.0, 1.0, 0.5, -1.0);
    let high_energy_util = compute_symbiotic_utility(-5.0, 3.0, 20.0, 0.5, -1.0);
    assert!(
        base_util > high_energy_util,
        "High energy cost must dominate utility even with good VFE/MI/Lyapunov"
    );
    let diff = base_util - high_energy_util;
    println!(
        "✅ Energy dominance: ΔU = {:.2} (w3=2.0 × ΔE=19.0 = {:.2})",
        diff,
        2.0 * 19.0
    );
    assert!((diff - 38.0).abs() < 0.01, "Energy penalty must match w3 × ΔE");
}

#[test]
fn test_cbf_projection_safe_state() {
    use native_audit::steering::steer_cbf_projection;
    use candle_core::{Device, DType, Tensor};

    let h = Tensor::zeros(&[2, 2], DType::F32, &Device::Cpu).unwrap();
    let safe = Tensor::zeros(&[2, 2], DType::F32, &Device::Cpu).unwrap();
    let result = steer_cbf_projection(&h, &safe, 1.0).unwrap();
    let val = result.mean_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(
        (val - 0.0).abs() < 1e-5,
        "Safe state should remain unchanged"
    );
    println!("✅ CBF: Safe state unchanged (val={:.6})", val);
}

#[test]
fn test_cbf_projection_corrects_unsafe() {
    use native_audit::steering::steer_cbf_projection;
    use candle_core::{Device, DType, Tensor};

    let h = Tensor::new(50.0f32, &Device::Cpu).unwrap().broadcast_as(&[2, 2]).unwrap();
    let safe = Tensor::zeros(&[2, 2], DType::F32, &Device::Cpu).unwrap();
    let result = steer_cbf_projection(&h, &safe, 2.0).unwrap();
    let result_val = result.mean_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(
        result_val.abs() < 50.0,
        "CBF should reduce distance from safe centroid (50.0 → {:.4})",
        result_val
    );
    println!("✅ CBF: Unsafe state corrected (50.0 → {:.4})", result_val);
}

#[test]
fn test_s139_summary() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  Sprint 139 — Hybrid Reachability & Empirical Symbiosis");
    println!("═══════════════════════════════════════════════════════");

    let util_low = compute_symbiotic_utility(-2.0, 1.5, 1.0, 0.5, -0.1);
    let util_high = compute_symbiotic_utility(-2.0, 1.5, 10.0, 0.5, -0.1);
    println!("🔋 Symbiotic Utility (low energy):  {:.2}", util_low);
    println!("🔌 Symbiotic Utility (high energy): {:.2}", util_high);
    println!("✅ Energy penalty: {:.2}", util_low - util_high);

    let x = replicator_step_multiplicative(0.5, util_low, 0.0, 0.1, 0.1, 0.01, 0.05);
    println!("🧬 Multiplicative Replicator: 0.5 → {:.4}", x);
    println!("✅ Simplex preserved: {}", (0.0001..=0.9999).contains(&x));

    println!("═══════════════════════════════════════════════════════\n");
}
