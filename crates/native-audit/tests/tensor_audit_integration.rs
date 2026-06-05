use candle_core::Device;
use native_audit::TensorAudit;

#[test]
fn test_smollm2_tensor_extraction_and_tcm() {
    let device = Device::Cpu;
    // Cargamos el modelo y apuntamos a la capa 6
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

    let z_axis = audit
        .compute_tcm_z_axis(&hidden_state)
        .expect("Fallo en cálculo TCM");
    println!("📐 TCM Z-axis: {:.4}", z_axis);

    assert!(
        z_axis > 0.0,
        "Z-axis debe detectar magnitud (max abs Z > 0)"
    );
}
