## 🚀 Performance: RTT-Based Geographic Federation Routing

**RFC:** [RFC-001](../rfc/rfc-001-latency-mitigation-v1.7.md) §2.3
**Difficulty:** Intermediate
**Estimated Time:** 10-14 hours
**Labels:** `performance`, `good-first-issue`, `p2p`, `bridge`

### Descripción

Implementar scoring basado en Round-Trip Time (RTT) de libp2p para enrutar tensores al nodo federado más cercano geográficamente, reduciendo la latencia de red en 30-50%.

### Contexto

Actualmente, `FederationZKPBridgeV7` selecciona federaciones basado en credibilidad y capacidad. Agregar RTT como factor de scoring permite preferir nodos cercanos geográficamente sin sacrificar confiabilidad.

### Criterios de Aceptación

- [ ] Agregar campo `rtt_ms: f64` a `FederationNodeV7`
- [ ] Implementar scoring compuesto: `score = credibility × (1 / rtt_ms) × capacity_factor`
- [ ] Actualizar `select_best_federation()` para usar RTT scoring
- [ ] Fallback a nodo alternativo si el más cercano está sobrecargado (< 100ms)
- [ ] ≥ 15 tests unitarios
- [ ] Documentación completa

### Módulos Afectados

- `src/bridge/federation_zkp_bridge_v7.rs` — RTT scoring
- `src/federation/scaling_v7.rs` — Geographic-aware shard assignment

### Recursos

- [RFC-001 §2.3](../rfc/rfc-001-latency-mitigation-v1.7.md)
- [libp2p identify protocol](https://docs.libp2p.io/reference/kube/protocol/identify/)
- [`src/bridge/federation_zkp_bridge_v7.rs`](../../src/bridge/federation_zkp_bridge_v7.rs)
