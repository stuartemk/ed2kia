//! Sprint 121 (v12.1.0) — THE NOOSFERA SYMBIOTIC LAUNCH & VERIFIABLE GLOBAL DEFENSE
//!
//! Academic validation benchmarks for the planetary-scale Noosfera symbiotic launch.
//! Simulates 5000-node deployment across the full hardware spectrum:
//! - 40% Smartwatch/IoT (ultra-light edge)
//! - 30% Mobile/OldDesktop (mid-tier)
//! - 20% Desktop (standard)
//! - 10% Datacenter (heavy infrastructure)

use candle_core::{Device, Tensor};
use native_audit::edge_runtime::{
    compute_multimodal_vfe_symbiosis, compute_multimodal_vfe_with_cbf,
    evaluate_proportional_hybrid, DeviceType,
};
use native_audit::global_bootstrap::{
    DeviceDistribution, GlobalAltruistMesh, GlobalBootstrapConfig,
};

/// Benchmark: Noosfera Symbiotic Launch — 5000-node planetary simulation.
#[test]
fn benchmark_noosfera_symbiotic_launch() {
    let dist = DeviceDistribution {
        smartwatch_pct: 0.20,
        iot_pct: 0.20,
        mobile_pct: 0.20,
        old_desktop_pct: 0.10,
        desktop_pct: 0.20,
        datacenter_pct: 0.10,
    };
    assert!(dist.is_valid());

    let config = GlobalBootstrapConfig {
        node_count: 5000,
        distribution: dist,
        trust_threshold: 0.6,
        churn_probability: 0.05,
        simulation_steps: 10,
        base_latency_ms: 300,
        latency_variance_ms: 100,
    };

    let mut mesh = GlobalAltruistMesh::new(config);
    let result = mesh.bootstrap_planetary();

    // Validate device distribution matches target
    assert_eq!(
        result.ultra_light_nodes
            + result.mid_tier_nodes
            + result.standard_nodes
            + result.heavy_nodes,
        5000
    );

    // Ultra-light (smartwatch + IoT) should be ~40% = 2000
    assert!(result.ultra_light_nodes >= 1800);
    assert!(result.ultra_light_nodes <= 2200);

    // Heavy (datacenter) should be ~10% = 500
    assert!(result.heavy_nodes >= 400);
    assert!(result.heavy_nodes <= 600);

    // Energy savings should be positive across the mesh
    assert!(result.total_energy_saved_mwh > 0.0);

    // Average contribution factor should be > 1.0 (ultra-light bonus)
    assert!(result.avg_contribution_factor > 1.0);

    // PoSym participation should be reasonable
    assert!(result.posym_participation_rate >= 0.0);
    assert!(result.posym_participation_rate <= 1.0);

    println!("=== Noosfera Symbiotic Launch Benchmark ===");
    println!("{}", result);
}

/// Benchmark: Proportional scaling across full hardware spectrum.
#[test]
fn benchmark_proportional_scaling_spectrum() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let hidden = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
    let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
    let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

    let devices = [
        DeviceType::Smartwatch,
        DeviceType::Iot,
        DeviceType::Mobile,
        DeviceType::OldDesktop,
        DeviceType::Desktop,
        DeviceType::Datacenter,
    ];

    for dt in &devices {
        let (_safe, _certified, _steered, trust_delta, energy_used) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, *dt, 0.8, 0.8)?;

        assert!(
            trust_delta > 0.0,
            "Trust delta should be positive for {:?}",
            dt
        );
        assert!(
            energy_used > 0.0,
            "Energy used should be positive for {:?}",
            dt
        );

        // Energy should generally increase with device capability
        // (Smartwatch uses ultra-light path, datacenter uses full hybrid)
        if *dt != DeviceType::Smartwatch && *dt != DeviceType::Iot {
            // Full hybrid path: energy should be reasonable
            assert!(energy_used < 100.0, "Energy too high for {:?}", dt);
        }

        let _ = energy_used;
    }

    println!("=== Proportional Scaling Spectrum ===");
    println!("All device types evaluated successfully across the spectrum");
    Ok(())
}

/// Benchmark: Multi-modal VFE symbiosis with realistic modality weights.
#[test]
fn benchmark_multimodal_vfe_symbiosis() {
    // Simulate 4 modalities: text, image, audio, video
    let modal_vfes = vec![0.12, 0.18, 0.15, 0.22];
    let modal_weights = vec![0.40, 0.25, 0.20, 0.15];

    let combined = compute_multimodal_vfe_symbiosis(&modal_vfes, &modal_weights);

    // Combined VFE should be within the range of individual VFEs
    assert!(
        combined
            >= modal_vfes
                .iter()
                .min_by_key(|&&v| (v * 1000.0) as i64)
                .unwrap()
                - 0.01
    );
    assert!(
        combined
            <= modal_vfes
                .iter()
                .max_by_key(|&&v| (v * 1000.0) as i64)
                .unwrap()
                + 0.01
    );

    // With CBF safety threshold
    let (combined_cbf, margin, is_safe) =
        compute_multimodal_vfe_with_cbf(&modal_vfes, &modal_weights, 0.5);
    assert!((combined_cbf - combined).abs() < 1e-10);
    assert!(margin > 0.0);
    assert!(is_safe);

    println!("=== Multi-Modal VFE Symbiosis ===");
    println!("Combined VFE: {:.6}", combined);
    println!("CBF Margin: {:.6}", margin);
    println!("Is Safe: {}", is_safe);
}

/// Benchmark: Device contribution factor fairness.
#[test]
fn benchmark_contribution_factor_fairness() {
    // Verify that lower-capability devices get higher PoSym bonus
    let contributions = [
        (DeviceType::Smartwatch, 5.0),
        (DeviceType::Iot, 3.0),
        (DeviceType::Mobile, 2.0),
        (DeviceType::OldDesktop, 1.5),
        (DeviceType::Desktop, 1.0),
        (DeviceType::Datacenter, 0.5),
    ];

    for (dt, expected) in &contributions {
        let actual = dt.contribution_factor();
        assert!(
            (actual - expected).abs() < 1e-9,
            "Contribution factor mismatch for {:?}: expected {}, got {}",
            dt,
            expected,
            actual
        );
    }

    // Verify monotonic decrease: lower capability → higher factor
    let factors: Vec<f64> = contributions.iter().map(|(_, f)| *f).collect();
    for i in 0..factors.len() - 1 {
        assert!(
            factors[i] > factors[i + 1],
            "Contribution factor should decrease with capability"
        );
    }

    println!("=== Contribution Factor Fairness ===");
    println!("PoSym bonus correctly rewards lower-capability devices");
}

/// Benchmark: Energy savings across device types.
#[test]
fn benchmark_energy_savings_by_device() {
    let devices = [
        (DeviceType::Smartwatch, 98.0),
        (DeviceType::Iot, 96.0),
        (DeviceType::Mobile, 90.0),
        (DeviceType::OldDesktop, 85.0),
        (DeviceType::Desktop, 88.0),
        (DeviceType::Datacenter, 80.0),
    ];

    for (dt, min_savings_pct) in &devices {
        let base_cost = dt.base_energy_cost();
        let dc_baseline = dt.dc_baseline_cost();
        let estimated_savings = dc_baseline - base_cost;
        let savings_pct = (estimated_savings / dc_baseline) * 100.0;

        assert!(
            savings_pct >= *min_savings_pct,
            "{:?} energy savings {:.1}% below minimum {:.1}%",
            dt,
            savings_pct,
            min_savings_pct
        );
    }

    println!("=== Energy Savings by Device ===");
    for (dt, _min) in &devices {
        let base = dt.base_energy_cost();
        let baseline = dt.dc_baseline_cost();
        let savings_pct = ((baseline - base) / baseline) * 100.0;
        println!(
            "  {:12?}: {:.1}% savings (base={:.3}mWh, baseline={:.1}mWh)",
            dt, savings_pct, base, baseline
        );
    }
}

/// Benchmark: Compute budget scaling.
#[test]
fn benchmark_compute_budget_scaling() {
    let budgets = [
        (DeviceType::Smartwatch, 0.1),
        (DeviceType::Iot, 0.15),
        (DeviceType::Mobile, 0.4),
        (DeviceType::OldDesktop, 0.6),
        (DeviceType::Desktop, 1.0),
        (DeviceType::Datacenter, 1.0),
    ];

    for (dt, expected) in &budgets {
        let actual = dt.compute_budget();
        assert!(
            (actual - expected).abs() < 1e-9,
            "Compute budget mismatch for {:?}: expected {}, got {}",
            dt,
            expected,
            actual
        );
    }

    // Verify ultra-light threshold
    assert!(DeviceType::Smartwatch.compute_budget() < 0.3);
    assert!(DeviceType::Iot.compute_budget() < 0.3);
    assert!(DeviceType::Mobile.compute_budget() >= 0.3);

    println!("=== Compute Budget Scaling ===");
    println!("Budget correctly scales from smartwatch (0.1) to datacenter (1.0)");
}

/// Benchmark: Install command generation for all device types.
#[test]
fn benchmark_install_commands_all_devices() {
    use native_audit::edge_runtime::AltruistOnboarding;

    let devices = [
        (DeviceType::Smartwatch, "--smartwatch"),
        (DeviceType::Iot, "--iot"),
        (DeviceType::Mobile, "--mobile"),
        (DeviceType::OldDesktop, "--old-desktop"),
        (DeviceType::Desktop, "--desktop"),
        (DeviceType::Datacenter, "--datacenter"),
    ];

    for (dt, expected_flag) in &devices {
        let onboarding = AltruistOnboarding::new(1, *dt);
        let cmd = onboarding.install_command();
        assert!(
            cmd.contains(expected_flag),
            "Install command for {:?} should contain '{}': {}",
            dt,
            expected_flag,
            cmd
        );
        assert!(cmd.contains("ed2k start --altruist"));
    }

    println!("=== Install Commands ===");
    println!("All device types generate correct install commands");
}

/// Benchmark: Large-scale mesh with realistic churn.
#[test]
fn benchmark_large_mesh_realistic_churn() {
    let config = GlobalBootstrapConfig {
        node_count: 5000,
        churn_probability: 0.08,
        simulation_steps: 15,
        base_latency_ms: 350,
        latency_variance_ms: 150,
        ..GlobalBootstrapConfig::default()
    };

    let mut mesh = GlobalAltruistMesh::new(config);
    let result = mesh.bootstrap_planetary();

    assert_eq!(
        result.ultra_light_nodes
            + result.mid_tier_nodes
            + result.standard_nodes
            + result.heavy_nodes,
        5000
    );

    // With realistic churn, energy should still accumulate
    assert!(result.total_energy_saved_mwh > 0.0);

    // VFE reduction should be positive
    assert!(result.avg_vfe_reduction >= 0.0);

    println!("=== Large Mesh Realistic Churn ===");
    println!("5000 nodes, 8% churn, 15 steps: {}", result);
}

/// Benchmark: Multi-modal CBF safety verification.
#[test]
fn benchmark_cbf_safety_verification() {
    // Safe configuration: all modalities well below threshold
    let safe_vfes = vec![0.05, 0.08, 0.06, 0.10];
    let weights = vec![0.40, 0.25, 0.20, 0.15];
    let (_, safe_margin, safe_ok) = compute_multimodal_vfe_with_cbf(&safe_vfes, &weights, 0.5);
    assert!(safe_ok);
    assert!(safe_margin > 0.3);

    // Unsafe configuration: modalities exceed threshold
    let unsafe_vfes = vec![0.60, 0.70, 0.65, 0.80];
    let (_, unsafe_margin, unsafe_ok) =
        compute_multimodal_vfe_with_cbf(&unsafe_vfes, &weights, 0.5);
    assert!(!unsafe_ok);
    assert!(unsafe_margin < 0.0);

    // Boundary: close to threshold
    let boundary_vfes = vec![0.48, 0.50, 0.49, 0.51];
    let (_, boundary_margin, boundary_ok) =
        compute_multimodal_vfe_with_cbf(&boundary_vfes, &weights, 0.5);
    assert!(boundary_margin.abs() < 0.1); // Near boundary

    println!("=== CBF Safety Verification ===");
    println!(
        "Safe margin: {:.4} ({})",
        safe_margin,
        if safe_ok { "SAFE" } else { "UNSAFE" }
    );
    println!(
        "Unsafe margin: {:.4} ({})",
        unsafe_margin,
        if unsafe_ok { "SAFE" } else { "UNSAFE" }
    );
    println!(
        "Boundary margin: {:.4} ({})",
        boundary_margin,
        if boundary_ok { "SAFE" } else { "UNSAFE" }
    );
}

/// Benchmark: Sprint 121 full pipeline integration.
#[test]
fn benchmark_sprint121_full_pipeline() -> candle_core::Result<()> {
    let device = Device::Cpu;

    // Step 1: Evaluate proportional hybrid for each device type
    let hidden = Tensor::new(vec![2.0f32, 4.0, 6.0], &device)?;
    let safe = Tensor::new(vec![1.0f32, 2.0, 3.0], &device)?;
    let toxic = Tensor::new(vec![10.0f32, 20.0, 30.0], &device)?;

    let mut total_trust = 0.0;
    let mut total_energy = 0.0;

    for dt in [
        DeviceType::Smartwatch,
        DeviceType::Mobile,
        DeviceType::Desktop,
        DeviceType::Datacenter,
    ] {
        let (_safe, _certified, _steered, trust, energy) =
            evaluate_proportional_hybrid(&hidden, &safe, &toxic, dt, 0.7, 0.7)?;
        total_trust += trust;
        total_energy += energy;
    }

    assert!(total_trust > 0.0);
    assert!(total_energy > 0.0);

    // Step 2: Multi-modal VFE
    let combined = compute_multimodal_vfe_symbiosis(&[0.1, 0.2, 0.15], &[0.5, 0.3, 0.2]);
    assert!(combined > 0.0);

    // Step 3: Bootstrap small mesh
    let config = GlobalBootstrapConfig {
        node_count: 100,
        simulation_steps: 3,
        ..GlobalBootstrapConfig::default()
    };
    let mut mesh = GlobalAltruistMesh::new(config);
    let result = mesh.bootstrap_planetary();
    assert_eq!(
        result.ultra_light_nodes
            + result.mid_tier_nodes
            + result.standard_nodes
            + result.heavy_nodes,
        100
    );

    println!("=== Sprint 121 Full Pipeline ===");
    println!("Total trust delta: {:.4}", total_trust);
    println!("Total energy used: {:.4}mWh", total_energy);
    println!("Multi-modal VFE: {:.6}", combined);
    println!("Mesh result: {}", result);
    Ok(())
}
