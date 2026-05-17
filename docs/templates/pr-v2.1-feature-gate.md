# PR Template: v2.1 Feature Gate Contribution

---

## Pull Request: [Feature Gate] Descripcion Corta

**Feature Gate:** `v2.1-observability` | `v2.1-security-hardening` | `v2.1-zkp-v3` | `v2.1-gui` | `v2.1-enterprise` | `v2.1-sprint1`
**RFC:** [RFC-XXX](../governance/rfc-tracking.md) (si aplica)
**Autor:** @username
**Fecha:** YYYY-MM-DD

---

## 1. Descripcion

[Descripcion breve del cambio, max 3 lineas]

## 2. Motivo

[Por que es necesario este cambio? Que problema resuelve?]

## 3. Cambios

### Archivos Modificados

| Archivo | Tipo | Descripcion |
|---------|------|-------------|
| `src/...` | Modificado | [Descripcion] |
| `tests/...` | Nuevo | [Descripcion] |

### Feature Gate Afectado

```toml
# Cargo.toml
[features]
v2.1-XXXX = ["dep:XXXX"]
```

## 4. Testing

### Comandos Ejecutados

```bash
# Verificacion basica
cargo check --all-targets

# Feature gate specific
cargo check --tests --features v2.1-XXXX
cargo test --features v2.1-XXXX
cargo bench --features v2.1-XXXX --no-run
```

### Resultados

| Comando | Resultado | Tiempo |
|---------|-----------|--------|
| cargo check --all-targets | PASADO | Xs |
| cargo check --tests --features v2.1-XXXX | PASADO | Xs |
| cargo test --features v2.1-XXXX | PASADO (X/Y tests) | Xs |
| cargo bench --features v2.1-XXXX --no-run | PASADO | Xs |

## 5. CODEOWNERS Review

**Rutas Protegidas Afectadas:**

| Ruta | Owner | Notificado |
|------|-------|------------|
| `/src/monitoring/` | @ed2kia/ops-team | [ ] |
| `/src/security/` | @ed2kia/crypto-team | [ ] |
| `/docs/governance/` | @ed2kia/governance-team | [ ] |

## 6. CI/CD Integration

**Jobs Esperados:**

| Job | Estado |
|-----|--------|
| feature-gate-check | [ ] |
| feature-gate-tests (matrix) | [ ] |
| codeowners-sync | [ ] |

## 7. Ethics & Licensing

- [ ] Cumple con Apache 2.0 + Ethical Use Clause
- [ ] CERO lогica financiera
- [ ] CERO telemetry
- [ ] CERO unsafe code
- [ ] Governance & ethics first

## 8. Checklists

### Pre-Submit
- [ ] `cargo check --all-targets` pasa
- [ ] `cargo test --features v2.1-XXXX` pasa
- [ ] `cargo bench --features v2.1-XXXX --no-run` compila
- [ ] Tests agregados/modificados para el cambio
- [ ] Documentacion actualizada (si aplica)
- [ ] CHANGELOG.md actualizado

### Post-Merge
- [ ] Badge de reconocimiento asignado
- [ ] Contributor agregado a CONTRIBUTORS.md
- [ ] Release notes actualizados

## 9. Notas Adicionales

[Cualquier nota adicional para reviewers]

---

*Template basado en CONTRIBUTING.md v2.1 Ambassador Workflow*
*Feature Gates: v2.1-observability | v2.1-security-hardening | v2.1-zkp-v3 | v2.1-gui | v2.1-enterprise | v2.1-sprint1*
