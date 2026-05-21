# Audit Preparation Guide — ed2kIA v2.1.0-sprint18

**Version:** v2.1.0-sprint18  
**Fecha:** 2026-05-21  
**Estado:** AUDIT-READY  
**Auditoría:** Preparación para revisión externa  

---

## 1. Threat Model v2.0

### 1.1 Assets Críticos

| Asset | Sensibilidad | Protección |
|-------|-------------|------------|
| Modelos GGUF base | Alta | SHA256 validation, zero-copy mmap |
| Gradientes QLoRA | Alta | BFT aggregation, SCTGuard inspection |
| Reputation CRDTs | Media | Merge idempotency, VersionVector ordering |
| Mesh GossipSub | Media | Peer backoff, message deduplication |
| Cláusula ética | Crítica | Automated compliance scanning |

### 1.2 Amenazas Identificadas

| Threat | Vector | Mitigación | Severidad |
|--------|--------|------------|-----------|
| Byzantine gradients | Malicious node | BFT coordinate-wise median + Multi-Krum | Alta |
| SCT alignment bypass | Adversarial prompts | SCTGuard Z-axis rejection + slashing | Alta |
| CRDT state divergence | Network partition | Idempotent merge + VersionVector | Media |
| GossipSub amplification | DDoS | Rate limiting + peer backoff | Media |
| GGUF tampering | Supply chain | SHA256 integrity check on load | Alta |
| Financial logic injection | Code contribution | verify-ethical-compliance.sh CI gate | Crítica |

### 1.3 Confianzas (Trust Assumptions)

- Rust memory safety prevents buffer overflows
- libp2p provides authenticated transport
- Cargo ecosystem provides dependency verification
- Community stewards perform adversarial review

---

## 2. Arquitectura del Kernel Estuardiano

### 2.1 Ley 1: Soberanía P2P

- **Módulo:** `src/async_gossip/mesh.rs`
- **Feature Gate:** `v2.1-async-gossip`
- **Implementación:** GossipSub mesh con heartbeat 500ms, fanout_ttl 120s, mesh_n 6/4/12
- **Garantía:** Tolerancia a particiones, deduplicación determinista por message_id

### 2.2 Ley 2: Transparencia & Auditoría

- **Módulos:** `src/alignment/sct_guard.rs`, `src/federated/bft_aggregator.rs`, `src/stuartian_filter/divergence.rs`, `src/stuartian_filter/slashing.rs`
- **Feature Gates:** `v2.1-sct-guard`, `v2.1-bft-aggregation`, `v2.1-stuartian-filter`
- **Implementación:** SCTGuard (Z < 0 rejection), BFT coordinate-wise median, KL divergence detection, deterministic slashing
- **Garantía:** Gradientes no éticos rechazados, consenso tolerante a fallos bizantinos

### 2.3 Ley 3: Cero Waste

- **Módulos:** `src/qlora_gguf/loader.rs`, `src/qlora_gguf/adapter.rs`, `src/qlora_gguf/payload.rs`
- **Feature Gates:** `v2.1-qlora-gguf`
- **Implementación:** GGUF immutable base + QLoRA quantized diffs (FP8/INT4), zero-copy memory mapping, compressed payload distribution
- **Garantía:** Diffs ≤50MB, verificación SHA256, serialización bincode

### 2.4 Ley 4: Edge Distribution

- **Módulo:** WASM micro-sharding (integración libp2p)
- **Feature Gate:** `wasm` target
- **Implementación:** Shards ≤50MB para distribución edge, bridge a browser nodes
- **Garantía:** Ejecución en hardware modesto, sin dependencias nativas

### 2.5 Ley 5: Múltiples Posibilidades & Resiliencia

- **Módulos:** `src/async_gossip/crdt.rs`, `src/async_gossip/cache.rs`, `src/chaos/engine.rs`
- **Feature Gates:** `v2.1-crdt-state`, `v2.1-offline-cache`, `v2.1-chaos-engine`
- **Implementación:** CRDTs (GCounter, PNCounter, ORSet, ReputationCrdt), offline cache con priority sync, chaos engine para fault injection
- **Garantía:** Convergencia eventual sin locks, operación offline, resiliencia probada

### 2.6 Diagrama de Flujo E2E

```
GGUF Loader → QLoRA Adapter → PoC Task → SCT Guard → BFT Aggregator
    (Ley 3)       (Ley 3)        (Ley 2)    (Ley 2)       (Ley 2)
                                              ↓
CRDT Merge ← Gossip Mesh ← Compressed Payload
  (Ley 5)       (Ley 1)         (Ley 1+3)
```

**Documentación completa:** [`kernel-architecture.md`](kernel-architecture.md)

---

## 3. Cobertura de Tests

### 3.1 Tests por Módulo

| Módulo | Tests Unitarios | Tests E2E | Coverage |
|--------|----------------|-----------|----------|
| GGUF Loader | 8 | — | ≥80% |
| QLoRA Adapter | 12 | — | ≥80% |
| QLoRA Payload | 10 | — | ≥80% |
| SCT Core | 12 | — | ≥80% |
| SCT Guard | 12 | — | ≥80% |
| BFT Aggregator | 14 | — | ≥80% |
| CRDTs | 25 | — | ≥80% |
| Gossip Mesh | 20 | — | ≥80% |
| Offline Cache | 20 | — | ≥80% |
| Chaos Engine | 12 | — | ≥80% |
| **Kernel E2E** | — | **24** | **100%** |

### 3.2 Kernel E2E Pipeline (Sprint17)

24 tests validando pipeline completo: GGUF→QLoRA→PoC→SCT→BFT→CRDT→Gossip→Cache

Ejecutar: `cargo test --test kernel_e2e_test --features "v2.1-kernel-integration"`

---

## 4. Known Limitations

### 4.1 Limitaciones Técnicas Conocidas

1. **GGUF Architecture Extraction:** Placeholder implementation — retorna arquitectura hardcodeada. La extracción real de archivos GGUF requiere parsing completo del header.
2. **SCTGuard Gradient Inspection:** Simula evaluación de gradientes sin acceso a modelo real. En producción, requiere integración con runtime de inferencia.
3. **Chaos Engine Fault Injection:** Fault injection es simulada (no modifica estado real del sistema). Diseñada para testing operacional.
4. **WASM Target:** Sharding WASM está en fase de diseño. Implementación completa pendiente de Sprint19+.
5. **BFT Network Integration:** BFT aggregator opera en memoria. Integración con red P2P real requiere coordination con GossipSub.

### 4.2 Limitaciones de Escala

- Tests E2E usan datos sintéticos (no modelos GGUF reales de GB)
- Gossip mesh simulation es in-memory (no red real)
- CRDT convergence tests usan ≤3 nodos (producción puede tener 100+)

---

## 5. Proceso de Bug Bounty / Reporte Ético

### 5.1 Clasificación de Severidad

| Severidad | Ejemplo | Tiempo de Respuesta |
|-----------|---------|---------------------|
| **Crítica** | Unsafe code, backdoor, lógica financiera | 24h |
| **Alta** | Bypass SCTGuard, corrupción CRDT | 48h |
| **Media** | Race condition, memory leak | 1 semana |
| **Baja** | UX, documentación, optimización | 2 semanas |

### 5.2 Cómo Reportar

1. **Canal Preferido:** GitHub Issues con label `security`
2. **Reporte Privado:** Usar GitHub Security Advisories
3. **Formato:** Incluir pasos de reproducción, impacto esperado, entorno

### 5.3 Cláusula de Uso Ético

Este software está licenciado bajo Apache 2.0 + Cláusula de Uso Ético:
- Prohibido uso para sistemas financieros especulativos
- Prohibido uso para vigilancia masiva no consentida
- Prohibido uso para generación de desinformación
- Requerido: transparencia en despliegue, auditabilidad, beneficio humano

Verificación automatizada: `bash scripts/verify-ethical-compliance.sh`

---

## 6. Contacto Seguro

| Canal | Uso |
|-------|-----|
| GitHub Issues | Bugs públicos, feature requests |
| GitHub Security Advisories | Reportes de seguridad privados |
| GOVERNANCE.md | Propuestas de gobernanza |
| RFC Process | Cambios arquitectónicos |

**Nota:** Este es un proyecto comunitario open-source. No hay equipo centralizado de soporte. La revisión de seguridad se realiza mediante auditoría comunitaria y procesos automatizados.

---

## 7. Recursos para Auditores

| Recurso | Ruta |
|---------|------|
| Kernel Architecture | [`docs/kernel-architecture.md`](kernel-architecture.md) |
| Feature Gates | [`Cargo.toml`](../Cargo.toml) (líneas 440+) |
| CI/CD Pipeline | [`.github/workflows/`](../.github/workflows/) |
| Ethical Compliance | [`scripts/verify-ethical-compliance.sh`](../scripts/verify-ethical-compliance.sh) |
| Audit Scanner | [`scripts/audit-scan.sh`](../scripts/audit-scan.sh) |
| Governance | [`GOVERNANCE.md`](../GOVERNANCE.md) |
| CHANGELOG | [`CHANGELOG.md`](../CHANGELOG.md) |
| Kernel E2E Tests | [`tests/integration/kernel_e2e_test.rs`](../tests/integration/kernel_e2e_test.rs) |

---

## 8. Checklist Pre-Auditoría

- [ ] `cargo check --all-targets` → PASS
- [ ] `cargo clippy -- -D warnings` → PASS (zero warnings)
- [ ] `cargo audit` → Sin CVEs críticos
- [ ] `cargo test --test kernel_e2e_test --features "v2.1-kernel-integration"` → 24/24 PASS
- [ ] `bash scripts/verify-ethical-compliance.sh` → ÉTICA VALIDADA
- [ ] `bash scripts/audit-scan.sh` → 🟢 AUDIT READY
- [ ] CHANGELOG.md actualizado con versión actual
- [ ] README.md con badges actualizados
- [ ] Documentación de arquitectura sincronizada

---

*Documento preparado para auditoría externa — ed2kIA v2.1.0-sprint18*
