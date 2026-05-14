# Phase 6 Sprint 1 - Backlog

## Sprint Goals

1. Establish development workflow and branch strategy
2. Implement interoperability adapter foundation
3. Begin FedAvg aggregation with Krum filtering
4. Start staking registry skeleton
5. Define API v2 OpenAPI specification

## User Stories (Prioritized)

### P1 - Interoperability

| ID | Story | Estimate | Dependencies |
|----|-------|----------|--------------|
| US-601 | Como operador, quiero adaptar Llama-3 hidden states para usar SAEs de Qwen-Scope, para interpretar modelos externos | 5 pts | None |
| US-602 | Como nodo, quiero importar modelos ONNX del ecosistema externo, para expandir la compatibilidad de SAEs | 3 pts | US-601 |
| US-603 | Como desarrollador, quiero un schema mapper que traduzca features entre modelos, para interoperabilidad cross-model | 5 pts | US-601 |

### P2 - Federation

| ID | Story | Estimate | Dependencies |
|----|-------|----------|--------------|
| US-604 | Como nodo, quiero participar en FedAvg con filtro Krum, para agregar pesos SAE de forma tolerante a Byzantine | 8 pts | None |
| US-605 | Como coordinador, quiero sincronizar rondas de federación con los nodos, para mantener consistencia de pesos | 5 pts | US-604 |
| US-606 | Como auditor, quiero verificar que los updates de pesos no sean maliciosos, para proteger la integridad del modelo | 3 pts | US-604 |

### P3 - Staking

| ID | Story | Estimate | Dependencies |
|----|-------|----------|--------------|
| US-607 | Como operador, quiero registrar mis recursos (CPU, RAM, GPU) como compromiso de staking, para recibir asignación de capas | 5 pts | None |
| US-608 | Como validador, quiero verificar proofs de utilización de recursos, para asegurar que los nodos cumplen su compromiso | 3 pts | US-607 |

### P4 - API v2

| ID | Story | Estimate | Dependencies |
|----|-------|----------|--------------|
| US-609 | Como desarrollador externo, quiero una API REST documentada con OpenAPI, para integrar ed2kIA con mis herramientas | 3 pts | None |

---

## Technical Tasks

### interoperability/ Module

| Task | File | Description | Estimate |
|------|------|-------------|----------|
| T-601 | `src/interoperability/adapter.rs` | Model adapter trait + Llama-3 implementation | 3d |
| T-602 | `src/interoperability/schema.rs` | Feature schema mapping between models | 2d |
| T-603 | `tests/interoperability_test.rs` | Cross-model feature extraction tests | 1d |

### federation/ Module

| Task | File | Description | Estimate |
|------|------|-------------|----------|
| T-604 | `src/federation/avg_aggregator.rs` | FedAvg with Krum filtering | 4d |
| T-605 | `src/federation/sync_protocol.rs` | Round synchronization protocol | 3d |
| T-606 | `tests/federation_test.rs` | Byzantine tolerance tests (f < n/3) | 2d |

### staking/ Module

| Task | File | Description | Estimate |
|------|------|-------------|----------|
| T-607 | `src/staking/registry.rs` | Resource commitment registry | 2d |
| T-608 | `src/staking/proof.rs` | Resource utilization proof generation | 2d |
| T-609 | `tests/staking_test.rs` | Registry and proof tests | 1d |

### api/ Module

| Task | File | Description | Estimate |
|------|------|-------------|----------|
| T-610 | `src/api/openapi.rs` | OpenAPI 3.0 spec generator | 2d |
| T-611 | `src/api/routes.rs` | REST endpoint implementations | 3d |
| T-612 | `tests/api_test.rs` | API integration tests | 1d |

---

## Acceptance Criteria

### Sprint Completion Criteria

- [ ] All P1 stories implemented and tested
- [ ] P2 stories ≥ 50% complete (FedAvg core working)
- [ ] P3 stories skeleton complete (registry functional)
- [ ] P4 stories spec complete (OpenAPI generated)
- [ ] `cargo clippy --features "phase6-experimental"` → 0 warnings
- [ ] `cargo test --features "phase6-experimental"` → all pass
- [ ] Documentation updated for new modules
- [ ] CI pipeline passing on `dev/fase6` branch

### Definition of Done (per story)

- [ ] Code implemented following project standards
- [ ] Unit tests with ≥ 80% coverage
- [ ] Integration tests for cross-module interactions
- [ ] Documentation comments on all public APIs
- [ ] `cargo clippy` clean (no warnings)
- [ ] Code reviewed by ≥ 1 maintainer
- [ ] Merged to `dev/fase6` via squash merge

---

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| ONNX runtime compatibility issues | High | Medium | Use safetensors format as fallback |
| Krum filtering performance on large updates | Medium | Medium | Benchmark early, optimize with parallelization |
| Staking proof verification too slow | Medium | Low | Use lightweight proofs (Ed25519 signatures) |
| API v2 breaks existing clients | Low | Low | Versioned endpoints (/api/v1/, /api/v2/) |

---

## Sprint Timeline

| Day | Focus | Deliverable |
|-----|-------|-------------|
| 1-2 | Setup + US-601 | Branch strategy, adapter trait |
| 3-4 | US-601 + US-602 | Llama-3 adapter, ONNX import |
| 5-6 | US-603 + US-604 | Schema mapper, FedAvg core |
| 7-8 | US-604 + US-607 | Krum filtering, staking registry |
| 9-10 | US-605 + US-609 | Sync protocol, OpenAPI spec |
| 11 | Testing + Docs | Integration tests, documentation |
| 12 | Review + Polish | Code review, clippy fixes |
| 13-14 | Sprint review | Demo, retrospective, planning Sprint 2 |
