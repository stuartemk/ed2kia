# Release Notes: ed2kIA v0.6.0-RC

> **Version**: 0.6.0-RC (Release Candidate)
> **Date**: 2026-05-04
> **Previous**: v0.5.0 STABLE
> **License**: Apache 2.0 + Cláusula de Uso Ético
> **Status**: ⚠️ Release Candidate — Suitable for canary deployment, not yet production-stable

---

## Resumen Ejecutivo

ed2kIA v0.6.0-RC introduce **Phase 6**: un conjunto completo de funcionalidades que transforman la red de un sistema de inferencia federada a un **ecosistema de staking, gobernanza y interoperabilidad de modelos**. Esta versión añade staking con registro de recursos, federación con FedAvg+Krum, API v2 con autenticación Ed25519, y un adaptador ONNX para interoperabilidad con modelos externos.

**Validación**: 170 tests unitarios pasando, 0 warnings de clippy, feature gates completos para aislamiento de módulos.

---

## Tabla de Features

| Feature | Módulo | Feature Flag | Estado |
|---|---|---|---|
| Staking Registry | `staking/registry.rs` | `phase6-core` | ✅ RC |
| Staking Proofs | `staking/proof.rs` | `phase6-core` | ✅ RC |
| FedAvg Aggregation | `federation/avg_aggregator.rs` | `phase6-core` | ✅ RC |
| Federation Sync Protocol | `federation/sync_protocol.rs` | `phase6-core` | ✅ RC |
| Tensor Adapter | `interoperability/adapter.rs` | `phase6-core` | ✅ RC |
| Qwen-Scope Schema | `interoperability/schema.rs` | `phase6-core` | ✅ RC |
| ONNX Model Adapter | `interoperability/onnx_adapter.rs` | `phase6-sprint2` | ✅ RC |
| API v2 Routes | `api/routes.rs` | `phase6-sprint2` | ✅ RC |
| Ed25519 Auth | `api/auth.rs` | `phase6-sprint2` | ✅ RC |
| OpenAPI Spec Generator | `api/openapi.rs` | `phase6-sprint2` | ✅ RC |
| ZKP Circuits | `zkp/circuit.rs` | `phase6-experimental` | ⚠️ Experimental |
| ZKP Verifier | `zkp/verifier.rs` | `phase6-experimental` | ⚠️ Experimental |

---

## Breaking Changes

### Ninguno para v0.5.0 users

Todas las funcionalidades de Phase 6 están detrás de feature gates. Los usuarios que compilen con `features = ["core-only"]` (default) obtienen exactamente el mismo comportamiento que v0.5.0.

**Para activar Phase 6:**
```toml
# Cargo.toml
[dependencies]
ed2kia = { path = ".", features = ["phase6-sprint2"] }
```

---

## Guía de Migración: v0.5.0 → v0.6.0-RC

### Paso 1: Backup
```bash
# Exportar estado actual
cp /var/lib/ed2kia/data/*.redb /backup/
cp /etc/ed2kia/config.toml /backup/
```

### Paso 2: Actualizar Código
```bash
git fetch origin
git checkout release/v0.6.0
```

### Paso 3: Configurar Feature Flags
```toml
# Cargo.toml
[features]
default = ["phase6-sprint2"]  # Cambiar de "core-only" a "phase6-sprint2"
```

### Paso 4: Actualizar Configuración
```toml
# config.toml — Nuevas secciones para Phase 6

[staking]
max_heartbeat_age = 86400        # 24h en segundos
slash_threshold = 3               # Missed heartbeats antes de slash

[federation]
min_participants = 3              # Mínimo para aggregation
voting_period = 604800            # 7 días en segundos

[api.auth]
require_signature = false         # false por defecto (dev mode)
signature_timeout_secs = 300      # 5 minutos
```

### Paso 5: Build y Test
```bash
cargo build --release --features "phase6-sprint2"
cargo test --features "phase6-sprint2"
cargo clippy --features "phase6-sprint2" -- -D warnings
```

### Paso 6: Deploy
```bash
# Usar script de canary deployment
./ops/canary_deploy.sh --phase canary --seed-nodes launch/genesis/seed_nodes.json
```

### Paso 7: Verificar
```bash
# API v1 (v0.5.0 compatible)
curl http://localhost:3030/api/v1/health

# API v2 (Phase 6)
curl http://localhost:3030/api/v2/health

# OpenAPI spec
curl http://localhost:3030/api/v2/openapi | jq '.info.version'
```

---

## Feature Flags Reference

| Flag | Descripción | Módulos Habilitados | Uso |
|---|---|---|---|
| `core-only` | v0.5.0 behavior (default) | Solo módulos v0.5.0 | Producción estable |
| `phase6-core` | Phase 6 base | Staking, federation, interoperability base | RC canary |
| `phase6-sprint2` | Phase 6 completo | API v2, auth, OpenAPI, ONNX | RC recomendado |
| `phase6-experimental` | Features experimentales | ZKP circuits, advanced verification | Research only |
| `experimental` | Todo experimental | All experimental features | Development only |

### Combinaciones Recomendadas

```toml
# Producción estable (v0.5.0 behavior)
default = ["core-only"]

# RC canary (Phase 6 completo, sin experimental)
default = ["phase6-core", "phase6-sprint2"]

# Research mode (todo incluido)
default = ["phase6-core", "phase6-sprint2", "phase6-experimental", "experimental"]
```

---

## Métricas de Validación

| Métrica | Resultado | Target | Estado |
|---|---|---|---|
| Unit tests | 170 passing | ≥150 | ✅ |
| Ignored tests | 3 | — | ⚠️ Documented |
| Clippy warnings | 0 | 0 | ✅ |
| Test coverage | ~85% | ≥80% | ✅ |
| Build time (release) | ~45s | <60s | ✅ |
| Binary size | ~15MB | <20MB | ✅ |

---

## API v2 Endpoints

### Health & Network
- `GET /api/v2/health` — Health check con estado de Phase 6
- `GET /api/v2/network` — Topología de red y métricas
- `GET /api/v2/openapi` — Especificación OpenAPI 3.0

### SAE Analysis
- `POST /api/v2/sae/analyze` — Analizar activaciones SAE

### Federation
- `GET /api/v2/federation/rounds` — Estado de rounds de federación
- `POST /api/v2/federation/round` — Iniciar round de federación

### Staking
- `GET /api/v2/staking/registry` — Estado del registro de staking

### Governance
- `GET /api/v2/governance/proposals` — Listar propuestas
- `POST /api/v2/governance/proposal` — Crear propuesta

### Autenticación
Todos los endpoints soportan autenticación Ed25519 vía headers:
```
X-Node-ID: <node_id>
X-Signature: <ed25519_signature_hex>
X-Timestamp: <unix_timestamp>
```

---

## Rollback Procedure

Para revertir a v0.5.0 STABLE:

```bash
# One-command rollback
./ops/rollback_v0.6.0.sh

# O manualmente:
# 1. Detener servicio
sudo systemctl stop ed2kia

# 2. Restaurar binary
sudo cp /usr/local/bin/ed2kia.v0.5.0.backup /usr/local/bin/ed2kia

# 3. Desactivar Phase 6
# Editar config.toml: features = ["core-only"]

# 4. Reiniciar
sudo systemctl start ed2kia

# 5. Verificar
curl http://localhost:3030/api/v1/health
```

---

## Known Issues

| Issue | Severity | Workaround | Target Fix |
|---|---|---|---|
| 3 tests ignored (bytemuck f32 alignment) | Low | N/A (tests marcados como ignored) | v0.7.0 |
| ONNX adapter usa placeholder tensors | Medium | Modelos reales requieren ONNX Runtime | v0.7.0 |
| ZKP proofs no composicionales | Low | Usar Merkle proofs como fallback | v0.8.0 |
| Memory Guard no soporta GPU memory | Medium | Monitoreo GPU vía herramientas externas | v0.7.0 |

---

## Equipo & Agradecimientos

**Core Team:**
- Lead Architect & Release Manager
- Core Rust Developers
- Security Auditors
- Community Contributors

**Dependencias Clave:**
- candle-core (Hugging Face) — Tensor operations
- ed25519-dalek — Digital signatures
- axum 0.7 — HTTP framework
- libp2p — P2P networking
- redb — Embedded database

---

## Contacto & Soporte

| Canal | URL | Uso |
|---|---|---|
| GitHub Issues | repo/issues | Bugs y feature requests |
| Governance Channel | [TBD] | Discusiones de gobernanza |
| Security Reports | security@ed2kia.io | Reportes de vulnerabilidades |
| Documentation | docs/ | Documentación completa |

---

## Próximos Pasos

1. **Canary Deployment** (Day 0): Deploy a 3-5 seed nodes
2. **24h Monitoring**: Validar métricas críticas
3. **Expand to 50%** (Day 1): Nodos con reputación ≥0.7
4. **Full Network** (Day 3): Todos los nodos
5. **v0.6.0 STABLE** (Day 10): Después de 7 días de operación exitosa

---

## Mandato Ético

> La evolución debe ser gradual, verificable y reversible. La confianza se construye con transparencia, no con velocidad. Cada feature, cada línea de código, cada decisión de gobernanza debe servir al progreso humano y al desarrollo consciente de la IA.

**Principios de esta release:**
- ✅ **Gradual**: Feature gates permiten adopción progresiva
- ✅ **Verificable**: 170 tests, 0 warnings, OpenAPI spec público
- ✅ **Reversible**: Rollback en un comando a v0.5.0
- ✅ **Transparente**: Código abierto, documentación completa, métricas públicas
- ✅ **Ético**: Apache 2.0 + Cláusula de Uso Ético

---

*ed2kIA v0.6.0-RC — Construyendo IA federada, consciente y alineada.*
