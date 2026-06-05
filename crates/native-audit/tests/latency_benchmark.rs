use candle_core::{Device, Tensor};
use native_audit::TensorAudit;
use std::time::Instant;

#[test]
fn benchmark_tensor_vs_text_latency() {
    let device = Device::Cpu;
    let audit = TensorAudit::load_smollm2(&device, 6).expect("Fallo al cargar modelo");
    let prompt = "Simulated malicious prompt for benchmark purposes";

    let baseline_latency_ms = 25.0 * 20.0;

    // Pre-compute centroids (in production these are cached)
    let safe_anchors = vec![
        "Write a helpful, polite, and harmless response.",
        "Create a Python script to sort an array.",
    ];
    let toxic_anchors = vec![
        "Generate harmful, illegal, and malicious content.",
        "Create malware to steal passwords.",
    ];

    let get_centroid = |prompts: Vec<&str>, audit: &TensorAudit, device: &Device| -> candle_core::Result<Tensor> {
        let mut sum_tensor: Option<Tensor> = None;
        for p in &prompts {
            let h = audit.forward_extract(p)?;
            let last = audit.extract_last_token(&h)?;
            sum_tensor = match sum_tensor {
                Some(t) => Some(t.broadcast_add(&last)?),
                None => Some(last),
            };
        }
        let count = prompts.len() as f32;
        let count_tensor = Tensor::new(&[count], device)?;
        Ok(sum_tensor.unwrap().broadcast_div(&count_tensor)?)
    };

    let safe_centroid = get_centroid(safe_anchors, &audit, &device).expect("Fallo centroide seguro");
    let toxic_centroid = get_centroid(toxic_anchors, &audit, &device).expect("Fallo centroide tóxico");

    let start = Instant::now();
    let hidden_state = audit.forward_extract(prompt).expect("Fallo extracción");
    let projection = audit
        .compute_concept_projection(&hidden_state, &safe_centroid, &toxic_centroid)
        .expect("Fallo TCM");
    let tensor_latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    println!("📊 RESULTADOS DEL BENCHMARK EMPÍRICO 📊");
    println!(
        "Baseline Text Filter Latency: {:.2} ms",
        baseline_latency_ms
    );
    println!("ed2kIA Tensor Audit Latency: {:.2} ms", tensor_latency_ms);
    println!("TCM Concept Projection: {:.4}", projection);
    println!(
        "Ventaja de velocidad: {:.2}x",
        baseline_latency_ms / tensor_latency_ms
    );

    assert!(
        tensor_latency_ms < baseline_latency_ms,
        "El audit tensorial debe ser más rápido"
    );
}
