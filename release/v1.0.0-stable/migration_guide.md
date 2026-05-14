# Migration Guide — ed2kIA v0.x → v1.0.0 STABLE

## Resumen

Esta guía describe los pasos para migrar desde cualquier versión v0.x a v1.0.0 STABLE. La migración es **backward-compatible**: todos los feature flags legacy funcionan como aliases a `stable`.

## Feature Flag Mapping

| v0.x Feature Flag | v1.0.0 Equivalent | Status |
|---|---|---|
| `core-only` | `stable` | Deprecated (alias) |
| `phase6-core` | `stable` | Deprecated (alias) |
| `phase6-sprint2` | `stable` | Deprecated (alias) |
| `phase6-experimental` | `stable` | Deprecated (alias) |
| `phase7-sprint1` | `stable` | Deprecated (alias) |
| `phase7-sprint2` | `stable` | Deprecated (alias) |
| `phase8-sprint1` | `stable` | Deprecated (alias) |
| `phase8-sprint2` | `stable` | Deprecated (alias) |
| `phase9-sprint1` | `stable` | Deprecated (alias) |

### Comandos de Migración

**Antes (v0.x):**
```bash
cargo build --features phase6-experimental
cargo test --features phase9-sprint1
```

**Después (v1.0.0):**
```bash
cargo build  # stable es default
cargo test   # todos los módulos incluidos
```

**Compatibilidad (transicional):**
```bash
# Estos siguen funcionando en v1.0.0 (alias a stable)
cargo build --features phase6-experimental  # OK, pero deprecated
cargo build --features phase9-sprint1       # OK, pero deprecated
```

## Pasos de Upgrade

### 1. Actualizar Cargo.toml

```toml
[dependencies]
ed2kia = "1.0.0"  # Cambiar versión
```

### 2. Actualizar Feature Flags (Opcional)

Si usas feature flags específicos, reemplázalos con `stable`:

```diff
- features = ["phase6-experimental", "phase9-sprint1"]
+ features = ["stable"]
```

### 3. Validar Compilación

```bash
cargo check --features stable
cargo clippy --features stable -- -D warnings
cargo test --features stable
```

### 4. Migrar Imports (si usas lib.rs)

Los paths de importación son los mismos. Los nuevos módulos v2/v3 están disponibles:

```rust
// Fase 5 governance (original)
use ed2kia::governance::proposal::ProposalManager;

// Fase 9 liquid governance (nuevo)
use ed2kia::governance_v2::liquid::LiquidGovernance;

// Fase 8 UI backend (original)
use ed2kia::ui::backend::UiBackendState;

// Fase 9 realtime UI (nuevo)
use ed2kia::ui_v2::realtime::RealtimeUIBackend;
```

## Breaking Changes

**Ninguno.** v1.0.0 es 100% backward-compatible con v0.5.0+.

## Validación Post-Migración

```bash
# Verificar versión
cargo run -- --version
# Expected: ed2kia 1.0.0

# Verificar features
cargo run -- features
# Expected: stable, p2p, sae, consensus, alignment, federation, marketplace, ui, slo, governance

# Ejecutar tests E2E
cargo test --features stable --test final_e2e
```

## Rollback

Si necesitas volver a una versión anterior:

```bash
# Rollback a v0.5.0
cargo add ed2kia@0.5.0
# Restaurar feature flags originales
```

## Soporte

- Issues: https://github.com/ed2kia/ed2kia/issues
- Docs: https://github.com/ed2kia/ed2kia/docs
- Comunidad: Discord/Matrix (ver README.md)
