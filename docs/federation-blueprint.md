# Federation Blueprint — Interoperabilidad P2P & Escalado Federado

> **ed2kIA v2.1.0-sprint21** — Arquitectura de federación orgánica, sin coordinación centralizada.

---

## 🌐 Arquitectura Cross-Mesh

La federación ed2kIA se basa en **mallas GossipSub independientes** que se peerean orgánicamente a través de `CrossMeshRouter`. Cada malla opera como una región autónoma, sin dependencias jerárquicas.

```
┌─────────────────────────────────────────────────────────────────┐
│                  ed2kIA Federation v2.1                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │  Region A    │  │  Region B    │  │  Region C    │         │
│  │  (Americas)  │  │  (Europe)    │  │  (APAC)      │         │
│  │              │  │              │  │              │         │
│  │ GossipSub    │  │ GossipSub    │  │ GossipSub    │         │
│  │ + CRDTs      │  │ + CRDTs      │  │ + CRDTs      │         │
│  │ Port 3001    │  │ Port 3002    │  │ Port 3003    │         │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘         │
│         │                 │                 │                  │
│         └─────────────────┼─────────────────┘                  │
│                   CrossMeshRouter                              │
│              (Deterministic Peering)                           │
│                                                                 │
│  ┌─────────────────────────────────────────────────────┐       │
│  │              Region Sync Engine                     │       │
│  │  Delta-Encoding | Batch Merge | VersionVector       │       │
│  │  Latency: 50ms | 500ms | 2000ms (satellite)        │       │
│  └─────────────────────────────────────────────────────┘       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 🔗 Modelo de Peering

### Handshake Determinista

1. **Mesh A** envía `PeeringRequest { mesh_id, signature, capabilities }`
2. **Mesh B** valida firma criptográfica (`validate_signature`)
3. Si válida, crea `PeerLink` con rate-limiting y backoff exponencial
4. Peering activo → relay de payloads `QLoRAPayload`, `SCTDecision`, `CRDTState`

### Validación de Firmas

```rust
router.validate_signature("mesh-2", "sig-2")?;
```

- Firma SHA-256 del mesh_id + timestamp
- Rechazo determinista si firma inválida (anti-Sybil hopping)
- Cero excepciones, cero backdoors

### Rate Limiting & Backoff

| Parámetro | Valor Default | Descripción |
|-----------|--------------|-------------|
| `rate_limit` | 100 msgs/10s | Máximo de mensajes por ventana |
| `backoff_base` | 100ms | Backoff exponencial base |
| `backoff_max` | 2^10 | Tope de backoff (aprox 102s) |
| `max_payload` | 1MB | Tamaño máximo de payload |

## 🔄 Estrategia de Sync Multi-Región

### Delta-Encoding

Reducción de payload 60-80% mediante encoding diferencial:

```
DeltaEntry {
    node_id: "node-1",
    new_value: 0.7,
    previous_value: 0.5,
    delta: 0.2,
    version: 3,
    timestamp: 1716300000000,
}
```

### Batch Merge

- `max_batch_size`: 1000 entries por sync
- Merge idempotente: `merge(a, b) == merge(b, a)`
- Resolución por `VersionVector` + timestamp determinista

### Latency Awareness

| Latencia | Comportamiento |
|----------|---------------|
| < 100ms | Sync inmediato, delta encoding |
| 100-500ms | Sync con debounce 500ms |
| 500-2000ms | Batch merge, compresión agresiva |
| > 5000ms | Fallback a broadcast directo |

## 🛡️ Threat Model

### Sybil Hopping

**Amenaza:** Nodo malicioso crea múltiples identidades entre regiones.

**Mitigación:**
- Validación criptográfica de firmas de mesh
- Rate limiting por mesh (no por IP)
- CRDT max-registry (reputación no se puede inflar artificialmente)

### Partition Attacks

**Amenaza:** Atacador fuerza partición prolongada entre regiones.

**Mitigación:**
- CRDTs garantizan convergencia eventual sin coordinación
- Offline cache (`GossipCache`) almacena payloads hasta reconexión
- Exponential backoff + auto-recovery en `PeerLink`

### Data Poisoning Cross-Mesh

**Amenaza:** Payload malicioso inyectado en una región se propaga a todas.

**Mitigación:**
- SCT Guard (`Z < 0 → REJECTED`) bloquea payloads no éticos
- BFT Aggregator filtra gradientes bizantinos
- Stuartian Filter detecta divergencia KL en activations

## 📋 Runbook Operativo

### Bootstrap de Federación

```bash
# 1. Validar entorno
./scripts/federate-mesh.sh --dry-run

# 2. Ejecutar bootstrap (3 regiones)
./scripts/federate-mesh.sh

# 3. Verificar reporte
cat docs/federation-test-report-YYYYMMDD.md
```

### Diagnóstico de Sync

```bash
# Verificar estado de mallas
curl http://localhost:3001/api/health
curl http://localhost:3002/api/health
curl http://localhost:3003/api/health

# Verificar CRDT convergence
curl http://localhost:3001/api/crdt/state
curl http://localhost:3002/api/crdt/state
curl http://localhost:3003/api/crdt/state

# Comparar reputaciones (deben converger)
diff <(curl -s http://localhost:3001/api/crdt/state | jq -S .) \
     <(curl -s http://localhost:3002/api/crdt/state | jq -S .)
```

### Rollback

```bash
# Detener federación
pkill -f orchestrator-node

# Limpiar estado
rm -rf /tmp/ed2kia-federation-*

# Re-ejecutar bootstrap
./scripts/federate-mesh.sh
```

## 🔑 Cláusula Ética

Esta federación opera bajo los principios del **Kernel Estuardiano**:

1. **Ley 1 (Diversidad):** Mallas independientes, peering orgánico, cero jerarquía
2. **Ley 2 (Error):** Validación de firmas, SCT Guard, BFT Aggregator
3. **Ley 3 (Holística):** Delta-encoding, cero desperdicio computacional
4. **Ley 4 (Simbiosis):** Hardware modesto, conexiones inestables soportadas
5. **Ley 5 (Múltiples Posibilidades):** CRDTs, convergencia eventual, tolerancia a particiones

**Cero lógica financiera.** **Cero tokens.** **CeroProof of Work extractivo.**

La federación ed2kIA es infraestructura voluntaria global para la interpretabilidad ética de IA.

---

## 📚 Referencias

- [Kernel Architecture](kernel-architecture.md)
- [GOVERNANCE.md](../GOVERNANCE.md)
- [CRDT Implementation](../src/async_gossip/crdt.rs)
- [Cross-Mesh Router](../src/network/cross_mesh.rs)
- [Region Sync Engine](../src/network/region_sync.rs)
- [Federation Bootstrap Script](../scripts/federate-mesh.sh)

---

*Blueprint generado para ed2kIA v2.1.0-sprint21 — Federación Orgánica, Cero Centralización*
