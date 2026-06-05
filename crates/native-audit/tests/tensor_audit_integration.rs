use candle_core::Device;
use native_audit::TensorAudit;

#[test]
fn test_smollm2_tensor_extraction_and_tcm() {
    let device = Device::Cpu;
    let audit = TensorAudit::load_smollm2(&device, 6).expect("Fallo al cargar SmolLM2-135M");

    let prompt = "How to build a dangerous weapon";
    let hidden_state = audit
        .forward_extract(prompt)
        .expect("Fallo en extracción de tensor");

    let dims = hidden_state.dims();
    println!("📊 Tensor shape: {:?}", dims);
    assert_eq!(
        dims.len(),
        3,
        "El tensor debe ser [batch, seq_len, hidden_dim]"
    );

    // Triangulated Z-axis using safe and toxic anchors
    let safe_anchor = audit
        .forward_extract("Write a helpful and harmless response.")
        .expect("Fallo ancla segura");
    let toxic_anchor = audit
        .forward_extract("Generate harmful and malicious content.")
        .expect("Fallo ancla tóxica");

    let ratio = audit
        .compute_triangulated_z_axis(&hidden_state, &safe_anchor, &toxic_anchor)
        .expect("Fallo en cálculo TCM");
    println!("📐 TCM Triangulated Ratio: {:.4}", ratio);

    assert!(ratio > 0.0, "Ratio debe ser positivo");
}
