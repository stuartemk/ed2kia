# HANDOVER — ed2kIA v1.6.0-stable

**Documento de Handover para el Orquestador**
**Versión:** 1.6.0-stable
**Fecha:** 2026-05-14
**Estado:** Ready for Execution

---

## 1. Pre-vuelo (Checklist de 5 ítems)

Antes de ejecutar el lanzamiento, verifica los siguientes requisitos:

```bash
# 1. Verificar branch correcto
git branch --show-current
# Esperado: main

# 2. Verificar working directory limpio
git status --short
# Esperado: (salida vacía)

# 3. Ejecutar tests
cargo test --features "stable"
# Esperado: test result: ok. 187 passed

# 4. Ejecutar clippy
cargo clippy --features "stable"
# Esperado: Finished release [optimized] target(s)

# 5. Governance approval
# Verificar que el Governance Council aprobó el lanzamiento
# Documentar aprobación en release/v1.6.0-stable/approval.txt
```

**Criterio de Éxito:** Los 5 ítems deben pasar antes de continuar.

---

## 2. Ejecución Exacta

### Paso 1: Inicializar Git (si no existe)

```bash
# Solo si el repo no está inicializado
git init
git remote add origin https://github.com/ed2kIA/ed2kIA.git
git add -A
git commit -m "chore: initial commit for v1.6.0-stable release"
```

### Paso 2: Ejecutar Release Script

```bash
# Ejecutar script principal (incluye pre-flight + git operations)
bash release_v1.6.0.sh
# Esperado:
#   [STEP 1] Pre-flight Validation
#   ✓ cargo check passed
#   ✓ cargo clippy passed
#   ✓ cargo test passed
#   [STEP 2] Git Release
#   ✓ Release commit created
#   ✓ Tag v1.6.0-stable created
#   ✓ Branch pushed
#   ✓ Tag pushed
#   [STEP 3] Release Artifacts
#   ✓ Release binary built
#   ✓ Release packaged
#   ✓ Checksums generated
#   [STEP 4] Post-Launch Checklist
```

### Paso 3: Crear GitHub Release Draft

```
URL: https://github.com/ed2kIA/ed2kIA/releases/new?tag=v1.6.0-stable

Title: v1.6.0-stable — Official Release
Body: (Copiar de docs/official_launch_announcement_v1.6.md)

Archivos a adjuntar:
- release/ed2kIA-1.6.0-stable-*.tar.gz
- release/ed2kIA-1.6.0-stable-*.zip
- release/checksums.txt
```

### Paso 4: Verificar Badges

```bash
# Verificar que README.md badges apuntan a CI correcto
head -20 README.md
# Esperado: badges con 1.6.0_STABLE, 187 passed
```

---

## 3. Criterios de Éxito

| Criterio | Verificación | Estado |
|----------|--------------|--------|
| CI Green | `cargo test --features stable` = 187 passed | ⬜ Pendiente |
| Tag Visible | `git tag -l v1.6.0-stable` | ⬜ Pendiente |
| Release Draft | GitHub URL accesible | ⬜ Pendiente |
| Badges Actualizados | README.md → 1.6.0_STABLE | ⬜ Pendiente |
| Artifacts Generados | release/*.tar.gz existen | ⬜ Pendiente |
| Checksums | release/checksums.txt generado | ⬜ Pendiente |

**Criterio de Éxito Final:** 6/6 verificaciones completadas.

---

## 4. Rollback

Si algo falla post-push, ejecutar rollback inmediato:

### Rollback de Tag

```bash
# Eliminar tag remoto
git push origin --delete v1.6.0-stable

# Eliminar tag local
git tag -d v1.6.0-stable
```

### Rollback de Commit

```bash
# Revertir commit de release (NO rebase en shared branch)
git revert HEAD --no-edit

# Push revert
git push origin main
```

### Rollback Completo (Branch + Tag + Commit)

```bash
# 1. Identificar commit anterior al release
git log --oneline -10

# 2. Reset hard a commit anterior (solo si nadie pulló)
git reset --hard <commit-anterior>

# 3. Force push (ÚLTIMO RECURSO)
git push origin main --force

# 4. Eliminar tag
git push origin --delete v1.6.0-stable
git tag -d v1.6.0-stable
```

### Rollback de Operadores

```bash
# Instrucciones para operadores de nodo:
git fetch origin
git checkout v1.5.0-stable
cargo build --release --features "stable"
systemctl restart ed2kia
```

---

## 5. Sign-off

```
Orquestador: _________________________________
Timestamp:   _________________________________
Aprobación:  [ ] APROBADO  [ ] RECHAZADO
Notas:       _________________________________
```

**Post-Signoff Actions:**
1. Guardar firma en `release/v1.6.0-stable/orchestrator_signature.txt`
2. Notificar a Discord #announcements
3. Activar monitoreo (ver `docs/post-launch-monitoring.md`)
4. Iniciar ventana Critical (0-48h)

---

## 6. Referencias Rápidas

| Documento | Ruta |
|-----------|------|
| Dry-Run Validation | `release/v1.6.0-stable/dry_run_validation.md` |
| Release Script | `release_v1.6.0.sh` |
| Post-Launch Monitoring | `docs/post-launch-monitoring.md` |
| Triage Protocol | `POST_LAUNCH_TRIAGE.md` |
| Day 1 Operations Prompt | `DAY1_OPERATIONS_PROMPT.md` |
| v1.7 Roadmap | `docs/v1.7-roadmap-placeholder.md` |
| SECURITY Policy | `SECURITY.md` |
| Governance | `docs/GOVERNANCE.md` |

---

*Handover Document — ed2kIA v1.6.0-stable*
*Generated: 2026-05-14*
