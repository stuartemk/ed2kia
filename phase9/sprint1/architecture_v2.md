# Phase 9 Sprint 1 — Architecture v2

## Arquitectura de Gobernanza Líquida, UI Real-Time y Federación ZKP Asíncrona

### Vista General

```
┌─────────────────────────────────────────────────────────────────┐
│                     ed2kIA v0.9.0-alpha                         │
│                                                                 │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │
│  │ LiquidGovernance │  │ RealtimeUIBackend│  │ AsyncZKP     │  │
│  │                  │  │                  │  │ Federation   │  │
│  │ • Delegation     │  │ • WebSocket      │  │ • Batch      │  │
│  │ • Voting         │  │ • Broadcast      │  │   Proofs     │  │
│  │ • Time-lock      │  │ • Rate Limit     │  │ • Light ZKP  │  │
│  │ • Sybil Detect   │  │ • Sessions       │  │ • Merkle     │  │
│  └──────────────────┘  └──────────────────┘  │   Fallback   │  │
│         │                   │                  └──────────────┘  │
│         └───────────────────┼───────────────────────────────────┘  │
│                             │                                      │
│                  ┌──────────┴──────────┐                           │
│                  │   Event Bus (WS)    │                           │
│                  │   governance_vote   │                           │
│                  │   alignment_drift   │                           │
│                  │   federation_sync   │                           │
│                  │   slo_breach        │                           │
│                  │   marketplace_trade │                           │
│                  └─────────────────────┘                           │
└─────────────────────────────────────────────────────────────────┘
```

### Módulo 1: LiquidGovernance (`src/governance/liquid.rs`)

```
┌────────────────────────────────────────────┐
│           LiquidGovernance                 │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ NodeProfile                        │   │
│  │ • trust_score                      │   │
│  │ • staking_credits                  │   │
│  │ • uptime_history                   │   │
│  │ • crypto_signature                 │   │
│  │ • asn, ip_prefix                   │   │
│  │ • voting_history                   │   │
│  └────────────────────────────────────┘   │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ Proposal                           │   │
│  │ • id, title, description           │   │
│  │ • votes_for, votes_against         │   │
│  │ • time_lock_until (24h)            │   │
│  │ • executed                         │   │
│  └────────────────────────────────────┘   │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ SybilCluster                       │   │
│  │ • cluster_id                       │   │
│  │ • node_ids                         │   │
│  │ • avg_trust_score < 0.45           │   │
│  │ • detection_reason                 │   │
│  └────────────────────────────────────┘   │
│                                            │
│  Flows:                                   │
│  1. register_node() → NodeProfile         │
│  2. create_proposal() → 24h time-lock     │
│  3. delegate_weight() → chain resolution  │
│  4. cast_vote() → quorum check            │
│  5. execute_proposal() → after time-lock  │
│  6. detect_sybil_cluster() → flag bad     │
└────────────────────────────────────────────┘
```

**Weight Calculation**: `voting_weight = trust_score × staking_credits × uptime_history`

**Sybil Detection Heuristics**:
- Same ASN + Same IP prefix → Potential cluster
- Same ASN + Voting similarity > 0.7 → Potential cluster
- Avg trust score < 0.45 → Flag as Sybil

### Módulo 2: RealtimeUIBackend (`src/ui/realtime.rs`)

```
┌────────────────────────────────────────────┐
│           RealtimeUIBackend                │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ SessionState (DashMap)             │   │
│  │ • session_id                       │   │
│  │ • connected_at                     │   │
│  │ • messages_sent                    │   │
│  │ • message_timestamps (1s window)   │   │
│  │ • rate_limit_per_sec (50 default)  │   │
│  └────────────────────────────────────┘   │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ RealtimeEvent                      │   │
│  │ • event_type (5 types)             │   │
│  │ • payload (JSON)                   │   │
│  │ • timestamp_ms                     │   │
│  │ • source_node                      │   │
│  └────────────────────────────────────┘   │
│                                            │
│  Flows:                                   │
│  1. upgrade_to_ws() → WebSocket handler   │
│  2. broadcast_event() → all sessions      │
│  3. sync_state() → session snapshot       │
│  4. rate_limit_session() → 50 msg/s       │
│  5. cleanup_expired_sessions() → 1h TTL   │
└────────────────────────────────────────────┘
```

**Event Types**: `governance_vote`, `alignment_drift`, `federation_sync`, `slo_breach`, `marketplace_trade`

**Rate Limiting**: Sliding window de 1 segundo, 50 mensajes/segundo por sesión

### Módulo 3: AsyncZKPFederation (`src/federation/async_zkp.rs`)

```
┌────────────────────────────────────────────┐
│          AsyncZKPFederation                │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ DeltaProof                         │   │
│  │ • delta_id, source_node            │   │
│  │ • layer_id                         │   │
│  │ • data_hash (SHA-256)              │   │
│  │ • proof_bytes                      │   │
│  └────────────────────────────────────┘   │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ PendingBatch (10-50 proofs)        │   │
│  │ • batch_id                         │   │
│  │ • proofs[]                         │   │
│  │ • proof_hash                       │   │
│  │ • created_at_ms                    │   │
│  └────────────────────────────────────┘   │
│                                            │
│  ┌────────────────────────────────────┐   │
│  │ MerkleProof (fallback)             │   │
│  │ • root_hash                        │   │
│  │ • proof_path[]                     │   │
│  │ • leaf_index                       │   │
│  │ • vrf_nonce                        │   │
│  └────────────────────────────────────┘   │
│                                            │
│  Flows:                                   │
│  1. batch_proofs() → group 10-50 deltas   │
│  2. generate_light_proof() → ark-bn254    │
│  3. verify_async() → async verification   │
│  4. fallback_to_merkle() → Merkle+VRF     │
│                                            │
│  Fallback triggers:                        │
│  • gas_used > 30M                          │
│  • CPU cores < 4                           │
│  • pending_batches > 2x max_batch_size     │
└────────────────────────────────────────────┘
```

**ZKP Optimization**: Light proofs optimized for SAE forward pass using `ark-ec`/`ark-bn254` circuits

**Fallback Strategy**: Auto-fallback to `MerkleProof + VRF` when resources insufficient

### Integración con Fases Anteriores

| Fase | Punto de Integración | Detalle |
|------|---------------------|---------|
| Phase 5 | `governance/proposal.rs` | LiquidGovernance extiende proposals con delegación |
| Phase 6 | `federation/avg_aggregator.rs` | AsyncZKP agrega proofs a FedAvg |
| Phase 7 | `alignment/engine.rs` | RealtimeUI broadcast alignment_drift events |
| Phase 8 | `ui/backend.rs` | Coexistencia REST + WebSocket |
| Phase 8 | `slo/engine.rs` | RealtimeUI broadcast slo_breach events |

### Feature Flag Isolation

```
phase9-sprint1
├── src/governance/liquid.rs    (22 tests)
├── src/ui/realtime.rs          (18 tests)
├── src/federation/async_zkp.rs (22 tests)
├── src/phase9/mod.rs           (3 tests)
└── tests/integration/phase9_sprint1_e2e.rs (3 tests)
```

**Constraint**: NO modifica `main`, `p2p/`, `sae/`, `consensus/`, `phase6/`, `phase7/`, `phase8/`

### Roadmap hacia v1.0.0

1. **Sprint 2**: Proof-of-Personhood + SSE Streams + ZKP Circuit Real
2. **Sprint 3**: Cross-Chain Governance + UI Dashboard + Gas Optimization
3. **v0.9.0-beta**: Consolidación Sprint 1-2
4. **v0.9.0-rc**: Hardening + Security Audit
5. **v0.9.0**: Release estable Phase 9
6. **v1.0.0**: Unificación todas fases 6-9
