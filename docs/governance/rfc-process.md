# Proceso RFC (Request for Comments) — ed2kIA

**Versión:** 1.0
**Fecha:** 2026-05-16
**Estado:** Activo

---

## 1. Introducción

El proceso RFC (Request for Comments) es el mecanismo formal para proponer, discutir y aprobar cambios significativos en ed2kIA. Todo cambio que afecte arquitectura, API pública, gobernanza o política del proyecto debe pasar por este proceso.

### 1.1 Alcance

**Requiere RFC:**
- Cambios en arquitectura core
- Nuevos módulos o features mayores
- Breaking changes en API pública
- Cambios en política de gobernanza
- Nuevas dependencias mayores
- Cambios en modelo de seguridad

**No requiere RFC:**
- Bug fixes
- Mejoras de rendimiento internas
- Actualizaciones de documentación
- Dependency updates menores
- Refactorings sin cambios de API

---

## 2. Flujo RFC

```
Propuesta → Discusión → Aprobación → Implementación → Cierre
    │           │            │             │              │
    │           │            │             │              │
    ▼           ▼            ▼             ▼              ▼
 Draft     Discussion    Accepted      Implemented     Merged/Rejected
  (0-7)      (7-14)        (14)          (14-30)         (30-60)
```

### 2.1 Estados

| Estado | Descripción | Duración |
|--------|-------------|----------|
| **Draft** | Propuesta inicial en desarrollo | Sin límite |
| **Discussion** | Abierto a comentarios comunitarios | 14-30 días |
| **Accepted** | Aprobado por Core Team | — |
| **Implemented** | En implementación | 14-30 días |
| **Merged** | Implementado y fusionado a main | — |
| **Rejected** | Rechazado con justificación | — |
| **Superseded** | Reemplazado por RFC posterior | — |
| **Deferred** | Pospuesto a futuro | — |

---

## 3. Roles

### 3.1 Propositor (Author)

- Crea el RFC usando la plantilla
- Responde preguntas y comentarios
- Mantiene el RFC actualizado
- Implementa la propuesta si es aceptada (opcional)

### 3.2 Reviewers

- Miembros del Core Team asignados automáticamente
- Mínimo 2 reviewers por RFC
- Evalúan técnica, ética y alineación con constitución
- Proporcionan feedback constructivo

### 3.3 Core Team

- Decide aprobación/rechazo
- Resuelve disputas
- Asigna reviewers
- Monitorea progreso

### 3.4 Comunidad

- Participa en discusión pública
- Proporciona feedback
- Reporta issues
- Sugiere mejoras

---

## 4. Criterios de Aceptación

### 4.1 Técnicos

- [ ] Soluciona problema identificado
- [ ] Diseño técnico sólido y bien documentado
- [ ] Considera edge cases y failure modes
- [ ] Plan de testing adecuado
- [ ] Impacto en rendimiento evaluado
- [ ] Backward compatibility considerada (o breaking change justificado)

### 4.2 Éticos

- [ ] Alineado con Constitución del Proyecto
- [ ] No introduce lógica financiera
- [ ] Respeto a privacidad y seguridad
- [ ] Impacto comunitario evaluado
- [ ] Transparencia garantizada

### 4.3 Operativos

- [ ] Plan de implementación claro
- [ ] Recursos necesarios identificados
- [ ] Timeline realista
- [ ] Plan de rollback documentado
- [ ] Documentación actualizada

### 4.4 Votación

- **Aprobación:** ≥2 de Core Team + 0 vetos
- **Veto:** Cualquier miembro Core Team puede veto con justificación
- **Rechazo:** Mayoría Core Team + justificación pública

---

## 5. Timeline Estándar

| Fase | Duración | Responsable |
|------|----------|-------------|
| Draft → Discussion | Cuando esté listo | Author |
| Discussion | 14-30 días | Comunidad + Reviewers |
| Discussion → Accepted | Cuando consenso | Core Team |
| Implementation | 14-30 días | Author/Assignee |
| Implementation → Merged | Cuando PR merge | Core Team |

**Total estimado:** 30-60 días

---

## 6. Numeración

Los RFCs se numeran secuencialmente:

- **RFC-001** a **RFC-099:** v1.x series
- **RFC-100** a **RFC-199:** v2.x series
- **RFC-200**+: v3.x+ series

Número asignado al crear el RFC en estado Draft.

---

## 7. Plantilla

Ver [`docs/rfc/rfc-template.md`](../rfc/rfc-template.md) para plantilla completa.

Campos requeridos:
- Título
- Autor(es)
- Estado
- Motivación
- Alcance
- Diseño
- Impacto
- Alternativas
- Plan de Implementación
- Riesgos

---

## 8. Comunicación

### 8.1 Canales

| Canal | Uso |
|-------|-----|
| GitHub Issues | RFC tracking, discusión |
| Discord | Discusión informal, preguntas |
| Email | Notificaciones formales |
| Quarterly Review | RFC status report |

### 8.2 Etiquetado

Todos los RFCs usan etiquetas en GitHub:

| Label | Uso |
|-------|-----|
| `RFC` | Es un RFC |
| `RFC-Draft` | En estado Draft |
| `RFC-Discussion` | En discusión |
| `RFC-Accepted` | Aprobado |
| `RFC-Implemented` | En implementación |
| `RFC-Merged` | Completado |
| `RFC-Rejected` | Rechazado |

---

## 9. Archivos & Referencias

- **Plantilla RFC:** [`docs/rfc/rfc-template.md`](../rfc/rfc-template.md)
- **Constitución:** [`docs/governance/project-constitution.md`](project-constitution.md)
- **Source of Truth:** [`docs/roadmap/source-of-truth.md`](../roadmap/source-of-truth.md)
- **Issue Template:** [`.github/ISSUE_TEMPLATE/rfc-proposal.md`](../../.github/ISSUE_TEMPLATE/rfc-proposal.md)

---

## 10. FAQ

### ¿Cuánto tiempo toma un RFC?

30-60 días típicamente, dependiendo de complejidad y nivel de discusión.

### ¿Puedo retractar mi RFC?

Sí, el author puede cerrar su RFC en cualquier momento con justificación.

### ¿Qué pasa si hay disagreement?

Core Team facilita resolución. Si no hay consenso en 30 días, se marca como `Deferred` o `Rejected` con justificación.

### ¿Los RFCs son vinculantes?

Una vez `Accepted` e `Implemented`, el RFC se convierte en parte del proyecto. Cambios posteriores requieren nuevo RFC.

---

*RFC Process v1.0 — Última actualización: 2026-05-16*
