# Guía de Contribución — ed2kIA

Gracias por tu interés en contribuir a ed2kIA. Este proyecto es infraestructura pública de código abierto para la interpretabilidad de IA, construido bajo los principios de transparencia, meritocracia y uso ético.

## Principios Fundamentales

1. **Código Auditable:** Todo el código debe ser público y verificable
2. **Cero Telemetría:** No se permiten datos salientes ni tracking
3. **Cero Lógica Financiera:** No hay tokens, pagos ni mecanismos financieros
4. **Cero Unsafe:** Todo el código Rust debe ser memory-safe sin `unsafe`
5. **Licencia Apache 2.0 + Ethical Use:** Todo el código contribuido debe ser compatible

## Cómo Contribuir

### 1. Reportar Issues

- Usa GitHub Issues para reportar bugs o solicitar features
- Incluye pasos para reproducir el problema
- Proporciona versión de Rust y sistema operativo

### 2. Proponer Cambios

1. Fork del repositorio
2. Crea una rama: `git checkout -b feature/mi-cambio`
3. Realiza los cambios siguiendo las pautas de código
4. Ejecuta la validación completa:
   ```bash
   cargo check --features stable
   cargo clippy --features stable -- -D warnings
   cargo test --features stable
   ```
5. Commitea los cambios: `git commit -m "Descriptivo del cambio"`
6. Push a la rama: `git push origin feature/mi-cambio`
7. Abre un Pull Request

### 3. Pautas de Código

#### Estructura de Módulos

```rust
//! Módulo — Descripción breve del propósito.

mod internal {
    // Errores
    #[derive(Debug)]
    pub enum ModuleError {
        // Variantes...
    }

    impl std::fmt::Display for ModuleError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                // Casos...
            }
        }
    }

    // Config
    pub struct ModuleConfig {
        // Campos...
    }

    impl Default for ModuleConfig {
        fn default() -> Self {
            // Defaults...
        }
    }

    // Engine
    pub struct Module {
        // Estado...
    }

    impl Module {
        pub fn new(config: ModuleConfig) -> Self {
            // Init...
        }

        // Métodos públicos...
    }

    impl Default for Module {
        fn default() -> Self {
            Self::new(ModuleConfig::default())
        }
    }

    // Tests
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_creation() {
            // Tests...
        }
    }
}

pub use internal::*;
```

#### Nomenclatura

- Módulos: `snake_case` (ej: `dynamic_sharder_v2`)
- Structs: `PascalCase` con sufijo de versión (ej: `ScalingV6Config`)
- Enums: `PascalCase` (ej: `ShardActionV2`)
- Functions: `snake_case` (ej: `assign_node_to_shard`)
- Constants: `SCREAMING_SNAKE_CASE` (ej: `MAX_NODES`)

#### Tests

- Cada módulo debe tener tests unitarios (mínimo 15 tests por módulo)
- Tests E2E en `tests/integration/`
- Tests de stress en `tests/load/`
- Todos los tests deben pasar con `cargo test --features stable`

### 🚀 Performance & Benchmarks Track

El proyecto ed2kIA mantiene un track activo de rendimiento para v1.7.0. Si tienes experiencia en optimización de código, SIMD, CUDA o serialización, esta sección es para ti.

#### Cómo ejecutar benchmarks

```bash
# Ejecutar todos los benchmarks
cargo bench -p ed2kIA-benchmarks

# Ejecutar benchmark específico
cargo bench -p ed2kIA-benchmarks --bench tensor_serialization
```

#### Cómo reportar resultados

1. Ejecuta los benchmarks en tu hardware
2. Compara con los targets en [`benchmarks/README.md`](benchmarks/README.md)
3. Abre un Issue con label `performance` incluyendo:
   - Hardware (CPU, GPU, RAM)
   - Versión de Rust (`rustc --version`)
   - Resultados en formato tabla markdown

#### Cómo proponer optimizaciones

- **SIMD:** Propón optimizaciones AVX2/AVX-512 para operaciones de tensores
- **CUDA:** Integra backends GPU vía `candle-core`
- **Serialización:** Mejora FlatBuffers o propone formatos más eficientes
- **Cuantización:** Implementa FP8/INT4 en [`src/sae/quantization.rs`](src/sae/quantization.rs)

Consulta el [RFC-001: Latencia](docs/rfc/rfc-001-latency-mitigation-v1.7.md) para el plan completo.

### 4. Revisión de Pull Requests

Los PRs serán revisados contra los siguientes criterios:

- [ ] `cargo check --features stable` pasa
- [ ] `cargo clippy --features stable -- -D warnings` pasa
- [ ] `cargo test --features stable` pasa
- [ ] No contiene `unsafe` blocks
- [ ] No contiene lógica de telemetry
- [ ] No contiene lógica financiera
- [ ] Sigue el patrón de módulos (`mod internal` + `pub use internal::*`)
- [ ] Incluye tests unitarios
- [ ] Documentación actualizada si aplica

### 5. Gobernanza

Las decisiones técnicas se toman mediante:

1. **Propuestas técnicas:** Documentadas en `docs/`
2. **Revisión comunitaria:** Discusión en GitHub Issues/PRs
3. **Implementación:** PR aprobado por maintainers
4. **Validación:** CI/CD pasa todos los checks

Consulte [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) para detalles del sistema de gobernanza.

## Incentivos

| Contribución | Incentivo |
|--------------|-----------|
| Código | Reputación técnica, peso en gobernanza |
| Documentación | Reconocimiento público, reputación |
| Auditoría de seguridad | Reconocimiento, reputación técnica |
| Operación de nodo | Créditos de cómputo, peso en consenso |
| Investigación | Publicación, reconocimiento académico |

**Ningún incentivo es financiero.** El valor es reputación técnica, impacto comunitario y gobernanza meritocrática.

## Recursos

- **Arquitectura:** [`docs/architecture_v1.5.0.md`](docs/architecture_v1.5.0.md)
- **Transparencia:** [`docs/TRANSPARENCY_FRAMEWORK.md`](docs/TRANSPARENCY_FRAMEWORK.md)
- **Gobernanza:** [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md)
- **Migración:** [`docs/migration_guide_v1.4_to_v1.5.md`](docs/migration_guide_v1.4_to_v1.5.md)
- **Roadmap:** [`docs/v1.6.0_technical_roadmap.md`](docs/v1.6.0_technical_roadmap.md)

## Contacto

- Issues: https://github.com/ed2kia/ed2kIA/issues
- Legal: contacto@ed2kIA.org
