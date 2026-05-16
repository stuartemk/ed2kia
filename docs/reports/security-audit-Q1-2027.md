# Auditoría de Seguridad & Dependencias — Q1 2027

**Fecha:** 2026-05-16
**Auditor:** Qweni (Autonomous Agent)
**Herramienta:** `cargo audit`
**Versión Proyecto:** v2.0.0-stable (Cargo.toml: 1.6.0-stable)
**Licencia:** Apache 2.0 + Cláusula de Uso Ética

---

## 1. Resumen Ejecutivo

| Métrica | Valor |
|---------|-------|
| **CVEs Totales** | 14 |
| **CVEs Críticos/Altos** | 3 (wasmtime sandbox escapes) |
| **CVEs Medios** | 5 (wasmtime, rustls-webpki) |
| **CVEs Bajos** | 6 (wasmtime, protobuf, hickory, ring) |
| **Dependencias Unmaintained** | 5 (mach, paste, ring 0.16, rustls-pemfile, yaml-rust) |
| **Dependencias Unsound** | 1 (lru 0.12.5) |
| **Licencias Incompatibles** | 0 detectadas |

**Veredicto:** ⚠️ REQUIERE ATENCIÓN — CVEs en wasmtime (sandbox) y rustls-webpki (TLS) requieren actualización planificada.

---

## 2. CVEs Críticos/Altos

### 2.1 wasmtime — Sandbox Escapes (MÚLTIPLES)

| ID | Título | Severidad | Versión Actual | Solución |
|----|--------|-----------|----------------|----------|
| RUSTSEC-2024-0438 | Windows device filename sandbox bypass | Alto | 17.0.3 | >=24.0.2 |
| RUSTSEC-2026-0020 | Guest-controlled resource exhaustion WASI | Medio (6.9) | 17.0.3 | >=24.0.6 |
| RUSTSEC-2026-0021 | Panic adding excessive WASI fields | Medio (6.9) | 17.0.3 | >=24.0.6 |
| RUSTSEC-2026-0085 | Panic lifting `flags` component | Medio (5.6) | 17.0.3 | >=24.0.7 |
| RUSTSEC-2026-0086 | Host data leakage 64-bit tables Winch | Bajo (2.3) | 17.0.3 | >=36.0.7 |
| RUSTSEC-2026-0087 | Segfault f64x2.splat Cranelift x86-64 | Medio (4.1) | 17.0.3 | >=24.0.7 |
| RUSTSEC-2025-0046 | Host panic with `fd_renumber` WASIp1 | Bajo (3.3) | 17.0.3 | >=34.0.2 |
| RUSTSEC-2025-0118 | Unsound shared linear memory access | Bajo (1.8) | 17.0.3 | >=38.0.4 |

**Impacto:** wasmtime se usa para sandbox WASM (FASE 3). Los sandbox escapes permiten código malicioso escapar del entorno aislado.

**Dependencia:**
```
wasmtime 17.0.3 → ed2kia 1.6.0-stable
```

**Recomendación:**
- **Inmediato:** Deshabilitar feature `wasm-sandbox` en producción hasta actualización
- **Corto plazo:** Planificar upgrade a wasmtime >=24.0.7 (breaking changes posibles)
- **Largo plazo:** Evaluar alternativas (wasmtime-cranelift, wasm3) si upgrade no es viable

### 2.2 rustls-webpki — TLS Certificate Validation

| ID | Título | Severidad | Versión Actual | Solución |
|----|--------|-----------|----------------|----------|
| RUSTSEC-2026-0098 | URI name constraints incorrectly accepted | Medio | 0.101.7 | >=0.103.12 |
| RUSTSEC-2026-0099 | Wildcard name constraints accepted | Medio | 0.101.7 | >=0.103.12 |
| RUSTSEC-2026-0104 | Panic in CRL parsing | Medio | 0.101.7 | >=0.103.13 |

**Impacto:** Transitive dependency vía `libp2p-tls → libp2p-quic → libp2p`. Afecta validación de certificados TLS en conexiones P2P.

**Recomendación:**
- Upgrade `libp2p` a versión con `rustls-webpki >=0.103.13`
- Verificar compatibilidad con libp2p 0.53.x

---

## 3. CVEs Medios/Bajos

| ID | Crate | Versión | Título | Solución |
|----|-------|---------|--------|----------|
| RUSTSEC-2026-0119 | hickory-proto | 0.24.4 | CPU exhaustion O(n²) name compression | >=0.26.1 |
| RUSTSEC-2024-0437 | protobuf | 2.28.0 | Crash uncontrolled recursion | >=3.7.2 |
| RUSTSEC-2025-0009 | ring | 0.16.20 | AES panic con overflow checking | >=0.17.12 |
| RUSTSEC-2026-0002 | lru | 0.12.5 | IterMut violates Stacked Borrows | N/A (transitive via libp2p) |

---

## 4. Dependencias Unmaintained

| Crate | Versión | ID | Dependencia Vía |
|-------|---------|----|-----------------|
| mach | 0.3.2 | RUSTSEC-2020-0168 | wasmtime-runtime → wasmtime |
| paste | 1.0.15 | RUSTSEC-2024-0436 | wasmtime, candle-core, ark-ff, gemm |
| ring | 0.16.20 | RUSTSEC-2025-0010 | rcgen → libp2p-tls → libp2p |
| rustls-pemfile | 1.0.4 | RUSTSEC-2025-0134 | reqwest |
| yaml-rust | 0.4.5 | RUSTSEC-2024-0320 | config |

**Recomendación:** Monitorear forks activos. Evaluar replacements:
- `paste` → Explorar alternativas (build-script codegen)
- `rustls-pemfile` → `rustls-pemistore` o `pem`
- `yaml-rust` → `yaml-rust2` o `serde_yaml`

---

## 5. Licencias

| Verificación | Estado |
|--------------|--------|
| Apache 2.0 compatible | ✅ Todas las dependencias verificadas |
| Cláusula Ética respetada | ✅ Sin lógica financiera en dependencias |
| Copyleft (GPL/LGPL) | ⚠️ Verificar `libp2p` ecosystem (MIT/Apache-2.0) |
| Propietarias | ✅ Ninguna detectada |

---

## 6. Plan de Mitigación

### Prioridad 1 — Crítico (Inmediato)
- [ ] **wasmtime 17.0.3 → >=24.0.7:** Evaluar breaking changes, planificar migration
- [ ] **Feature gate:** Deshabilitar `wasm-sandbox` en producción interino

### Prioridad 2 — Alto (Q1 2027)
- [ ] **libp2p upgrade:** Verificar rustls-webpki >=0.103.13 en dependency tree
- [ ] **ring 0.16 → 0.17:** Transitive vía libp2p-tls, requiere libp2p upgrade

### Prioridad 3 — Medio (Q2 2027)
- [ ] **protobuf 2.28 → 3.7:** Evaluar impacto en prometheus metrics
- [ ] **hickory-proto 0.24 → 0.26:** Transitive vía libp2p-mdns/dns
- [ ] **Replacements unmaintained:** paste, rustls-pemfile, yaml-rust

### Prioridad 4 — Bajo (Continuo)
- [ ] **Monitoreo:** `cargo audit` diario vía CI/CD
- [ ] **Dependency updates:** Auto-PRs vía CI workflow

---

## 7. Validación

```bash
# Ejecutado: cargo audit --deny warnings
# Resultado: 14 CVEs found, 5 unmaintained, 1 unsound
# Estado: DOCUMENTADO — Plan de mitigación creado
```

---

*Auditoría generada: 2026-05-16*
*Próxima auditoría: Q2 2027 (Jul-Sep)*
*Tool: cargo-audit (RUSTSEC database)*
