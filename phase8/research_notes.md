# Phase 8 Research Notes - State of the Art

> **Fecha**: 2026-05-04  
> **VersiÃ³n**: Phase 8 Planning  
> **PropÃ³sito**: Referencias tÃ©cnicas y state-of-the-art para decisiones arquitectÃ³nicas  
> **Licencia**: Apache 2.0 + Ethical Use Clause  

---

## 1. ZKP Escalable (Zero-Knowledge Proofs)

### 1.1 Contexto

Los ZKP permiten verificar computaciones sin revelar los datos subyacentes. En ed2kIA, se usan para:
- Verificar compromiso de pesos sin revelar los pesos
- Validar participaciÃ³n en federaciÃ³n sin revelar identidad
- Probar alineaciÃ³n sin revelar feedback individual

### 1.2 State of the Art (2026)

| TÃ©cnica | Ventajas | Desventajas | Caso de Uso ed2kIA |
|---|---|---|---|
| **PLONK** | Universal, percursive, ~1ms verify | Setup trustless pero lento | Batch commitment verification |
| **Halo2** | No setup, polynomial-based | Proof size grande (~10KB) | Dynamic circuit composition |
| **Marlin** | Universal, transparent setup | Proof size mediano | Cross-network verification |
| **STARKs** | Post-quantum, fast proof | Proof size grande (~100KB) | Long-term security |
| **Bulletproofs** | No setup, rangeproofs eficientes | Verification lenta O(n) | Resource commitment proofs |

### 1.3 RecomendaciÃ³n

**Para v0.8.0-alpha**: Mantener PLONK para batch verification + Merkle fallback.  
**Para v1.0.0**: Evaluar Halo2 para circuitos dinÃ¡micos si se requiere composiciÃ³n de pruebas.

**Referencias**:
- Gabizon et al. "PLONK: Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge" (2019)
- Alessandro Chiesa et al. "Marlin: Constraints over rings with efficient assumptions and linear-time setup" (2019)
- Privacy and Scaling Explorations Team. "Halo 2: The next generation of recursive SNARKs" (2020)

### 1.4 Benchmarks de Referencia

| MÃ©trica | PLONK | Halo2 | Marlin | STARKs |
|---|---|---|---|---|
| Proof time | ~50ms | ~100ms | ~30ms | ~10ms |
| Verify time | ~1ms | ~2ms | ~1.5ms | ~5ms |
| Proof size | ~2KB | ~10KB | ~5KB | ~100KB |
| Setup | Trustless | None | Transparent | None |

---

## 2. FederaciÃ³n AsÃ­ncrona

### 2.1 Contexto

La federaciÃ³n asÃ­ncrona permite que nodos participen en el entrenamiento federado sin requerir sincronizaciÃ³n estricta, mejorando la tolerancia a latencia y fallos.

### 2.2 State of the Art (2026)

| Enfoque | DescripciÃ³n | Ventajas | Desventajas |
|---|---|---|---|
| **Stale Gradient Tolerance** | Aceptar updates con delay â¤Î´ | Simple, robusto | Convergence mÃ¡s lenta |
| **Async FedAvg** | Agregar updates sin esperar round completo | Alta throughput | Consistency eventual |
| **Heterogeneous FL** | Nodos con diferentes frecuencias de update | Flexible | Complex scheduling |
| **Communication Compression** | Quantization + sparsification | Menos bandwidth | Precision loss |
| **Adaptive Aggregation** | Ponderar updates por frescura y calidad | Optimizado | Complex implementation |

### 2.3 RecomendaciÃ³n

**Para v0.8.0-alpha**: Implementar stale gradient tolerance con Î´=30s.  
**Para v0.9.0-rc**: Agregar adaptive aggregation basado en trust score.  
**Para v1.0.0**: Evaluar communication compression si el bandwidth es bottleneck.

**Referencias**:
- Stich "Error Feedback Compensates for Low-Precision Communication in Distributed Optimization" (2018)
- Chen et al. "Adaptive Federated Optimization" (2020)
- Li et al. "Federated Optimization in Heterogeneous Networks" (2020)

### 2.4 IntegraciÃ³n con ed2kIA

```
SyncProtocol (Phase 6)
âââ Round-based (sincrÃ³nico) â Actual
âââ Async mode (propuesto)
    âââ Stale gradient tolerance (Î´=30s)
    âââ Adaptive weighting (frescura â trust)
    âââ Eventual consistency guarantees
```

---

## 3. UI Reactiva (Alpine.js + WebSockets)

### 3.1 Contexto

El dashboard operacional requiere actualizaciones en tiempo real sin la complejidad de frameworks pesados como React o Vue.

### 3.2 State of the Art (2026)

| Framework | Bundle Size | Learning Curve | Real-time Support | Caso de Uso |
|---|---|---|---|---|
| **Alpine.js** | ~6KB | Baja | Manual (WS) | Dashboards ligeros |
| **Preact** | ~3KB | Media | Manual (WS) | Apps mÃ³viles |
| **Svelte** | ~1KB (compiled) | Baja | Manual (WS) | Apps completas |
| **Solid.js** | ~4KB | Media | Manual (WS) | High-performance UIs |
| **HTMX** | ~14KB | Muy baja | Built-in HTMX ws | Server-driven UIs |

### 3.3 RecomendaciÃ³n

**Para v0.8.0-alpha**: Alpine.js + WebSockets para dashboard inicial.  
**JustificaciÃ³n**:
- Bundle size mÃ­nimo (6KB)
- Curva de aprendizaje baja
- Compatible con HTML existente
- Sin build step requerido

**Stack propuesto**:
```
Frontend
âââ Alpine.js (reactividad)
âââ Tailwind CSS (estilos)
ââââ Chart.js (grÃ¡ficas)
ââââ WebSocket client (tiempo real)
ââââ HTMX (server-driven updates)
```

**Referencias**:
- Alpine.js: https://alpinejs.dev/
- HTMX: https://htmx.org/
- Chart.js: https://www.chartjs.org/

### 3.4 PatrÃ³n de WebSocket

```javascript
// PatrÃ³n recomendado para ed2kIA
const ws = new WebSocket('ws://host/api/v3/ws/metrics');

ws.onmessage = (event) => {
  const metrics = JSON.parse(event.data);
  // Alpine.js reactivity
  window.metrics = metrics;
};

ws.onclose = () => {
  // Exponential backoff reconnection
  const delay = Math.min(1000 * Math.pow(2, retries), 30000);
  setTimeout(() => reconnect(), delay);
};
```

---

## 4. Governance LÃ­quida

### 4.1 Contexto

La governance lÃ­quida permite delegaciÃ³n de votos con capacidad de revocaciÃ³n, creando un sistema mÃ¡s dinÃ¡mico y participativo que la democracia directa o representativa tradicional.

### 4.2 State of the Art (2026)

| Componente | DescripciÃ³n | Implementaciones | Madurez |
|---|---|---|---|
| **Weighted Delegation** | Delegar voto con peso personalizado | Liquid Democracy (Polkadot), Tally | Alta |
| **Dynamic Quorum** | QuÃ³rum se ajusta basado en participaciÃ³n | Conviction Voting (Balancer) | Media |
| **Continuous Voting** | VotaciÃ³n continua (no por perÃ­odos) | Quadratic Funding (Gitcoin) | Alta |
| **Signal Boosting** | Amplificar seÃ±ales de expertos | Prediction Markets (Polymarket) | Media |
| **Delegation Markets** | Mercados de delegaciÃ³n (pago por delegar) | TeÃ³rico | Baja |

### 4.3 RecomendaciÃ³n

**Para v0.9.0-rc**: Implementar weighted delegation + dynamic quorum.  
**Para v1.0.0**: Evaluar continuous voting si la frecuencia de propuestas lo justifica.

**DiseÃ±o propuesto**:
```
Governance v2
âââ Weighted Delegation
â   âââ Delegar a nodo con peso w â [0, 1]
â   âââ Revocar instantÃ¡neamente
â   âââ Cadena de delegaciÃ³n (mÃ¡x 3 niveles)
âââ Dynamic Quorum
â   âââ QuÃ³rum base: 66% de nodos activos
â   âââ Ajuste: âquorum = f(participaciÃ³n, urgencia)
â   âââ MÃ­nimo absoluto: 33%
âââ Continuous Voting
    ââââ Ventana de votaciÃ³n: 7 dÃ­as
    ââââ EjecuciÃ³n automÃ¡tica si quÃ³rum alcanzado
    ââââ Emergency pause: multisig 3/5
```

**Referencias**:
- Liquid Democracy: https://wiki.polkadot.network/docs/learn-democracy
- Conviction Voting: https://balancer.fi/blog/conviction-voting/
- Quadratic Funding: https://gitcoin.co/grants

### 4.4 FÃ³rmula de QuÃ³rum DinÃ¡mico

```
quorum = quorum_base + adjustment

donde:
  quorum_base = 0.66 (66%)
  adjustment = Î± * (participation_rate - 0.5) + Î² * urgency_factor

con:
  Î± = 0.2 (peso de participaciÃ³n)
  Î² = 0.1 (peso de urgencia)
  participation_rate â [0, 1]
  urgency_factor â [0, 1] (1 = emergency)

lÃ­mites:
  quorum_min = 0.33 (33%)
  quorum_max = 0.80 (80%)
```

---

## 5. Multi-Model Adaptation

### 5.1 Contexto

La adaptaciÃ³n cross-model permite usar SAEs entrenados en diferentes modelos base (Qwen, Llama, Mistral) dentro de la misma red federada.

### 5.2 State of the Art (2026)

| TÃ©cnica | DescripciÃ³n | Precision Loss | Overhead |
|---|---|---|---|
| **Linear Projection** | Matriz de proyecciÃ³n entre espacios | ~2-5% | ~10ms |
| **LoRA-style Adaptation** | Low-rank adaptation | ~1-3% | ~20ms |
| **Cross-Attention** | Attention entre espacios | ~0.5-2% | ~50ms |
| **Knowledge Distillation** | Transferir conocimiento entre modelos | ~1-2% | ~100ms |
| **Semantic Alignment** | Alinear por significado semÃ¡ntico | ~0.5-1% | ~30ms |

### 5.3 RecomendaciÃ³n

**Para v0.8.0-alpha**: Linear projection como baseline (simple, rÃ¡pido).  
**Para v0.9.0-rc**: LoRA-style adaptation si la precision loss es inaceptable.  
**Para v1.0.0**: Evaluar semantic alignment si se requiere mÃ¡xima precision.

**Referencias**:
- Hu et al. "LoRA: Low-Rank Adaptation of Large Language Models" (2021)
- Liu et al. "Cross-Attention for Multi-Model Integration" (2023)
- Sanh et al. "Knowledge Distillation: A Survey" (2019)

---

## 6. Continuous Alignment

### 6.1 Contexto

La alineaciÃ³n continua mantiene el modelo alineado con valores humanos a travÃ©s de feedback constante, no solo durante el entrenamiento inicial.

### 6.2 State of the Art (2026)

| Enfoque | DescripciÃ³n | Ventajas | Desventajas |
|---|---|---|---|
| **Online RLHF** | RLHF continuo con feedback en tiempo real | AdaptaciÃ³n continua | Complex, costoso |
| **Preference Learning** | Aprender preferencias de comparaciones | Simple, efectivo | Requiere muchos datos |
| **Constitutional AI** | Reglas constitucionales como guardrails | Interpretable | RÃ­gido |
| **Rejection Sampling** | Muestrear y rechazar outputs mal alineados | Simple | Ineficiente |
| **DPO (Direct Preference Optimization)** | Optimizar directamente con preferencias | Eficiente, simple | Requiere pares de preferencia |

### 6.3 RecomendaciÃ³n

**Para v0.9.0-rc**: DPO como mÃ©todo principal (eficiente, simple).  
**Para v1.0.0**: Online RLHF si se requiere adaptaciÃ³n en tiempo real.

**Pipeline propuesto**:
```
Continuous Alignment
ââââ Human Feedback (CLI/UI/API)
ââââ FeedbackStore (redb)
ââââ AlignmentFeedbackLoop (Phase 7)
ââââ DPO Training (offline, batch)
ââââ Model Update (hot swap)
ââââ Validation (drift check)
ââââ Rollback (si drift > threshold)
```

**Referencias**:
- Rafailov et al. "Direct Preference Optimization: Your Language Model is Secretly a Reward Model" (2023)
- Bai et al. "Constitutional AI: Harmlessness from AI Feedback" (2022)
- Christiano et al. "Deep Reinforcement Learning from Human Preferences" (2017)

---

## 7. Referencias Generales

### 7.1 Papers Fundamentales

1. McMahan et al. "Communication-Efficient Learning of Deep Networks from Decentralized Data" (FedAvg, 2017)
2. Abadi et al. "Deep Learning with Differential Privacy" (2016)
3. Yang et al. "Federated Machine Learning: Concept and Applications" (2019)
4. Shokri et al. "Membership Inference Attacks Against Machine Learning Models" (2017)
5. Carlini et al. "The Secret Sharer: Evaluating and Testing Unintended Memorization in Neural Networks" (2019)

### 7.2 Herramientas y Frameworks

| Herramienta | PropÃ³sito | Link |
|---|---|---|
| **Candle** | ML inference en Rust | https://github.com/huggingface/candle |
| **Wasmtime** | WASM runtime | https://wasmtime.dev/ |
| **libp2p** | P2P networking | https://libp2p.io/ |
| **redb** | Embedded DB en Rust | https://github.com/cberner/redb |
| **Axum** | Web framework en Rust | https://github.com/tokio-rs/axum |

### 7.3 EstÃ¡ndares y Protocolos

| EstÃ¡ndar | PropÃ³sito | VersiÃ³n |
|---|---|---|
| **OpenAPI 3.0** | API specification | 3.0.3 |
| **Ed25519** | Firmas digitales | RFC 8032 |
| **SHA-256** | Hashing | RFC 6234 |
| **Semantic Versioning** | Versionado | 2.0.0 |
| **JSONL** | Data export | IETF Draft |

---

## 8. Contactos

| Ãrea | Contacto | Responsabilidad |
|---|---|---|
| ZKP | `@ed2kia/zkp-team` | InvestigaciÃ³n e implementaciÃ³n ZKP |
| Federation | `@ed2kia/fed-team` | FederaciÃ³n asÃ­ncrona |
| UI/UX | `@ed2kia/ux-team` | Dashboard reactivo |
| Governance | `@ed2kia/governance-team` | Governance lÃ­quida |
| Alignment | `@ed2kia/alignment-team` | AlineaciÃ³n continua |

---

*Documento generado para Phase 8 Planning. PrÃ³xima revisiÃ³n: Sprint 1 kickoff.*
