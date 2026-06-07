//! Cross-Attention Fusion Scalability Tests — Sprint 109
//!
//! Validates cross-attention multi-modal fusion, alignment scores,
//! and scalability of fusion operations across varying modality counts and dimensions.

use candle_core::{Device, Tensor};
use native_audit::cross_attention::{CrossAttentionConfig, CrossAttentionFusion};

#[test]
fn test_cross_attention_config_default() {
    let config = CrossAttentionConfig::default();
    assert!(config.embed_dim > 0, "embed_dim must be positive");
    assert!(config.num_heads > 0, "num_heads must be positive");
    assert!(
        config.embed_dim % config.num_heads == 0,
        "embed_dim must be divisible by num_heads"
    );
}

#[test]
fn test_fusion_layer_creation() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let _fusion = CrossAttentionFusion::new(&config, &device).unwrap();
    // Fusion layer creation should not panic
}

#[test]
fn test_fusion_basic() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let modality_a = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let modality_b = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let modalities = vec![modality_a, modality_b];

    let result = fusion.fuse(&modalities).unwrap();
    assert_eq!(
        result.fused.shape().dims(),
        &[2, config.embed_dim],
        "fused output shape should match input"
    );
    assert_eq!(result.gate_scores.len(), 2, "should have 2 gate scores");
    println!("Fusion result shape: {:?}", result.fused.shape().dims());
}

#[test]
fn test_fusion_three_modalities() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (1, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (1, config.embed_dim), &device).unwrap();
    let m3 = Tensor::rand(0.0f32, 1.0f32, (1, config.embed_dim), &device).unwrap();

    let result = fusion.fuse(&[m1, m2, m3]).unwrap();
    assert_eq!(result.fused.shape().dims(), &[1, config.embed_dim]);
    assert_eq!(result.gate_scores.len(), 3);

    // Gate scores should sum to approximately 1.0 (softmax normalization)
    let gate_sum: f32 = result.gate_scores.iter().sum();
    assert!(
        (gate_sum - 1.0).abs() < 0.01,
        "gate scores should sum to 1.0, got {:.4}",
        gate_sum
    );
}

#[test]
fn test_fusion_gate_scores_are_softmax() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (1, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (1, config.embed_dim), &device).unwrap();

    let result = fusion.fuse(&[m1, m2]).unwrap();
    for g in &result.gate_scores {
        assert!(*g >= 0.0, "gate score should be non-negative: {}", g);
        assert!(!g.is_nan(), "gate score should not be NaN");
    }
}

#[test]
fn test_alignment_score_range() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let result = fusion.fuse(&[m1, m2]).unwrap();

    let target = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let score = fusion.alignment_score(&result.fused, &target).unwrap();

    assert!(
        (0.0..=1.0).contains(&score),
        "alignment score in [0,1]: {}",
        score
    );
    assert!(!score.is_nan(), "alignment score should not be NaN");
    println!("Alignment score: {:.4}", score);
}

#[test]
fn test_alignment_identical_tensors() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let tensor = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let score = fusion.alignment_score(&tensor, &tensor).unwrap();

    assert!(
        score > 0.9,
        "identical tensors should have near-perfect alignment: {:.4}",
        score
    );
}

#[test]
fn test_fusion_result_structure() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let result = fusion.fuse(&[m1, m2]).unwrap();

    // Verify FusionResult fields
    assert_eq!(result.fused.shape().dims(), &[2, config.embed_dim]);
    assert_eq!(result.gate_scores.len(), 2);
    assert!(!result.gate_scores.iter().any(|g| g.is_nan()));
    assert!(!result.fusion_energy.is_nan());
}

#[test]
fn test_scalability_batch_sizes() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    for batch_size in [1, 4, 8, 16] {
        let m1 = Tensor::rand(0.0f32, 1.0f32, (batch_size, config.embed_dim), &device).unwrap();
        let m2 = Tensor::rand(0.0f32, 1.0f32, (batch_size, config.embed_dim), &device).unwrap();
        let result = fusion.fuse(&[m1, m2]).unwrap();
        assert_eq!(
            result.fused.shape().dims(),
            &[batch_size, config.embed_dim],
            "batch={} shape mismatch",
            batch_size
        );
        println!("batch={} fusion OK", batch_size);
    }
}

#[test]
fn test_scalability_embed_dimensions() {
    let device = Device::Cpu;

    for (embed_dim, num_heads) in [(64, 4), (128, 8)] {
        let config = CrossAttentionConfig {
            embed_dim,
            num_heads,
            ..CrossAttentionConfig::default()
        };
        let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

        let m1 = Tensor::rand(0.0f32, 1.0f32, (2, embed_dim), &device).unwrap();
        let m2 = Tensor::rand(0.0f32, 1.0f32, (2, embed_dim), &device).unwrap();
        let result = fusion.fuse(&[m1, m2]).unwrap();

        assert_eq!(
            result.fused.shape().dims(),
            &[2, embed_dim],
            "embed_dim={} shape mismatch",
            embed_dim
        );
        println!("embed_dim={} fusion OK", embed_dim);
    }
}

#[test]
fn test_scalability_modality_count() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    for n_modalities in 2..=config.num_modalities {
        let modalities: Vec<Tensor> = (0..n_modalities)
            .map(|_| Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap())
            .collect();
        let result = fusion.fuse(&modalities).unwrap();
        assert_eq!(
            result.gate_scores.len(),
            n_modalities,
            "should have {} gate scores",
            n_modalities
        );
        println!("{} modalities fusion OK", n_modalities);
    }
}

#[test]
fn test_fusion_deterministic() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (1, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (1, config.embed_dim), &device).unwrap();

    let r1 = fusion.fuse(&[m1.clone(), m2.clone()]).unwrap();
    let r2 = fusion.fuse(&[m1, m2]).unwrap();

    // Same fusion layer + same inputs = same output
    let diff_tensor = r1.fused.sub(&r2.fused).unwrap().abs().unwrap();
    let diff = diff_tensor.sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(
        diff < 1e-6,
        "fusion should be deterministic: diff={:.8}",
        diff
    );
}

#[test]
fn test_generate_stub_modalities() {
    let device = Device::Cpu;
    let modalities =
        native_audit::cross_attention::generate_stub_modalities(3, 2, 8, &device).unwrap();
    assert_eq!(modalities.len(), 3);
    for (i, m) in modalities.iter().enumerate() {
        assert_eq!(m.shape().dims(), &[2, 8], "modality {} shape", i);
    }
}

#[test]
fn test_fusion_energy_consistency() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let result = fusion.fuse(&[m1, m2]).unwrap();

    // Fusion energy should be finite
    assert!(
        result.fusion_energy.is_finite(),
        "fusion energy should be finite: {}",
        result.fusion_energy
    );
    println!("Fusion energy: {:.4}", result.fusion_energy);
}

#[test]
fn test_single_modality_passthrough() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let result = fusion.fuse(std::slice::from_ref(&m1)).unwrap();

    // Single modality: gate score should be 1.0
    assert_eq!(result.gate_scores.len(), 1);
    assert!(
        (result.gate_scores[0] - 1.0).abs() < 0.01,
        "single modality gate score should be ~1.0: {:.4}",
        result.gate_scores[0]
    );
}

#[test]
fn test_attention_weights_structure() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (2, config.embed_dim), &device).unwrap();
    let result = fusion.fuse(&[m1, m2]).unwrap();

    // attention_weights is [M x M] matrix
    assert_eq!(result.attention_weights.len(), 2, "should have 2 rows");
    for row in &result.attention_weights {
        assert_eq!(row.len(), 2, "each row should have 2 cols");
        for w in row {
            assert!(!w.is_nan(), "attention weight should not be NaN");
        }
    }
}

#[test]
fn test_gate_temperature_effect() {
    let device = Device::Cpu;

    // High temperature -> more uniform gating
    let config_hot = CrossAttentionConfig {
        gate_temperature: 10.0,
        ..CrossAttentionConfig::default()
    };
    let fusion_hot = CrossAttentionFusion::new(&config_hot, &device).unwrap();

    // Low temperature -> more selective gating
    let config_cold = CrossAttentionConfig {
        gate_temperature: 0.1,
        ..CrossAttentionConfig::default()
    };
    let fusion_cold = CrossAttentionFusion::new(&config_cold, &device).unwrap();

    // Use deterministic inputs with clear difference so temperature effect is observable
    let m1 = Tensor::full(2.0f32, (1, config_hot.embed_dim), &device).unwrap();
    let m2 = Tensor::zeros((1, config_hot.embed_dim), candle_core::DType::F32, &device).unwrap();

    let result_hot = fusion_hot.fuse(&[m1.clone(), m2.clone()]).unwrap();
    let result_cold = fusion_cold.fuse(&[m1, m2]).unwrap();

    // Cold should have more extreme gate scores (closer to 0 or 1)
    let hot_spread = (result_hot.gate_scores[0] - result_hot.gate_scores[1]).abs();
    let cold_spread = (result_cold.gate_scores[0] - result_cold.gate_scores[1]).abs();

    assert!(
        cold_spread >= hot_spread,
        "cold temperature should produce more selective gating: hot={:.4}, cold={:.4}",
        hot_spread,
        cold_spread
    );
}

#[test]
fn test_fusion_output_finite() {
    let config = CrossAttentionConfig::default();
    let device = Device::Cpu;
    let fusion = CrossAttentionFusion::new(&config, &device).unwrap();

    let m1 = Tensor::rand(0.0f32, 1.0f32, (4, config.embed_dim), &device).unwrap();
    let m2 = Tensor::rand(0.0f32, 1.0f32, (4, config.embed_dim), &device).unwrap();
    let result = fusion.fuse(&[m1, m2]).unwrap();

    let vals = result.fused.to_vec2::<f32>().unwrap();
    for row in &vals {
        for v in row {
            assert!(
                v.is_finite(),
                "fused output contains non-finite value: {}",
                v
            );
        }
    }
}
