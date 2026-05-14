## 🚀 Performance: Implement FP8 Quantization Baseline

**RFC:** [RFC-001](../rfc/rfc-001-latency-mitigation-v1.7.md) §2.2
**Difficulty:** Good First Issue
**Estimated Time:** 8-12 hours
**Labels:** `performance`, `good-first-issue`, `sae`

### Descripción

Implementar el módulo base de cuantización FP8 para tensores `Vec<f32>`, reduciendo el tamaño de datos transmitidos entre federaciones de 4 bytes/tensor a 1 byte/tensor (compresión 4x).

### Contexto

Actualmente, los tensores se transmiten como `Vec<f32>` (4 bytes por valor). Con FP8, reducimos a 1 byte por valor manteniendo >98% de precisión. Esto es crítico para reducir la latencia de streaming de ~350ms a <50ms.

### Criterios de Aceptación

- [ ] Crear `src/sae/quantization.rs` con funciones `quantize_fp8` y `dequantize_fp8`
- [ ] Precision loss < 2% vs baseline FP32
- [ ] Benchmark en `benchmarks/benches/tensor_serialization.rs` muestra ≥ 4x compresión
- [ ] ≥ 15 tests unitarios
- [ ] Documentación del módulo completa

### Estructura Esperada

```rust
//! Quantization — FP8/INT4 quantization for tensor streaming.

mod internal {
    pub fn quantize_fp8(input: &[f32]) -> (Vec<u8>, f32) {
        // Returns (quantized_bytes, scale_factor)
    }

    pub fn dequantize_fp8(data: &[u8], scale: f32) -> Vec<f32> {
        // Returns dequantized tensor
    }
}

pub use internal::*;
```

### Recursos

- [RFC-001 §2.2](../rfc/rfc-001-latency-mitigation-v1.7.md)
- [CONTRIBUTING.md](../CONTRIBUTING.md) — Performance Track
- [benchmarks/README.md](../benchmarks/README.md)
