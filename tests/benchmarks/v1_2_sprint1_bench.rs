//! v1.2.0 Sprint 1 Benchmarks
//!
//! Benchmarks de rendimiento para Multi-Chain Registry, Cross-Chain Identity,
//! SAE Fine-Tuning Engine, Gradient Aggregation v3 y Ethical Constraint Engine.
//!
//! Feature-gated: `--features v1.2-sprint1`
//! No harness: benchmarks manuales con timing explícito.

#![cfg(feature = "v1.2-sprint1")]

use ed2kia::alignment::ethical_constraint_engine::{
    ConstraintEngine, ConstraintSeverity, ConstraintType, EthicalConstraint,
};
use ed2kia::federation::cross_chain_identity::CrossChainIdentity;
use ed2kia::federation::gradient_aggregator_v3::{AggregatorConfig, GradientAggregatorV3};
use ed2kia::federation::multi_chain_registry::{
    ChainConfig, ChainProtocol, ChainState, MultiChainRegistry,
};
use ed2kia::sae::fine_tuning_engine::{FineTuningConfig, FineTuningEngine, LearningRateSchedule};
use std::time::Instant;

// ============================================================================
// Helpers
// ============================================================================

fn bench(name: &str, f: impl FnOnce()) -> f64 {
    let start = Instant::now();
    f();
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    println!("  {:<50} {:8.2} ms", name, elapsed);
    elapsed
}

// ============================================================================
// LP-46: Multi-Chain Registry Benchmarks
// ============================================================================

fn bench_multi_chain_registry_100_chains() {
    let elapsed = bench("Registry: register 100 chains", || {
        let mut registry = MultiChainRegistry::new();
        for i in 0..100 {
            let config = ChainConfig::new(
                format!("chain-{}", i),
                format!("https://chain-{}.example.com", i),
                match i % 3 {
                    0 => ChainProtocol::Ethereum,
                    1 => ChainProtocol::Solana,
                    _ => ChainProtocol::Polkadot,
                },
            );
            let _ = registry.register_chain(config);
        }
    });
    assert!(elapsed < 100.0, "Registry 100 chains exceeded 100ms");
}

fn bench_multi_chain_registry_health_check() {
    let elapsed = bench("Registry: health check 100 chains", || {
        let mut registry = MultiChainRegistry::new();
        for i in 0..100 {
            let config = ChainConfig::new(
                format!("chain-{}", i),
                format!("https://chain-{}.example.com", i),
                ChainProtocol::Ethereum,
            );
            let _ = registry.register_chain(config);
            registry.update_state(&format!("chain-{}", i), ChainState::Connected);
        }
        let _ = registry.health_check();
    });
    assert!(elapsed < 100.0, "Registry health check exceeded 100ms");
}

fn bench_multi_chain_registry_active_filter() {
    let elapsed = bench("Registry: filter active chains (100)", || {
        let mut registry = MultiChainRegistry::new();
        for i in 0..100 {
            let config = ChainConfig::new(
                format!("chain-{}", i),
                format!("https://chain-{}.example.com", i),
                ChainProtocol::Ethereum,
            );
            let _ = registry.register_chain(config);
            if i % 2 == 0 {
                registry.update_state(&format!("chain-{}", i), ChainState::Connected);
            } else {
                registry.update_state(&format!("chain-{}", i), ChainState::Disconnected);
            }
        }
        let _ = registry.get_active_chains();
    });
    assert!(elapsed < 100.0, "Registry active filter exceeded 100ms");
}

// ============================================================================
// LP-47: Cross-Chain Identity Benchmarks
// ============================================================================

fn bench_cross_chain_identity_derive_500() {
    let elapsed = bench("Identity: derive 500 chain keys", || {
        let seed = [0x42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let identity = CrossChainIdentity::new("bench-node".to_string(), signing_key);
        for i in 0..500 {
            let _ = identity.derive_chain_key(&format!("chain-{}", i));
        }
    });
    assert!(elapsed < 100.0, "Identity derive 500 keys exceeded 100ms");
}

fn bench_cross_chain_identity_sign_verify_10() {
    let elapsed = bench("Identity: sign + verify 10 messages", || {
        let seed = [0x42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let identity = CrossChainIdentity::new("bench-node".to_string(), signing_key);
        for i in 0..10 {
            let message = format!("message-{}", i);
            let signature = identity.sign_message(message.as_bytes());
            let _ = identity.verify_signature("default", message.as_bytes(), &signature.to_bytes());
        }
    });
    assert!(elapsed < 100.0, "Identity sign+verify 10 exceeded 100ms");
}

fn bench_cross_chain_identity_proof_generation() {
    let elapsed = bench("Identity: generate 20 proofs", || {
        let seed = [0x42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let identity = CrossChainIdentity::new("bench-node".to_string(), signing_key);
        for i in 0..20 {
            let _ = identity.generate_proof(&format!("chain-{}", i % 10));
        }
    });
    assert!(elapsed < 100.0, "Identity proof generation exceeded 100ms");
}

// ============================================================================
// LP-48: SAE Fine-Tuning Engine Benchmarks
// ============================================================================

fn bench_fine_tuning_epoch_cycle() {
    let elapsed = bench("Fine-tuning: 100 epochs with 10 batches each", || {
        let config = FineTuningConfig {
            learning_rate: 0.01,
            schedule: LearningRateSchedule::CosineDecay { max_steps: 1000 },
            batch_size: 32,
            max_epochs: 200,
            convergence_threshold: 0.001,
        };
        let mut engine = FineTuningEngine::new(config);
        for epoch in 0..100usize {
            let _ = engine.start_epoch();
            for batch in 0..10 {
                let exponent = -((epoch * 10 + batch + 1) as i32);
                let loss = 0.5 * 2f32.powi(exponent);
                let _ = engine.record_batch(loss, 1.0);
            }
        }
    });
    assert!(elapsed < 100.0, "Fine-tuning epoch cycle exceeded 100ms");
}

fn bench_fine_tuning_checkpoint_100() {
    let elapsed = bench("Fine-tuning: create 100 checkpoints", || {
        let config = FineTuningConfig {
            learning_rate: 0.01,
            schedule: LearningRateSchedule::Constant,
            batch_size: 32,
            max_epochs: 200,
            convergence_threshold: 0.001,
        };
        let mut engine = FineTuningEngine::new(config);
        for i in 0..100 {
            let _ = engine.start_epoch();
            let _ = engine.record_batch(0.5 - i as f32 * 0.001, 1.0);
            let _ = engine.create_checkpoint();
        }
    });
    assert!(elapsed < 100.0, "Fine-tuning checkpoints exceeded 100ms");
}

// ============================================================================
// LP-49: Gradient Aggregation v3 Benchmarks
// ============================================================================

fn bench_gradient_aggregation_100_participants() {
    let elapsed = bench("Aggregator: 100 participants × 1024-dim gradients", || {
        let config = AggregatorConfig {
            compression_ratio: 0.5,
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: true,
        };
        let mut aggregator = GradientAggregatorV3::new(config);
        for i in 0..100 {
            let gradient: Vec<f32> = (0..1024).map(|j| (i + j) as f32 * 0.001).collect();
            let _ = aggregator.submit_gradient(format!("node-{}", i), gradient);
        }
        let _ = aggregator.aggregate();
    });
    assert!(
        elapsed < 100.0,
        "Gradient aggregation 100 participants exceeded 100ms"
    );
}

fn bench_gradient_aggregation_outlier_detection() {
    let elapsed = bench("Aggregator: outlier detection (1000 gradients)", || {
        let config = AggregatorConfig {
            compression_ratio: 1.0,
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let aggregator = GradientAggregatorV3::new(config);
        let gradients: Vec<Vec<f32>> = (0..1000)
            .map(|i| (0..64).map(|j| (i + j) as f32 * 0.01).collect())
            .collect();
        let slices: Vec<&[f32]> = gradients.iter().map(|v| v.as_slice()).collect();
        let _ = aggregator.detect_outliers(&slices);
    });
    assert!(elapsed < 100.0, "Outlier detection exceeded 100ms");
}

fn bench_gradient_aggregation_multi_round() {
    let elapsed = bench("Aggregator: 50 rounds × 10 participants", || {
        let config = AggregatorConfig {
            compression_ratio: 0.8,
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let mut aggregator = GradientAggregatorV3::new(config);
        for round in 0..50u64 {
            for i in 0..10 {
                let gradient = vec![(round + i as u64) as f32 * 0.1; 128];
                let _ = aggregator.submit_gradient(format!("node-{}", i), gradient);
            }
            let _ = aggregator.aggregate();
            aggregator.reset_round();
        }
    });
    assert!(elapsed < 100.0, "Multi-round aggregation exceeded 100ms");
}

// ============================================================================
// LP-50: Ethical Constraint Engine Benchmarks
// ============================================================================

fn bench_constraint_validation_1000_gradients() {
    let elapsed = bench(
        "Constraints: validate 1000 gradients (5 constraints)",
        || {
            let mut engine = ConstraintEngine::new();
            // Add 5 constraints
            for i in 0..5 {
                let constraint = EthicalConstraint::new(
                    format!("bound-{}", i),
                    ConstraintType::ValueBound {
                        min: -2.0,
                        max: 2.0,
                        feature_index: i,
                    },
                    ConstraintSeverity::Warning,
                );
                engine.add_constraint(constraint);
            }
            for _ in 0..1000 {
                let gradient = vec![0.5f32; 10];
                let _ = engine.validate_gradient(&gradient);
            }
        },
    );
    assert!(elapsed < 100.0, "Constraint validation exceeded 100ms");
}

fn bench_constraint_correction_1000_gradients() {
    let elapsed = bench(
        "Constraints: correct 1000 gradients (5 constraints)",
        || {
            let mut engine = ConstraintEngine::new();
            for i in 0..5 {
                let constraint = EthicalConstraint::new(
                    format!("bound-{}", i),
                    ConstraintType::ValueBound {
                        min: -1.0,
                        max: 1.0,
                        feature_index: i,
                    },
                    ConstraintSeverity::Correction,
                );
                engine.add_constraint(constraint);
            }
            for _ in 0..1000 {
                let mut gradient = vec![5.0f32; 10];
                let _ = engine.correct_gradient(&mut gradient);
            }
        },
    );
    assert!(elapsed < 100.0, "Constraint correction exceeded 100ms");
}

fn bench_constraint_mixed_types() {
    let elapsed = bench("Constraints: validate with mixed constraint types", || {
        let mut engine = ConstraintEngine::new();
        engine.add_constraint(EthicalConstraint::new(
            "value-bound".to_string(),
            ConstraintType::ValueBound {
                min: -1.0,
                max: 1.0,
                feature_index: 0,
            },
            ConstraintSeverity::Warning,
        ));
        engine.add_constraint(EthicalConstraint::new(
            "feature-mask".to_string(),
            ConstraintType::FeatureMask {
                masked_features: vec![1, 3, 5],
            },
            ConstraintSeverity::Correction,
        ));
        engine.add_constraint(EthicalConstraint::new(
            "divergence".to_string(),
            ConstraintType::DivergenceLimit {
                max_divergence: 10.0,
            },
            ConstraintSeverity::Warning,
        ));
        for _ in 0..500 {
            let gradient = vec![0.0f32; 8];
            let _ = engine.validate_gradient(&gradient);
        }
    });
    assert!(elapsed < 100.0, "Mixed constraint types exceeded 100ms");
}

// ============================================================================
// Full Pipeline Benchmarks
// ============================================================================

fn bench_full_pipeline_registry_identity() {
    let elapsed = bench("Pipeline: registry + identity (50 chains)", || {
        let mut registry = MultiChainRegistry::new();
        for i in 0..50 {
            let config = ChainConfig::new(
                format!("chain-{}", i),
                format!("https://chain-{}.example.com", i),
                ChainProtocol::Ethereum,
            );
            let _ = registry.register_chain(config);
        }
        let seed = [0x42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let mut identity = CrossChainIdentity::new("pipeline-node".to_string(), signing_key);
        for i in 0..50 {
            let key = identity.derive_chain_key(&format!("chain-{}", i)).unwrap();
            identity.register_chain_key(&format!("chain-{}", i), key);
        }
        let _ = registry.get_active_chains();
    });
    assert!(elapsed < 100.0, "Registry+identity pipeline exceeded 100ms");
}

fn bench_full_pipeline_training_aggregation() {
    let elapsed = bench("Pipeline: training + aggregation (10 nodes)", || {
        // Train locally on 10 nodes
        let mut aggregators = Vec::new();
        for i in 0..10 {
            let config = FineTuningConfig {
                learning_rate: 0.01,
                schedule: LearningRateSchedule::Constant,
                batch_size: 32,
                max_epochs: 5,
                convergence_threshold: 0.01,
            };
            let mut engine = FineTuningEngine::new(config);
            for _ in 0..5 {
                let _ = engine.start_epoch();
                let _ = engine.record_batch(0.5 - i as f32 * 0.01, 1.0);
            }
            aggregators.push(engine);
        }

        // Aggregate gradients
        let agg_config = AggregatorConfig {
            compression_ratio: 0.8,
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let mut aggregator = GradientAggregatorV3::new(agg_config);
        for (i, engine) in aggregators.iter().enumerate() {
            let state = engine.get_state();
            let gradient = vec![state.current_loss; 64];
            let _ = aggregator.submit_gradient(format!("node-{}", i), gradient);
        }
        let _ = aggregator.aggregate();
    });
    assert!(
        elapsed < 100.0,
        "Training+aggregation pipeline exceeded 100ms"
    );
}

fn bench_full_pipeline_complete() {
    let elapsed = bench(
        "Pipeline: complete (registry→identity→train→agg→constraints)",
        || {
            // Registry
            let mut registry = MultiChainRegistry::new();
            for i in 0..20 {
                let config = ChainConfig::new(
                    format!("chain-{}", i),
                    format!("https://chain-{}.example.com", i),
                    ChainProtocol::Ethereum,
                );
                let _ = registry.register_chain(config);
            }

            // Identity
            let seed = [0x42u8; 32];
            let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
            let identity = CrossChainIdentity::new("full-node".to_string(), signing_key);
            let _ = identity.derive_chain_key("chain-0");

            // Training
            let config = FineTuningConfig {
                learning_rate: 0.01,
                schedule: LearningRateSchedule::Constant,
                batch_size: 32,
                max_epochs: 10,
                convergence_threshold: 0.01,
            };
            let mut engine = FineTuningEngine::new(config);
            for _ in 0..5 {
                let _ = engine.start_epoch();
                let _ = engine.record_batch(0.3, 1.0);
            }

            // Aggregation
            let agg_config = AggregatorConfig {
                compression_ratio: 1.0,
                outlier_threshold: 2.0,
                min_participants: 1,
                use_reputation_weights: false,
            };
            let mut aggregator = GradientAggregatorV3::new(agg_config);
            let gradient = vec![0.01; 64];
            let _ = aggregator.submit_gradient("full-node".to_string(), gradient);
            let _ = aggregator.aggregate();

            // Constraints
            let mut constraint_engine = ConstraintEngine::new();
            constraint_engine.add_constraint(EthicalConstraint::new(
                "final-bound".to_string(),
                ConstraintType::ValueBound {
                    min: -1.0,
                    max: 1.0,
                    feature_index: 0,
                },
                ConstraintSeverity::Warning,
            ));
            let _ = constraint_engine.validate_gradient(&[0.5]);
        },
    );
    assert!(elapsed < 100.0, "Complete pipeline exceeded 100ms");
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("=== ed2kIA v1.2.0 Sprint 1 Benchmarks ===\n");

    println!("--- Multi-Chain Registry ---");
    bench_multi_chain_registry_100_chains();
    bench_multi_chain_registry_health_check();
    bench_multi_chain_registry_active_filter();

    println!("\n--- Cross-Chain Identity ---");
    bench_cross_chain_identity_derive_500();
    bench_cross_chain_identity_sign_verify_10();
    bench_cross_chain_identity_proof_generation();

    println!("\n--- SAE Fine-Tuning Engine ---");
    bench_fine_tuning_epoch_cycle();
    bench_fine_tuning_checkpoint_100();

    println!("\n--- Gradient Aggregation v3 ---");
    bench_gradient_aggregation_100_participants();
    bench_gradient_aggregation_outlier_detection();
    bench_gradient_aggregation_multi_round();

    println!("\n--- Ethical Constraint Engine ---");
    bench_constraint_validation_1000_gradients();
    bench_constraint_correction_1000_gradients();
    bench_constraint_mixed_types();

    println!("\n--- Full Pipeline ---");
    bench_full_pipeline_registry_identity();
    bench_full_pipeline_training_aggregation();
    bench_full_pipeline_complete();

    println!("\n=== Benchmarks completados ===");
}
