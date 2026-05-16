# RFC-XXX: [Título del RFC]

**RFC:** XXX
**Título:** [Título conciso y descriptivo]
**Autor(es):** [Nombre(s)]
**Estado:** Draft
**Creado:** 2026-05-16
**Actualizado:** 2026-05-16
**Discusión:** [Link a GitHub Issue]

---

## Motivación

Describe el problema que este RFC resuelve. Incluye:

- Contexto del problema
- Por qué es importante ahora
- Qué intentaste antes (si aplica)
- Referencias a issues/discusiones relacionadas

---

## Alcance

### Incluido

- [ ] [Componente/feature 1]
- [ ] [Componente/feature 2]
- [ ] [Documentación]

### Excluido

- [ ] [Componente fuera de alcance]
- [ ] [Feature para futuro RFC]

---

## Diseño

### Arquitectura

Describe el diseño propuesto. Incluye diagramas si aplica:

```
[Diagrama ASCII o descripción]
```

### API Changes

Si hay cambios en API pública, documenta aquí:

```rust
// Nueva API
pub struct NewFeature {
    // ...
}

// Breaking changes (si aplica)
// ~~~ OLD ~~~
// pub fn old_function() -> Result<T>;
// ~~~ NEW ~~~
pub fn new_function(param: Param) -> Result<T>;
```

### Configuración

Nuevos parámetros de configuración:

| Param | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `new_feature.enabled` | bool | `true` | Enable/disable feature |

---

## Impacto

### Rendimiento

| Métrica | Antes | Después | Δ |
|---------|-------|---------|---|
| [metric] | [valor] | [valor] | +/− |

### Breaking Changes

- [ ] Ninguno
- [ ] API pública (documentar)
- [ ] Configuración (documentar)
- [ ] Formato de datos (documentar migración)

### Dependencias

| Dependency | Versión | Razón |
|------------|---------|-------|
| [crate] | [version] | [reason] |

---

## Alternativas

Describe alternativas consideradas y por qué se rechazaron:

1. **[Alternativa 1]**
   - Pros: [...]
   - Contras: [...]
   - Razón de rechazo: [...]

2. **[Alternativa 2]**
   - Pros: [...]
   - Contras: [...]
   - Razón de rechazo: [...]

---

## Plan de Implementación

### Fase 1: Core (Semanas 1-2)

- [ ] Implementar estructura base
- [ ] Tests unitarios
- [ ] Documentación API

### Fase 2: Integración (Semanas 3-4)

- [ ] Integrar con módulos existentes
- [ ] Tests E2E
- [ ] Benchmarks

### Fase 3: Polish (Semanas 5-6)

- [ ] Optimización rendimiento
- [ ] Documentación completa
- [ ] Migration guide (si aplica)

### Fase 4: Release (Semana 7-8)

- [ ] Feature flag
- [ ] Canary deploy
- [ ] Release notes

---

## Riesgos

| Riesgo | Impacto | Probabilidad | Mitigación |
|--------|---------|--------------|------------|
| [riesgo] | Alto/Medio/Bajo | Alta/Media/Baja | [acción] |

---

## Backward Compatibility

Describe cómo se mantiene compatibilidad con versiones anteriores:

- [ ] 100% backward compatible
- [ ] Breaking change con migration guide
- [ ] Feature flag para opt-in

---

## Testing Strategy

- **Unit Tests:** [descripción]
- **Integration Tests:** [descripción]
- **E2E Tests:** [descripción]
- **Benchmarks:** [descripción]

---

## Referencias

- [Constitución del Proyecto](../governance/project-constitution.md)
- [Source of Truth](../roadmap/source-of-truth.md)
- [RFC Process](../governance/rfc-process.md)
- [Issue #XXX](https://github.com/ed2kia/ed2kIA/issues/XXX)

---

*RFC Template v1.0 — Última actualización: 2026-05-16*
