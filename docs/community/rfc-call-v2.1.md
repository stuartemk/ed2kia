# Llamada Abierta RFC v2.1 — ed2kIA

**Fecha:** 2026-05-16
**Versión Target:** v2.1
**Fecha Límite Propuestas:** 2026-06-30
**Revisión Core Team:** 2026-07-01 a 2026-07-15
**Inicio Implementación:** 2026-07-16

---

## Introducción

ed2kIA v2.0.0-stable está en modo Estipulación (Stewardship). Ahora abrimos el proceso comunitario para definir la dirección de v2.1 a través de RFCs (Request for Comments).

**Tu voz importa.** Esta es tu oportunidad para influir directamente en el futuro de ed2kIA.

---

## Temas Abiertos para RFC

### 1. GUI Desktop/Mobile Completa

**Contexto:** v2.0 introdujo Neural Tauri Bridge y Neural Steer UI. v2.1 podría completar la experiencia GUI.

**Posibles RFCs:**
- GUI completa con visualización 3D de conceptos SAE
- Mobile app (iOS/Android) con Tauri Mobile / Capacitor
- Browser extension para integración con herramientas de IA
- Real-time collaboration en análisis de modelos

**Qué necesitamos:**
- Propuestas de arquitectura
- User stories y wireframes
- Plan de testing cross-platform

### 2. ZKP v3 — Proof Compression

**Contexto:** v2.0 tiene multi-curve ZKP y proof aggregation. v2.1 podría optimizar tamaño y velocidad.

**Posibles RFCs:**
- Proof compression con techniques (e.g., recursive proofs)
- Ultra-lightweight verification para mobile/IoT
- Cross-chain proof interoperability
- ZKP para SAE activations (privacy-preserving interpretability)

**Qué necesitamos:**
- Análisis de rendimiento actual vs target
- Propuesta criptográfica específica
- Security review plan

### 3. Gobernanza DAO-lite

**Contexto:** v2.0 tiene constitución y RFC process. v2.1 podría añadir mecanismos de votación comunitaria.

**Posibles RFCs:**
- Sistema de votación basado en reputación (no tokens)
- Propuestas comunitarias con quórum
- Delegación de voto (liquid democracy)
- Treasury transparente para grants

**Qué necesitamos:**
- Diseño de mecanismo de votación
- Protección contra Sybil attacks
- Alineación con cláusula de cero lógica financiera

**Nota crítica:** Cualquier propuesta de gobernanza debe respetar explícitamente la cláusula de cero lógica financiera. No tokens, no staking financiero, no acumulación de valor.

### 4. Enterprise Integrations

**Contexto:** v2.0 tiene K8s manifests y Docker deploy. v2.1 podría facilitar adopción enterprise.

**Posibles RFCs:**
- K8s Operator para auto-scaling y management
- SSO integration (OIDC, SAML)
- Prometheus/Grafana dashboards pre-configurados
- API gateway con rate limiting y auth
- Compliance reporting (SOC2, GDPR)

**Qué necesitamos:**
- Requirements de enterprise reales
- Propuesta de arquitectura
- Licensing considerations

### 5. Temas Abiertos (Comunidad)

¿Tienes otra idea? ¡Propón tu propio RFC! Temas sugeridos:

- Performance optimizations (SIMD, GPU, WASM)
- New SAE models o architectures
- Federated learning improvements
- Tooling para researchers
- Documentation y onboarding
- Internationalización (i18n)
- Accessibility (a11y)

---

## Cómo Participar

### Opción 1: Crear RFC Completo

1. Lee el [RFC Process](../governance/rfc-process.md)
2. Copia la [plantilla RFC](../rfc/rfc-template.md)
3. Crea issue con template [RFC Proposal](https://github.com/ed2kia/ed2kIA/issues/new?template=rfc-proposal.md)
4. Discute en Discord y GitHub

### Opción 2: Propuesta Simplificada

1. Crea issue con título `[RFC Idea] [Tema]`
2. Describe tu idea en 2-3 párrafos
3. Core Team ayuda a desarrollar en RFC formal

### Opción 3: Participar en Discusión

1. Comenta en RFCs existentes
2. Proporciona feedback técnico
3. Ayuda con testing y review
4. Sugiere mejoras

---

## Timeline

| Fecha | Hito |
|-------|------|
| 2026-05-16 | Llamada abierta |
| 2026-06-30 | Fecha límite propuestas |
| 2026-07-01 | Inicio revisión Core Team |
| 2026-07-15 | Decisiones de aprobación |
| 2026-07-16 | Inicio implementación |
| 2026-09-30 | Target release v2.1 |

---

## Criterios de Aceptación

Todas las propuestas deben:

1. **Alinear con Constitución:** Respetar misión, visión y principios éticos
2. **Cero Lógica Financiera:** No introducir tokens, staking financiero o mecanismos especulativos
3. **Valor Comunitario:** Beneficiar a la comunidad, no a intereses privados
4. **Viabilidad Técnica:** Plan de implementación realista
5. **Testing Adecuado:** Estrategia de testing clara
6. **Documentación:** Plan de documentación incluido

---

## Canales de Discusión

| Canal | Uso |
|-------|-----|
| [GitHub Issues](https://github.com/ed2kia/ed2kIA/issues) | RFC formales, tracking |
| Discord | Discusión informal, preguntas |
| [RFC Process](../governance/rfc-process.md) | Documentación del proceso |

---

## Recursos

- [RFC Process](../governance/rfc-process.md)
- [RFC Template](../rfc/rfc-template.md)
- [Project Constitution](../governance/project-constitution.md)
- [Source of Truth](../roadmap/source-of-truth.md)
- [State of ed2kIA v2.0](../announcements/state-of-ed2kIA-v2.0.md)

---

## FAQ

### ¿Necesito ser experto para proponer?

No. Cualquier idea bienvenida. Core Team ayuda a desarrollar propuestas en RFCs formales.

### ¿Cuánto tiempo toma?

30-60 días típicamente. Discusión (14-30 días) + Implementación (14-30 días).

### ¿Qué pasa si mi RFC es rechazado?

Recibirás justificación detallada. Puedes iterar y re-proponer, o aplicar learnings a nueva propuesta.

### ¿Puedo implementar RFC de otro?

¡Sí! Colaboración bienvenida. Contacta al author para coordinar.

---

*RFC Call v2.1 — Abierta hasta 2026-06-30*
