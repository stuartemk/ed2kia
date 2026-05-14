# Changelog: ed2kIA v0.5.0 → v0.6.0-RC

> All notable changes between v0.5.0 STABLE and v0.6.0-RC.
> Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
> Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

---

## [v0.6.0-RC] — 2026-05-04

### Added

#### Staking & Resource Registry
- **`src/staking/registry.rs`** — Resource commitment registry with node lifecycle management
  - `register()` — Register nodes with CPU/GPU resources
  - `heartbeat()` — Process periodic heartbeats to maintain active status
  - `slash()` — Penalize nodes for missed heartbeats or invalid proofs
  - `verify_proof()` — Validate staking proofs against registry state
  - `stats()` — Aggregate registry statistics (active/inactive/slashed nodes)
- **`src/staking/proof.rs`** — Staking proof generation and verification
  - Nonce-based replay prevention
  - Compute metrics validation (hashrate, memory, bandwidth)
  - Proof expiration and cleanup

#### Federation & Aggregation
- **`src/federation/avg_aggregator.rs`** — FedAvg aggregation with Krum Byzantine fault tolerance
  - `add_update()` — Accept weight updates with hash validation
  - `aggregate()` — Compute weighted average of accepted updates
  - `apply_krum_filter()` — Exclude outlier updates (Byzantine resistance)
  - Confidence scoring based on participant count
- **`src/federation/sync_protocol.rs`** — P2P synchronization protocol for federated deltas
  - Round-based message ordering
  - State machine: Pending → Active → Aggregating → Completed
  - Automatic cleanup of completed rounds

#### Interoperability
- **`src/interoperability/adapter.rs`** — Tensor adapter for cross-model compatibility
  - `normalize_dtype()` — Convert F16/F64 to F32
  - `reshape_to_qwen()` — Dimension alignment (shrink/expand/pad)
  - `validate_schema()` — Shape and dtype verification
  - `adapt()` — Full normalization pipeline
- **`src/interoperability/schema.rs`** — Qwen-Scope normalization schema
  - Default schema for Qwen-Scope hidden states
  - Custom adaptation rules per source model
  - Value range validation
- **`src/interoperability/onnx_adapter.rs`** — ONNX model loader (Phase 6 Sprint 2)
  - Load `.onnx` files and convert to `candle::Tensor<f32>`
  - Model caching to avoid redundant loads
  - Schema validation and hash-based cache invalidation

#### API v2
- **`src/api/routes.rs`** — Axum 0.7 routes for `/api/v2/*`
  - `GET /api/v2/health` — Health check with Phase 6 status
  - `GET /api/v2/network` — Network topology and metrics
  - `POST /api/v2/sae/analyze` — SAE activation analysis
  - `GET /api/v2/federation/rounds` — Federation round status
  - `POST /api/v2/federation/round` — Start federation round
  - `GET /api/v2/staking/registry` — Staking registry state
  - `GET /api/v2/governance/proposals` — Governance proposals
  - `POST /api/v2/governance/proposal` — Create governance proposal
  - `GET /api/v2/openapi` — OpenAPI 3.0 specification
- **`src/api/auth.rs`** — Ed25519 signature validation middleware
  - `AuthValidator` — Key management and signature verification
  - `auth_middleware()` — Axum middleware for request authentication
  - Authorized key cache with hot-reload support
- **`src/api/openapi.rs`** — OpenAPI 3.0 specification generator
  - Complete spec for all API v2 endpoints
  - JSON and YAML output formats
  - Schema definitions for all request/response types

#### Build & Deployment
- **`src/phase6/mod.rs`** — Feature-gated re-exports for Phase 6 modules
  - `enabled_features()` — Runtime feature detection
  - Sprint identifier and version tracking

### Changed

#### Configuration
- **Feature flags**: Added `phase6-core`, `phase6-sprint2`, `phase6-experimental`
- **Default features**: `core-only` remains default (v0.5.0 compatibility)
- **Cargo.toml**: New dependencies for candle-core, ed25519-dalek, axum 0.7

#### API
- **Response format**: Standardized `ApiResponse<T>` with `success`, `data`, `error` fields
- **Error handling**: Consistent error codes across all v2 endpoints
- **Health endpoint**: Extended with Phase 6 module status

#### Consensus
- **Validator**: Extended to support ZKP verification alongside Merkle proofs
- **Reputation**: Integrated with staking registry for node scoring

### Deprecated

- **API v1 `/health`**: Still functional but API v2 `/api/v2/health` is preferred
- **Direct tensor operations**: Use `interoperability/adapter.rs` for cross-model operations
- **Manual key management**: Use `api/auth.rs` for Ed25519 authentication

### Security

- **Ed25519 authentication**: All API v2 endpoints support signature validation
- **Memory Guard**: Enforced memory limits prevent resource exhaustion
- **WASM Sandbox**: Isolated execution for untrusted SAE forward passes
- **ZKP Verification**: 3-tier fallback (ZKP → Merkle → VRF) ensures verification continuity
- **Replay Prevention**: Nonce-based proof validation prevents proof reuse

### Ethics

- **License**: Apache 2.0 + Cláusula de Uso Ético maintained
- **Transparency**: All staking decisions logged and auditable
- **Fairness**: Krum aggregation resists Byzantine manipulation
- **Inclusion**: Feature gates allow gradual adoption without forcing upgrades
- **Reversibility**: All Phase 6 features can be disabled via feature flags

### Metrics

| Metric | v0.5.0 | v0.6.0-RC | Change |
|---|---|---|---|
| Unit tests | ~120 | 170 | +41.7% |
| Clippy warnings | 0 | 0 | Maintained |
| Modules | ~20 | ~30 | +50% |
| API endpoints | ~8 | ~15 | +87.5% |
| Feature flags | 1 | 5 | +400% |

---

## [v0.5.0] — 2026-04-15

### Summary
- Stable production release with core SAE federation
- P2P swarm with gossipsub
- Basic consensus and reputation systems
- WASM sandbox for isolated execution
- Human-in-the-loop feedback CLI

---

## Migration Guide: v0.5.0 → v0.6.0-RC

### For Node Operators
1. **Backup**: Export reputation ledger and governance state
2. **Update**: Pull v0.6.0-RC Docker image or build from source
3. **Configure**: Add `features = ["phase6-sprint2"]` to your build config
4. **Deploy**: Run `ops/canary_deploy.sh` for automated rollout
5. **Monitor**: Check `/api/v2/health` for Phase 6 status

### For Developers
1. **Feature gates**: All Phase 6 code is behind `#[cfg(feature = "phase6-sprint2")]`
2. **API v2**: New endpoints at `/api/v2/*`; v1 endpoints unchanged
3. **Tensor API**: Use `interoperability/adapter.rs` for model conversions
4. **Auth**: Ed25519 keys required for production API v2 access

### Rollback
- Set `features = ["core-only"]` to revert to v0.5.0 behavior
- Run `ops/rollback_v0.6.0.sh` for full rollback

---

*v0.6.0-RC is a release candidate. Promote to STABLE after successful canary rollout.*
