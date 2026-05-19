# Reddit — r/machinelearning + r/rust Post

---

## Título

Las corporaciones cierran sus modelos. Construí una red P2P en Rust para auditar IA desde el navegador.

## Cuerpo

Hola r/machinelearning / r/rust,

Soy desarrollador open source y me frustraba algo: los modelos más potentes del planeta (GPT, Claude, Gemini) son cajas negras totales. Las empresas que los hacen controlan la narrativa sobre qué hacen y cómo deciden. La ciencia quiere entender estos sistemas, pero las herramientas de interpretabilidad están centralizadas y requieren GPUs costosas.

Así que construí **ed2kIA**: una red P2P en Rust que distribuye la auditoría de Sparse Autoencoders entre nodos voluntarios. Incluyendo tu navegador.

### ¿Qué hace?

- **Auditoría SAE descentralizada:** Tu navegador ejecuta fragmentos de Sparse Autoencoders (Qwen-Scope) vía WASM y devuelve activaciones sparse a la red
- **Grafo semántico global:** Las activaciones se agregan en un grafo 3D que mapea tokens ↔ features, visible en tiempo real
- **Consenso determinista:** Múltiples nodos auditan el mismo input; si hay mayoría, el resultado se acepta. Si no, se rechaza
- **Reputación con slashing:** Los nodos que devuelven resultados inconsistentes pierden reputación y eventualmente son baneados
- **Feedback humano:** Cualquier persona puede corregir activaciones erróneas vía UI interactiva (RLHF bridge)

### Stack técnico

- Rust + libp2p (GossipSub con firmas Ed25519)
- WASM browser node (wasm-pack + WebRTC relay)
- Sparse Autoencoders (Qwen-Scope, 4-tensor top-k)
- Atlas Semántico 3D (WebGL force-graph)
- Micro-PoW para resistencia Sybil (~2s/nodo, cero staking)
- 3038 tests, 0 warnings, código 100% auditable

### ¿Por qué importa?

Porque la interpretabilidad de IA no debería ser un privilegio de corporaciones con clusters GPU. Cualquier persona con un navegador puede auditar estos modelos y contribuir a hacerlos más comprensibles.

No es un token. No es una DAO. No hay lógica financiera. Es código abierto para hacer la IA más transparente.

**Código:** https://github.com/Stuartemk/ed2kIA
**Demo:** [GitHub Pages URL pending deploy]

Feedback welcome. Issues y PRs bienvenidos.

---

## Notas para publicación

### r/machinelearning
1. Ir a https://www.reddit.com/r/machinelearning/submit
2. Pegar título y cuerpo
3. Subreddit requiere título descriptivo y contenido técnico
4. Responder preguntas técnicas en las primeras 4h

### r/rust
1. Ir a https://www.reddit.com/r/rust/submit
2. Enfatizar arquitectura Rust, libp2p, WASM
3. Comunidad Rust valora código limpio y tests
4. Mencionar 3038 tests, 0 warnings, `#![forbid(unsafe_code)]`

### r/open_source
1. Ir a https://www.reddit.com/r/open_source/submit
2. Enfatizar transparencia, descentralización, acceso abierto
3. Comunidad valora proyectos sin lógica financiera
