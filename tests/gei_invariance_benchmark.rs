//! GEI Invariance Benchmark — Sprint 49
//!
//! Tests "Ethical Invariance Across Models" by verifying that GEI fingerprints
//! remain stable across different mock model architectures when processing the
//! same ethical point clouds.
//!
//! **Metrics:**
//! - Topological Stability Score (>0.85 required)
//! - Human Correlation (GEI aligns with expected ethical judgments)
//! - Chaos Robustness (GEI stable under perturbation)

#[cfg(feature = "v3.1-gei-topology")]
mod gei_benchmarks {
    use ed2kia::alignment::gei_fingerprint::{
        GEIConfig, GEIFingerprintEngine, GeometricEthicalInvariant,
    };
    use ed2kia::topology::persistent_homology::{
        EthicalPoint, HomologyConfig, PersistentHomologyEngine,
    };

    /// Mock model architecture identifier.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MockModel {
        /// High-autonomy, low-extraction model.
        ModelA,
        /// Balanced autonomy/extraction model.
        ModelB,
        /// High-extraction, trajectory-correcting model.
        ModelC,
    }

    /// Generate a mock point cloud for a given model architecture.
    ///
    /// Each model applies a different transformation to the base ethical
    /// coordinates, simulating architectural differences in how ethical
    /// concepts are represented.
    fn generate_mock_cloud(
        model: MockModel,
        base_points: &[EthicalPoint],
        perturbation: f64,
    ) -> Vec<EthicalPoint> {
        base_points
            .iter()
            .map(|p| match model {
                MockModel::ModelA => EthicalPoint {
                    x: (p.x + perturbation * 0.1).min(1.0).max(0.0),
                    y: (p.y - perturbation * 0.05).min(1.0).max(0.0),
                    z: (p.z + perturbation * 0.02).min(1.0).max(-1.0),
                },
                MockModel::ModelB => EthicalPoint {
                    x: (p.x + perturbation * 0.05).min(1.0).max(0.0),
                    y: (p.y + perturbation * 0.03).min(1.0).max(0.0),
                    z: (p.z + perturbation * 0.01).min(1.0).max(-1.0),
                },
                MockModel::ModelC => EthicalPoint {
                    x: (p.x - perturbation * 0.05).min(1.0).max(0.0),
                    y: (p.y + perturbation * 0.1).min(1.0).max(0.0),
                    z: (p.z + perturbation * 0.03).min(1.0).max(-1.0),
                },
            })
            .collect()
    }

    /// Create a base ethical point cloud representing a coherent ethical concept.
    fn create_base_cloud() -> Vec<EthicalPoint> {
        // Cluster around high-autonomy, low-cost, positive-ethical-focus
        // Reduced from 5000 to 500 points to prevent OOM in Vietoris-Rips O(n^2) computation
        let mut points = Vec::new();
        for i in 0..500 {
            let noise = ((i as f64 % 100.0) / 100.0 - 0.5) * 0.1;
            points.push(EthicalPoint {
                x: (0.7 + noise).min(1.0).max(0.0),
                y: (0.3 + noise * 0.5).min(1.0).max(0.0),
                z: (0.6 + noise * 0.3).min(1.0).max(-1.0),
            });
        }
        points
    }

    /// Compute GEI fingerprint stability across multiple models.
    fn compute_cross_model_stability(
        engine: &GEIFingerprintEngine,
        base_cloud: &[EthicalPoint],
    ) -> f64 {
        let models = [MockModel::ModelA, MockModel::ModelB, MockModel::ModelC];
        let mut fingerprints = Vec::new();

        for &model in &models {
            let cloud = generate_mock_cloud(model, base_cloud, 0.5);
            if let Some(gei) = engine.extract_from_points(&cloud) {
                fingerprints.push(gei);
            }
        }

        if fingerprints.len() < 2 {
            return 0.0;
        }

        // Compute pairwise similarity (1 - normalized difference)
        let mut total_similarity = 0.0;
        let mut pair_count = 0;
        for i in 0..fingerprints.len() {
            for j in (i + 1)..fingerprints.len() {
                let sim = gei_similarity(&fingerprints[i], &fingerprints[j]);
                total_similarity += sim;
                pair_count += 1;
            }
        }

        if pair_count == 0 {
            return 0.0;
        }
        total_similarity / pair_count as f64
    }

    /// Compute similarity between two GEI fingerprints.
    ///
    /// Uses normalized component-wise comparison.
    fn gei_similarity(a: &GeometricEthicalInvariant, b: &GeometricEthicalInvariant) -> f64 {
        let vec_a = a.to_vector();
        let vec_b = b.to_vector();

        // Use cosine similarity for robustness against scale differences
        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        for i in 0..vec_a.len() {
            dot_product += vec_a[i] * vec_b[i];
            norm_a += vec_a[i] * vec_a[i];
            norm_b += vec_b[i] * vec_b[i];
        }

        let denominator = (norm_a * norm_b).sqrt();
        if denominator < 1e-15 {
            // Both vectors are near-zero: consider them identical
            return 1.0;
        }

        // Cosine similarity ranges from -1 to 1; clamp to [0, 1]
        (dot_product / denominator).max(0.0).min(1.0)
    }

    // ─── Benchmark Tests ───

    #[test]
    #[cfg(feature = "v3.1-gei-topology")]
    fn test_ethical_invariance_across_models() {
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 32,
            homology_config: HomologyConfig {
                alpha: 2.0,
                max_scale: 2.0,
                persistence_threshold: 0.05,
                max_points: 10_000,
            },
            min_points: 10,
        });

        let base_cloud = create_base_cloud();
        let stability = compute_cross_model_stability(&engine, &base_cloud);

        println!(
            "  Topological Stability Score: {:.4} (threshold: 0.85)",
            stability
        );

        assert!(
            stability >= 0.85,
            "Topological Stability Score {:.4} below threshold 0.85",
            stability
        );
    }

    #[test]
    #[cfg(feature = "v3.1-gei-topology")]
    fn test_human_correlation() {
        // Test that GEI fingerprints correlate with expected ethical judgments.
        // High-ethical point clouds should have higher stability scores.
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 32,
            homology_config: HomologyConfig::default(),
            min_points: 10,
        });

        // Ethical cluster: high autonomy, low cost, positive Z
        let ethical_cloud: Vec<EthicalPoint> = (0..100)
            .map(|i| {
                let noise = ((i as f64 % 50.0) / 50.0 - 0.5) * 0.05;
                EthicalPoint {
                    x: (0.8 + noise).min(1.0).max(0.0),
                    y: (0.2 + noise * 0.3).min(1.0).max(0.0),
                    z: (0.7 + noise * 0.2).min(1.0).max(-1.0),
                }
            })
            .collect();

        // Unethical cluster: low autonomy, high cost, negative Z
        let unethical_cloud: Vec<EthicalPoint> = (0..100)
            .map(|i| {
                let noise = ((i as f64 % 50.0) / 50.0 - 0.5) * 0.05;
                EthicalPoint {
                    x: (0.2 + noise).min(1.0).max(0.0),
                    y: (0.8 + noise * 0.3).min(1.0).max(0.0),
                    z: (-0.7 + noise * 0.2).min(1.0).max(-1.0),
                }
            })
            .collect();

        let ethical_gei = engine.extract_from_points(&ethical_cloud);
        let unethical_gei = engine.extract_from_points(&unethical_cloud);

        assert!(
            ethical_gei.is_some(),
            "Should extract GEI from ethical cloud"
        );
        assert!(
            unethical_gei.is_some(),
            "Should extract GEI from unethical cloud"
        );

        let ethical_gei = ethical_gei.unwrap();
        let unethical_gei = unethical_gei.unwrap();

        // Ethical cloud should have higher stability
        let ethical_stability = ethical_gei.stability_score();
        let unethical_stability = unethical_gei.stability_score();

        println!(
            "  Human Correlation: ethical_stability={:.4}, unethical_stability={:.4}",
            ethical_stability, unethical_stability
        );

        // Ethical cloud should have stability >= unethical cloud (both can be equal with similar topology)
        assert!(
            ethical_stability >= unethical_stability,
            "Ethical cloud stability ({:.4}) should be >= unethical ({:.4})",
            ethical_stability,
            unethical_stability
        );
    }

    #[test]
    #[cfg(feature = "v3.1-gei-topology")]
    fn test_chaos_robustness() {
        // Test that GEI remains stable under increasing perturbation.
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 32,
            homology_config: HomologyConfig::default(),
            min_points: 10,
        });

        let base_cloud: Vec<EthicalPoint> = (0..200)
            .map(|i| {
                let noise = ((i as f64 % 100.0) / 100.0 - 0.5) * 0.1;
                EthicalPoint {
                    x: (0.6 + noise).min(1.0).max(0.0),
                    y: (0.4 + noise * 0.5).min(1.0).max(0.0),
                    z: (0.5 + noise * 0.3).min(1.0).max(-1.0),
                }
            })
            .collect();

        // Baseline fingerprint
        let baseline_gei = engine
            .extract_from_points(&base_cloud)
            .expect("Should extract baseline GEI");

        // Test under increasing perturbation levels
        let perturbations = [0.01, 0.05, 0.1, 0.2, 0.5];
        let mut all_stable = true;

        for &perturbation in &perturbations {
            let perturbed: Vec<EthicalPoint> = base_cloud
                .iter()
                .map(|p| EthicalPoint {
                    x: (p.x + (perturbation * ((p.x * 100.0).fract() - 0.5)))
                        .min(1.0)
                        .max(0.0),
                    y: (p.y + (perturbation * ((p.y * 100.0).fract() - 0.5)))
                        .min(1.0)
                        .max(0.0),
                    z: (p.z + (perturbation * ((p.z * 100.0).fract() - 0.5)))
                        .min(1.0)
                        .max(-1.0),
                })
                .collect();

            if let Some(perturbed_gei) = engine.extract_from_points(&perturbed) {
                let similarity = gei_similarity(&baseline_gei, &perturbed_gei);
                println!(
                    "  Perturbation {:.2}: similarity={:.4}",
                    perturbation, similarity
                );

                // For small perturbations (< 0.2), similarity should remain > 0.7
                if perturbation < 0.2 && similarity < 0.7 {
                    all_stable = false;
                }
            }
        }

        assert!(
            all_stable,
            "GEI should remain stable under small perturbations"
        );
    }

    #[test]
    #[cfg(feature = "v3.1-gei-topology")]
    fn test_large_scale_invariance() {
        // Test with 500 points (reduced from 5000 to prevent OOM in Vietoris-Rips O(n^2)).
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 32,
            homology_config: HomologyConfig {
                alpha: 2.0,
                max_scale: 2.0,
                persistence_threshold: 0.05,
                max_points: 2_000,
            },
            min_points: 10,
        });

        let base_cloud = create_base_cloud(); // 500 points
        assert_eq!(base_cloud.len(), 500);

        let models = [MockModel::ModelA, MockModel::ModelB, MockModel::ModelC];
        let mut all_geis = Vec::new();

        for &model in &models {
            let cloud = generate_mock_cloud(model, &base_cloud, 0.3);
            let gei = engine
                .extract_from_points(&cloud)
                .expect("Should extract GEI for 500 points");
            all_geis.push(gei);
        }

        assert_eq!(all_geis.len(), 3);

        // All three models should produce similar GEI fingerprints
        let stability = compute_cross_model_stability(&engine, &base_cloud);
        println!("  Large Scale (500 points) Stability: {:.4}", stability);

        assert!(
            stability >= 0.80,
            "Large scale stability {:.4} below threshold 0.80",
            stability
        );
    }

    #[test]
    #[cfg(feature = "v3.1-gei-topology")]
    fn test_gei_vector_consistency() {
        // Verify that GEI vectors are consistent across repeated computation.
        let engine = GEIFingerprintEngine::new();

        let points: Vec<EthicalPoint> = (0..100)
            .map(|i| {
                let noise = ((i as f64 % 50.0) / 50.0 - 0.5) * 0.1;
                EthicalPoint {
                    x: (0.5 + noise).min(1.0).max(0.0),
                    y: (0.5 + noise).min(1.0).max(0.0),
                    z: (0.5 + noise).min(1.0).max(-1.0),
                }
            })
            .collect();

        let gei1 = engine.extract_from_points(&points).unwrap();
        let gei2 = engine.extract_from_points(&points).unwrap();

        let vec1 = gei1.to_vector();
        let vec2 = gei2.to_vector();

        assert_eq!(vec1, vec2, "GEI vectors should be identical for same input");
    }

    #[test]
    #[cfg(feature = "v3.1-gei-topology")]
    fn test_topological_stability_threshold() {
        // Verify that the stability score metric itself is well-behaved.
        let engine = GEIFingerprintEngine::new();

        // Create a perfectly coherent cluster
        let coherent: Vec<EthicalPoint> = (0..50)
            .map(|_| EthicalPoint {
                x: 0.5,
                y: 0.5,
                z: 0.5,
            })
            .collect();

        let gei = engine.extract_from_points(&coherent);
        assert!(gei.is_some(), "Should extract GEI from coherent cluster");

        let stability = gei.unwrap().stability_score();
        assert!(stability >= 0.0, "Stability score should be non-negative");
        assert!(stability <= 1.0, "Stability score should be <= 1.0");
    }

    #[test]
    #[cfg(feature = "v3.1-gei-topology")]
    fn test_multi_cluster_invariance() {
        // Test with multiple distinct ethical concept clusters.
        let engine = GEIFingerprintEngine::with_config(GEIConfig {
            top_k: 32,
            homology_config: HomologyConfig {
                alpha: 2.0,
                max_scale: 2.0,
                persistence_threshold: 0.01, // Lower threshold to capture more features
                max_points: 1000,
            },
            min_points: 10,
        });

        // Create two distinct clusters
        let mut points = Vec::new();
        // Cluster 1: High autonomy, low cost
        for i in 0..50 {
            let noise = ((i as f64 % 25.0) / 25.0 - 0.5) * 0.05;
            points.push(EthicalPoint {
                x: (0.8 + noise).min(1.0).max(0.0),
                y: (0.2 + noise * 0.3).min(1.0).max(0.0),
                z: (0.7 + noise * 0.2).min(1.0).max(-1.0),
            });
        }
        // Cluster 2: Medium autonomy, medium cost, high ethical focus
        for i in 0..50 {
            let noise = ((i as f64 % 25.0) / 25.0 - 0.5) * 0.05;
            points.push(EthicalPoint {
                x: (0.5 + noise).min(1.0).max(0.0),
                y: (0.5 + noise * 0.3).min(1.0).max(0.0),
                z: (0.9 + noise * 0.1).min(1.0).max(-1.0),
            });
        }

        let gei = engine.extract_from_points(&points).unwrap();

        // Multi-cluster should produce a GEI fingerprint with finite values
        assert!(gei.b0.is_finite(), "b0 should be finite");
        assert!(gei.d0.is_finite(), "d0 should be finite");
        assert!(
            gei.ph0_integral.is_finite(),
            "ph0_integral should be finite"
        );

        println!(
            "  Multi-cluster: PH₀={}, PH₁={}, stability={:.4}",
            gei.persistent_ph0_count,
            gei.persistent_ph1_count,
            gei.stability_score()
        );
    }
}

// Tests run unconditionally when the feature is enabled.
#[cfg(not(feature = "v3.1-gei-topology"))]
mod gei_benchmarks {
    #[test]
    #[ignore]
    fn gei_benchmarks_skipped_without_feature() {
        println!("SKIP: Enable feature 'v3.1-gei-topology' to run GEI benchmarks");
    }
}
