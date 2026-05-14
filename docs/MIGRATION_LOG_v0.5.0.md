# Migration Log v0.5.0 (Release Candidate)

**Fecha:** 2026-05-03
**Estado:** ✅ Build Limpio (`cargo check --features "core-only"` exit code 0)
**Estrategia:** Híbrida (Fix-only + Feature Flags)

## Resumen

Migración exitosa de **306 errores → 0 errores** mediante estrategia híbrida:
- Migración API de dependencias (libp2p 0.53, wasmtime 17.0, arkworks stack)
- Feature flags para módulos experimentales (Fase 4-6)
- Correcciones en cascada de borrow/move errors

## Archivos Modificados

### 1. `src/p2p/swarm.rs` - libp2p 0.53 API
- `behaviour()` → `behaviour_mut()` para `send_request()` (línea 481)

### 2. `src/bridge/tensor_flow.rs` - Borrow/Move Fixes
- Added `impl std::fmt::Display for PipelineState`
- Fixed error message formatting (`error_msg` variable)
- Defer metrics updates to avoid borrow conflicts (`drop(entries)`)

### 3. `src/bridge/consciousness.rs` - Field Name Fix
- `ConflictReport` field: `activation` → `activation_value`
- `process_feedback_queue(&self)` → `&mut self` para mutable access

### 4. `src/consensus/validator.rs` - Serde + Borrow Fixes
- Added `Default` derive to `BatchState` con `#[default]` en `Collecting`
- Replaced `find_batch_key()` con inline search
- `#[serde(skip)]` + `#[serde(default = "std::time::Instant::now")]` para `Instant`
- Fixed `batch_id` moved error con `.clone()` antes de insert
- `set_zkp_confidence_threshold(&self)` → `&mut self`

### 5. `src/security/memory_guard.rs` - Array Comparison
- Fixed `&[u8]` vs `[u8]` comparison: `*c == first_block.as_slice()`

### 6. `src/zkp/circuit.rs` - Arkworks API
- `format_with` → `serialize_compressed(&mut Vec::new())`
- `to_ascii_hex_digit` → `format!("{:x}", b)`
- `is_zero()` → Added `use ark_ec::AffineRepr;`
- `into_big()` → `CanonicalSerialize` + `serialize_compressed()`

### 7. `src/zkp/verifier.rs` - MerkleTree API
- `MerkleTree::from_data()` returns `Result`, added error handling
- `root()` → `root.hash` field access
- `leaf_count()` → `leaf_count` field access
- `verify_proof()` API changes (static method)
- `set_min_confidence(&self)` → `&mut self`
- Fixed `merkle_root` String → `[u8; 32]` conversion con `sha2::Sha256::digest()`

### 8. `src/human/feedback_cli.rs` - Field Name + Move Fix
- `FeedbackStats` field: `total` → `total_feedback`
- Fixed `feedback` moved error con pre-extract `feedback_id`

### 9. `src/main.rs` - Type Annotations
- Added `Vec<&str>` type annotation para features
- `let analyzer` → `let mut analyzer`

### 10. `src/human/concept_updater.rs` - Borrow/Move Fixes
- Pre-extract values antes de push operations
- Fixed snapshot hash computation borrow conflict
- Inline hash computation para evitar `self` borrow conflict

### 11. `src/sae/router.rs` - Borrow Fix
- Collect `peer_ids` antes de iteración
- Clone `lease` antes de reassign para evitar borrow conflict

### 12. `src/security/wasm_sandbox.rs` - wasmtime 17.0 API
- `Caller<'_, T>` requiere generic param
- Closures en `func_wrap` deben ser `'static` (no pueden capturar `&self`)
- Reemplazar `self.read_memory_from_caller()` con lectura directa de memoria
- `read_memory_from_caller(&caller)` → `&mut caller`

## Feature Flags

### `core-only` (Default)
- Fases 1-3: Core SAE, P2P, Interpretación
- Compilación limpia sin dependencias experimentales

### `phase6-experimental`
- Fases 4-6: Interoperabilidad, Federación, Staking, API v2
- Requiere `--features "phase6-experimental"`

## Known Issues (Warnings)

- 131 warnings de código no utilizado (imports, variables, methods)
- Módulos experimentales con dead code (esperado para feature gates)
- No afecta funcionalidad ni compilación

## Comandos de Verificación

```bash
# Build limpio (core modules)
cargo check --features "core-only"

# Build completo (incluye experimental)
cargo check --all-features

# Fix warnings automáticos
cargo fix --bin "ed2kia" -p ed2kia
```

## Migraciones API Clave

| Library | Versión Anterior | Versión Nueva | Cambio Principal |
|---------|-----------------|---------------|------------------|
| libp2p | 0.52.x | 0.53.x | `behaviour()` → `behaviour_mut()`, `cbor::Behaviour` |
| wasmtime | 16.x | 17.0 | `Caller<'a, T>` requiere generic, closures `'static` |
| arkworks | 0.4.x | 0.5.x | `format_with` → `serialize_compressed`, `into_big()` → `to_bytes_le()` |
| MerkleTree | Custom | Custom | `from_data()` returns `Result`, `root.hash` field |

## Próximos Pasos

1. [ ] `cargo clippy` para limpieza de warnings
2. [ ] Actualizar `docs/RELEASE_NOTES_v0.5.0.md` a RC
3. [ ] Tests de integración (`cargo test --features "core-only"`)
4. [ ] Build de release (`cargo build --release --features "core-only"`)
