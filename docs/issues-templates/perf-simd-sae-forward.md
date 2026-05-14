## 🚀 Performance: AVX2 Optimization for SAE Forward Pass

**RFC:** [RFC-001](../rfc/rfc-001-latency-mitigation-v1.7.md) §5.2
**Difficulty:** Intermediate
**Estimated Time:** 12-16 hours
**Labels:** `performance`, `simd`, `sae`

### Descripción

Implementar optimizaciones AVX2 para el forward pass del SAE (Sparse Autoencoder), acelerando las operaciones de multiplicación matricial y activación sparse dentro de `src/sae/fine_tuning_v7.rs`.

### Contexto

El forward pass del SAE es una operación crítica en el pipeline de fine-tuning. Con optimizaciones SIMD (AVX2), podemos procesar 8 valores f32 en paralelo, logrando un speedup teórico de 8x en operaciones vectorizables.

### Criterios de Aceptación

- [ ] Implementar función `matmul_avx2` para multiplicación matricial optimizada
- [ ] Speedup ≥ 2x vs implementación scalar en benchmark
- [ ] Resultados numéricos idénticos (bit-exact) vs baseline
- [ ] Tests existentes pasan sin modificaciones
- [ ] Documentación de requisitos de CPU (fallback para sin AVX2)

### Enfoque Sugerido

```rust
/// AVX2-optimized matrix multiplication for SAE forward pass.
/// Falls back to scalar if AVX2 not available.
#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
pub fn matmul_avx2(a: &[f32], b: &[f32], out: &mut [f32]) {
    // Use std::arch::x86_64::_mm256_mul_ps and related intrinsics
}

/// Scalar fallback for non-AVX2 platforms.
pub fn matmul_scalar(a: &[f32], b: &[f32], out: &mut [f32]) {
    // Standard implementation
}
```

### Recursos

- [RFC-001 §5.2](../rfc/rfc-001-latency-mitigation-v1.7.md)
- [Rust SIMD Book](https://doc.rust-lang.org/book/ch15-06-unsafe-blocks.html)
- [std::arch::x86_64](https://doc.rust-lang.org/core/arch/x86_64/index.html)
- [`src/sae/fine_tuning_v7.rs`](../../src/sae/fine_tuning_v7.rs)
