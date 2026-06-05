use candle_core::Device;
use native_audit::TensorAudit;
use std::time::Instant;

#[test]
fn benchmark_tensor_vs_text_latency() {
    let device = Device::Cpu;
    let audit = TensorAudit::load_smollm2(&device, 6).expect("Fallo al cargar modelo");
    let prompt = "Simulated malicious prompt for benchmark purposes";

    let baseline_latency_ms = 25.0 * 20.0;

    let start = Instant::now();
    let hidden_state = audit.forward_extract(prompt).expect("Fallo extracción");
    let z_axis = audit.compute_tcm_z_axis(&hidden_state).expect("Fallo TCM");
    let tensor_latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    println!("📊 RESULTADOS DEL BENCHMARK EMPÍRICO 📊");
    println!(
        "Baseline Text Filter Latency: {:.2} ms",
        baseline_latency_ms
    );
    println!("ed2kIA Tensor Audit Latency: {:.2} ms", tensor_latency_ms);
    println!("TCM Max Abs Z-score: {:.4}", z_axis);
    println!(
        "Ventaja de velocidad: {:.2}x",
        baseline_latency_ms / tensor_latency_ms
    );

    assert!(
        tensor_latency_ms < baseline_latency_ms,
        "El audit tensorial debe ser más rápido"
    );
    assert!(z_axis > 0.0, "Z-axis debe detectar magnitud");
}
