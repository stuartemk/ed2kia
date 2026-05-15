# Guía del Primer Contribuidor — ed2kIA

Bienvenido/a a ed2kIA. Esta guía te llevará paso a paso desde cero hasta tu primer Pull Request mergeado. Cada paso incluye comandos exactos, troubleshooting y enlaces a recursos relevantes.

**Tiempo estimado:** 2-4 horas para completar el onboarding completo.

---

## Tabla de Contenidos

1. [Prerrequisitos](#1-prerrequisitos)
2. [Fork y Clone](#2-fork-y-clone)
3. [Setup del Entorno](#3-setup-del-entorno)
4. [Verificar que Todo Funciona](#4-verificar-que-todo-funciona)
5. [Elegir tu Primer Issue](#5-elegir-tu-primer-issue)
6. [Desarrollar tu Cambio](#6-desarrollar-tu-cambio)
7. [Submit tu Pull Request](#7-submit-tu-pull-request)
8. [Proceso de Review](#8-proceso-de-review)
9. [Troubleshooting Común](#9-troubleshooting-común)
10. [Recursos Adicionales](#10-recursos-adicionales)

---

## 1. Prerrequisitos

### Herramientas Requeridas

| Herramienta | Versión Mínima | Instalación |
|-------------|----------------|-------------|
| Git | 2.30+ | https://git-scm.com/downloads |
| Rust (rustup) | 1.75+ | https://rustup.rs/ |
| Cargo | Incluido en rustup | Automático con rustup |
| GitHub Account | Cuenta activa | https://github.com/signup |

### Verificar Instalación

```bash
# Verificar Git
git --version
# Esperado: git version 2.x.x

# Verificar Rust
rustc --version
# Esperado: rustc 1.75.x o superior

# Verificar Cargo
cargo --version
# Esperado: cargo 1.75.x o superior
```

### Troubleshooting: Rust no instalado

```bash
# Windows (PowerShell)
winget install Rustlang.Rustup

# Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# macOS
brew install rustup
rustup-init
```

---

## 2. Fork y Clone

### Paso 1: Fork del Repositorio

1. Ve a https://github.com/Stuartemk/ed2kIA
2. Haz clic en el botón **Fork** (esquina superior derecha)
3. Espera a que se complete la copia en tu cuenta

### Paso 2: Clone tu Fork

```bash
# Clonar tu fork (reemplaza TU_USUARIO con tu username de GitHub)
git clone https://github.com/TU_USUARIO/ed2kIA.git
cd ed2kIA

# Agregar el repositorio original como upstream
git remote add upstream https://github.com/Stuartemk/ed2kIA.git

# Verificar remotes
git remote -v
# Esperado:
# origin  https://github.com/TU_USUARIO/ed2kIA.git (fetch)
# origin  https://github.com/TU_USUARIO/ed2kIA.git (push)
# upstream        https://github.com/Stuartemk/ed2kIA.git (fetch)
# upstream        https://github.com/Stuartemk/ed2kIA.git (push)
```

### Troubleshooting: "remote origin already exists"

```bash
# Si ya tienes un origin configurado diferente
git remote remove origin
git remote add origin https://github.com/TU_USUARIO/ed2kIA.git
```

---

## 3. Setup del Entorno

### Instalar Herramientas de Desarrollo

```bash
# Instalar clippy (linting)
rustup component add clippy

# Instalar rustfmt (formateo)
rustup component add rustfmt

# Opcional: Instalar cargo-tarpaulin (coverage)
cargo install cargo-tarpaulin

# Opcional: Instalar criterion (benchmarks)
# Criterion se descarga automáticamente con cargo bench
```

### Verificar Features del Proyecto

```bash
# Verificar que el proyecto compila con features stable
cargo check --features stable

# Si es tu primer build, puede tomar 5-10 minutos descargar dependencias
```

### Troubleshooting: Cargo check falla

```bash
# Limpiar cache y reintentar
cargo clean
cargo check --features stable

# Si persiste, actualizar Rust
rustup update
```

---

## 4. Verificar que Todo Funciona

### Pipeline de Validación Local

Antes de hacer cualquier cambio, verifica que el proyecto compila y los tests pasan:

```bash
# Paso 1: Verificar compilación
cargo check --features stable

# Paso 2: Linting
cargo clippy --features stable -- -D warnings

# Paso 3: Formateo
cargo fmt -- --check

# Paso 4: Tests unitarios
cargo test --features stable

# Paso 5: Tests de integración
cargo test --features stable --test '*'
```

### Resultados Esperados

| Comando | Resultado Esperado |
|---------|-------------------|
| `cargo check` | `Finished dev [unoptimized + debuginfo] target(s)` |
| `cargo clippy` | `Finished dev [unoptimized + debuginfo] target(s)` |
| `cargo fmt -- --check` | (sin output = todo formateado correctamente) |
| `cargo test` | `test result: ok. X passed; 0 failed` |

### Troubleshooting: Tests fallan

```bash
# Ver qué test falló específicamente
cargo test --features stable -- --nocapture

# Si es un test intermitente, reintentar
cargo test --features stable

# Si persiste, reportar en GitHub Issues con:
# - rustc --version
# - Sistema operativo
# - Output completo del test
```

---

## 5. Elegir tu Primer Issue

### Issues Recomendados para Primeros Contribuidores

Busca issues con el label `good-first-issue`:

1. **En GitHub:** https://github.com/Stuartemk/ed2kIA/issues?q=label:good-first-issue
2. **En la CLI:** Pregunta a un maintainer para que te asigne uno

### Categorías de Issues Disponibles

| Categoría | Ejemplo | Dificultad |
|-----------|---------|------------|
| **Documentación** | Actualizar README, agregar ejemplos | Fácil |
| **Tests** | Agregar tests unitarios a módulos existentes | Fácil-Media |
| **Benchmarks** | Medir performance de módulos nuevos | Media |
| **RFC-001** | Implementar optimizaciones de latencia | Media-Alta |
| **Features** | Nuevos módulos siguiendo el patrón internal | Media |

### Cómo Claimar un Issue

1. Comenta en el issue: "Me gustaría trabajar en este issue"
2. Incluye un plan breve de cómo lo abordarías
3. Espera asignación (un maintainer te asignará)
4. Una vez asignado, comienza el desarrollo

---

## 6. Desarrollar tu Cambio

### Crear Rama de Trabajo

```bash
# Actualizar main desde upstream
git checkout main
git pull upstream main

# Crear rama para tu cambio (usa naming convention clara)
git checkout -b feature/descripcion-corta
# Ejemplo: git checkout -b feature/add-sae-validation-tests
# Ejemplo: git checkout -b docs/update-contributor-guide
```

### Estructura de Módulos (Patrones del Proyecto)

Si estás agregando un nuevo módulo, sigue este patrón:

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

### Validar tu Cambio

```bash
# Después de cada cambio significativo, ejecuta:
cargo check --features stable
cargo clippy --features stable -- -D warnings
cargo test --features stable
```

### Commits

```bash
# Conventional commits (REQUERIDO)
git add -A
git commit -m "type(scope): descripción concisa

- Cambio 1
- Cambio 2
- Métricas relevantes (si aplica)
"

# Tipos permitidos:
# feat: Nueva feature
# fix: Corrección de bug
# docs: Documentación
# test: Tests
# refactor: Refactorización
# perf: Optimización de performance
# chore: Mantenimiento
```

---

## 7. Submit tu Pull Request

### Push tu Rama

```bash
# Push tu rama a tu fork
git push origin feature/descripcion-corta
```

### Crear Pull Request

1. Ve a https://github.com/Stuartemk/ed2kIA/pulls
2. Clic en **New Pull Request**
3. Selecciona tu rama como compare
4. Llena el template:

```markdown
## Descripción
[Breve descripción del cambio]

## Tipo de Cambio
- [ ] Bug fix
- [ ] Nueva feature
- [ ] Documentación
- [ ] Test
- [ ] Refactorización
- [ ] Optimización de performance

## Validación
- [ ] `cargo check --features stable` pasa
- [ ] `cargo clippy --features stable -- -D warnings` pasa
- [ ] `cargo test --features stable` pasa
- [ ] No contiene `unsafe` blocks
- [ ] No contiene lógica financiera
- [ ] No contiene telemetría

## Issue Relacionado
Closes #XXX (si aplica)
```

---

## 8. Proceso de Review

### Qué Esperar

| Paso | Tiempo Estimado | Responsable |
|------|----------------|-------------|
| Asignación de reviewer | 24-48 horas | Maintainer |
| Primera review | 48-72 horas | Reviewer asignado |
| Iteraciones | 24-48 horas por ciclo | Ambos |
| Merge | Inmediato tras approval | Maintainer |

### Criterios de Review

Los reviewers verificarán:

- [ ] `cargo check --features stable` pasa
- [ ] `cargo clippy --features stable -- -D warnings` pasa
- [ ] `cargo test --features stable` pasa
- [ ] No contiene `unsafe` blocks
- [ ] No contiene lógica de telemetría
- [ ] No contiene lógica financiera
- [ ] Sigue el patrón de módulos (`mod internal` + `pub use internal::*`)
- [ ] Incluye tests unitarios
- [ ] Documentación actualizada si aplica

### Responder a Feedback

1. Realiza los cambios solicitados en tu rama local
2. Commit y push (se actualizará el PR automáticamente)
3. Comenta en el PR indicando que los cambios están listos

```bash
# Hacer cambios y actualizar PR
git add -A
git commit -m "address(review): feedback de reviewer"
git push origin feature/descripcion-corta
```

---

## 9. Troubleshooting Común

### Problemas de Compilación

| Error | Solución |
|-------|----------|
| `could not compile` | `cargo clean && cargo check --features stable` |
| `feature not found` | Verificar `--features stable` en el comando |
| `dependency not found` | `cargo update && cargo check --features stable` |
| `rustc version too old` | `rustup update` |

### Problemas de Git

| Error | Solución |
|-------|----------|
| `rejected` (push) | `git pull --rebase upstream main && git push --force-with-lease` |
| `detached HEAD` | `git checkout main` |
| `untracked files` | `git status` para revisar, `git add` los necesarios |

### Problemas de Tests

| Error | Solución |
|-------|----------|
| Test intermitente | Reintentar `cargo test --features stable` |
| `test panicked` | `cargo test --features stable -- --nocapture` para ver detalles |
| Timeout | Verificar si el test tiene loops infinitos |

### Problemas de CI

| Error | Solución |
|-------|----------|
| CI falla en GitHub | Revisar logs en Actions, replicar localmente |
| `clippy` warnings | `cargo clippy --features stable -- -D warnings` |
| Coverage bajo | Agregar más tests al módulo |

---

## 10. Recursos Adicionales

### Documentación Técnica

- **RFC-001: Latencia Mitigation** — [`docs/rfc/rfc-001-latency-mitigation-v1.7.md`](../rfc/rfc-001-latency-mitigation-v1.7.md)
- **Arquitectura v1.6** — [`docs/architecture_v1.6.0.md`](../architecture_v1.6.0.md)
- **Roadmap v1.8** — [`docs/roadmap/v1.8-chatgpt-moment.md`](../roadmap/v1.8-chatgpt-moment.md)
- **Framework de Transparencia** — [`docs/TRANSPARENCY_FRAMEWORK.md`](../TRANSPARENCY_FRAMEWORK.md)

### Benchmarks

- **Benchmark Runner** — [`benchmarks/README.md`](../benchmarks/README.md)
- **Baseline v1.7** — [`benchmarks/results/baseline-v1.7.json`](../benchmarks/results/baseline-v1.7.json)
- **Ejecutar benchmarks:** `cargo bench -p ed2kIA-benchmarks`

### Ética y Código de Conducta

- **Cero Lógica Financiera:** No tokens, no pagos, no mecanismos financieros
- **Cero Telemetría:** No datos salientes, no tracking
- **Cero Unsafe:** Todo el código Rust debe ser memory-safe
- **Código de Conducta:** Respeto, transparencia y meritocracia

### Gobernanza

- **Sistema de Gobernanza** — [`docs/GOVERNANCE.md`](../GOVERNANCE.md)
- **Embudo de Contribuidores** — [`docs/community/contributor-funnel.md`](contributor-funnel.md)
- **Contributing Guide** — [`CONTRIBUTING.md`](../../CONTRIBUTING.md)

### Pipeline CI/CD

- **CI Pipeline** — [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)
- **CI v1.8 Pipeline** — [`.github/workflows/ci-v1.8.yml`](../../.github/workflows/ci-v1.8.yml)
- **Codecov Config** — [`.github/codecov.yml`](../../.github/codecov.yml)

### Issues Activos

- **Good First Issue:** https://github.com/Stuartemk/ed2kIA/issues?q=label:good-first-issue
- **Help Wanted:** https://github.com/Stuartemk/ed2kIA/issues?q=label:help-wanted
- **All Issues:** https://github.com/Stuartemk/ed2kIA/issues

---

## Checklist de Onboarding

Usa esta checklist para trackear tu progreso:

- [ ] Rust 1.75+ instalado y verificado
- [ ] Git configurado con tu cuenta
- [ ] Fork creado y clonado
- [ ] Upstream remote configurado
- [ ] `cargo check --features stable` pasa
- [ ] `cargo test --features stable` pasa
- [ ] Issue `good-first-issue` elegido y asignado
- [ ] Rama de trabajo creada
- [ ] Cambio desarrollado y validado localmente
- [ ] PR creado con template completo
- [ ] Esperando review

---

## Después de tu Primer Merge

¡Felicidades! Una vez mergeado tu primer PR:

1. **Reputación:** Ganarás créditos de reputación en el sistema
2. **Tier:** Serás promovido/a a **Contributor** (Tier 2)
3. **Privilegios:** Acceso a issues más complejos, participación en triage
4. **Siguiente:** Explora issues `help-wanted` o propone tus propias features

Consulta el [Embudo de Contribuidores](contributor-funnel.md) para ver el camino completo de crecimiento.

---

## Contacto

- **Issues:** https://github.com/Stuartemk/ed2kIA/issues
- **Discusión:** GitHub Discussions
- **Legal:** contacto@ed2kIA.org

---

*Última actualización: Mayo 2026 | v1.8 Sprint 1*
