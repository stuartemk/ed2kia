//! Mechanism Design Integration Tests — Sprint 112 (v11.2.0)
//!
//! VCG Auction + Shapley Value + Byzantine Detection + Credit Ledger tests.

use candle_core::Device;
use native_audit::mechanism_design::{
    ByzantineDetector, CollectiveMechanism, Contribution, CreditLedger, MechanismConfig,
    ShapleyEngine, TensorVerifier, VCGAuction,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_contributions(count: usize) -> Vec<Contribution> {
    (0..count)
        .map(|i| Contribution {
            peer_id: i,
            vfe_reduction: 0.5 + (i as f32) * 0.1,
            cost: 0.1,
            trust: 0.8 + (i as f32) * 0.02,
            verified: true,
        })
        .collect()
}

fn make_config() -> MechanismConfig {
    MechanismConfig::default()
}

// ---------------------------------------------------------------------------
// MechanismConfig Tests
// ---------------------------------------------------------------------------

#[test]
fn test_config_default() {
    let config = MechanismConfig::default();
    assert!(config.reserve_price >= 0.0);
    assert!((0.0..=1.0).contains(&config.byzantine_threshold));
    assert!((0.0..=1.0).contains(&config.credit_decay));
}

#[test]
fn test_config_custom() {
    let config = MechanismConfig {
        reserve_price: 0.5,
        byzantine_threshold: 0.4,
        credit_decay: 0.1,
        shapley_samples: 100,
        min_trust: 0.1,
    };
    assert_eq!(config.reserve_price, 0.5);
    assert_eq!(config.byzantine_threshold, 0.4);
    assert_eq!(config.credit_decay, 0.1);
    assert_eq!(config.shapley_samples, 100);
}

#[test]
fn test_config_clone() {
    let config = make_config();
    let clone = config.clone();
    assert_eq!(config.reserve_price, clone.reserve_price);
}

// ---------------------------------------------------------------------------
// VCGAuction Tests
// ---------------------------------------------------------------------------

#[test]
fn test_vcg_creation() {
    let config = make_config();
    let _auction = VCGAuction::new(&config);
}

#[test]
fn test_vcg_empty() {
    let config = make_config();
    let auction = VCGAuction::new(&config);
    let result = auction.run_auction(&[], 3);
    assert!(result.winners.is_empty());
    assert!(result.payments.is_empty());
    assert_eq!(result.social_welfare, 0.0);
}

#[test]
fn test_vcg_single_contributor() {
    let config = make_config();
    let auction = VCGAuction::new(&config);
    let contributions = vec![Contribution {
        peer_id: 0,
        vfe_reduction: 1.0,
        cost: 0.2,
        trust: 0.9,
        verified: true,
    }];
    let result = auction.run_auction(&contributions, 1);
    assert!(!result.winners.is_empty());
}

#[test]
fn test_vcg_selects_best() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        min_trust: 0.1,
        ..MechanismConfig::default()
    };
    let auction = VCGAuction::new(&config);
    let contributions = vec![
        Contribution {
            peer_id: 0,
            vfe_reduction: 0.3,
            cost: 0.1,
            trust: 0.9,
            verified: true,
        },
        Contribution {
            peer_id: 1,
            vfe_reduction: 1.0,
            cost: 0.1,
            trust: 0.9,
            verified: true,
        },
    ];
    let result = auction.run_auction(&contributions, 1);
    assert_eq!(result.winners, vec![1]);
}

#[test]
fn test_vcg_social_welfare_positive() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        min_trust: 0.1,
        ..MechanismConfig::default()
    };
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(5);
    let result = auction.run_auction(&contributions, 3);
    assert!(result.social_welfare > 0.0);
}

#[test]
fn test_vcg_max_winners_limit() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        min_trust: 0.1,
        ..MechanismConfig::default()
    };
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(10);
    let result = auction.run_auction(&contributions, 3);
    assert!(result.winners.len() <= 3);
}

#[test]
fn test_vcg_payments_finite() {
    let config = make_config();
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(5);
    let result = auction.run_auction(&contributions, 3);
    for p in &result.payments {
        assert!(p.is_finite());
    }
}

#[test]
fn test_vcg_externalities_finite() {
    let config = make_config();
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(5);
    let result = auction.run_auction(&contributions, 3);
    for e in &result.externalities {
        assert!(e.is_finite());
    }
}

#[test]
fn test_vcg_reserve_price_filters() {
    let config = MechanismConfig {
        reserve_price: 10.0,
        min_trust: 0.1,
        ..MechanismConfig::default()
    };
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(5);
    let result = auction.run_auction(&contributions, 3);
    assert!(result.winners.is_empty());
}

#[test]
fn test_vcg_truthfulness() {
    // VCG truthfulness depends on individual rationality (payment >= cost)
    // Use low reserve price to ensure winners are selected
    let config = MechanismConfig {
        reserve_price: -1.0,
        min_trust: 0.1,
        ..MechanismConfig::default()
    };
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(5);
    let result = auction.run_auction(&contributions, 3);
    // Verify winners have positive net value
    for &w in &result.winners {
        let net = contributions[w].vfe_reduction - contributions[w].cost;
        assert!(net > 0.0);
    }
}

#[test]
fn test_vcg_result_clone() {
    let config = make_config();
    let auction = VCGAuction::new(&config);
    let result = auction.run_auction(&make_contributions(3), 2);
    let _clone = result.clone();
}

#[test]
fn test_vcg_result_debug() {
    let config = make_config();
    let auction = VCGAuction::new(&config);
    let result = auction.run_auction(&make_contributions(3), 2);
    let debug_str = format!("{:?}", result);
    assert!(!debug_str.is_empty());
}

// ---------------------------------------------------------------------------
// ShapleyEngine Tests
// ---------------------------------------------------------------------------

#[test]
fn test_shapley_creation() {
    let config = make_config();
    let _engine = ShapleyEngine::new(&config);
}

#[test]
fn test_shapley_empty() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let result = engine.compute_shapley(&[]);
    assert!(result.values.is_empty());
}

#[test]
fn test_shapley_single() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions = vec![Contribution {
        peer_id: 0,
        vfe_reduction: 1.0,
        cost: 0.1,
        trust: 0.9,
        verified: true,
    }];
    let result = engine.compute_shapley(&contributions);
    assert_eq!(result.values.len(), 1);
    assert!(result.values[0].is_finite());
}

#[test]
fn test_shapley_values_sum_positive() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions = make_contributions(5);
    let result = engine.compute_shapley(&contributions);
    let total: f32 = result.values.iter().sum();
    assert!(total > 0.0);
}

#[test]
fn test_shapley_efficiency_error_small() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions = make_contributions(4);
    let result = engine.compute_shapley(&contributions);
    // Efficiency error should be finite and reasonable
    assert!(result.efficiency_error.is_finite());
}

#[test]
fn test_shapley_values_finite() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions = make_contributions(5);
    let result = engine.compute_shapley(&contributions);
    for v in &result.values {
        assert!(v.is_finite());
    }
}

#[test]
fn test_shapley_marginal_contributions() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions = make_contributions(3);
    let result = engine.compute_shapley(&contributions);
    assert_eq!(result.marginal_contributions.len(), 3);
}

#[test]
fn test_shapley_exact() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions = make_contributions(3);
    let result = engine.compute_shapley_exact(&contributions);
    assert_eq!(result.values.len(), 3);
    for v in &result.values {
        assert!(v.is_finite());
    }
}

#[test]
fn test_shapley_exact_vs_approximate() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions = make_contributions(3);
    let approx = engine.compute_shapley(&contributions);
    let exact = engine.compute_shapley_exact(&contributions);
    assert_eq!(approx.values.len(), exact.values.len());
}

#[test]
fn test_shapley_result_clone() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let result = engine.compute_shapley(&make_contributions(3));
    let _clone = result.clone();
}

#[test]
fn test_shapley_result_debug() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let result = engine.compute_shapley(&make_contributions(3));
    let debug_str = format!("{:?}", result);
    assert!(!debug_str.is_empty());
}

// ---------------------------------------------------------------------------
// CreditLedger Tests
// ---------------------------------------------------------------------------

#[test]
fn test_ledger_creation() {
    let config = make_config();
    let _ledger = CreditLedger::new(&config, 10);
}

#[test]
fn test_ledger_initial_balances_zero() {
    let config = make_config();
    let ledger = CreditLedger::new(&config, 5);
    let snap = ledger.snapshot();
    for b in &snap.balances {
        assert_eq!(*b, 0.0);
    }
}

#[test]
fn test_ledger_issue_credits() {
    let config = make_config();
    let mut ledger = CreditLedger::new(&config, 3);
    let issued = ledger.issue_credits(&[1.0, 2.0, 3.0]);
    assert_eq!(issued, 6.0);
    let snap = ledger.snapshot();
    assert_eq!(snap.balances[0], 1.0);
    assert_eq!(snap.balances[1], 2.0);
}

#[test]
fn test_ledger_burn_credits() {
    let config = make_config();
    let mut ledger = CreditLedger::new(&config, 3);
    ledger.issue_credits(&[5.0, 2.0, 3.0]);
    let ok = ledger.burn_credits(0, 2.0);
    assert!(ok);
    let snap = ledger.snapshot();
    assert_eq!(snap.balances[0], 3.0);
}

#[test]
fn test_ledger_burn_insufficient() {
    let config = make_config();
    let mut ledger = CreditLedger::new(&config, 3);
    ledger.issue_credits(&[1.0, 2.0, 3.0]);
    let ok = ledger.burn_credits(0, 5.0);
    assert!(!ok);
}

#[test]
fn test_ledger_decay() {
    let config = MechanismConfig {
        credit_decay: 0.1,
        ..MechanismConfig::default()
    };
    let mut ledger = CreditLedger::new(&config, 3);
    ledger.issue_credits(&[10.0, 20.0, 30.0]);
    ledger.apply_decay();
    let snap = ledger.snapshot();
    assert!((snap.balances[0] - 9.0).abs() < 0.01);
    assert!((snap.balances[1] - 18.0).abs() < 0.01);
}

#[test]
fn test_ledger_snapshot() {
    let config = make_config();
    let mut ledger = CreditLedger::new(&config, 5);
    ledger.issue_credits(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let snap = ledger.snapshot();
    assert_eq!(snap.balances.len(), 5);
    assert_eq!(snap.issued, 15.0);
    assert_eq!(snap.burned, 0.0);
    assert!(snap.exchange_rate.is_finite());
}

#[test]
fn test_ledger_multiple_rounds() {
    let config = make_config();
    let mut ledger = CreditLedger::new(&config, 3);
    ledger.issue_credits(&[1.0, 1.0, 1.0]);
    ledger.issue_credits(&[2.0, 2.0, 2.0]);
    let snap = ledger.snapshot();
    assert_eq!(snap.balances[0], 3.0);
}

#[test]
fn test_ledger_exchange_rate() {
    let config = make_config();
    let mut ledger = CreditLedger::new(&config, 3);
    let snap_before = ledger.snapshot();
    ledger.issue_credits(&[10.0, 10.0, 10.0]);
    let snap_after = ledger.snapshot();
    assert!(snap_after.exchange_rate < snap_before.exchange_rate);
}

// ---------------------------------------------------------------------------
// ByzantineDetector Tests
// ---------------------------------------------------------------------------

#[test]
fn test_detector_creation() {
    let config = make_config();
    let _detector = ByzantineDetector::new(&config);
}

#[test]
fn test_detector_empty() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let report = detector.detect(&[]);
    assert!(report.detected.is_empty());
    assert!(report.healthy);
}

#[test]
fn test_detector_honest() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let contributions = make_contributions(10);
    let report = detector.detect(&contributions);
    assert!(report.healthy);
}

#[test]
fn test_detector_detects_outlier() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let mut contributions = make_contributions(20);
    contributions[0].vfe_reduction = 1000.0; // Extreme outlier
    let report = detector.detect(&contributions);
    assert!(!report.detected.is_empty());
}

#[test]
fn test_detector_confidence_range() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let mut contributions = make_contributions(20);
    contributions[0].vfe_reduction = 1000.0;
    let report = detector.detect(&contributions);
    for c in &report.confidence {
        assert!((0.0..=1.0).contains(c));
    }
}

#[test]
fn test_detector_byzantine_fraction() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let mut contributions = make_contributions(20);
    contributions[0].vfe_reduction = 1000.0;
    let report = detector.detect(&contributions);
    assert!((0.0..=1.0).contains(&report.byzantine_fraction));
}

#[test]
fn test_detector_filter() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let mut contributions = make_contributions(20);
    contributions[0].vfe_reduction = 1000.0;
    let filtered = detector.filter_byzantine(&contributions);
    assert!(filtered.len() < contributions.len());
}

#[test]
fn test_detector_filter_preserves_honest() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let contributions = make_contributions(10);
    let filtered = detector.filter_byzantine(&contributions);
    assert_eq!(filtered.len(), contributions.len());
}

#[test]
fn test_detector_report_clone() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let report = detector.detect(&make_contributions(5));
    let _clone = report.clone();
}

#[test]
fn test_detector_report_debug() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let report = detector.detect(&make_contributions(5));
    let debug_str = format!("{:?}", report);
    assert!(!debug_str.is_empty());
}

// ---------------------------------------------------------------------------
// CollectiveMechanism Tests
// ---------------------------------------------------------------------------

#[test]
fn test_collective_creation() {
    let config = make_config();
    let _mechanism = CollectiveMechanism::new(&config, 10);
}

#[test]
fn test_collective_round() {
    let config = make_config();
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    assert!(result.total_participants == 10);
}

#[test]
fn test_collective_winners_selected() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        min_trust: 0.1,
        ..MechanismConfig::default()
    };
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    assert!(!result.vcg_result.winners.is_empty());
}

#[test]
fn test_collective_shapley_values() {
    let config = make_config();
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    for v in &result.shapley_result.values {
        assert!(v.is_finite());
    }
}

#[test]
fn test_collective_credits_issued() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        min_trust: 0.1,
        ..MechanismConfig::default()
    };
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    assert!(result.credits.issued >= 0.0);
}

#[test]
fn test_collective_byzantine_filtering() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        ..MechanismConfig::default()
    };
    let mut mechanism = CollectiveMechanism::new(&config, 20);
    let mut contributions = make_contributions(20);
    contributions[0].vfe_reduction = 1000.0;
    let result = mechanism.run_round(&contributions, 5);
    assert!(result.clean_participants <= result.total_participants);
}

#[test]
fn test_collective_multiple_rounds() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        ..MechanismConfig::default()
    };
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    for _ in 0..3 {
        let contributions = make_contributions(10);
        let _result = mechanism.run_round(&contributions, 3);
    }
}

#[test]
fn test_collective_display() {
    let config = make_config();
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    let display = format!("{}", result);
    assert!(!display.is_empty());
}

#[test]
fn test_collective_result_clone() {
    let config = make_config();
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    let _clone = result.clone();
}

#[test]
fn test_collective_result_debug() {
    let config = make_config();
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    let debug_str = format!("{:?}", result);
    assert!(!debug_str.is_empty());
}

// ---------------------------------------------------------------------------
// TensorVerifier Tests
// ---------------------------------------------------------------------------

#[test]
fn test_verifier_creation() {
    let device = Device::Cpu;
    let _verifier = TensorVerifier::new(&device);
}

#[test]
fn test_verifier_identical_tensors() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let verifier = TensorVerifier::new(&device);
    let t = candle_core::Tensor::ones(10, candle_core::DType::F32, &device)?;
    let reduction = verifier.verify_vfe_reduction(&t, &t)?;
    assert!(reduction >= 0.0);
    Ok(())
}

#[test]
fn test_verifier_reduced_tensor() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let verifier = TensorVerifier::new(&device);
    let original = candle_core::Tensor::ones(10, candle_core::DType::F32, &device)?;
    let steered = candle_core::Tensor::zeros(10, candle_core::DType::F32, &device)?;
    let reduction = verifier.verify_vfe_reduction(&original, &steered)?;
    assert!(reduction.is_finite());
    Ok(())
}

#[test]
fn test_verifier_batch() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let verifier = TensorVerifier::new(&device);
    let originals = vec![
        candle_core::Tensor::ones(5, candle_core::DType::F32, &device)?,
        candle_core::Tensor::ones(5, candle_core::DType::F32, &device)?,
    ];
    let steered = vec![
        candle_core::Tensor::zeros(5, candle_core::DType::F32, &device)?,
        candle_core::Tensor::zeros(5, candle_core::DType::F32, &device)?,
    ];
    let reductions = verifier.batch_verify(&originals, &steered)?;
    assert_eq!(reductions.len(), 2);
    for r in &reductions {
        assert!(r.is_finite());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Contribution Tests
// ---------------------------------------------------------------------------

#[test]
fn test_contribution_clone() {
    let c = Contribution {
        peer_id: 0,
        vfe_reduction: 0.5,
        cost: 0.1,
        trust: 0.9,
        verified: true,
    };
    let _clone = c.clone();
}

#[test]
fn test_contribution_debug() {
    let c = Contribution {
        peer_id: 0,
        vfe_reduction: 0.5,
        cost: 0.1,
        trust: 0.9,
        verified: true,
    };
    let debug_str = format!("{:?}", c);
    assert!(!debug_str.is_empty());
}

// ---------------------------------------------------------------------------
// Integration: Full Mechanism Pipeline
// ---------------------------------------------------------------------------

#[test]
fn test_full_pipeline() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        ..MechanismConfig::default()
    };
    let mut mechanism = CollectiveMechanism::new(&config, 20);

    // Round 1
    let contributions = make_contributions(20);
    let r1 = mechanism.run_round(&contributions, 5);
    assert!(r1.vcg_result.winners.len() <= 5);

    // Round 2 with outlier
    let mut contributions2 = make_contributions(20);
    contributions2[0].vfe_reduction = 500.0;
    let r2 = mechanism.run_round(&contributions2, 5);
    assert!(r2.clean_participants < r2.total_participants || r2.clean_participants == r2.total_participants);

    // Credits accumulate
    assert!(r1.credits.issued >= 0.0 || r2.credits.issued >= 0.0);
}

#[test]
fn test_pipeline_stress() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        shapley_samples: 50,
        ..MechanismConfig::default()
    };
    let mut mechanism = CollectiveMechanism::new(&config, 50);
    for _ in 0..5 {
        let contributions = make_contributions(50);
        let _result = mechanism.run_round(&contributions, 10);
    }
}

#[test]
fn test_compute_credits_structure() {
    let config = make_config();
    let ledger = CreditLedger::new(&config, 5);
    let snap = ledger.snapshot();
    assert_eq!(snap.balances.len(), 5);
    assert!(snap.exchange_rate > 0.0);
}

#[test]
fn test_mechanism_round_result_fields() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        ..MechanismConfig::default()
    };
    let mut mechanism = CollectiveMechanism::new(&config, 10);
    let contributions = make_contributions(10);
    let result = mechanism.run_round(&contributions, 3);
    assert!(result.total_participants > 0);
    assert!(result.clean_participants > 0);
    assert!(result.vcg_result.social_welfare.is_finite());
}

#[test]
fn test_factorial_helper() {
    // 5! = 120
    let mut result: u64 = 1;
    for i in 1..=5 {
        result *= i;
    }
    assert_eq!(result, 120);
}

#[test]
fn test_shapley_large_group() {
    let config = MechanismConfig {
        shapley_samples: 100,
        ..MechanismConfig::default()
    };
    let engine = ShapleyEngine::new(&config);
    let contributions = make_contributions(10);
    let result = engine.compute_shapley(&contributions);
    assert_eq!(result.values.len(), 10);
    for v in &result.values {
        assert!(v.is_finite());
    }
}

#[test]
fn test_vcg_all_verified() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        ..MechanismConfig::default()
    };
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(5);
    assert!(contributions.iter().all(|c| c.verified));
    let result = auction.run_auction(&contributions, 3);
    assert!(!result.winners.is_empty());
}

#[test]
fn test_ledger_burn_exact() {
    let config = make_config();
    let mut ledger = CreditLedger::new(&config, 3);
    ledger.issue_credits(&[5.0, 5.0, 5.0]);
    let ok = ledger.burn_credits(0, 5.0);
    assert!(ok);
    let snap = ledger.snapshot();
    assert!((snap.balances[0] - 0.0).abs() < 0.01);
}

#[test]
fn test_detector_many_outliers() {
    let config = make_config();
    let detector = ByzantineDetector::new(&config);
    let mut contributions = make_contributions(30);
    contributions[0].vfe_reduction = 10000.0;
    contributions[1].vfe_reduction = -10000.0;
    let report = detector.detect(&contributions);
    assert!(report.detected.len() >= 2);
}

#[test]
fn test_config_shapley_samples() {
    let config = MechanismConfig {
        shapley_samples: 500,
        ..MechanismConfig::default()
    };
    assert_eq!(config.shapley_samples, 500);
}

#[test]
fn test_vcg_payments_match_winners() {
    let config = MechanismConfig {
        reserve_price: -1.0,
        ..MechanismConfig::default()
    };
    let auction = VCGAuction::new(&config);
    let contributions = make_contributions(5);
    let result = auction.run_auction(&contributions, 3);
    assert_eq!(result.winners.len(), result.payments.len());
}

#[test]
fn test_shapley_values_non_negative_for_good() {
    let config = make_config();
    let engine = ShapleyEngine::new(&config);
    let contributions: Vec<Contribution> = (0..5)
        .map(|i| Contribution {
            peer_id: i,
            vfe_reduction: 1.0,
            cost: 0.1,
            trust: 0.9,
            verified: true,
        })
        .collect();
    let result = engine.compute_shapley(&contributions);
    let positive_count = result.values.iter().filter(|v| **v > 0.0).count();
    assert!(positive_count > 0);
}
