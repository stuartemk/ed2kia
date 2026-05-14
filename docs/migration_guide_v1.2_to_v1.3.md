# Migration Guide: ed2kIA v1.2.0 → v1.3.0

**Date:** 2026-05-10
**Target Audience:** Node operators, integrators, and developers

---

## Summary of Changes

v1.3.0 introduces three major module families (Async ZKP v5, Federation Scaling v3, SAE Fine-Tuning v3) alongside the Federation ZKP Bridge for cross-shard verification. This guide covers breaking changes, API migrations, and configuration updates.

## 1. Feature Flag Consolidation

### Before (v1.2.0)
```toml
[features]
stable = ["v1.2-sprint1", "v1.2-sprint2", "v1.2-sprint3", "v1.2-sprint4"]
```

### After (v1.3.0)
```toml
[features]
stable = [
  "v1.2-sprint1", "v1.2-sprint2", "v1.2-sprint3", "v1.2-sprint4",
  "v1.3-sprint1", "v1.3-sprint2", "v1.3-sprint3"
]
```

**Action:** Update `Cargo.toml` or build commands to use `--features stable`. No individual sprint flags needed.

## 2. Version Bump

### Cargo.toml
```diff
- version = "1.2.0"
+ version = "1.3.0"
```

### src/lib.rs
```diff
- pub const SPRINT_IDENTIFIER: &str = "v1.2.0-stable";
+ pub const SPRINT_IDENTIFIER: &str = "v1.3.0-stable";
```

**Action:** Update version references in your integration tests and CI pipelines.

## 3. Breaking API Changes

### 3.1 ScalingDecisionType Renamed Variant

**File:** `src/federation/scaling_v3.rs`

```diff
- ScalingDecisionType::ScaleUp
+ ScalingDecisionType::AddShard
```

**Full Enum:**
```rust
pub enum ScalingDecisionType {
    AddShard,      // was ScaleUp
    RemoveShard,
    Rebalance,
    NoOp,
}
```

**Migration:**
```rust
// Before (v1.2)
if decision.decision_type == ScalingDecisionType::ScaleUp { ... }

// After (v1.3)
if decision.decision_type == ScalingDecisionType::AddShard { ... }
```

### 3.2 VRF Sampling Logic

**File:** `src/zkp/async_zkp_v5.rs`

The VRF sampling condition changed. Proofs are now sampled when `batch.statements.len() > max_batch_size / 2`.

**Impact on Tests:**
- If your test uses 16 statements and `max_batch_size: 100`, VRF sampling will NOT trigger (16 ≤ 50).
- Set `max_batch_size: 10` to ensure VRF triggers (16 > 5).

```rust
// Before (v1.2 — didn't exist)
// After (v1.3)
let config = ZKPV5Config {
    max_batch_size: 10,        // Ensures VRF triggers with 16 statements
    vrf_sampling_rate: 1.0,    // 100% sampling for deterministic tests
    ..Default::default()
};
```

### 3.3 Fallback Generation Trigger

**File:** `src/zkp/async_zkp_v5.rs`

Fallback proofs trigger when `statement.complexity_score > avg_complexity * 2.0`. With `vrf_sampling_rate < 1.0`, high-complexity statements may be sampled out before fallback check.

**Migration:** Set `vrf_sampling_rate: 1.0` in tests where you need to verify fallback behavior.

### 3.4 Checkpoint Counting

**File:** `src/sae/fine_tuning_v3.rs`

`train_step()` now auto-creates checkpoints when `total_rounds.is_multiple_of(checkpoint_interval)`. If you also call `create_checkpoint()` explicitly, you get **2 checkpoints per iteration**.

```rust
// Before (v1.2) — only explicit checkpoints counted
for _ in 0..100 {
    engine.create_checkpoint()?;
}
// Total: 100 checkpoints

// After (v1.3) — auto + explicit
let config = FineTuningV3Config {
    checkpoint_interval: 1,  // Auto-checkpoint every round
    ..Default::default()
};
for _ in 0..100 {
    engine.train_step(&gradients)?;  // Auto-checkpoint
    engine.create_checkpoint()?;      // Explicit checkpoint
}
// Total: 200 checkpoints
```

**Migration:** Adjust expected checkpoint counts in tests or set `checkpoint_interval` to a value larger than total training rounds to disable auto-checkpointing.

### 3.5 Cross-Shard Aggregation Logic

**File:** `src/bridge/federation_zkp_bridge.rs`

`needs_cross_shard_aggregation()` now returns `true` ONLY when `target_shards.len() > 1`:

```rust
pub fn needs_cross_shard_aggregation(&self, proof: &FederationProof) -> bool {
    self.config.cross_shard_aggregation && proof.target_shards.len() > 1
}
```

**Migration:** Single-target proofs no longer trigger cross-shard aggregation. Update assertions:
```rust
// Before (incorrect)
assert!(bridge.needs_cross_shard_aggregation(&single_target_proof));

// After (correct)
assert!(!bridge.needs_cross_shard_aggregation(&single_target_proof));
assert!(bridge.needs_cross_shard_aggregation(&multi_target_proof));
```

### 3.6 Resource Cost for Proof Acceptance

**File:** `src/bridge/federation_zkp_bridge.rs`

`can_accept_proof()` checks `available_resources >= cost && active_proofs < max_active`. Default `resource_cost_per_proof` is `5.0`.

**Migration:** If your tests create many proofs, ensure shard resources are sufficient:
```rust
// Before — 500 resources, cost 10.0 → only 50 proofs fit
let shard = ShardBridgeState::new("s1".to_string(), 500.0, 1.0);

// After — 1000 resources, cost 5.0 → up to 200 proofs fit
let shard = ShardBridgeState::new("s1".to_string(), 1_000.0, 1.0);
```

### 3.7 Public Inputs Type

**File:** Multiple ZKP structs

`public_inputs` field is `Vec<u8>`. All values must be 0-255.

```rust
// Before (v1.2 — may have accepted u32)
public_inputs: vec![100, 200, 300]  // 300 overflows u8!

// After (v1.3 — strict u8)
public_inputs: vec![100u8, 200, 150]
// Or with modulo for loop variables:
public_inputs: vec![(i % 256) as u8, ((i + 1) % 256) as u8]
```

## 4. New Modules

### 4.1 Async ZKP v5
- **Path:** `src/zkp/async_zkp_v5.rs`
- **Key Types:** `AsyncZKPV5`, `ZKPV5Config`, `ZKPStatement`, `ZKPProof`, `ProofBatch`, `IncrementalAccumulator`
- **Feature:** `cfg(feature = "stable")`

### 4.2 Federation Scaling v3
- **Path:** `src/federation/scaling_v3.rs`
- **Key Types:** `FederationScalingV3`, `ScalingV3Config`, `NodeCapabilityV3`, `ShardConfigV3`, `ScalingDecisionV3`
- **Feature:** `cfg(feature = "stable")`

### 4.3 Federation ZKP Bridge
- **Path:** `src/bridge/federation_zkp_bridge.rs`
- **Key Types:** `FederationZKPBridge`, `FederationZKPConfig`, `FederationProof`, `ShardBridgeState`
- **Feature:** `cfg(feature = "stable")`

### 4.4 SAE Fine-Tuning v3
- **Path:** `src/sae/fine_tuning_v3.rs`
- **Key Types:** `FineTuningV3`, `FineTuningV3Config`, `ModelProfile`, `Checkpoint`, `TrainingStats`
- **Feature:** `cfg(feature = "stable")`

### 4.5 Cross-Model Aligner
- **Path:** `src/sae/cross_model_aligner.rs`
- **Key Types:** `CrossModelAligner`, `AlignerConfig`, `GradientProfile`, `AlignmentResult`
- **Feature:** `cfg(feature = "stable")`

## 5. Configuration Defaults

### ZKPV5Config
```rust
pub struct ZKPV5Config {
    pub max_batch_size: usize,          // Default: 64
    pub max_parallel_workers: usize,    // Default: 8
    pub vrf_sampling_rate: f64,         // Default: 0.8
    pub fallback_threshold: f64,        // Default: 2.0
    pub max_pools: usize,               // Default: 16
    pub proof_cache_size: usize,        // Default: 1024
    pub accumulator_window: usize,      // Default: 128
    pub precompile_enabled: bool,       // Default: true
}
```

### ScalingV3Config
```rust
pub struct ScalingV3Config {
    pub scale_up_threshold: f64,        // Default: 0.7
    pub scale_down_threshold: f64,      // Default: 0.3
    pub min_nodes_per_shard: usize,     // Default: 2
    pub max_nodes_per_shard: usize,     // Default: 50
    pub rebalance_threshold: f64,       // Default: 0.5
    pub evaluation_interval_ms: u64,    // Default: 5000
}
```

### FederationZKPConfig
```rust
pub struct FederationZKPConfig {
    pub max_shards: usize,              // Default: 16
    pub consensus_threshold: f64,       // Default: 0.67
    pub proof_ttl_ms: u64,              // Default: 300000
    pub routing_strategy: u8,           // Default: 0 (capacity)
    pub cross_shard_aggregation: bool,  // Default: true
    pub max_verification_hops: usize,   // Default: 3
}
```

### FineTuningV3Config
```rust
pub struct FineTuningV3Config {
    pub learning_rate: f64,             // Default: 0.001
    pub max_models: usize,              // Default: 8
    pub checkpoint_interval: u64,       // Default: 10
    pub alignment_enabled: bool,        // Default: true
    pub compression_enabled: bool,      // Default: true
    pub min_node_uptime: f64,           // Default: 0.9
}
```

## 6. Build and Test Commands

### Build
```bash
# Full stable build
cargo build --release --features stable

# Specific module
cargo build --release --features v1.3-sprint3
```

### Test
```bash
# All stable tests
cargo test --features stable

# E2E tests only
cargo test --features stable --test v1_3_sprint3_e2e

# Stress tests only
cargo test --features stable --test sprint3_stress_v1_3

# Specific module tests
cargo test --features stable federation_zkp_bridge
cargo test --features stable fine_tuning_v3
cargo test --features stable async_zkp_v5
cargo test --features stable scaling_v3
```

## 7. CI/CD Updates

The new CI/CD workflow is at `.github/workflows/ci_cd_v1.3.yml`. Key changes:
- Matrix builds for ubuntu-latest, windows-latest, macos-latest
- Module-specific test jobs (federation_zkp_bridge, fine_tuning_v3, async_zkp_v5, scaling_v3, cross_model_aligner)
- Integration test job for E2E + stress
- `cargo audit` for security scanning
- Automated GitHub Release on `v1.3.*` tags

## 8. Rollback Plan

If migration issues arise:
1. Revert to `v1.2.0-stable` tag
2. Use `--features "v1.2-sprint1 v1.2-sprint2 v1.2-sprint3 v1.2-sprint4"`
3. Report issues at GitHub Issues with reproduction steps

## 9. Checklist

- [ ] Update `Cargo.toml` version to `1.3.0`
- [ ] Replace `ScalingDecisionType::ScaleUp` with `AddShard`
- [ ] Adjust VRF test configurations (`max_batch_size`, `vrf_sampling_rate`)
- [ ] Update checkpoint count expectations (×2 if auto + explicit)
- [ ] Fix cross-shard aggregation assertions
- [ ] Increase shard resources for high-volume proof tests
- [ ] Fix `public_inputs` to use `u8` values (0-255)
- [ ] Run `cargo test --features stable` to validate
- [ ] Update CI/CD workflow references

---

**Need Help?** See `docs/GOVERNANCE.md` for community support channels.
