# Migración de v1.3.0 a v1.4.0 STABLE

## Resumen de Cambios

Esta guía cubre la migración desde ed2kIA v1.3.0 STABLE a v1.4.0 STABLE.

### Cambios Principales

| Módulo | v1.3.0 | v1.4.0 | Breaking? |
|--------|--------|--------|-----------|
| SAE Fine-Tuning | v3 | v4 | Sí |
| Cross-Model Aligner | v1 | v2 | Sí |
| Adaptive Checkpoint | v1 | v2 | Sí |
| Federation Scaling | v3 | v4 | Sí |
| Predictive Sharder | — | v4 | Nuevo |
| Async ZKP | v5 | v8 | Sí |
| Cross-Federation Verification | — | Nuevo | Nuevo |

## Pasos de Migración

### 1. Actualizar Dependencias

```toml
# Cargo.toml
[dependencies]
ed2kia = "1.4.0"

[features]
stable = [...]  # Use --features stable
```

### 2. Actualizar Feature Flags

```bash
# v1.3.0
cargo build --features stable

# v1.4.0 (mismo comando, módulos actualizados internamente)
cargo build --features stable
```

### 3. Migrar APIs de SAE Fine-Tuning

#### v1.3.0 (FineTuningV3)
```rust
use ed2kia::sae::fine_tuning_v3::{FineTuningV3, FineTuningV3Config};
```

#### v1.4.0 (FineTuningV4)
```rust
use ed2kia::sae::fine_tuning_v4::{FineTuningV4, FineTuningV4Config};
```

**Cambios:**
- `FineTuningV3Config` → `FineTuningV4Config` (nuevos campos: `compression_threshold`, `fallback_uptime`)
- `execute_round()` ahora retorna `TrainingRoundResult` con campo `compressed_size`
- Checkpointing integrado con `AdaptiveCheckpointV2`

### 4. Migrar APIs de Federation Scaling

#### v1.3.0 (ScalingV3)
```rust
use ed2kia::federation::scaling_v3::{FederationScalingV3, ScalingV3Config};
```

#### v1.4.0 (ScalingV4)
```rust
use ed2kia::federation::scaling_v4::{FederationScalingV4, ScalingV4Config};
```

**Cambios:**
- `ScalingV3Config` → `ScalingV4Config` (nuevos campos: `ema_alpha`, `prediction_horizon`)
- `evaluate()` ahora retorna `Vec<ScalingDecisionV4>` con campo `predicted_load`
- Nuevo: `predict_shard_load()` para forecasting proactivo

### 5. Migrar APIs de Async ZKP

#### v1.3.0 (AsyncZKPv5)
```rust
use ed2kia::zkp::async_zkp_v5::{AsyncZKPV5, ZKPV5Config};
```

#### v1.4.0 (AsyncZKPv8)
```rust
use ed2kia::zkp::async_zkp_v8::{AsyncZKPV8, ZKPV8Config};
```

**Cambios:**
- Sistema de credibilidad por federación con decay configurable
- Multi-federation relay con detección de ciclos
- Budget management por federación
- `submit_proof()` ahora requiere `federation_id`

### 6. Nuevos Módulos

#### Predictive Sharder v4
```rust
use ed2kia::federation::predictive_sharder_v4::{PredictiveSharderV4, PredictiveSharderConfig};
```

#### Cross-Federation Verification
```rust
use ed2kia::zkp::cross_federation_verification::{CrossFederationVerifier, CrossFedConfig};
```

## Rollback

Para rollback a v1.3.0:
```bash
git checkout v1.3.0
cargo build --release --features stable
```

## Soporte

- Issues: GitHub Issues del repositorio
- Docs: `docs/` directory
- Community: Canal de Discord

---

**Versión:** v1.4.0 STABLE
**Fecha:** 2026-05-11
