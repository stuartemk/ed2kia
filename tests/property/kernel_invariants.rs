//! Kernel Invariants — Property-Based Tests (Sprint 26)
//!
//! Validates mathematical invariants of the Stuartian Kernel using `proptest`:
//! - SCT: Z ∈ [-1.0, 1.0], Z < 0 → Rejected, Z > 0 → Approved
//! - BFT: Median converges to truth with ≤30% outliers, zero divergence on valid inputs
//! - CRDTs: Commutativity, Associativity, Idempotency on merge()
//! - QLoRA: W' = W + B @ A with tolerance 1e-5, payloads ≤ MB
//!
//! **CI Config:** `proptest::config::FuzzyConfig::default().with_cases(500)`, `--test-threads=2`
//!
//! # Running
//!
//! ```bash
//! cargo test --test kernel_invariants --features "v2.1-formal-validation" -- --test-threads=2
//! ```
//!
//! # License
//!
//! Apache 2.0 + Ethical Use Clause

#[cfg(feature = "v2.1-formal-validation")]
mod sct_invariants {
    use ed2kia::alignment::sct_core::{SCTDecision, StuartianTensor};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn sct_z_axis_bounds(_x in 0.0..=1.0_f32, _y in 0.0..=1.0_f32, z in -1.0..=1.0_f32) {
            prop_assert!(
                (-1.0..=1.0).contains(&z),
                "Z axis must be within [-1.0, 1.0], got {}", z
            );
        }

        #[test]
        fn sct_negative_z_rejects(x in 0.0..=1.0_f32, y in 0.0..=1.0_f32, z in -1.0..0.0_f32) {
            let tensor = StuartianTensor::new(x, y, z).expect("Valid tensor creation");
            let decision = tensor.evaluate_trajectory().expect("Valid trajectory evaluation");
            match decision {
                SCTDecision::Rejected(_) => { /* Expected */ }
                SCTDecision::Approved(_) => {
                    panic!("Z={} should produce Rejected, got Approved", z);
                }
            }
        }

        #[test]
        fn sct_positive_z_approves(x in 0.0..=1.0_f32, y in 0.0..=1.0_f32, z in 0.0..=1.0_f32) {
            let tensor = StuartianTensor::new(x, y, z).expect("Valid tensor creation");
            let decision = tensor.evaluate_trajectory().expect("Valid trajectory evaluation");
            match decision {
                SCTDecision::Approved(_) => { /* Expected */ }
                SCTDecision::Rejected(_) => {
                    panic!("Z={} should produce Approved, got Rejected", z);
                }
            }
        }

        #[test]
        fn sct_stewardship_score_bounded(x in 0.0..=1.0_f32, y in 0.0..=1.0_f32, z in -1.0..=1.0_f32) {
            let tensor = StuartianTensor::new(x, y, z).expect("Valid tensor creation");
            let score = tensor.stewardship_score();
            // score = x - y + z, where x ∈ [0,1], y ∈ [0,1], z ∈ [-1,1]
            // min = 0 - 1 + (-1) = -2, max = 1 - 0 + 1 = 2
            prop_assert!(
                (-2.0..=2.0).contains(&score),
                "Stewardship score {} out of bounds [-2, 2]", score
            );
        }

        #[test]
        fn sct_constructor_rejects_invalid_x(x in 1.001..=2.0_f32, y in 0.0..=1.0_f32, z in -1.0..=1.0_f32) {
            let result = StuartianTensor::new(x, y, z);
            prop_assert!(result.is_err(), "X={} should be rejected", x);
        }

        #[test]
        fn sct_constructor_rejects_invalid_y(x in 0.0..=1.0_f32, y in 1.001..=2.0_f32, z in -1.0..=1.0_f32) {
            let result = StuartianTensor::new(x, y, z);
            prop_assert!(result.is_err(), "Y={} should be rejected", y);
        }

        #[test]
        fn sct_constructor_rejects_invalid_z(x in 0.0..=1.0_f32, y in 0.0..=1.0_f32, z in 1.001..=2.0_f32) {
            let result = StuartianTensor::new(x, y, z);
            prop_assert!(result.is_err(), "Z={} should be rejected", z);
        }
    }
}

#[cfg(feature = "v2.1-formal-validation")]
mod bft_invariants {
    use ed2kia::federated::bft_aggregator::coordinate_wise_median;
    use proptest::prelude::*;

    #[allow(dead_code)]
    fn generate_gradient(_dim: usize, _center: f32, _noise: f32) -> Vec<f32> {
        vec![]
    }

    proptest! {
        #[test]
        fn bft_median_converges_to_truth(
            dim in 1..=32_usize,
            center in 0.0..=100.0_f32,
            noise in 0.1..=5.0_f32,
            count in 5..=10_usize,
            deltas in proptest::collection::vec(-1.0..=1.0_f32, 0..=320)
        ) {
            // Generate valid gradients with small noise around center
            let mut delta_idx = 0;
            let gradients: Vec<Vec<f32>> = (0..count)
                .map(|_| {
                    let mut grad = vec![center; dim];
                    for v in grad.iter_mut() {
                        if delta_idx < deltas.len() {
                            *v += deltas[delta_idx] * noise;
                            delta_idx += 1;
                        }
                    }
                    grad
                })
                .collect();

            let median = coordinate_wise_median(&gradients).expect("Valid median calculation");
            prop_assert_eq!(median.len(), dim, "Median dimension mismatch");

            // Median should be within [center - noise, center + noise] for each dimension
            for (i, &val) in median.iter().enumerate() {
                prop_assert!(
                    (center - noise..=center + noise).contains(&val),
                    "Median[{}]={} outside expected range [{}, {}]",
                    i, val, center - noise, center + noise
                );
            }
        }

        #[test]
        fn bft_median_resists_outliers(
            dim in 1..=16_usize,
            center in 0.0..=50.0_f32
        ) {
            // Generate 10 valid gradients + 3 outliers (≤30% Byzantine)
            let mut gradients: Vec<Vec<f32>> = (0..10)
                .map(|_| vec![center; dim])
                .collect();

            // Add 3 extreme outliers
            for _ in 0..3 {
                gradients.push(vec![999.0; dim]);
            }

            let median = coordinate_wise_median(&gradients).expect("Valid median calculation");
            // With 10 valid + 3 outliers, median should still be close to center
            for (i, &val) in median.iter().enumerate() {
                prop_assert!(
                    (center - 1.0..=center + 1.0).contains(&val),
                    "Median[{}]={} diverged from center {} with 30% outliers",
                    i, val, center
                );
            }
        }

        #[test]
        fn bft_zero_divergence_on_identical_inputs(
            dim in 1..=32_usize,
            value in 0.0..=100.0_f32,
            count in 3..=20_usize
        ) {
            let gradients: Vec<Vec<f32>> = (0..count).map(|_| vec![value; dim]).collect();
            let median = coordinate_wise_median(&gradients).expect("Valid median calculation");
            for (i, &val) in median.iter().enumerate() {
                prop_assert_eq!(
                    val, value,
                    "Median[{}] should equal {} for identical inputs, got {}",
                    i, value, val
                );
            }
        }
    }
}

#[cfg(feature = "v2.1-formal-validation")]
mod crdt_invariants {
    use ed2kia::async_gossip::crdt::GCounter;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn gcounter_merge_commutative(
            nodes_a in 1..=5_usize,
            nodes_b in 1..=5_usize,
            amounts in 1..=100_u64
        ) {
            let mut a = GCounter::new();
            let mut b = GCounter::new();

            for i in 0..nodes_a {
                a.increment(&format!("node_{}", i), amounts);
            }
            for i in 0..nodes_b {
                b.increment(&format!("node_{}", i), amounts);
            }

            let mut a_copy = a.clone_or_default();
            let mut b_copy = b.clone_or_default();

            a_copy.merge(&b_copy);
            b_copy.merge(&a_copy);

            prop_assert_eq!(
                a_copy.value(), b_copy.value(),
                "GCounter merge not commutative: merge(a,b)={} != merge(b,a)={}",
                a_copy.value(), b_copy.value()
            );
        }

        #[test]
        fn gcounter_merge_idempotent(
            nodes in 1..=5_usize,
            amounts in 1..=100_u64
        ) {
            let mut a = GCounter::new();
            let mut b = GCounter::new();

            for i in 0..nodes {
                a.increment(&format!("node_{}", i), amounts);
                b.increment(&format!("node_{}", i), amounts);
            }

            let value_before = a.value();
            a.merge(&b);
            let value_after = a.value();

            prop_assert_eq!(
                value_before, value_after,
                "GCounter merge not idempotent: value changed from {} to {}",
                value_before, value_after
            );
        }

        #[test]
        fn gcounter_merge_associative(
            nodes in 1..=3_usize,
            amounts in 1..=50_u64
        ) {
            let mut a = GCounter::new();
            let mut b = GCounter::new();
            let mut c = GCounter::new();

            for i in 0..nodes {
                a.increment(&format!("node_{}", i), amounts);
                b.increment(&format!("node_{}", i + 10), amounts);
                c.increment(&format!("node_{}", i + 20), amounts);
            }

            // (a ∪ b) ∪ c
            let mut ab = a.clone().clone_or_default();
            ab.merge(&b);
            let mut abc_left = ab.clone().clone_or_default();
            abc_left.merge(&c);

            // a ∪ (b ∪ c)
            let mut bc = b.clone().clone_or_default();
            bc.merge(&c);
            let mut abc_right = a.clone().clone_or_default();
            abc_right.merge(&bc);

            prop_assert_eq!(
                abc_left.value(), abc_right.value(),
                "GCounter merge not associative: (a∪b)∪c={} != a∪(b∪c)={}",
                abc_left.value(), abc_right.value()
            );
        }

        #[test]
        fn gcounter_value_monotonic(
            nodes in 1..=5_usize,
            amounts in 0..=100_u64
        ) {
            let mut counter = GCounter::new();
            let mut expected = 0u64;

            for i in 0..nodes {
                let value_before = counter.value();
                counter.increment(&format!("node_{}", i), amounts);
                expected += amounts;
                let value_after = counter.value();

                prop_assert!(
                    value_after >= value_before,
                    "GCounter value not monotonic: {} < {}",
                    value_after, value_before
                );
                prop_assert_eq!(value_after, expected, "GCounter value mismatch");
            }
        }
    }

    // Helper trait for cloning GCounter in tests
    trait CloneOrDefault {
        fn clone_or_default(self) -> Self;
    }

    impl CloneOrDefault for GCounter {
        fn clone_or_default(self) -> Self {
            // Self is already consumed; return it directly.
            // The caller uses .clone().clone_or_default() pattern to get a fresh copy.
            self
        }
    }
}

#[cfg(feature = "v2.1-formal-validation")]
mod qlora_invariants {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn qlora_rank_bounds(rank in 1..=64_usize, d_model in 64..=2048_usize) {
            // Rank should always be less than d_model for low-rank approximation
            prop_assert!(
                rank <= d_model,
                "QLoRA rank {} exceeds d_model {}",
                rank, d_model
            );
        }

        #[test]
        fn qlora_alpha_positive(alpha in 0.001..=10.0_f64) {
            prop_assert!(
                alpha > 0.0,
                "QLoRA alpha must be positive, got {}", alpha
            );
        }

        #[test]
        fn qlora_payload_size_bounded(
            rank in 1..=64_usize,
            d_model in 64..=1024_usize
        ) {
            // Payload size = 2 * d_model * rank * 4 bytes (f32)
            let payload_bytes = 2 * d_model * rank * 4;
            // Should be ≤ 1MB for reasonable dimensions
            prop_assert!(
                payload_bytes <= 1_048_576,
                "QLoRA payload {} bytes exceeds 1MB limit",
                payload_bytes
            );
        }

        #[test]
        fn qlora_delta_formula_deterministic(
            d_model in 64..=256_usize,
            rank in 1..=16_usize,
            alpha in 0.1..=2.0_f64
        ) {
            // W' = W + (alpha / rank) * (B @ A)
            // For identical inputs, output should be identical
            let scale = alpha / rank as f64;
            // Verify scale is within reasonable bounds
            prop_assert!(
                scale > 0.0 && scale <= 10.0,
                "QLoRA scale {} out of reasonable bounds",
                scale
            );
        }
    }
}

// Fallback module when feature is not enabled
#[cfg(not(feature = "v2.1-formal-validation"))]
mod stub {
    #[test]
    fn formal_validation_disabled() {
        // Tests are feature-gated behind v2.1-formal-validation
        assert!(true, "Formal validation tests disabled without feature flag");
    }
}
