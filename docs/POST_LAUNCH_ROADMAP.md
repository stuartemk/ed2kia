# Post-Launch Roadmap — ed2kIA v1.0.0+

## Visión a 12 Meses

Consolidar ed2kIA como la infraestructura estándar para interpretabilidad distribuida de LLMs, con una red global de nodos autónomos, gobernanza comunitaria y ecosistema de contribuciones abiertas.

---

## v1.1.0 — Optimización (Meses 1-3)

### Objetivos
- Reducir latencia de inferencia SAE en >30%
- Mejorar throughput de federación FedAvg
- Optimizar consumo de memoria en nodos edge

### Hitos Técnicos
- [ ] SIMD optimization en forward pass SAE
- [ ] Parallel proof batching en Async ZKP
- [ ] Adaptive peer scoring en Federation
- [ ] Connection pooling en API v2
- [ ] Benchmark suite automatizada

### Gobernanza
- [ ] Primer ciclo de propuestas comunitarias
- [ ] Establecer quórum mínimo para votaciones
- [ ] Documentar proceso de review de PRs

---

## v1.2.0 — Ecosistema (Meses 4-6)

### Objetivos
- Expandir interoperabilidad a 5+ modelos (Llama, Mistral, Gemma, etc.)
- Lanzar SDK para desarrolladores
- Integrar con HuggingFace Hub

### Hitos Técnicos
- [ ] Auto-detection de schema por modelo
- [ ] SDK Rust + Python
- [ ] Plugin system para módulos personalizados
- [ ] Grafana dashboard oficial
- [ ] CLI improvements (autocomplete, colors)

### Comunidad
- [ ] Programa de bug bounty
- [ ] Contributor onboarding guide
- [ ] Primer meetup comunitario
- [ ] Blog técnico mensual

---

## v2.0.0 — Evolución (Meses 7-12)

### Objetivos
- Arquitectura v2 con mejoras de rendimiento
- Eliminación de feature flags legacy
- Soporte para hardware especializado (TPU, NPU)

### Hitos Técnicos
- [ ] Remover aliases de feature flags legacy
- [ ] Async runtime optimization
- [ ] Zero-copy tensor operations
- [ ] Hardware acceleration auto-detection
- [ ] Formal verification de ZKP circuits

### Gobernanza
- [ ] Transición a gobernanza 100% comunitaria
- [ ] DAO proposal system
- [ ] Treasury management
- [ ] Grants program para contribuidores

---

## Canales de Contribución

| Canal | Descripción | Link |
|-------|-------------|------|
| GitHub Issues | Reportar bugs, proponer features | github.com/ed2kia/ed2kia/issues |
| Pull Requests | Contribuir código | github.com/ed2kia/ed2kia/pulls |
| Discord | Chat comunitario | (TBD) |
| Matrix | Canal federado | (TBD) |
| Email | Contactos oficiales | team@ed2kia.org |
| Security | Disclosure responsable | security@ed2kia.org |

## Principios de Desarrollo

1. **Stability First**: Zero regressions en APIs estables
2. **Test Coverage**: >90% en código crítico
3. **Documentation**: Cada PR incluye docs
4. **Security**: Audit antes de cada release
5. **Ethics**: Mandato ético en cada decisión

## Métricas de Éxito

| Métrica | Target v1.1.0 | Target v1.2.0 | Target v2.0.0 |
|---------|---------------|---------------|---------------|
| Active Nodes | 50 | 200 | 1000 |
| Contributors | 10 | 30 | 100 |
| Models Supported | 3 | 5+ | 10+ |
| Uptime | 99.5% | 99.9% | 99.95% |
| Latency (p99) | <500ms | <300ms | <100ms |

---

**Última actualización**: 2026-05-05
**Mantenido por**: ed2kIA Core Team
