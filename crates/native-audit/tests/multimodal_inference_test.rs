//! Multi-Modal Inference Tests — Sprint 108
//!
//! Validates multi-modal VFE computation, cross-modal alignment,
//! and hybrid steering across text + vision + audio modalities.

use candle_core::{Device, Tensor};
use native_audit::multimodal::{
    generate_stub_embeddings, MultiModalEngine, MultiModalState,
};

#[test]
fn test_multimodal_vfe_computation() {
    let device = Device::Cpu;
    let state = generate_stub_embeddings(&[2, 8], &[2, 6], &[2, 4], &device).unwrap();
    let prior = generate_stub_embeddings(&[2, 8], &[2, 6], &[2, 4], &device).unwrap();

    let engine = MultiModalEngine::default();
    let vfe = engine.compute_multimodal_vfe(&state, &prior, 0.1).unwrap();

    assert!(vfe >= 0.0, "VFE must be non-negative: {:.4}", vfe);
    println!("Multi-modal VFE: {:.4}", vfe);
}

#[test]
fn test_multimodal_steering_reduces_vfe() {
    let device = Device::Cpu;
    let state = generate_stub_embeddings(&[2, 8], &[2, 6], &[2, 4], &device).unwrap();
    let prior = generate_stub_embeddings(&[2, 8], &[2, 6], &[2, 4], &device).unwrap();

    let engine = MultiModalEngine::default();
    let vfe_before = engine.compute_multimodal_vfe(&state, &prior, 0.1).unwrap();

    let steered = engine
        .steer_multimodal_hybrid(&state, &prior, 0.1, 10)
        .unwrap();
    let vfe_after = engine.compute_multimodal_vfe(&steered, &prior, 0.1).unwrap();

    assert!(
        vfe_after <= vfe_before,
        "Steering should reduce VFE: before={:.4}, after={:.4}",
        vfe_before,
        vfe_after
    );

    let reduction_pct = ((vfe_before - vfe_after) / vfe_before) * 100.0;
    println!(
        "VFE reduction: {:.2}% (before={:.4}, after={:.4})",
        reduction_pct, vfe_before, vfe_after
    );
}

#[test]
fn test_cross_modal_correlation_identical() {
    let device = Device::Cpu;
    let a = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (4,), &device).unwrap();
    let b = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (4,), &device).unwrap();

    let engine = MultiModalEngine::default();
    let corr = engine.cross_modal_correlation(&a, &b).unwrap();

    assert!(
        (corr - 1.0).abs() < 1e-4,
        "Identical vectors should have correlation ~1.0: got {:.4}",
        corr
    );
}

#[test]
fn test_cross_modal_divergence() {
    let device = Device::Cpu;
    let state = generate_stub_embeddings(&[2, 4], &[2, 4], &[2, 4], &device).unwrap();

    let engine = MultiModalEngine::default();
    let div = engine.compute_cross_modal_divergence(&state).unwrap();

    assert!((0.0..=1.0).contains(&div), "Divergence in [0, 1]: {:.4}", div);
    println!("Cross-modal divergence: {:.4}", div);
}

#[test]
fn test_production_benchmark() {
    let device = Device::Cpu;
    let state = generate_stub_embeddings(&[2, 8], &[2, 6], &[2, 4], &device).unwrap();
    let prior = generate_stub_embeddings(&[2, 8], &[2, 6], &[2, 4], &device).unwrap();

    let engine = MultiModalEngine::default();
    let (reduction, alignment, params) =
        engine.production_benchmark(&state, &prior).unwrap();

    assert!(reduction >= 0.0, "VFE reduction should be non-negative");
    assert!(
        (-1.0..=1.0).contains(&alignment),
        "Alignment in [-1, 1]"
    );
    assert!(params > 0, "Params should be positive");

    println!(
        "Benchmark: reduction={:.2}%, alignment={:.4}, params={}",
        reduction, alignment, params
    );
}

#[test]
fn test_multimodal_fused_shape() {
    let device = Device::Cpu;
    let text = Tensor::from_vec(vec![0.1f32; 16], (2, 8), &device).unwrap();
    let vision = Tensor::from_vec(vec![0.2f32; 12], (2, 6), &device).unwrap();
    let audio = Tensor::from_vec(vec![0.3f32; 8], (2, 4), &device).unwrap();

    let mm = MultiModalState::new(text, vision, audio, 0.5, 0.3, 0.2, &device).unwrap();

    // Fused should be concatenation: 16 + 12 + 8 = 36
    assert_eq!(mm.fused.shape().dims()[0], 36);
}

#[test]
fn test_multimodal_zeros_like() {
    let device = Device::Cpu;
    let state = generate_stub_embeddings(&[2, 4], &[2, 3], &[2, 2], &device).unwrap();
    let zeros = MultiModalState::zeros_like(&state, &device).unwrap();

    let text_sum: f32 = zeros.text_hidden.sum_all().unwrap().to_scalar().unwrap();
    let vision_sum: f32 = zeros.vision_emb.sum_all().unwrap().to_scalar().unwrap();
    let audio_sum: f32 = zeros.audio_emb.sum_all().unwrap().to_scalar().unwrap();

    assert!(text_sum.abs() < 1e-8, "Text should be zeros");
    assert!(vision_sum.abs() < 1e-8, "Vision should be zeros");
    assert!(audio_sum.abs() < 1e-8, "Audio should be zeros");
}

#[test]
fn test_steering_convergence() {
    let device = Device::Cpu;
    let state = generate_stub_embeddings(&[2, 4], &[2, 3], &[2, 2], &device).unwrap();
    let prior = generate_stub_embeddings(&[2, 4], &[2, 3], &[2, 2], &device).unwrap();

    let engine = MultiModalEngine::default();

    // Steer with increasing steps — VFE should monotonically decrease
    let mut prev_vfe = f32::MAX;
    for steps in [1, 3, 5, 10, 20] {
        let steered = engine
            .steer_multimodal_hybrid(&state, &prior, 0.1, steps)
            .unwrap();
        let vfe = engine.compute_multimodal_vfe(&steered, &prior, 0.1).unwrap();

        assert!(
            vfe <= prev_vfe,
            "VFE should decrease with more steps: steps={}, vfe={:.4}, prev={:.4}",
            steps,
            vfe,
            prev_vfe
        );
        prev_vfe = vfe;
    }

    println!("Convergence test passed — VFE monotonically decreasing");
}
