//! v1.2.0 Sprint 1 E2E Integration Tests
//!
//! Multi-Chain Federation & SAE Distributed Fine-Tuning Foundation
//!
//! Test Scenarios:
//! 1. Multi-chain registry full lifecycle
//! 2. Cross-chain identity key derivation and verification
//! 3. SAE fine-tuning training cycle with convergence
//! 4. Gradient aggregation with outlier detection
//! 5. Ethical constraint enforcement during training
//! 6. Full pipeline: registry → identity → training → aggregation → constraints

#[cfg(feature = "v1.2-sprint1")]
mod e2e {
    use ed2kia::alignment::ethical_constraint_engine::{
        ConstraintEngine, ConstraintSeverity, EthicalConstraint,
    };
    use ed2kia::federation::cross_chain_identity::CrossChainIdentity;
    use ed2kia::federation::gradient_aggregator_v3::{AggregatorConfig, GradientAggregatorV3};
    use ed2kia::federation::multi_chain_registry::{
        ChainConfig, ChainProtocol, ChainState, MultiChainRegistry,
    };
    use ed2kia::sae::fine_tuning_engine::{
        FineTuningConfig, FineTuningEngine, LearningRateSchedule,
    };

    // ─── LP-46: Multi-Chain Registry E2E Tests ───

    #[test]
    fn test_e2e_multi_chain_registry_full_lifecycle() {
        let mut registry = MultiChainRegistry::new();

        // Register chains
        let eth_config = ChainConfig {
            chain_id: "eth-mainnet".to_string(),
            endpoint: "https://eth-mainnet.gateway".to_string(),
            protocol: ChainProtocol::Ethereum,
            parameters: Default::default(),
        };
        let sol_config = ChainConfig {
            chain_id: "sol-mainnet".to_string(),
            endpoint: "https://sol-mainnet.gateway".to_string(),
            protocol: ChainProtocol::Solana,
            parameters: Default::default(),
        };
        let polka_config = ChainConfig {
            chain_id: "polka-mainnet".to_string(),
            endpoint: "https://polka-mainnet.gateway".to_string(),
            protocol: ChainProtocol::Polkadot,
            parameters: Default::default(),
        };

        assert!(registry.register_chain(eth_config).is_ok());
        assert!(registry.register_chain(sol_config).is_ok());
        assert!(registry.register_chain(polka_config).is_ok());
        assert_eq!(registry.chain_count(), 3);

        // Update states
        registry.update_state("eth-mainnet", ChainState::Connected);
        registry.update_state("sol-mainnet", ChainState::Syncing);
        registry.update_state("polka-mainnet", ChainState::Connected);

        // Health check
        let health = registry.health_check();
        assert_eq!(health.len(), 3);
        assert_eq!(*health.get("eth-mainnet").unwrap(), ChainState::Connected);
        assert_eq!(*health.get("sol-mainnet").unwrap(), ChainState::Syncing);

        // Get active chains (Connected | Syncing are both active)
        let active = registry.get_active_chains();
        assert_eq!(active.len(), 3); // Connected + Syncing are active

        // Unregister
        assert!(registry.unregister_chain("sol-mainnet").is_ok());
        assert_eq!(registry.chain_count(), 2);
        assert!(registry.get_chain("sol-mainnet").is_none());
    }

    #[test]
    fn test_e2e_multi_chain_duplicate_registration() {
        let mut registry = MultiChainRegistry::new();
        let config = ChainConfig {
            chain_id: "eth-mainnet".to_string(),
            endpoint: "https://eth-mainnet.gateway".to_string(),
            protocol: ChainProtocol::Ethereum,
            parameters: Default::default(),
        };
        assert!(registry.register_chain(config.clone()).is_ok());
        assert!(registry.register_chain(config).is_err()); // Duplicate
    }

    #[test]
    fn test_e2e_multi_chain_protocol_filtering() {
        let mut registry = MultiChainRegistry::new();
        for i in 0..5 {
            let config = ChainConfig {
                chain_id: format!("eth-{}", i),
                endpoint: format!("https://eth-{}.gateway", i),
                protocol: ChainProtocol::Ethereum,
                parameters: Default::default(),
            };
            registry.register_chain(config).unwrap();
        }
        for i in 0..3 {
            let config = ChainConfig {
                chain_id: format!("sol-{}", i),
                endpoint: format!("https://sol-{}.gateway", i),
                protocol: ChainProtocol::Solana,
                parameters: Default::default(),
            };
            registry.register_chain(config).unwrap();
        }
        assert_eq!(registry.chain_count(), 8);
        let eth_chains = registry.get_chains_by_protocol(&ChainProtocol::Ethereum);
        assert_eq!(eth_chains.len(), 5);
        let sol_chains = registry.get_chains_by_protocol(&ChainProtocol::Solana);
        assert_eq!(sol_chains.len(), 3);
    }

    // ─── LP-47: Cross-Chain Identity E2E Tests ───

    #[test]
    fn test_e2e_cross_chain_identity_key_derivation() {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0x42; 32]);
        let mut identity = CrossChainIdentity::new("node-alpha".to_string(), signing_key);
        assert_eq!(identity.node_id, "node-alpha");
        assert_eq!(identity.chain_count(), 0);

        // Derive + register chain keys (derive returns key, register adds to store)
        let eth_key = identity.derive_chain_key("eth-mainnet").unwrap();
        assert!(!eth_key.is_empty());
        identity.register_chain_key("eth-mainnet", eth_key.clone());
        let sol_key = identity.derive_chain_key("sol-mainnet").unwrap();
        assert!(!sol_key.is_empty());
        identity.register_chain_key("sol-mainnet", sol_key.clone());
        assert_ne!(eth_key, sol_key); // Different chains → different keys
        assert_eq!(identity.chain_count(), 2);
    }

    #[test]
    fn test_e2e_cross_chain_identity_signature_verification() {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0x42; 32]);
        let identity = CrossChainIdentity::new("node-beta".to_string(), signing_key);

        let message = b"cross-chain message";
        let signature = identity.sign_message(message);
        let sig_bytes = signature.to_bytes();
        assert!(identity.verify_signature("default", message, &sig_bytes));

        // Tampered message should fail
        let tampered = b"tampered message";
        assert!(!identity.verify_signature("default", tampered, &sig_bytes));
    }

    #[test]
    fn test_e2e_cross_chain_identity_proof_generation() {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0x42; 32]);
        let mut identity = CrossChainIdentity::new("node-gamma".to_string(), signing_key);
        identity.register_chain_key("eth-mainnet", "0xabc123".to_string());

        let proof = identity.generate_proof("eth-mainnet").unwrap();
        assert_eq!(proof.node_id, "node-gamma");
        assert_eq!(proof.chain_id, "eth-mainnet");
        assert!(!proof.signature.is_empty());
        assert!(!proof.public_key.is_empty());

        // Proof should not be expired immediately
        assert!(!proof.is_expired(60_000));
    }

    #[test]
    fn test_e2e_cross_chain_reputation_aggregation() {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0x42; 32]);
        let mut identity = CrossChainIdentity::new("node-delta".to_string(), signing_key);

        // Build reputation across chains using record_contribution (increments counter)
        for _ in 0..10 {
            identity.reputation.record_contribution("eth-mainnet");
        }
        for _ in 0..5 {
            identity.reputation.record_contribution("sol-mainnet");
        }

        let global = identity.get_global_reputation();
        assert!(global > 0.0);
        assert!(global <= 1.0); // Clamped to [0, 1]
        assert_eq!(identity.reputation.total_contributions, 15);
    }

    // ─── LP-48: SAE Fine-Tuning Engine E2E Tests ───

    #[test]
    fn test_e2e_sae_fine_tuning_training_cycle() {
        let config = FineTuningConfig {
            learning_rate: 0.01,
            schedule: LearningRateSchedule::Constant,
            batch_size: 32,
            max_epochs: 10,
            convergence_threshold: 0.001,
        };
        let mut engine = FineTuningEngine::new(config);

        // Simulate training epochs
        for epoch in 0..5 {
            engine.start_epoch();
            let lr = engine.get_learning_rate();
            assert_eq!(lr, 0.01); // Constant schedule
                                  // Record batches with decreasing loss
            for batch in 0..4 {
                let loss = 0.5 * 2f32.powi(-(epoch * 4 + batch + 1) as i32);
                engine.record_batch(loss, 1.0);
            }
        }

        let state = engine.get_state();
        assert_eq!(state.epoch, 5);
        assert!(state.best_loss < 0.5);
    }

    #[test]
    fn test_e2e_sae_fine_tuning_convergence() {
        let config = FineTuningConfig {
            learning_rate: 0.1,
            schedule: LearningRateSchedule::CosineDecay { max_steps: 100 },
            batch_size: 64,
            max_epochs: 20,
            convergence_threshold: 0.01,
        };
        let mut engine = FineTuningEngine::new(config);

        // Simulate converging training
        for epoch in 0..10 {
            engine.start_epoch();
            let initial_lr = engine.get_learning_rate();
            for _batch in 0..8 {
                let loss = 0.001 * (1.0 / (epoch + 1) as f32);
                engine.record_batch(loss, 0.5);
            }
            if epoch == 0 {
                assert!(engine.get_learning_rate() < initial_lr || epoch > 50);
            }
        }

        let converged = engine.check_convergence();
        assert!(converged);
        let state = engine.get_state();
        assert!(state.is_converged);
    }

    #[test]
    fn test_e2e_sae_fine_tuning_checkpoint_restore() {
        let config = FineTuningConfig {
            learning_rate: 0.005,
            schedule: LearningRateSchedule::StepDecay {
                step_size: 10,
                decay_factor: 0.5,
            },
            batch_size: 16,
            max_epochs: 50,
            convergence_threshold: 0.0001,
        };
        let mut engine = FineTuningEngine::new(config);

        // Train a few epochs
        for _ in 0..3 {
            engine.start_epoch();
            engine.record_batch(0.3, 1.0);
            engine.record_batch(0.25, 0.9);
        }

        // Create checkpoint
        let checkpoint = engine.create_checkpoint();
        assert_eq!(checkpoint.state.epoch, 3);
        assert!(checkpoint.state.best_loss <= 0.3);

        // Continue training
        for _ in 0..2 {
            engine.start_epoch();
            engine.record_batch(0.2, 0.8);
        }

        // Restore checkpoint
        engine.restore_checkpoint(&checkpoint);
        let state = engine.get_state();
        assert_eq!(state.epoch, 3);
        assert_eq!(state.best_loss, checkpoint.state.best_loss);
    }

    // ─── LP-49: Gradient Aggregation v3 E2E Tests ───

    #[test]
    fn test_e2e_gradient_aggregation_with_outlier_detection() {
        let config = AggregatorConfig {
            compression_ratio: 1.0, // No compression for accuracy
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let mut aggregator = GradientAggregatorV3::new(config);

        // Submit gradients from multiple nodes
        for i in 0..10 {
            let gradient = vec![1.0 + (i as f32 % 3.0) * 0.1; 8];
            aggregator
                .submit_gradient(format!("node-{}", i), gradient)
                .unwrap();
        }

        // Aggregate
        let result = aggregator.aggregate().unwrap();
        assert_eq!(result.participant_count, 10);
        assert_eq!(result.aggregated_gradient.len(), 8);
        assert!(result.gradient_norm > 0.0);

        // Detect outliers
        let g1 = vec![1.0; 8];
        let g2 = vec![1.1; 8];
        let g3 = vec![1.0; 8];
        let g4 = vec![100.0; 8]; // Clear outlier
        let gradients = [&g1[..], &g2[..], &g3[..], &g4[..]];
        let outliers = aggregator.detect_outliers(&gradients);
        assert!(outliers.contains(&3));
    }

    #[test]
    fn test_e2e_gradient_aggregation_reputation_weighted() {
        let config = AggregatorConfig {
            compression_ratio: 1.0,
            outlier_threshold: 3.0,
            min_participants: 1,
            use_reputation_weights: true,
        };
        let mut aggregator = GradientAggregatorV3::new(config);

        // Set reputation weights
        aggregator.set_participant_weight("high-rep".to_string(), 5.0);
        aggregator.set_participant_weight("low-rep".to_string(), 1.0);

        // Submit gradients
        aggregator
            .submit_gradient("high-rep".to_string(), vec![10.0, 20.0])
            .unwrap();
        aggregator
            .submit_gradient("low-rep".to_string(), vec![0.0, 0.0])
            .unwrap();

        let result = aggregator.aggregate().unwrap();
        // High reputation node should dominate
        // (10*5 + 0*1) / 6 = 8.33
        assert!((result.aggregated_gradient[0] - 8.333).abs() < 0.1);
    }

    #[test]
    fn test_e2e_gradient_aggregation_compression() {
        let config = AggregatorConfig {
            compression_ratio: 0.3, // Aggressive compression
            outlier_threshold: 3.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let mut aggregator = GradientAggregatorV3::new(config);

        // Submit gradient with varied values
        let gradient: Vec<f32> = (0..100).map(|i| i as f32 * 0.1).collect();
        aggregator
            .submit_gradient("compressor".to_string(), gradient)
            .unwrap();

        let result = aggregator.aggregate().unwrap();
        // Compression should zero out small gradients
        let zero_count = result
            .aggregated_gradient
            .iter()
            .filter(|v| **v == 0.0)
            .count();
        assert!(zero_count > 0, "Compression should produce zeros");
    }

    #[test]
    fn test_e2e_gradient_aggregation_multi_round() {
        let config = AggregatorConfig {
            compression_ratio: 1.0,
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let mut aggregator = GradientAggregatorV3::new(config);

        for round in 0..5u64 {
            aggregator
                .submit_gradient(format!("node-{}", round), vec![round as f32; 4])
                .unwrap();
            let result = aggregator.aggregate().unwrap();
            assert_eq!(result.round, round); // Round starts at 0, aggregate uses current
            assert_eq!(result.aggregated_gradient[0], round as f32);
            if round < 4 {
                aggregator.reset_round(); // Increment round counter for next iteration
                assert_eq!(aggregator.current_round(), round + 1);
            }
        }
    }

    // ─── LP-50: Ethical Constraint Engine E2E Tests ───

    #[test]
    fn test_e2e_ethical_constraint_value_bounds() {
        let mut engine = ConstraintEngine::new();

        // Add value bound constraint
        let constraint = EthicalConstraint {
            id: "bounds-1".to_string(),
            constraint_type:
                ed2kia::alignment::ethical_constraint_engine::ConstraintType::ValueBound {
                    min: -2.0,
                    max: 2.0,
                    feature_index: 0,
                },
            parameters: Default::default(),
            severity: ConstraintSeverity::Correction,
        };
        engine.add_constraint(constraint);

        // Valid gradient
        let valid_gradient = vec![1.0, 0.5, -0.5];
        assert!(engine.validate_gradient(&valid_gradient).is_ok());

        // Out-of-bounds gradient
        let bad_gradient = vec![5.0, 0.5, -0.5];
        assert!(engine.validate_gradient(&bad_gradient).is_err());

        // Correct gradient
        let mut corrected = bad_gradient.clone();
        engine.correct_gradient(&mut corrected).ok();
        assert!(corrected[0] <= 2.0); // Clamped to max
    }

    #[test]
    fn test_e2e_ethical_constraint_feature_mask() {
        let mut engine = ConstraintEngine::new();

        let constraint = EthicalConstraint {
            id: "mask-1".to_string(),
            constraint_type:
                ed2kia::alignment::ethical_constraint_engine::ConstraintType::FeatureMask {
                    masked_features: vec![1, 3],
                },
            parameters: Default::default(),
            severity: ConstraintSeverity::Correction,
        };
        engine.add_constraint(constraint);

        // Gradient with non-zero masked features should fail validation
        let gradient = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(engine.validate_gradient(&gradient).is_err()); // Features 1,3 non-zero

        // Correct gradient (zeros out masked features)
        let mut corrected = gradient.clone();
        engine.correct_gradient(&mut corrected).ok();
        assert_eq!(corrected[1], 0.0); // Masked
        assert_eq!(corrected[3], 0.0); // Masked
        assert_eq!(corrected[0], 1.0); // Unchanged

        // Now the corrected gradient should pass validation
        assert!(engine.validate_gradient(&corrected).is_ok());
    }

    #[test]
    fn test_e2e_ethical_constraint_halt_on_severe_violation() {
        let mut engine = ConstraintEngine::new();

        let constraint = EthicalConstraint {
            id: "halt-1".to_string(),
            constraint_type:
                ed2kia::alignment::ethical_constraint_engine::ConstraintType::ValueBound {
                    min: -1.0,
                    max: 1.0,
                    feature_index: 0,
                },
            parameters: Default::default(),
            severity: ConstraintSeverity::Halt,
        };
        engine.add_constraint(constraint);

        assert!(!engine.should_halt());

        // Severe violation
        let bad_gradient = vec![10.0, 0.0];
        let _ = engine.validate_gradient(&bad_gradient);

        assert!(engine.should_halt());
        let violations = engine.get_violations();
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_e2e_ethical_constraint_divergence_limit() {
        let mut engine = ConstraintEngine::new();

        let constraint = EthicalConstraint {
            id: "div-1".to_string(),
            constraint_type:
                ed2kia::alignment::ethical_constraint_engine::ConstraintType::DivergenceLimit {
                    max_divergence: 0.5,
                },
            parameters: Default::default(),
            severity: ConstraintSeverity::Warning,
        };
        engine.add_constraint(constraint);

        // Small gradient (low divergence)
        let small_gradient = vec![0.1, 0.1, 0.1];
        assert!(engine.validate_gradient(&small_gradient).is_ok());

        // Large gradient (high divergence)
        let large_gradient = vec![5.0, 5.0, 5.0];
        assert!(engine.validate_gradient(&large_gradient).is_err());
    }

    // ─── LP-51: Full Pipeline E2E Tests ───

    #[test]
    fn test_e2e_full_pipeline_registry_to_identity() {
        // 1. Register chains
        let mut registry = MultiChainRegistry::new();
        let config = ChainConfig {
            chain_id: "eth-mainnet".to_string(),
            endpoint: "https://eth.gateway".to_string(),
            protocol: ChainProtocol::Ethereum,
            parameters: Default::default(),
        };
        registry.register_chain(config).unwrap();
        registry.update_state("eth-mainnet", ChainState::Connected);

        // 2. Create identity for registered chain
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0x42; 32]);
        let mut identity = CrossChainIdentity::new("pipeline-node".to_string(), signing_key);
        identity.register_chain_key("eth-mainnet", "0xpipeline".to_string());

        // 3. Generate proof
        let proof = identity.generate_proof("eth-mainnet").unwrap();
        assert_eq!(proof.chain_id, "eth-mainnet");
        assert!(registry.get_chain("eth-mainnet").is_some());
    }

    #[test]
    fn test_e2e_full_pipeline_training_to_aggregation() {
        // 1. Fine-tune locally
        let config = FineTuningConfig {
            learning_rate: 0.01,
            schedule: LearningRateSchedule::Constant,
            batch_size: 32,
            max_epochs: 5,
            convergence_threshold: 0.01,
        };
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch();
        engine.record_batch(0.5, 1.0);
        engine.record_batch(0.4, 0.9);

        // 2. Submit gradients to aggregator
        let agg_config = AggregatorConfig {
            compression_ratio: 1.0,
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let mut aggregator = GradientAggregatorV3::new(agg_config);
        let gradient = vec![0.01; 8]; // Simulated gradient from fine-tuning
        aggregator
            .submit_gradient("trainer-1".to_string(), gradient)
            .unwrap();

        // 3. Aggregate
        let result = aggregator.aggregate().unwrap();
        assert_eq!(result.participant_count, 1);
        assert_eq!(result.aggregated_gradient.len(), 8);
    }

    #[test]
    fn test_e2e_full_pipeline_training_with_constraints() {
        // 1. Set up ethical constraints
        let mut constraint_engine = ConstraintEngine::new();
        constraint_engine.add_constraint(EthicalConstraint {
            id: "safe-gradient".to_string(),
            constraint_type:
                ed2kia::alignment::ethical_constraint_engine::ConstraintType::ValueBound {
                    min: -1.0,
                    max: 1.0,
                    feature_index: 0,
                },
            parameters: Default::default(),
            severity: ConstraintSeverity::Correction,
        });

        // 2. Fine-tune and check gradients
        let config = FineTuningConfig {
            learning_rate: 0.01,
            schedule: LearningRateSchedule::Constant,
            batch_size: 32,
            max_epochs: 3,
            convergence_threshold: 0.01,
        };
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch();
        engine.record_batch(0.3, 0.5);

        // 3. Validate gradient against constraints
        let safe_gradient = vec![0.5, 0.3, -0.2];
        assert!(constraint_engine.validate_gradient(&safe_gradient).is_ok());

        // 4. Correct unsafe gradient
        let mut unsafe_gradient = vec![2.0, 0.3, -0.2];
        constraint_engine
            .correct_gradient(&mut unsafe_gradient)
            .ok();
        assert!(unsafe_gradient[0] <= 1.0);
    }

    #[test]
    fn test_e2e_full_pipeline_registry_identity_training_aggregation_constraints() {
        // Complete pipeline: registry → identity → training → aggregation → constraints

        // 1. Multi-chain registry
        let mut registry = MultiChainRegistry::new();
        registry
            .register_chain(ChainConfig {
                chain_id: "fed-chain".to_string(),
                endpoint: "https://fed.gateway".to_string(),
                protocol: ChainProtocol::Ethereum,
                parameters: Default::default(),
            })
            .unwrap();
        registry.update_state("fed-chain", ChainState::Connected);
        assert_eq!(registry.chain_count(), 1);

        // 2. Cross-chain identity
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0x99; 32]);
        let mut identity = CrossChainIdentity::new("fed-trainer".to_string(), signing_key);
        identity.register_chain_key("fed-chain", "0xfed123".to_string());
        let proof = identity.generate_proof("fed-chain").unwrap();
        assert_eq!(proof.node_id, "fed-trainer");

        // 3. SAE fine-tuning
        let config = FineTuningConfig {
            learning_rate: 0.005,
            schedule: LearningRateSchedule::CosineDecay { max_steps: 50 },
            batch_size: 16,
            max_epochs: 5,
            convergence_threshold: 0.01,
        };
        let mut engine = FineTuningEngine::new(config);
        engine.start_epoch();
        engine.record_batch(0.4, 0.8);
        engine.record_batch(0.35, 0.7);

        // 4. Ethical constraints
        let mut constraint_engine = ConstraintEngine::new();
        constraint_engine.add_constraint(EthicalConstraint {
            id: "pipeline-bounds".to_string(),
            constraint_type:
                ed2kia::alignment::ethical_constraint_engine::ConstraintType::ValueBound {
                    min: -0.5,
                    max: 0.5,
                    feature_index: 0,
                },
            parameters: Default::default(),
            severity: ConstraintSeverity::Correction,
        });

        // 5. Gradient aggregation with constraint enforcement
        let agg_config = AggregatorConfig {
            compression_ratio: 1.0,
            outlier_threshold: 2.0,
            min_participants: 1,
            use_reputation_weights: false,
        };
        let mut aggregator = GradientAggregatorV3::new(agg_config);

        // Validate before submitting
        let gradient = vec![0.1; 4];
        assert!(constraint_engine.validate_gradient(&gradient).is_ok());
        aggregator
            .submit_gradient("fed-trainer".to_string(), gradient)
            .unwrap();

        let result = aggregator.aggregate().unwrap();
        assert!(!result.aggregated_gradient.is_empty());
        assert!(!constraint_engine.should_halt());

        // 6. Build reputation
        identity.update_reputation("fed-chain", 0.1);
        assert!(identity.get_global_reputation() > 0.0);
    }

    #[test]
    fn test_e2e_feature_flag_enabled() {
        // Verify v1.2-sprint1 feature is active
        assert!(cfg!(feature = "v1.2-sprint1"));
    }
} // mod e2e
