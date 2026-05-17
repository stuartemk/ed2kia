# Guía de Contribución — ed2kIA

Gracias por tu interés en contribuir a ed2kIA. Este proyecto es infraestructura pública de código abierto para la interpretabilidad de IA, construido bajo los principios de transparencia, meritocracia y uso ético.

## Principios Fundamentales

1. **Código Auditable:** Todo el código debe ser público y verificable
2. **Cero Telemetría:** No se permiten datos salientes ni tracking
3. **Cero Lógica Financiera:** No hay tokens, pagos ni mecanismos financieros
4. **Cero Unsafe:** Todo el código Rust debe ser memory-safe sin `unsafe`
5. **Licencia Apache 2.0 + Ethical Use:** Todo el código contribuido debe ser compatible

## 🚀 Primeros Pasos

**¿Eres nuevo/a?** Consulta la [Guía del Primer Contribuidor](docs/community/first-contributor-guide.md) para un walkthrough paso a paso desde cero hasta tu primer PR mergeado.

- [Issues good-first-issue](https://github.com/Stuartemk/ed2kIA/issues?q=label:good-first-issue) — Issues recomendados para empezar
- [Pipeline CI/CD](.github/workflows/ci.yml) — Configuración del pipeline de validación
- [Codecov Config](.github/codecov.yml) — Umbrales de coverage

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

## Protocolo Auto-Push Permanente

A partir de v1.6.0-stable, el proyecto adopta un protocolo de auto-push automatizado para mantener velocidad de desarrollo y transparencia.

### Flujo

1. **Validar:** Cada fase termina con checks automáticos:
   ```bash
   cargo check --features stable
   cargo test --features stable
   ```
2. **Commit:** Si validación = PASS, commit automático con conventional commits:
   ```bash
   git add -A
   git commit -m "type(scope): descripción concisa

   - Cambio 1
   - Cambio 2
   - Métricas relevantes
   "
   ```
3. **Push:** Si commit = SUCCESS, push automático:
   ```bash
   git push origin main
   ```

### Reglas

| Regla | Descripción |
|-------|-------------|
| Validación obligatoria | NUNCA push sin cargo check + cargo test PASS |
| Conventional commits | type(scope): description (feat, fix, docs, chore, perf, refactor, test) |
| Transparencia | Cada commit incluye métricas de validación en el mensaje |
| Error handling | Si validación = FAIL, detener y reportar — NO forzar push |
| Branch protection | Main protegido: requiere CI green en GitHub Actions |

### Excepciones

- **Hotfixes SEV-1:** Permitido push directo con label `hotfix` + rollback plan documentado
- **Drafts experimentales:** Usar ramas `experiment/*` sin auto-push a main

## Embudo de Contribuidores

ed2kIA usa un sistema de tiers progresivos para reconocer y promover contribuidores. Consulta [`docs/community/contributor-funnel.md`](docs/community/contributor-funnel.md) para detalles completos.

| Tier | Entrada | Privilegios | Gobernanza |
|------|---------|-------------|-----------|
| **Spectator** | Automática | Lectura, discusión | Sin voto |
| **Contributor** | Primer PR mergeado | Write access (PR), issues | 0.5x voto |
| **Advocate** | 500 rep + 5 reviews | Triage, labels | 1x voto |
| **Steward** | Nomination + election | PR approval, proposals | 2x voto |
| **Guardian** | Election + community | Admin, releases, steering | 3x voto |

## Recursos

### Onboarding

- **Guía del Primer Contribuidor:** [docs/community/first-contributor-guide.md](docs/community/first-contributor-guide.md)
- **Embudo de Contribuidores:** [docs/community/contributor-funnel.md](docs/community/contributor-funnel.md)
- **Issues good-first-issue:** https://github.com/Stuartemk/ed2kIA/issues?q=label:good-first-issue

### Pipeline CI/CD

- **CI Pipeline:** [.github/workflows/ci.yml](.github/workflows/ci.yml)
- **CI v1.8 Pipeline:** [.github/workflows/ci-v1.8.yml](.github/workflows/ci-v1.8.yml)
- **Codecov Config:** [.github/codecov.yml](.github/codecov.yml)

### Arquitectura & Documentación

- **Arquitectura:** [docs/architecture_v1.6.0.md](docs/architecture_v1.6.0.md)
- **Transparencia:** [docs/TRANSPARENCY_FRAMEWORK.md](docs/TRANSPARENCY_FRAMEWORK.md)
- **Gobernanza:** [docs/GOVERNANCE.md](docs/GOVERNANCE.md)
- **Migración:** [docs/migration_guide_v1.5_to_v1.6.md](docs/migration_guide_v1.5_to_v1.6.md)
- **Roadmap v1.7:** [docs/v1.7-roadmap-placeholder.md](docs/v1.7-roadmap-placeholder.md)
- **Roadmap v1.8 (ChatGPT Moment):** [docs/roadmap/v1.8-chatgpt-moment.md](docs/roadmap/v1.8-chatgpt-moment.md)
- **Arquitectura Reputación:** [docs/architecture/reputation-gamification.md](docs/architecture/reputation-gamification.md)
- **Arquitectura WASM/Mobile:** [docs/architecture/mobile-browser-expansion.md](docs/architecture/mobile-browser-expansion.md)
- **Estrategia de Financiamiento:** [docs/funding-strategy.md](docs/funding-strategy.md)

### RFC & Benchmarks

- **RFC-001: Latencia Mitigation:** [docs/rfc/rfc-001-latency-mitigation-v1.7.md](docs/rfc/rfc-001-latency-mitigation-v1.7.md)
- **Benchmark Runner:** [benchmarks/README.md](benchmarks/README.md)
- **Baseline v1.7:** [benchmarks/results/baseline-v1.7.json](benchmarks/results/baseline-v1.7.json)

## 🎓 Programa de Mentorship

¿Quieres contribuir pero no sabes por dónde empezar? Nuestro programa de mentorship te guía paso a paso.

### Niveles de Mentorship

| Nivel | Requisitos | Descripción |
|-------|-----------|-------------|
| 🌱 **Seed** | Primer PR | Guía completa desde setup hasta primer merge |
| 🌿 **Sprout** | 2+ PRs mergeados | Asignación a módulo específico + mentor dedicado |
| 🌳 **Tree** | Dueño de módulo | Mentor de otros contribuidores + code review |

### Cómo Unirse

1. Abre un issue con el template "Mentorship Request"
2. Indica tu área de interés: `p2p`, `sae`, `zkp`, `governance`, `bridge`, `ui`
3. Un mentor te asignará dentro de 48 horas
4. Check-ins semanales durante 4 semanas (extendible)

### Recursos para Mentores

- [Onboarding Script](scripts/mentorship_onboarding.sh) — Automatización de asignación y seguimiento
- [First Contributor Guide](docs/community/first-contributor-guide.md) — Walkthrough paso a paso
- [Grant Follow-up Tracker](docs/grants/follow-up-tracker.md) — Estado de grants activos

### Para Mentees

```bash
# Verificar que el onboarding está completo
bash scripts/mentorship_onboarding.sh onboarding-check

# Verificar estado de grants
bash scripts/mentorship_onboarding.sh grants-status
```

## 📦 Versioning & Release Strategy

ed2kIA usa **Semantic Versioning** (`vMAJOR.MINOR.PATCH`) con identificadores pre-release (`alpha`, `beta`, `rc`).

### Feature Gates

El proyecto usa feature gates en `Cargo.toml` para controlar el scope de cada build:

| Feature Gate | Uso | Ejemplo |
|--------------|-----|---------|
| `stable` | Producción | `cargo build --features stable` |
| `v1.8-sprint1` | Beta features | `cargo build --features v1.8-sprint1` |
| `v1.8-sprint2` | Beta features | `cargo build --features v1.8-sprint2` |
| `cuda` / `metal` | Hardware acceleration | `cargo build --features cuda` |

### Cómo Verificar tu Build

```bash
# Build de producción (recomendado)
cargo build --release --features stable

# Build con features beta
cargo build --features "stable,v1.8-sprint1,v1.8-sprint2"

# Validación completa
cargo check --features stable
cargo clippy --features stable -- -D warnings
cargo test --features stable
```

### Release Tags

Los releases se marcan con tags anotados: `v1.8.0-beta.1`, `v1.7.0-stable`, etc.

```bash
# Ver tags disponibles
git tag -l 'v*' | sort -v

# Checkout versión específica
git checkout v1.8.0-beta.1
```

### Referencias

- **Versioning Alignment:** [`docs/roadmap/versioning-alignment.md`](docs/roadmap/versioning-alignment.md) — Matriz completa Fase ↔ Versión
- **Changelog:** [`release/changelog.md`](release/changelog.md) — Historial de cambios
- **Source of Truth:** [`docs/roadmap/source-of-truth.md`](docs/roadmap/source-of-truth.md) — Referencia maestra

## 🌍 Ambassador & Grant Support

### Ambassador Program

¿Quieres representar a ed2kIA en tu comunidad? Únete al programa de embajadores:

- **Programa completo:** [`docs/community/ambassador-program.md`](docs/community/ambassador-program.md)
- **Niveles:** Seed → Sprout → Tree (con beneficios crecientes)
- **Responsabilidades:** Eventos, contenido educativo, mentoría, feedback comunitario
- **Aplicar:** Join Discord #ambassadors channel o abre un issue con label `ambassador`

### Grant Execution Support

El proyecto participa en programas de financiamiento (Gitcoin, NSF, OSSF). Para apoyar:

- **Grant drafts:** [`docs/grants/`](docs/grants/) — Revisar y mejorar propuestas
- **Submission tracker:** [`docs/grants/submission-tracker.md`](docs/grants/submission-tracker.md)
- **Execution script:** [`scripts/grant_execution_support.sh`](scripts/grant_execution_support.sh) — Genera paquetes con checksums SHA256
- **Manifest:** Ejecuta `./scripts/grant_execution_support.sh --manifest` para ver el estado actual

**IMPORTANTE:** Los grants requieren autenticación humana en portales externos. Los scripts solo preparan paquetes — nunca simulan envíos.

## 🏆 Sistema de Reconocimiento

### Niveles de Contribuidor

| Nivel | Requisitos | Beneficios |
|-------|-----------|------------|
| **Bronze** | 1 PR mergeado | Badge, página de contribuidor |
| **Silver** | 5 PRs, 10 tests | Early access, reconocimiento mensual |
| **Gold** | 20 PRs, 50 tests | Derechos de voto, path a maintainer |
| **Platinum** | 50 PRs, 150 tests | Nominación maintainer |
| **Diamond** | 100 PRs, 300 tests | Hall of Fame |

### Insignias (Badges)

Las insignias se generan automáticamente basadas en contribuciones:

| Insignia | Criterio |
|----------|---------|
| 🌱 Seed | Primer PR mergeado |
| 🐛 Bug Hunter | 5 bugs arreglados |
| 🔨 Builder | 3 features implementadas |
| ✅ Test Master | 50 tests escritos |
| 📚 Wizard | 10 docs mejorados |
| ⚡ Guru | Mejora de performance |
| 🛡️ Sentinel | Issue de seguridad encontrado |
| 🚀 Engineer | Release validado |
| 🏛️ Sage | RFC aprobado |
| 🌟 Pioneer | Contribuyó a v2.0 |

### Generar Badges

```bash
./scripts/generate_contributor_badges.sh --output ./badges --tier all
```

### Hall of Fame

Miembros destacados por contribuciones excepcionales:

| Nombre | Contribución | Inducido |
|--------|-------------|----------|
| @Stuartemk | Fundador, lead developer | 2026-05-16 |
| Roo (AI) | Operaciones autónomas, CI/CD | 2026-05-16 |

### Referencias

- [Milestone Tracker](docs/community/milestone-tracker.md) — Tracking completo de hitos
- [Project Constitution](docs/governance/project-constitution.md) — Principios del proyecto
- [GOVERNANCE.md](GOVERNANCE.md) — Framework de gobernanza

## 🌐 v2.1 Ambassador Workflow

Esta sección describe cómo los embajadores y contribuidores pueden participar en el desarrollo de v2.1 usando feature gates, testing protocol y gobernanza comunitaria.

### Feature Gates v2.1

Los módulos v2.1 están protegidos con feature gates para mantener la estabilidad de v2.0.0-stable:

| Feature Gate | Módulo | Descripción |
|--------------|--------|-------------|
| `v2.1-sprint1` | Core scaffolds | Infraestructura base v2.1 |
| `v2.1-observability` | NodeMetrics, HealthEndpoint | Métricas Prometheus, health checks |
| `v2.1-security-hardening` | DependencyPin, CVE tracking | Remediation de 14 CVEs (wasmtime, rustls-webpki) |
| `v2.1-zkp-v3` | Multi-curve ZKP | BN254/BLS12-381/Pasta curves |
| `v2.1-gui` | Tauri scaffold | Desktop GUI foundation |
| `v2.1-enterprise` | Federation v5+ | Multi-tenant federation |

**Activar feature gates:**
```bash
# Build con observability
cargo build --features v2.1-observability

# Build con security hardening
cargo build --features v2.1-security-hardening

# Build con múltiples features
cargo build --features "v2.1-observability,v2.1-security-hardening"
```

### Testing Protocol v2.1

```bash
# Tests de integración con feature gates
cargo test --features v2.1-observability
cargo test --features v2.1-security-hardening

# Benchmarks (placeholders — miden overhead de scaffolds)
cargo bench --features v2.1-observability --no-run
cargo bench --features v2.1-security-hardening --no-run

# Validación completa v2.1
cargo check --all-targets --features "v2.1-observability,v2.1-security-hardening"
```

**Criterios de aceptación:**
- `cargo check` → 0 errores
- `cargo test --features v2.1-*` → Todos los tests pasan
- Benchmarks → Compilan sin errores (resultados son referencia)

### Governance & RFCs

Participa en la gobernanza comunitaria de v2.1:

- **Voting Dashboard:** [`docs/community/voting-dashboard-active.md`](docs/community/voting-dashboard-active.md) — Propuestas activas, pesos de votación
- **RFC Tracking:** [`docs/governance/rfc-tracking.md`](docs/governance/rfc-tracking.md) — Estado de RFCs abiertos/cerrados
- **Pesos de votación:** Spectator (0x) → Contributor (0.5x) → Advocate (1x) → Steward (2x) → Guardian (3x)

**Flujo de votación:**
1. Revisa propuestas activas en el Voting Dashboard
2. Verifica tu peso de votación en el Milestone Tracker
3. Vota vía GitHub Issues (reacción + comentario justificado)
4. Tally automático vía `scripts/voting-tally.sh`

### Ethics & Licensing

- **Licencia:** Apache 2.0 + Cláusula de Uso Ética
- **Cero lógica financiera:** No tokens, no pagos, no mecanismos financieros
- **Transparencia de datos:** Cero telemetría, cero tracking, datos locales
- **Cero unsafe:** Todo el código Rust debe ser memory-safe

### CI/CD Integration

Los PRs que toquen paths protegidos requieren review de CODEOWNERS:

| Path | Equipo Responsable |
|------|-------------------|
| `/docs/governance/` | @ed2kia/governance-team |
| `/docs/grants/` | @ed2kia/maintainers |
| `/infra/` | @ed2kia/ops-team |
| `/tests/integration/` | @ed2kia/core-team |
| `/benchmarks/` | @ed2kia/core-team |
| `CHANGELOG.md` | @ed2kia/maintainers |
| `Cargo.toml` | @ed2kia/core-team |

**Validación automática:**
- `feature-gate-check`: Verifica que v2.1 features no estén en default
- `feature-gate-tests`: Ejecuta tests con cada feature gate v2.1
- `codeowners-sync`: Verifica que PRs toquen paths protegidos

## Contacto

- Issues: https://github.com/ed2kia/ed2kIA/issues
- Legal: contacto@ed2kIA.org
