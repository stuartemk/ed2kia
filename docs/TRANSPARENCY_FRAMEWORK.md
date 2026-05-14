# Framework de Transparencia & Financiamiento Comunitario

**Versión:** v1.5.0 STABLE
**Actualizado:** 2026-05-12

## Filosofía: Analogía Linux

ed2kIA es infraestructura pública de código abierto, análogo a Linux para la interpretabilidad de IA:

- **Código libre:** Apache 2.0 + Cláusula de Uso Ético
- **Gobernanza meritocrática:** Contribuciones técnicas determinan peso en gobernanza
- **Incentivo técnico:** Reputación, impacto comunitario y reconocimiento
- **Cero especulación:** No hay tokens, pools de liquidez ni mecanismos financieros en el código

## Separación Estricta: Código vs. Finanzas

### Lo que SÍ está en el código
- `src/reputation/`: Sistema de reputación técnica (créditos de cómputo, peso en gobernanza)
- `src/staking/`: Mecanismo técnico de stake (validación, anti-Sybil)
- `src/federation/`: Escalado adaptativo con awareness de capacidad
- `src/zkp/`: Verificación criptográfica con quorum dinámico
- Métricas de rendimiento, SLOs y contratos predictivos

### Lo que NO está en el código
- Tokens criptográficos especulativos
- Pools de liquidez
- Tesorerías financieras
- Mecanismos de yield farming o staking financiero

**Los módulos de reputación y staking son puramente técnicos.** Representan créditos de cómputo, peso en gobernanza y métricas anti-Sybil. No tienen valor financiero inherente.

## Estado Actual v1.5.0

### Métricas de Transparencia

| Métrica | Valor |
|---------|-------|
| Tests Passing | 132 (108 unit + 15 E2E + 9 stress) |
| Clippy Errors | 0 |
| Unsafe Blocks | 0 |
| Lógica Financiera | Ninguna |
| Telemetry | Ninguna |
| Licencia | Apache 2.0 + Ethical Use |
| Feature Flags | `--features stable` (consolidado) |

### Módulos Públicos v1.5.0

| Módulo | Versión | Tests | Propósito |
|--------|---------|-------|-----------|
| SAE Fine-Tuning | v6 | 81 | Fine-tuning distribuido con alignment v3 |
| Federation Scaling | v6 | 20 | Escalado con awareness de capacidad |
| Dynamic Sharder | v2 | 20 | Distribución adaptativa de shards |
| Gradient Sync | v6 | 20 | Sincronización con alignment cross-model |
| Async ZKP | v11 | 24 | Batching dinámico con quorum |
| Cross-Fed Verifier | v2 | 24 | Verificación con agregación Merkle |

## Canales de Financiamiento

Los fondos del proyecto se canalizan exclusivamente a través de:

### 1. Open Collective
- Gastos técnicos del proyecto (infraestructura, auditorías, CI/CD)
- Transparencia total de ingresos y egresos
- Separación estricta entre finanzas personales del creador y gastos del proyecto

### 2. Gitcoin Grants
- Financiamiento comunitario vía matching pool
- Contribuciones de la comunidad como validación de impacto
- Ciclos de financiamiento transparentes y auditables
- Plantilla de aplicación: [`docs/GITCOIN_GRANTS_APPLICATION_TEMPLATE.md`](GITCOIN_GRANTS_APPLICATION_TEMPLATE.md)

### 3. GitHub Sponsors
- Apoyo mensual de individuos y organizaciones
- Niveles de patrocinio transparentes
- Reconocimiento público de contribuidores

## Disclaimer Legal (México)

> **Aviso Importante:** ed2kIA es un proyecto de software de código abierto registrado en México. Las contribuciones financieras a través de Open Collective, Gitcoin Grants y GitHub Sponsors son donaciones voluntarias sin expectativa de retorno financiero. Este proyecto NO emite valores, tokens ni instrumentos financieros. Las contribuciones no constituyen inversión según la Ley del Mercado de Valores de México. El proyecto opera bajo la Licencia Apache 2.0 con Cláusula de Uso Ético. Para consultas legales: contacto@ed2kIA.org

## Principios de Transparencia

1. **Código Auditable:** Todo el código es público y verificable
2. **Finanzas Públicas:** Todos los gastos son visibles en Open Collective
3. **Gobernanza Abierta:** Decisiones documentadas en `docs/GOVERNANCE.md`
4. **Telemetría Cero:** Cero datos salientes, cero tracking
5. **Acceso Equitativo:** Infraestructura accesible para todos, sin barreras económicas
6. **Reportes de Sprint:** Cada sprint publica release notes con métricas de calidad
7. **CI/CD Público:** Todos los pipelines de build/test son visibles en GitHub Actions

## Incentivos de Contribución

| Contribución | Incentivo |
|--------------|-----------|
| Código | Reputación técnica, peso en gobernanza |
| Documentación | Reconocimiento público, reputación |
| Auditoría de seguridad | Reconocimiento, reputación técnica |
| Operación de nodo | Créditos de cómputo, peso en consenso |
| Investigación | Publicación, reconocimiento académico |

**Ningún incentivo es financiero.** El valor es reputación técnica, impacto comunitario y gobernanza meritocrática.

## Cómo Verificar

```bash
# Verificar ausencia de unsafe code
grep -rn "unsafe" src/ --include="*.rs"

# Verificar ausencia de telemetry
grep -rn "telemetry\|analytics\|tracking" src/ --include="*.rs" -i

# Verificar ausencia de lógica financiera
grep -rn "payment\|wallet\|token\|coin" src/ --include="*.rs" -i

# Ejecutar suite completa de tests
cargo test --features stable

# Verificar con clippy
cargo clippy --features stable -- -D warnings
```

## Contacto

- Issues: https://github.com/ed2kia/ed2kIA/issues
- Transparencia: https://opencollective.com/ed2kia (próximamente)
- Legal: contacto@ed2kIA.org
