# Guía de Migración: v1.1.0 → v1.2.0

## Resumen de Cambios

v1.2.0 introduce mejoras significativas en federación, marketplace y alineación. La mayoría de cambios son aditivos; algunos APIs de federation son breaking.

## Cambios en Feature Flags

### v1.1.0
```toml
# Feature flags individuales por sprint
features = ["v1.1-sprint1", "v1.1-sprint2", ...]
```

### v1.2.0
```toml
# Todo consolidado en `stable`
features = ["stable"]
```

**Acción:** Reemplazar cualquier referencia a feature flags individuales por `--features stable`.

## Cambios en APIs de Federation

### `create_shard()` — Breaking Change

**v1.1.0:**
```rust
sharder.create_shard(shard_id, node_id)?;  // 2 args
```

**v1.2.0:**
```rust
sharder.create_shard(primary_node)?;  // 1 arg (shard_id auto-generado)
```

**Acción:** Actualizar llamadas a `create_shard()` para usar solo `primary_node`. Los shard IDs se auto-generan como `shard_N`.

### `start_migration()` — Breaking Change

**v1.1.0:**
```rust
sharder.start_migration(shard_id, &target_node)?;  // &String
```

**v1.2.0:**
```rust
sharder.start_migration(shard_id, target_node)?;  // String
```

**Acción:** Pasar `String` en vez de `&String` para `target_node`.

### `get_active_shards()` — Comportamiento Cambiado

**v1.1.0:** Devolvía todos los shards.
**v1.2.0:** Filtra por `ShardState::Active` solo.

**Acción:** Usar `analyze_balance()` para inspección completa de shards.

## Nuevos Módulos

| Módulo | Feature Flag | Descripción |
|--------|-------------|-------------|
| `marketplace_v3` | `v1.2-sprint4` | Marketplace descentralizado |
| `alignment_v3` | `v1.2-sprint4` | Alignment Loop v3 |
| `federation_scaling_v3` | `v1.2-sprint4` | Federation Scaling v3 |

## Configuración de Red

**Sin cambios.** La configuración de red (`seed_config.toml`, `genesis/config.toml`) es backward compatible.

## Actualización de Dependencias

```bash
# Actualizar Cargo.lock
cargo update

# Verificar build
cargo build --release --features stable

# Ejecutar tests
cargo test --test v1_2_sprint4_e2e --features v1.2-sprint4
cargo test --test sprint4_stress --features v1.2-sprint4
```

## Checklist de Migración

- [ ] Reemplazar feature flags por `--features stable`
- [ ] Actualizar llamadas a `create_shard()` (1 arg)
- [ ] Actualizar llamadas a `start_migration()` (String param)
- [ ] Reemplazar `get_active_shards()` con `analyze_balance()` donde aplique
- [ ] Verificar build: `cargo build --release --features stable`
- [ ] Verificar tests: `cargo test --features v1.2-sprint4`
- [ ] Verificar clippy: `cargo clippy --features stable -- -D warnings`

## Soporte

- Issue Tracker: https://github.com/ed2kia/ed2kIA/issues
- Discusión: Canal #migration en Discord
