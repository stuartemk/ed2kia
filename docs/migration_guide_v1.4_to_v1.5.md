# Migración de v1.4.0 a v1.5.0 STABLE

## Resumen de Cambios

Esta guía cubre la migración desde ed2kIA v1.4.0 STABLE a v1.5.0 STABLE.

### Cambios Principales

| Módulo | v1.4.0 | v1.5.0 | Breaking? |
|--------|--------|--------|-----------|
| SAE Fine-Tuning | v4 | v6 | Sí |
| Cross-Model Aligner | v2 | v3 (integrado en Gradient Sync v6) | Sí |
| Adaptive Checkpoint | v2 | v4 | Sí |
| Federation Scaling | v4 | v6 | Sí |
| Predictive Sharder | v4 | Dynamic Sharder v2 | Sí |
| Async ZKP | v8 | v11 | Sí |
| Cross-Federation Verification | v1 | v2 | Sí |
| — | — | Gradient Sync v6 | Nuevo |

## Pasos de Migración

### 1. Actualizar Dependencias

```toml
# Cargo.toml
[dependencies]
ed2kia = "1.5.0"

[features]
stable = [...]  # Use --features stable
```

### 2. Actualizar Feature Flags

```bash
# v1.4.0
cargo build --features stable

# v1.5.0 (mismo comando, módulos actualizados internamente)
cargo build --features stable
```

### 3. Migrar APIs de SAE Fine-Tuning

#### v1.4.0 (FineTuningV4)
```rust
use ed2kia::sae::fine_tuning_v4::{FineTuningV4, FineTuningV4Config};
```

#### v1.5.0 (FineTuningV6)
```rust
use ed2kia::sae::fine_tuning_v5::{FineTuningV5, FineTuningV5Config};
```

**Cambios:**
- `FineTuningV4Config` → `FineTuningV5Config` (nuevos campos: `convergence_threshold`, `lr_decay_rate`, `multi_pass_enabled`)
- `execute_round()` ahora retorna `TrainingRoundResultV5` con campos `alignment_score` y `checkpoint_round`
- Cross-model alignment v3 integrado directamente en el motor
- Adaptive checkpointing v4 con integridad criptográfica

### 4. Migrar APIs de Federation Scaling

#### v1.4.0 (ScalingV4)
```rust
use ed2kia::federation::scaling_v4::{FederationScalingV4, ScalingV4Config};
```

#### v1.5.0 (ScalingV6)
```rust
use ed2kia::federation::scaling_v6::{ScalingV6, ScalingV6Config};
```

**Cambios:**
- `ScalingV4Config` → `ScalingV6Config` (nuevos campos: `partition_tolerance`, `max_nodes_per_shard`, `capacity_weight`)
- `evaluate()` → `assign_node_to_shard()` con selección basada en shard_score
- Nuevo: `predict_load()` con horizonte configurable
- Nuevo: `should_rebalance()` para detección automática de desbalanceo
- Tolerancia a particiones ≥99.5% garantizada

### 5. Migrar APIs de Predictive Sharder

#### v1.4.0 (PredictiveSharderV4)
```rust
use ed2kia::federation::predictive_sharder_v4::{PredictiveSharderV4, SharderV4Config};
```

#### v1.5.0 (DynamicSharderV2)
```rust
use ed2kia::federation::dynamic_sharder_v2::{DynamicSharderV2, DynamicSharderV2Config};
```

**Cambios:**
- `SharderV4Config` → `DynamicSharderV2Config` (nuevos campos: `split_threshold`, `merge_threshold`, `ema_alpha`)
- `predict()` → `predict_load(shard_id, horizon)` por shard individual
- Nuevo: `generate_actions()` retorna `Vec<ShardActionV2>` con decisiones de split/merge
- Nuevo: `execute_split()` y `execute_merge()` para acciones automáticas
- Nuevo: `health_check()` para monitoreo de salud de shards

### 6. Migrar APIs de Async ZKP

#### v1.4.0 (AsyncZKPv8)
```rust
use ed2kia::zkp::async_zkp_v8::{AsyncZKPV8, ZKPV8Config};
```

#### v1.5.0 (AsyncZKPv11)
```rust
use ed2kia::zkp::async_zkp_v11::{AsyncZKPV11, ZKPV11Config};
```

**Cambios:**
- `ZKPV8Config` → `ZKPV11Config` (nuevos campos: `batch_size`, `quorum_threshold`, `merkle_aggregation`)
- `submit_proof()` ahora incluye prioridad y TTL
- Nuevo: `create_batch()`, `add_to_batch()`, `complete_batch()` para batching dinámico
- Nuevo: `record_vote()` para verificación por quorum
- Credibilidad adaptativa con decay temporal automático

### 7. Migrar APIs de Cross-Federation Verification

#### v1.4.0 (CrossFederationVerifier)
```rust
use ed2kia::zkp::cross_federation_verifier::{CrossFederationVerifier, VerifierConfig};
```

#### v1.5.0 (CrossFederationVerifierV2)
```rust
use ed2kia::zkp::cross_federation_verifier_v2::{CrossFederationVerifierV2, CrossFederationVerifierV2Config};
```

**Cambios:**
- `VerifierConfig` → `CrossFederationVerifierV2Config` (nuevos campos: `quorum_threshold`, `reputation_weight`, `merkle_aggregation`)
- `verify()` → `create_session()` + `submit_vote()` + `check_quorum()`
- Nuevo: `aggregate_merkle_roots()` para agregación criptográfica
- Nuevo: `add_challenge()` para proof challenges
- Historial de verificación con auditoría completa

### 8. Nuevo: Gradient Sync v6

```rust
use ed2kia::federation::gradient_sync_v6::{GradientSyncV6, GradientSyncV6Config};

let config = GradientSyncV6Config {
    compression_ratio: 0.3,
    cross_model_weight: 0.2,
    reputation_weight: 0.5,
    // ...
};
let mut sync = GradientSyncV6::new(config);
sync.register_model("model_a".to_string(), 512);
sync.submit_gradients("model_a".to_string(), vec![0.1, 0.2, 0.3], current_ms());
let result = sync.execute_sync()?;
```

**Características:**
- Sincronización de gradientes con alineación cross-model
- Compresión top-k adaptativa
- Promedio ponderado por reputación
- Registro de modelos con EMA de gradientes

## Configuración por Defecto

| Módulo | Parámetro | Valor Default |
|--------|-----------|---------------|
| ScalingV6 | `partition_tolerance` | 0.995 |
| ScalingV6 | `max_nodes_per_shard` | 100 |
| DynamicSharderV2 | `split_threshold` | 0.85 |
| DynamicSharderV2 | `merge_threshold` | 0.15 |
| GradientSyncV6 | `compression_ratio` | 0.3 |
| GradientSyncV6 | `cross_model_weight` | 0.2 |
| AsyncZKPv11 | `batch_size` | 50 |
| AsyncZKPv11 | `quorum_threshold` | 0.67 |
| CrossFedVerifierV2 | `quorum_threshold` | 0.67 |
| CrossFedVerifierV2 | `reputation_weight` | 0.5 |

## Checklist de Migración

- [ ] Actualizar `Cargo.toml` a `ed2kia = "1.5.0"`
- [ ] Reemplazar imports de módulos v1.4.0 con v1.5.0
- [ ] Actualizar configs con nuevos campos requeridos
- [ ] Migrar llamadas API según tablas anteriores
- [ ] Ejecutar `cargo check --features stable`
- [ ] Ejecutar `cargo clippy --features stable -- -D warnings`
- [ ] Ejecutar `cargo test --features stable`
- [ ] Verificar guardrails (no unsafe, no telemetry, no financial logic)

## Rollback

Si experimenta problemas, puede revertir a v1.4.0:

```bash
git checkout v1.4.0
cargo build --release --features stable
```

## Soporte

Para problemas de migración, consulte:
- Issues en GitHub: https://github.com/ed2kia/ed2kia/issues
- Documentación: https://github.com/ed2kia/ed2kia/tree/main/docs
- Arquitectura: [`docs/architecture_v1.5.0.md`](architecture_v1.5.0.md)
