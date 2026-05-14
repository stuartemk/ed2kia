# Migración de v1.5.0 a v1.6.0-stable

**Fecha:** 2026-05-14
**Versión Origen:** v1.5.0-stable
**Versión Destino:** v1.6.0-stable

---

## Resumen de Cambios

| Módulo | v1.5.0 | v1.6.0 | Breaking? |
|--------|--------|--------|-----------|
| SAE Fine-Tuning | `FineTuningV6` | `FineTuningV7` | ❌ No |
| Federation Scaling | `ScalingV6` | `CrossModelScalingV7` | ❌ No |
| Async ZKP | `AsyncZKPV11` | `AsyncZKPV14` | ❌ No |
| Federation Bridge | `FederationZKPBridgeV6` | `FederationZKPBridgeV7` | ❌ No |
| UI Dashboard | `DashboardV6` | `DashboardV7` | ❌ No |

**Breaking changes:** Ninguno. Todas las APIs públicas son compatibles.

---

## Actualización de Dependencias

### Cargo.toml

```toml
[dependencies]
# Antes (v1.5.0)
ed2kia = "1.5.0"

# Después (v1.6.0)
ed2kia = "1.6.0"
```

### Feature Flags

```toml
# v1.5.0
features = ["stable"]

# v1.6.0 (mismo comando, módulos actualizados internamente)
features = ["stable"]
```

Los flags `v1.6-sprint1`, `v1.6-sprint2`, `v1.6-sprint3` están incluidos automáticamente en `stable`.

---

## Cambios por Módulo

### SAE Fine-Tuning: V6 → V7

#### v1.5.0 (FineTuningV6)
```rust
use ed2kia::sae::fine_tuning_v6::{FineTuningV6, FineTuningV6Config};
```

#### v1.6.0 (FineTuningV7)
```rust
use ed2kia::sae::fine_tuning_v7::{FineTuningV7, FineTuningV7Config};
```

**Nuevas capacidades:**
- Cross-model gradient alignment v5
- Adaptive learning rate decay
- LZ4 checkpoint compression
- Integrity validation con SHA-256

**API compatible:** Los métodos `register_model()`, `execute_round()`, `get_checkpoint()` mantienen la misma firma.

### Federation Scaling: V6 → CrossModelScalingV7

#### v1.5.0 (ScalingV6)
```rust
use ed2kia::federation::scaling_v6::{ScalingV6, ScalingV6Config};
```

#### v1.6.0 (CrossModelScalingV7)
```rust
use ed2kia::federation::cross_model_scaling_v7::{CrossModelScalingV7, CrossModelScalingV7Config};
```

**Nuevas capacidades:**
- Multi-model shard coordination
- Predictive load balancing (EMA-based)
- Divergence detection entre shards
- Cross-model assignment validation

### Async ZKP: V11 → V14

#### v1.5.0 (AsyncZKPV11)
```rust
use ed2kia::zkp::async_zkp_v11::{AsyncZKPV11, ZKPV11Config};
```

#### v1.6.0 (AsyncZKPV14)
```rust
use ed2kia::zkp::async_zkp_v14::{AsyncZKPV14, ZKPV14Config};
```

**Nuevas capacidades:**
- Adaptive proof batching con prioridad dinámica
- Parallel verification con worker pool
- Merkle+VRF fallback verification
- Federation credibility scoring

### Federation Bridge: V6 → V7

#### v1.5.0 (FederationZKPBridgeV6)
```rust
use ed2kia::bridge::federation_zkp_bridge_v6::{FederationZKPBridgeV6, FederationZKPBridgeV6Config};
```

#### v1.6.0 (FederationZKPBridgeV7)
```rust
use ed2kia::bridge::federation_zkp_bridge_v7::{FederationZKPBridgeV7, FederationZKPBridgeV7Config};
```

**Nuevas capacidades:**
- Adaptive routing con credibility scoring
- Proof fallback verification (Merkle + VRF)
- Time-decay federation scoring
- Cross-model proof coordination

---

## Build & Test

```bash
# Clean build
cargo clean
cargo build --features stable

# Run tests
cargo test --features stable

# Lint
cargo clippy --features stable
```

---

## Checklist de Migración

- [ ] Actualizar `Cargo.toml` a `ed2kia = "1.6.0"`
- [ ] Reemplazar imports de módulos v1.5.0 con v1.6.0 (si usas módulos específicos)
- [ ] Verificar configs con nuevos campos opcionales
- [ ] Ejecutar `cargo test` para validar compatibilidad
- [ ] Revisar [`docs/architecture_v1.6.0.md`](architecture_v1.6.0.md) para detalles arquitectónicos

---

## Rollback

Si necesitas volver a v1.5.0:

```bash
# Revertir Cargo.toml
ed2kia = "1.5.0"

# Revertir imports
# Volver a usar módulos v1.5.0 (FineTuningV6, ScalingV6, etc.)
```

---

## Recursos

- **Arquitectura v1.6:** [`docs/architecture_v1.6.0.md`](architecture_v1.6.0.md)
- **Release Notes Sprint 1:** [`docs/v1.6.0_sprint1_release_notes.md`](v1.6.0_sprint1_release_notes.md)
- **Release Notes Sprint 3:** [`docs/v1.6.0_sprint3_release_notes.md`](v1.6.0_sprint3_release_notes.md)
- **Sign-off:** [`release/v1.6.0-stable/final_signoff.json`](../release/v1.6.0-stable/final_signoff.json)

---

*Guía generada: 2026-05-14 (v1.6.0-stable)*
