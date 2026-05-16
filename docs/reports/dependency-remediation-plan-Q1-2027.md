# Plan de Remediación de Dependencias — Q1 2027

**Fecha:** 2026-05-16
**Autor:** Qweni (Autonomous Agent)
**Versión Proyecto:** v2.0.0-stable
**Licencia:** Apache 2.0 + Cláusula de Uso Ética

---

## 1. CVE Matrix

| ID | Severidad | Paquete | Versión Actual | Impacto en ed2kIA | Estado |
|----|-----------|---------|----------------|-------------------|--------|
| RUSTSEC-2024-0438 | Alto | wasmtime | 17.0.3 | Sandbox WASM (FASE 3) — escapes permiten código malicioso salir del entorno | pending |
| RUSTSEC-2026-0020 | Medio | wasmtime | 17.0.3 | Guest-controlled resource exhaustion WASI | pending |
| RUSTSEC-2026-0021 | Medio | wasmtime | 17.0.3 | Panic adding excessive WASI fields | pending |
| RUSTSEC-2026-0085 | Medio | wasmtime | 17.0.3 | Panic lifting `flags` component | pending |
| RUSTSEC-2026-0086 | Bajo | wasmtime | 17.0.3 | Host data leakage 64-bit tables Winch | pending |
| RUSTSEC-2026-0087 | Medio | wasmtime | 17.0.3 | Segfault f64x2.splat Cranelift x86-64 | pending |
| RUSTSEC-2025-0046 | Bajo | wasmtime | 17.0.3 | Host panic with `fd_renumber` WASIp1 | pending |
| RUSTSEC-2025-0118 | Bajo | wasmtime | 17.0.3 | Unsound shared linear memory access | pending |
| RUSTSEC-2026-0098 | Medio | rustls-webpki | 0.101.7 | TLS cert validation — transitive via libp2p-tls | pending |
| RUSTSEC-2026-0099 | Medio | rustls-webpki | 0.101.7 | Wildcard name constraints accepted | pending |
| RUSTSEC-2026-0104 | Medio | rustls-webpki | 0.101.7 | Panic in CRL parsing | pending |
| RUSTSEC-2026-0119 | Medio | hickory-proto | 0.24.4 | CPU exhaustion O(n²) name compression | pending |
| RUSTSEC-2024-0437 | Medio | protobuf | 2.28.0 | Crash uncontrolled recursion | pending |
| RUSTSEC-2025-0009 | Medio | ring | 0.16.20 | AES panic con overflow checking | pending |
| RUSTSEC-2026-0002 | Bajo | lru | 0.12.5 | IterMut violates Stacked Borrows | pending |

## 2. Pinning Strategy

### 2.1 Versiones Seguras a Fijar (Crítico)

| Paquete | Versión Actual | Versión Segura Mínima | Compatible con v2.0.0-stable |
|---------|----------------|------------------------|------------------------------|
| wasmtime | 17.0.3 | >=24.0.7 | ⚠️ Breaking changes posibles — requiere feature gate |
| rustls-webpki | 0.101.7 | >=0.103.13 | ⚠️ Transitive via libp2p — upgrade libp2p necesario |
| ring | 0.16.20 | >=0.17.12 | ⚠️ Transitive via libp2p-tls |
| protobuf | 2.28.0 | >=3.7.2 | ✅ Compatible con prometheus metrics |
| hickory-proto | 0.24.4 | >=0.26.1 | ⚠️ Transitive via libp2p-mdns/dns |

### 2.2 Pinning en Cargo.toml (Solo si crítico y compatible)

**DECISIÓN:** NO aplicar pinning directo en v2.0.0-stable. Los upgrades de wasmtime y libp2p tienen breaking changes que requieren:
1. Feature gate `v2.1-security-hardening`
2. Testing exhaustivo con `cargo test`
3. Validación de compatibilidad P2P

### 2.3 Feature-Gated Replacements

| Paquete | Reemplazo | Feature Gate | Estado |
|---------|-----------|--------------|--------|
| wasmtime 17.x | wasmtime >=24.0.7 | v2.1-security-hardening | scaffold |
| libp2p (con rustls-webpki 0.101) | libp2p con rustls-webpki >=0.103.13 | v2.1-security-hardening | scaffold |
| paste 1.0.15 | build-script codegen | v2.1-security-hardening | pending |
| rustls-pemfile 1.0.4 | rustls-pemistore o pem | v2.1-security-hardening | pending |
| yaml-rust 0.4.5 | yaml-rust2 o serde_yaml | v2.1-security-hardening | pending |

## 3. Rollback & Validation Plan

### 3.1 Pasos de Validación Pre-Upgrade
```bash
# 1. Backup actual state
git tag pre-remediation-Q1-2027

# 2. Aplicar cambios en feature gate
cargo check --features v2.1-security-hardening

# 3. Validar tests
cargo test --features v2.1-security-hardening

# 4. Validar clippy
cargo clippy --features v2.1-security-hardening -- -D warnings

# 5. Verificar audit
cargo audit --features v2.1-security-hardening
```

### 3.2 Pasos de Rollback
```bash
# Si cargo test o clippy fallan:
git reset --hard pre-remediation-Q1-2027
cargo check --all-targets  # Verificar restauración
```

### 3.3 Criterios de Éxito
- `cargo check --features v2.1-security-hardening`: 0 errores
- `cargo test --features v2.1-security-hardening`: 100% pass rate
- `cargo clippy --features v2.1-security-hardening`: 0 warnings nuevos
- `cargo audit --features v2.1-security-hardening`: CVEs reducidos vs baseline

## 4. Timeline de Ejecución

| Fase | Acción | Fecha Estimada | Dependencias |
|------|--------|----------------|--------------|
| 1 | Documentación (este plan) | 2026-05-16 | ✅ Completado |
| 2 | Feature gate `v2.1-security-hardening` | Q2 2027 | RFC-002 approval |
| 3 | wasmtime upgrade (feature-gated) | Q2 2027 | Fase 2 |
| 4 | libp2p upgrade (feature-gated) | Q2 2027 | Fase 2 |
| 5 | Replacements unmaintained | Q3 2027 | Fases 3-4 |
| 6 | Promoción a stable | Q3 2027 | Fase 5 + validación |

## 5. Reporte al Orquestador

**CVEs Críticos sin mitigación viable inmediata:**
- RUSTSEC-2024-0438 (wasmtime sandbox escape): Requiere upgrade mayor con breaking changes
- RUSTSEC-2026-0098/0099/0104 (rustls-webpki TLS): Requiere libp2p upgrade

**Recomendación:** Mantener feature gates hasta RFC-002 approval. Monitorear CI/CD con `cargo audit` diario.

---

*Plan generado: 2026-05-16*
*Próxima revisión: Q2 2027 (Jul-Sep)*
*Tool: cargo-audit (RUSTSEC database)*
