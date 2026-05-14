# Release Notes - v0.7.0-BETA

> **Fecha de Release**: 2026-05-04  
> **VersiÃ³n**: v0.7.0-beta  
> **CÃ³digo**: ed2kIA  
> **Licencia**: Apache 2.0 + Ethical Use Clause  
> **Estado**: Beta (PreparaciÃ³n para auditorÃ­a externa)  

---

## 1. Resumen Ejecutivo

v0.7.0-beta consolida los mÃ³dulos de Phase 7 (Alignment Engine + Federation Bridge + Trust Scoring + Schema Registry) en una versiÃ³n estable preparada para auditorÃ­a de seguridad externa y promociÃ³n a v0.8.0-alpha. Esta versiÃ³n introduce cierre de loop continuo de alineaciÃ³n, scoring dinÃ¡mico de confianza con resistencia Sybil, y registro versionado de esquemas con compatibilidad semÃ¡ntica.

**Highlights**:
- â Alignment Feedback Loop con rollback automÃ¡tico
- â Dynamic Trust Scoring con detecciÃ³n Sybil
- â Schema Registry con versionado semÃ¡ntico
- â 67 tests unitarios + 15 E2E tests
- â 0 errores, 0 warnings (clippy -D warnings)
- â Feature gates aislados (phase7-sprint1, phase7-sprint2)

---

## 2. Cambios desde v0.6.0-RC

### 2.1 Nuevos MÃ³dulos

| MÃ³dulo | Archivo | Feature Gate | DescripciÃ³n |
|---|---|---|---|
| **AlignmentScorer** | `src/alignment/engine.rs` | `phase7-sprint1` | Motor de alineaciÃ³n continua con cÃ¡lculo de drift y steering signals |
| **FederationBridge** | `src/federation/bridge.rs` | `phase7-sprint1` | Puente cross-red con handshake, delta sync y trust tracking |
| **AlignmentFeedbackLoop** | `src/alignment/feedback_loop.rs` | `phase7-sprint2` | Cierre de loop: feedback â drift â steering â rollback |
| **DynamicTrustScorer** | `src/federation/trust_scoring.rs` | `phase7-sprint2` | Scoring dinÃ¡mico con fÃ³rmula de decaimiento, detecciÃ³n Sybil, propagaciÃ³n cross-net |
| **SchemaRegistry** | `src/interoperability/schema_registry.rs` | `phase7-sprint2` | Registro versionado con compatibilidad backward/forward |

### 2.2 MÃ³dulos Actualizados

| MÃ³dulo | Cambio | Impacto |
|---|---|---|
| `src/phase7/mod.rs` | Re-exports para Sprint 1 + Sprint 2 | API pÃºblica unificada |
| `Cargo.toml` | Feature `phase7-sprint2` agregado | Feature gates actualizados |

### 2.3 Breaking Changes

**Ninguno**. Esta versiÃ³n es 100% backward compatible con v0.6.0-RC y v0.5.0 STABLE.

Los nuevos mÃ³dulos estÃ¡n aislados detrÃ¡s de feature gates y no modifican el comportamiento de mÃ³dulos existentes cuando los features estÃ¡n desactivados.

---

## 3. GuÃ­a de MigraciÃ³n

### 3.1 Desde v0.6.0-RC a v0.7.0-beta

**Paso 1**: Actualizar dependencias
```bash
git pull origin dev/fase7
cargo build --all-features
```

**Paso 2**: Verificar compilaciÃ³n
```bash
cargo check --all-features
cargo clippy --all-features -- -D warnings
```

**Paso 3**: Ejecutar tests
```bash
cargo test --all-features
```

**Paso 4**: Activar features (opcional)
```bash
# Solo Phase 7 Sprint 1
cargo run --features "phase7-sprint1,phase6-core"

# Phase 7 completo (Sprint 1 + Sprint 2)
cargo run --features "phase7-sprint1,phase7-sprint2,phase6-core"

# Todo (recomendado para beta testing)
cargo run --all-features
```

### 3.2 Desde v0.5.0 STABLE a v0.7.0-beta

**Paso 1**: Actualizar a v0.6.0-RC primero (ver guÃ­a de migraciÃ³n v0.6.0)  
**Paso 2**: Seguir migraciÃ³n v0.6.0-RC â v0.7.0-beta (secciÃ³n 3.1)

### 3.3 Feature Flags

| Feature | DescripciÃ³n | MÃ³dulos Incluidos |
|---|---|---|
| `core-only` | MÃ³dulos base (v0.5.0) | SAE, P2P, Security, Bridge, etc. |
| `phase6-core` | Phase 6 completo | Interoperability, Federation, Staking, API v2 |
| `phase7-sprint1` | Phase 7 Sprint 1 | AlignmentScorer, FederationBridge |
| `phase7-sprint2` | Phase 7 Sprint 2 | FeedbackLoop, TrustScorer, SchemaRegistry |

**Combinaciones vÃ¡lidas**:
```bash
# Core only
--features "core-only"

# Core + Phase 6
--features "phase6-core"

# Core + Phase 6 + Phase 7 Sprint 1
--features "phase7-sprint1,phase6-core"

# Core + Phase 6 + Phase 7 Sprint 1 + Sprint 2
--features "phase7-sprint1,phase7-sprint2,phase6-core"

# Todo
--all-features
```

---

## 4. MÃ©tricas de ValidaciÃ³n

### 4.1 CompilaciÃ³n

| MÃ©trica | Resultado |
|---|---|
| `cargo check --all-features` | â Exit code 0 |
| `cargo clippy --all-features -- -D warnings` | â 0 warnings |
| `cargo test --all-features` | â 67 unit + 15 E2E = 82 tests |

### 4.2 Coverage por MÃ³dulo

| MÃ³dulo | Tests Unitarios | Tests E2E | Coverage |
|---|---|---|---|
| AlignmentScorer | 10+ | 3 | â¥85% |
| FederationBridge | 12+ | 3 | â¥85% |
| AlignmentFeedbackLoop | 15 | 2 | â¥90% |
| DynamicTrustScorer | 18 | 2 | â¥90% |
| SchemaRegistry | 19 | 2 | â¥90% |

### 4.3 Performance (Target)

| MÃ©trica | Objetivo | Estado |
|---|---|---|
| SAE Latency p50 | â¤350ms | Pendiente benchmark |
| Consensus Rate | â¥88% | Pendiente benchmark |
| WASM Memory | â¤180MB | Pendiente benchmark |
| API v2 Throughput | â¥500 req/s | Pendiente benchmark |
| Alignment Drift p95 | â¤0.15 | Pendiente benchmark |
| Trust Score Update | â¤50ms/node | Pendiente benchmark |
| Schema Validation | â¤20ms/schema | Pendiente benchmark |

**Nota**: Los benchmarks se ejecutarÃ¡n durante la fase de auditorÃ­a beta.

---

## 5. Seguridad

### 5.1 AuditorÃ­a de Dependencias

```bash
cargo audit
```

**Resultado**: Pendiente de ejecuciÃ³n durante fase beta.

### 5.2 Modelo de Amenazas

El modelo STRIDE completo estÃ¡ documentado en [`release/v0.7.0-beta/security_audit_prep.md`](../release/v0.7.0-beta/security_audit_prep.md).

**Hallazgos preliminares**:
- 1 hallazgo P0 (rate limiting en API v2)
- 2 hallazgos P1 (TLS enforcement, key rotation)
- 2 hallazgos P2 (audit trail externo, WASM timeout)

### 5.3 Checklist de Hardening

El checklist completo estÃ¡ en [`release/v0.7.0-beta/security_audit_prep.md`](../release/v0.7.0-beta/security_audit_prep.md#6-checklist-de-hardening).

---

## 6. Soporte y Contactos

### 6.1 Canales de Soporte

| Canal | PropÃ³sito | URL |
|---|---|---|
| GitHub Issues | Bugs y feature requests | https://github.com/ed2kia/ed2kIA/issues |
| Discord | Comunidad y soporte | https://discord.gg/ed2kia |
| Email | Contactos oficiales | team@ed2kia.org |
| Security | Reportes de seguridad | security@ed2kia.org |

### 6.2 PolÃ­tica de Soporte

| VersiÃ³n | Estado | Soporte |
|---|---|---|
| v0.5.0 | STABLE | LTS hasta v1.0.0 |
| v0.6.0-RC | RC | Canary rollout |
| v0.7.0-beta | Beta | Activo (auditorÃ­a) |

### 6.3 SLA de Respuesta

| Severidad | Tiempo de Respuesta | Tiempo de ResoluciÃ³n |
|---|---|---|
| CrÃ­tica (P0) | 4h | 48h |
| Alta (P1) | 8h | 72h |
| Media (P2) | 24h | 2 semanas |
| Baja (P3) | 48h | 1 mes |

---

## 7. PrÃ³ximos Pasos

### 7.1 Fase Beta (Actual)

- [ ] Ejecutar benchmarks completos
- [ ] Completar auditorÃ­a de seguridad
- [ ] Recibir feedback de early adopters
- [ ] Remediar hallazgos P0/P1

### 7.2 v0.8.0-alpha (Sprint 1, Phase 8)

- Marketplace de Modelos
- UI Dashboard para operadores
- Model Registry v2

### 7.3 v0.9.0-rc (Sprint 2, Phase 8)

- Multi-model adapter (Llama + Mistral)
- FederaciÃ³n cross-model
- Tests de escalado (100+ nodos)

### 7.4 v1.0.0 STABLE (Sprint 4, Phase 8)

- SLOs definidos y monitoreados
- AuditorÃ­a de seguridad externa
- DocumentaciÃ³n completa
- Launch general

---

## 8. CrÃ©ditos

### 8.1 Equipo de Desarrollo

| Rol | ContribuciÃ³n |
|---|---|
| Release Engineer | ConsolidaciÃ³n beta, validaciÃ³n cross-phase |
| Security Auditor | Modelo STRIDE, checklist de hardening |
| Performance Architect | Benchmarks, umbrales de aceptaciÃ³n |
| Phase 7 Team | Alignment, Federation, Schema modules |
| Phase 6 Team | Interoperability, Federation, Staking, API v2 |
| Core Team | SAE, P2P, Security, Bridge, etc. |

### 8.2 Contribuidores

Ver [`docs/CONTRIBUTING.md`](CONTRIBUTING.md) para mÃ¡s informaciÃ³n sobre cÃ³mo contribuir.

---

## 9. Licencia

ed2kIA estÃ¡ licenciado bajo **Apache 2.0 + Ethical Use Clause**.

Ver [`LICENSE`](../LICENSE) para detalles completos.

---

## 10. Changelog Completo

Ver [`release/v0.7.0-alpha/changelog.md`](../release/v0.7.0-alpha/changelog.md) para el changelog detallado de la versiÃ³n alpha.

---

*Release notes generadas para v0.7.0-beta. PrÃ³xima versiÃ³n: v0.8.0-alpha (Phase 8 Sprint 1).*
