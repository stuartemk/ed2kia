# Phase 7 Research Notes: Estado del Arte

> **Propósito**: Referencias técnicas y análisis del estado del arte para las decisiones arquitectónicas de Phase 7.
> **Audiencia**: Equipo técnico, revisores de código, comunidad de investigación.
> **Última actualización**: 2026-05-04

---

## 1. FedAvg Adaptativo y Agregación Robusta

### 1.1 FedAvg Clásico (McMahan et al., 2017)

**Referencia**: "Communication-Efficient Learning of Deep Networks from Decentralized Data" — AISTATS 2017

**Concepto**: Los clientes entrenan localmente y envían deltas de pesos al servidor, que promedia y distribuye el modelo global.

**Limitaciones para ed2kIA:**
- Asume clientes honestos (sin tolerancia a Byzantine)
- Convergencia lenta con datos no-IID (común en SAEs especializados)
- No considera heterogeneidad de recursos

**Implementación actual**: `src/federation/avg_aggregator.rs` aplica FedAvg básico con Krum filter.

---

### 1.2 Krum y Multi-Krum (Lynagh et al., 2019)

**Referencia**: "Byzantine-Robust Learning with Healing" — ICML 2019

**Concepto**: Seleccionar los `f` updates más cercanos (en distancia L2) de `f + 2` candidatos, donde `f` es el número máximo de nodos Byzantine.

**Ventajas:**
- Tolerancia a hasta `f < n/3` nodos Byzantine
- Simple de implementar y verificar

**Limitaciones:**
- No funciona bien con datos altamente no-IID
- Multi-Krum (seleccionar múltiples updates) mejora convergencia pero aumenta complejidad

**Implementación actual**: `avg_aggregator.rs::apply_krum_filter()` usa Krum básico.

---

### 1.3 FedAvg Adaptativo — Direcciones de Investigación

#### 1.3.1 Adaptive Client Sampling
**Idea**: Muestrear clientes proporcional a su calidad de datos y capacidad computacional.

**Referencia**: "Adaptive Federated Optimization" — ICLR 2020 (FedAdapt)

**Aplicación a ed2kIA:**
- Usar `NodeScore` de `sae/router.rs` para ponderar participación
- Priorizar nodos con datos más diversos (mayor entropía de activaciones)
- Penalizar nodos con alta tasa de timeout

#### 1.3.2 Personalized FedAvg (pFedAvg)
**Idea**: Cada nodo mantiene capas personalizadas que no se sincronizan, permitiendo especialización local.

**Referencia**: "Personalized Federated Learning with Moreau Envelopes" — ICLR 2021

**Aplicación a ed2kIA:**
- Capas de normalización personalizadas por nodo
- Capas de shared representation sincronizadas globalmente
- Balance entre personalización y generalización

#### 1.3.3 Heterogeneous FedAvg (HeteroFL)
**Idea**: Soportar modelos de diferentes tamaños en la misma federación.

**Referencia**: "HeteroFL: Heterogeneous Federated Learning on the Edge" — NeurIPS 2020

**Aplicación a ed2kIA:**
- Nodos con GPU ejecutan SAEs completos
- Nodos con CPU ejecutan SAEs reducidos (knowledge distillation)
- Agregación por capas compartidas

---

## 2. Alineación Dinámica (Dynamic Alignment)

### 2.1 RLHF Tradicional (Christiano et al., 2017)

**Referencia**: "Deep Reinforcement Learning from Human Preferences" — NeurIPS 2017

**Pipeline**:
1. Collect human preferences (comparative judgments)
2. Train reward model from preferences
3. Optimize policy via PPO against reward model

**Limitaciones para ed2kIA:**
- Offline: requiere recolección batch de preferencias
- Costoso: requiere anotadores humanos dedicados
- No adapta en tiempo real

---

### 2.2 Constitutional AI (Bai et al., 2022)

**Referencia**: "Constitutional AI: Harmlessness from AI Feedback" — NeurIPS 2022

**Concepto**: En lugar de preferencias humanas directas, usar un "constitución" (conjunto de principios) para generar auto-crítica y auto-corrección.

**Ventajas:**
- Reduce dependencia de anotadores humanos
- Escala mejor que RLHF puro
- Transparente (la constitución es legible)

**Aplicación a ed2kIA:**
- Constitución como conjunto de reglas en `value_guard.rs`
- Auto-crítica basada en activaciones SAE (features que indican desalineación)
- Feedback humano como override, no como input primario

---

### 2.3 RLHF Continuo — Propuesta para ed2kIA

**Concepto**: Integrar RLHF en el ciclo de inferencia, no como fase de entrenamiento separada.

**Arquitectura Propuesta:**
```
Inference → SAE activations → Value Guard → [Aligned? → Output]
                                              ↓ No
                                          Steering Signal → Corrected Output
                                              ↓
                                          Feedback Store → Reward Model Update
```

**Componentes:**
1. **Value Guard** (`src/alignment/value_guard.rs`): Monitorea activaciones SAE para detectar desviaciones
2. **Steering Engine** (`src/alignment/steering_engine.rs`): Genera señales de corrección en tiempo real
3. **Reward Model** (`src/alignment/reward_model.rs`): Modelo ligero entrenado con feedback histórico
4. **Feedback Loop** (`src/alignment/feedback_loop.rs`): Integra feedback humano y automático

**Métricas Clave:**
- Latencia de detección: <100ms
- Latencia de corrección: <200ms
- Precision de detección: ≥95%
- False positive rate: <2%

---

## 3. ZKP Escalable para Verificación de Modelos

### 3.1 ZKP Actual en ed2kIA

**Implementación**: `src/zkp/circuit.rs` usa Pedersen commitments + Fiat-Shamir heuristic.

**Capacidades:**
- Verificar que un batch de features fue procesado correctamente
- Generar proofs de inclusión Merkle
- VRF (Verifiable Random Function) para selección de validadores

**Limitaciones:**
- Proofs relativamente grandes (~1KB)
- Verificación O(n) en tamaño del batch
- No composicional (no se pueden combinar proofs)

---

### 3.2 PLONK (Gabizon et al., 2019)

**Referencia**: "PLONK: Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge" — EUROCRYPT 2019

**Ventajas:**
- Universal SNARK: setup único para todos los circuitos
- Proofs pequeños (~2KB independientemente del circuito)
- Verificación rápida (~1ms)

**Desventajas:**
- Requiere trusted setup (con ceremony)
- Overhead de compilación de circuitos

**Aplicación a ed2kIA:**
- Reemplazar circuitos ad-hoc con PLONK para verificación de SAE forward passes
- Setup único para la red (ceremony distribuida)
- Proofs verificables por cualquier nodo

---

### 3.3 Halo2 (Privacy Scaling Solutions, 2021)

**Referencia**: "Halo2: Proof System" — PSS Whitepaper 2021

**Ventajas:**
- Sin trusted setup (transparent setup)
- Circuit programming flexible (custom gates)
- Ecosistema Rust maduro

**Desventajas:**
- Proofs más grandes que PLONK (~5-10KB)
- Verificación más lenta que PLONK

**Aplicación a ed2kIA:**
- Ideal para proofs de staking (sin trusted setup)
- Custom gates para verificación de activaciones SAE
- Integración nativa con Rust

---

### 3.4 Recomendación para Phase 7

| Escenario | ZKP Recomendado | Razón |
|---|---|---|
| Verificación de SAE forward | Halo2 | Transparent setup, flexibilidad |
| Staking proofs | Halo2 | Sin trusted setup |
| Cross-net verification | PLONK | Proofs pequeños, verificación rápida |
| Merkle inclusion | Actual (Fiat-Shamir) | Suficiente, bajo overhead |

**Plan de Migración:**
1. **Sprint 7.1**: Evaluar Halo2 con circuitos simples
2. **Sprint 7.2**: Implementar Halo2 para staking proofs
3. **Sprint 7.3**: Evaluar PLONK para cross-net verification
4. **Sprint 7.4**: Decisión final basada en benchmarks

---

## 4. Gobernanza Líquida y Delegación Ponderada

### 4.1 Gobernanza Líquida (Liquid Democracy)

**Concepto**: Los participantes pueden votar directamente o delegar su voto a otros, por tema o globalmente.

**Ventajas:**
- Combina democracia directa (participación) con representativa (eficiencia)
- Delegación flexible (por tema, temporal, revocable)
- Reduce apatía (delegar es más fácil que votar en todo)

**Referencia**: "Liquid Democracy — Delegating My Vote to You" — Various (concepto político establecido)

**Implementaciones existentes:**
- **Snapshot.org**: Plataforma de gobernanza DeFi con delegación
- **Pol.is**: Plataforma de deliberación con clustering de opiniones
- **Tezos**: Gobernanza on-chain con delegación

---

### 4.2 Delegación Ponderada por Stake y Reputación

**Concepto**: El peso de voto de un nodo es función de su stake y reputación, no 1-nodo-1-voto.

**Fórmula Propuesta:**
```
vote_weight = stake_weight * reputation_multiplier * time_factor

stake_weight = log(1 + stake_amount)  // Log para evitar dominación
reputation_multiplier = 1 + (reputation_score - 0.5)  // [0.5, 1.5]
time_factor = min(1, node_age / 365)  // Mature nodes get more weight
```

**Protecciones:**
- **Sybil resistance**: Stake requerido para participar
- **Whale resistance**: Log scaling del stake weight
- **New node inclusion**: Time factor crece gradualmente
- **Delegation cycles**: Detectar y romper ciclos de delegación mutua

---

### 4.3 Quórum Dinámico

**Concepto**: El quórum requerido varía basado en la importancia de la propuesta y la participación actual.

**Fórmula Propuesta:**
```
required_quorum = base_quorum * importance_factor / participation_factor

base_quorum = 0.5  // 50% mínimo
importance_factor = 1 + (proposal_impact / max_impact)  // [1, 2]
participation_factor = min(1, active_participants / total_nodes)  // [0, 1]
```

**Efectos:**
- Propuestas de alto impacto requieren más participación
- Si la participación es baja, el quórum se ajusta (pero no por debajo del mínimo)
- Incentiva la participación activa

---

### 4.4 Aplicación a ed2kIA

**Arquitectura:**
```
governance/
├── liquid_democracy.rs    # Delegación y votación líquida
├── auto_proposal.rs       # Propuestas automáticas basadas en métricas
├── quorum_engine.rs       # Cálculo de quórum dinámico
├── delegation_graph.rs    # Gestión del grafo de delegación
└── execution_engine.rs    # Ejecución de propuestas aprobadas
```

**Flujo de Propuesta:**
1. Propuesta creada (manual o automática)
2. Impacto evaluado → quórum calculado
3. Período de votación (con delegación activa)
4. Resultados calculados (con weights)
5. Si quórum alcanzado y mayoría → ejecución automática
6. Si no → propuesta archivada con razón

---

## 5. Referencias Adicionales

### Papers Fundamentales
1. McMahan et al. (2017) — FedAvg
2. Christiano et al. (2017) — RLHF
3. Bai et al. (2022) — Constitutional AI
4. Gabizon et al. (2019) — PLONK
5. PSS (2021) — Halo2
6. Lyngvig et al. (2019) — Krum/Byzantine FL

### Herramientas y Bibliotecas
- **Halo2**: https://github.com/privacy-scaling-explorations/halo2
- **PLONK**: https://github.com/AztecProtocol/aztec-packages
- **candle-core**: https://github.com/huggingface/candle
- **libp2p**: https://libp2p.io/

### Proyectos Relacionados
- **MLCommons**: Benchmarks para federated learning
- **OpenMined**: Privacy-preserving ML
- **FATE**: Federated learning framework (web3)
- **Bittensor**: Red de IA descentralizada con staking

---

*Estas notas de investigación son un documento vivo. Se actualizarán con nuevos hallazgos y referencias a medida que avance Phase 7.*
