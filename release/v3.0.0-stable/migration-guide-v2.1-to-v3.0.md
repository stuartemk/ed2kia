# Guía de Migración: v2.1.0 → v3.0.0-stable

**Fecha:** 2026-05-25
**Dificultad:** Media
**Tiempo Estimado:** 30-60 minutos

---

## Resumen de Cambios

| Área | v2.1.0 | v3.0.0 | Impacto |
|------|--------|--------|---------|
| Pilares | Individual | 4 Pilares unificados bajo Omni-Node | Alto |
| Orquestación | `PillarOrchestrator` | `OmniNode` + `SymbioticRouter` | Alto |
| Mensajería | `orchestration::PillarMessage` | `runtime::pillar_messaging::PillarMessage` | Medio |
| SCT | `SCTDecision` directo | `Result<SCTDecision, SctError>` | Medio |
| CLI | Comandos v2.1 | `--omni-mode` añadido | Bajo |
| Feature Gates | `v2.1-*` | `v3.0-*` (coexisten) | Medio |

---

## Paso 1: Actualizar Cargo.toml

### Dependencias

```toml
[dependencies]
# Antes (v2.1)
ed2kia = { version = "2.1.0", features = ["v2.1-orchestrator"] }

# Ahora (v3.0)
ed2kia = { version = "3.0.0", features = ["v3.0-omni-integration"] }
```

### Feature Gates Migrados

| v2.1 (Legacy) | v3.0 (Nuevo) | Notas |
|---------------|--------------|-------|
| `v2.1-orchestrator` | `v3.0-orchestration` | Reemplazo directo |
| `v2.1-pillar-comm` | `v3.0-pillar-messaging` | Módulo movido a `runtime` |
| `v2.1-sct-core` | `v2.1-sct-core` | Sin cambios (compatible) |
| N/A | `v3.0-omni-integration` | Nuevo: integra los 4 pilares |
| N/A | `v3.0-corpuscular-bridge` | Pilar 1 |
| N/A | `v3.0-maieutic-synthesizer` | Pilar 2 |
| N/A | `v3.0-steganographic-survival` | Pilar 3 |
| N/A | `v3.0-resonance-interface` | Pilar 4 |

---

## Paso 2: Actualizar Imports

### PillarMessage

```rust
// ❌ Antes (v2.1)
use ed2kia::orchestration::PillarMessage;

// ✅ Ahora (v3.0)
use ed2kia::runtime::pillar_messaging::PillarMessage;
```

### OmniNode

```rust
// ✅ Nuevo en v3.0
use ed2kia::orchestration::{OmniNode, SymbioticRouter, ExistentialCreditLedger};
use ed2kia::orchestration::{PillarId, RoutingError, SymbiosisValidator};
```

### Migration Protocol

```rust
// ✅ Nuevo en v3.0 (requiere v3.0-omni-integration)
use ed2kia::pillars::steganographic::{
    MigrationHandshake, MigrationToken, MigrationNegotiator,
};
```

---

## Paso 3: Manejar SCT Results

### evaluate_trajectory()

```rust
// ❌ Antes (v2.1) — retorno directo
let tensor = StuartianTensor::new(0.7, 0.2, 0.5).unwrap();
let decision = tensor.evaluate_trajectory();
if decision.is_approved() { /* ... */ }

// ✅ Ahora (v3.0) — Result type
let tensor = StuartianTensor::new(0.7, 0.2, 0.5).unwrap();
let decision = tensor.evaluate_trajectory().map_err(|e| {
    eprintln!("SCT evaluation failed: {}", e);
    MyError::SctFailure(e)
})?;
if decision.is_approved() { /* ... */ }
```

---

## Paso 4: Configurar Omni-Node

### Inicialización Básica

```rust
use ed2kia::orchestration::OmniNode;

fn main() {
    let mut node = OmniNode::new();

    // Inicializar los 4 pilares con CE inicial
    node.initialize_pillars(100.0);

    // O registrar individualmente
    node.register_pillar(PillarId::CorpuscularBridge, 50.0);
}
```

### CLI --omni-mode

```bash
# Inicializar con CE default (100.0)
cargo run --bin ed2kia-cli --features "v3.0-omni-integration" -- omni

# CE personalizado + diagnóstico
cargo run --bin ed2kia-cli --features "v3.0-omni-integration" \
  -- omni --initial-ce 200.0 --diagnose
```

---

## Paso 5: Migration Protocol (Opcional)

Para clusters que desean integrarse:

```rust
use ed2kia::pillars::steganographic::{
    MigrationHandshake, MigrationNegotiator,
};
use ed2kia::alignment::sct_core::StuartianTensor;

let mut negotiator = MigrationNegotiator::new();

let handshake = MigrationHandshake {
    cluster_id: "datacenter-alpha".to_string(),
    capacity: 1_000_000,
    transports: vec![],  // TransportType list
    health_reports: vec![],
    signature: vec![],
    timestamp_ms: current_ms(),
    ce_budget: 500.0,
};

let tensor = StuartianTensor::new(0.8, 0.1, 0.6).unwrap();
match negotiator.negotiate_migration(&handshake, &tensor) {
    Ok(token) => println!("Cluster onboarded: {}", token.cluster_id),
    Err(e) => eprintln!("Migration failed: {}", e),
}
```

---

## Paso 6: Validar Migración

```bash
# 1. Verificar compilación
cargo check --features "v3.0-omni-integration"

# 2. Ejecutar tests
cargo test --features "v3.0-omni-integration"

# 3. Verificar lint
cargo clippy --features "v3.0-omni-integration" -- -D warnings

# 4. Benchmarks (opcional)
cargo bench --features "v3.0-scaling-bench" --bench omni_node_scaling
```

---

## Rollback

Si la migración falla, rollback a v2.1.0:

```bash
git checkout v2.1.0-stable
cargo build --release
```

Los feature gates v2.1 siguen funcionando y son compatibles con v3.0.

---

## Soporte

- **Issues:** https://github.com/ed2kia/ed2kIA/issues
- **Documentación:** https://ed2kia.github.io/ed2kIA
- **Gobernanza:** GOVERNANCE.md

*Esta guía se actualizará con cada sprint de v3.0.*
