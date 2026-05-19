# Hacker News — Show HN Post

---

## Title

Show HN: ed2kIA — Audita LLMs desde tu navegador con Sparse Autoencoders P2P

## Body

Hola HN,

Construí **ed2kIA**: una red P2P en Rust que permite auditar modelos de IA (Qwen, Llama, etc.) directamente desde el navegador usando Sparse Autoencoders. Sin instalar nada, sin servidores centralizados, sin lógica financiera.

**El problema:** Los LLMs son cajas negras. Las organizaciones que los construyen controlan la narrativa sobre qué hacen y cómo deciden. La comunidad científica quiere entender estos sistemas, pero el acceso a herramientas de interpretabilidad está centralizado y requiere infraestructura costosa.

**La solución:** ed2kIA distribuye la carga de auditoría SAE entre nodos voluntarios (incluyendo navegadores web vía WASM). Cada nodo procesa fragmentos del modelo y devuelve activaciones sparse que se agregan en un grafo semántico global. El resultado: transparencia verificable, sin intermediarios.

**Stack técnico:**
- Rust + libp2p (GossipSub con firmas criptográficas)
- WASM browser node (wasm-pack + WebRTC relay)
- Sparse Autoencoders (Qwen-Scope, 4-tensor top-k architecture)
- Atlas Semántico 3D (WebGL force-graph visualizer)
- Micro-PoW para resistencia Sybil (~2s/nodo, cero staking)
- Feedback RLHF humano para corrección de alineación semántica

**Qué funciona hoy:**
- Nodo en el navegador que se conecta a la red P2P real
- Auditoría SAE distribuida con consenso determinista
- Visualización 3D del grafo semántico (tokens ↔ features)
- Sistema de reputación con slashing automático
- Feedback humano para corrección de sesgos semánticos

**Código:** https://github.com/Stuartemk/ed2kIA
**Demo:** [GitHub Pages URL pending deploy]

No es un token. No es una DAO. Es código abierto para hacer la IA más comprensible, uno a uno.

Feedback welcome. Issues y PRs bienvenidos.

---

## Notas para publicación

1. Ir a https://news.ycombinator.com/submitLink
2. Pegar título y URL
3. Esperar moderación (usualmente <1h)
4. Responder comentarios en las primeras 2h (crítico para visibilidad)
5. Mantener tono técnico y humilde
