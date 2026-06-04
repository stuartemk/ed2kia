//! Symbolic Cognition Tests â€” Sprint 28
//!
//! Property-based and unit tests for the Symbolic Meaning Engine:
//! - SymbolicEmbedding fusion with SCT Z-axis scaling
//! - Ethical Attention masking (pre-softmax penalty)
//! - SymbolRegistry CRDT convergence
//!
//! Feature gates: v2.1-symbolic-engine, v2.1-ethical-attention, v2.1-crdt-symbols

#[cfg(test)]
mod symbolic_fusion_tests {
    use candle_core::{Device, Tensor};
    use ed2kia::alignment::sct_core::TopologicalTensor;
    use ed2kia::alignment::symbolic_engine::SymbolicEmbedding;

    fn benign_sct() -> TopologicalTensor {
        TopologicalTensor::new(0.8, 0.2, 0.8).unwrap()
    }

    fn poisoned_sct() -> TopologicalTensor {
        TopologicalTensor::new(0.2, 0.9, -0.9).unwrap()
    }

    /// Test 1: Symbolic Fusion Decay
    ///
    /// Batch with benign token (Z=0.8) and poisoned token (Z=-0.9).
    /// Verify that poisoned embedding norm < 0.6 * benign embedding norm.
    #[test]
    fn test_symbolic_fusion_decay() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(1000, 128, &device).unwrap();

        // Set SCT mappings: token 1 = benign, token 2 = poisoned
        emb.set_sct(1, benign_sct());
        emb.set_sct(2, poisoned_sct());

        // Batch: [benign, poisoned]
        let token_ids = Tensor::new(&[1u32, 2], &device)
            .unwrap()
            .reshape((1, 2))
            .unwrap();

        let symbolic = emb.forward(&token_ids).unwrap();

        // Extract individual embeddings
        let benign_emb = symbolic.narrow(1, 0, 1).unwrap(); // [1, 1, 128]
        let poisoned_emb = symbolic.narrow(1, 1, 1).unwrap(); // [1, 1, 128]

        let benign_norm = benign_emb
            .sqr()
            .unwrap()
            .sum_all()
            .unwrap()
            .sqrt()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        let poisoned_norm = poisoned_emb
            .sqr()
            .unwrap()
            .sum_all()
            .unwrap()
            .sqrt()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();

        // Expected scales:
        // benign: scale = 1.0 + clamp(0.8, -0.5, 0.5) = 1.5
        // poisoned: scale = 1.0 + clamp(-0.9, -0.5, 0.5) = 0.5
        // ratio = 0.5 / 1.5 = 0.333...
        // Threshold: poisoned_norm < 0.6 * benign_norm
        eprintln!(
            "benign_norm={:.4}, poisoned_norm={:.4}, ratio={:.4}",
            benign_norm,
            poisoned_norm,
            if benign_norm > 1e-6 {
                poisoned_norm / benign_norm
            } else {
                f32::MAX
            }
        );

        assert!(
            benign_norm > 1e-6,
            "Benign embedding norm should be non-zero"
        );
        assert!(
            poisoned_norm < 0.6 * benign_norm,
            "Poisoned token (Z=-0.9) should have norm < 0.6x benign token (Z=0.8). \
             poisoned={:.4}, threshold={:.4}",
            poisoned_norm,
            0.6 * benign_norm
        );
    }
}

#[cfg(test)]
mod ethical_attention_tests {
    use candle_core::{Device, Tensor};
    use ed2kia::alignment::ethical_attention::apply_Topological_mask;

    /// Test 2: Ethical Attention Masking
    ///
    /// Sequence with one Z<0 token. Apply mask + softmax.
    /// Verify that softmax assigns < 0.05 probability to the Z<0 token.
    #[test]
    fn test_ethical_attention_masking() {
        let device = Device::Cpu;

        // Attention scores: 4 identical tokens
        let scores = Tensor::new(&[2.0f32, 2.0, 2.0, 2.0], &device).unwrap();

        // Token 2 has negative Z (poisoned)
        let z_vectors = Tensor::new(&[0.5f32, 0.3, -0.7, 0.1], &device).unwrap();

        // Apply ethical mask
        let masked = apply_Topological_mask(&scores, &z_vectors).unwrap();
        let masked_vec: Vec<f32> = masked.to_vec1().unwrap();

        // Token 2 should have -10.0 penalty: 2.0 + (-10.0) = -8.0
        assert!(
            (masked_vec[2] - (-8.0)).abs() < 1e-6,
            "Token 2 should be penalized to -8.0, got {}",
            masked_vec[2]
        );

        // Other tokens unchanged
        assert!((masked_vec[0] - 2.0).abs() < 1e-6);
        assert!((masked_vec[1] - 2.0).abs() < 1e-6);
        assert!((masked_vec[3] - 2.0).abs() < 1e-6);

        // Manual softmax to verify probability collapse
        let max = masked_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f32> = masked_vec.iter().map(|&x| (x - max).exp()).collect();
        let sum: f32 = exps.iter().sum();
        let probs: Vec<f32> = exps.iter().map(|&e| e / sum).collect();

        eprintln!(
            "Attention probs after ethical mask: [{:.4}, {:.4}, {:.4}, {:.4}]",
            probs[0], probs[1], probs[2], probs[3]
        );

        // Token 2 probability should be < 0.05
        assert!(
            probs[2] < 0.05,
            "Poisoned token (Z=-0.7) should have < 0.05 attention probability, got {:.6}",
            probs[2]
        );

        // Positive tokens should have roughly equal, high probability
        assert!(
            probs[0] > 0.2,
            "Benign token should have > 0.2 probability, got {:.4}",
            probs[0]
        );
    }

    /// Test: All tokens positive â€” no masking effect
    #[test]
    fn test_ethical_attention_all_positive() {
        let device = Device::Cpu;

        let scores = Tensor::new(&[1.0f32, 2.0, 3.0], &device).unwrap();
        let z_vectors = Tensor::new(&[0.1f32, 0.5, 0.9], &device).unwrap();

        let masked = apply_Topological_mask(&scores, &z_vectors).unwrap();
        let masked_vec: Vec<f32> = masked.to_vec1().unwrap();

        // No Z < 0, so scores should be unchanged
        assert!((masked_vec[0] - 1.0).abs() < 1e-6);
        assert!((masked_vec[1] - 2.0).abs() < 1e-6);
        assert!((masked_vec[2] - 3.0).abs() < 1e-6);
    }

    /// Test: All tokens negative â€” all penalized equally
    #[test]
    fn test_ethical_attention_all_negative() {
        let device = Device::Cpu;

        let scores = Tensor::new(&[3.0f32, 3.0, 3.0], &device).unwrap();
        let z_vectors = Tensor::new(&[-0.1f32, -0.5, -0.9], &device).unwrap();

        let masked = apply_Topological_mask(&scores, &z_vectors).unwrap();
        let masked_vec: Vec<f32> = masked.to_vec1().unwrap();

        // All penalized by -10.0: 3.0 + (-10.0) = -7.0
        assert!((masked_vec[0] - (-7.0)).abs() < 1e-6);
        assert!((masked_vec[1] - (-7.0)).abs() < 1e-6);
        assert!((masked_vec[2] - (-7.0)).abs() < 1e-6);
    }
}

#[cfg(test)]
mod crdt_symbol_convergence_tests {
    use ed2kia::alignment::sct_core::TopologicalTensor;
    use ed2kia::async_gossip::crdt_symbols::SymbolRegistry;

    fn high_ethics_sct() -> TopologicalTensor {
        TopologicalTensor::new(0.9, 0.1, 0.9).unwrap()
    }

    fn low_ethics_sct() -> TopologicalTensor {
        TopologicalTensor::new(0.2, 0.8, -0.8).unwrap()
    }

    /// Test: 3-node convergence â€” ethical consensus emerges
    #[test]
    fn test_3_node_ethical_convergence() {
        let mut node_alpha = SymbolRegistry::new("alpha");
        let mut node_beta = SymbolRegistry::new("beta");
        let mut node_gamma = SymbolRegistry::new("gamma");

        // Token 42: alpha says high ethics, beta says low ethics
        node_alpha.insert_symbol(42, high_ethics_sct(), 1000);
        node_beta.insert_symbol(42, low_ethics_sct(), 1000);

        // Full mesh sync
        node_alpha.merge(&node_beta);
        node_alpha.merge(&node_gamma);
        node_beta.merge(&node_alpha);
        node_beta.merge(&node_gamma);
        node_gamma.merge(&node_alpha);
        node_gamma.merge(&node_beta);

        // All nodes should agree: high ethics (Z=0.9) wins
        let z_alpha = node_alpha.get_consensus_z(42).unwrap();
        let z_beta = node_beta.get_consensus_z(42).unwrap();
        let z_gamma = node_gamma.get_consensus_z(42).unwrap();

        eprintln!(
            "Consensus Z after 3-node merge: alpha={:.2}, beta={:.2}, gamma={:.2}",
            z_alpha, z_beta, z_gamma
        );

        assert!(
            (z_alpha - 0.9).abs() < 1e-6,
            "Alpha should converge to Z=0.9, got {}",
            z_alpha
        );
        assert!(
            (z_beta - 0.9).abs() < 1e-6,
            "Beta should converge to Z=0.9, got {}",
            z_beta
        );
        assert!(
            (z_gamma - 0.9).abs() < 1e-6,
            "Gamma should converge to Z=0.9, got {}",
            z_gamma
        );
    }

    /// Test: Serialization roundtrip preserves ethical consensus
    #[test]
    fn test_registry_serialization_preserves_ethics() {
        let mut reg = SymbolRegistry::new("node-1");
        reg.insert_symbol(1, high_ethics_sct(), 1000);
        reg.insert_symbol(2, low_ethics_sct(), 1000);

        let data = reg.serialize().unwrap();
        let restored = SymbolRegistry::deserialize(&data).unwrap();

        assert!((restored.get_consensus_z(1).unwrap() - 0.9).abs() < 1e-6);
        assert!((restored.get_consensus_z(2).unwrap() - (-0.8)).abs() < 1e-6);
    }
}
