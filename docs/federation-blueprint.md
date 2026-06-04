# Federation Blueprint â€” Interoperabilidad P2P & Escalado Federado

> **ed2kIA v2.1.0-sprint21** â€” Arquitectura de federaciÃ³n orgÃ¡nica, sin coordinaciÃ³n centralizada.

---

## ðŸŒ Arquitectura Cross-Mesh

La federaciÃ³n ed2kIA se basa en **mallas GossipSub independientes** que se peerean orgÃ¡nicamente a travÃ©s de `CrossMeshRouter`. Cada malla opera como una regiÃ³n autÃ³noma, sin dependencias jerÃ¡rquicas.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                  ed2kIA Federation v2.1                         â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                                 â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”         â”‚
â”‚  â”‚  Region A    â”‚  â”‚  Region B    â”‚  â”‚  Region C    â”‚         â”‚
â”‚  â”‚  (Americas)  â”‚  â”‚  (Europe)    â”‚  â”‚  (APAC)      â”‚         â”‚
â”‚  â”‚              â”‚  â”‚              â”‚  â”‚              â”‚         â”‚
â”‚  â”‚ GossipSub    â”‚  â”‚ GossipSub    â”‚  â”‚ GossipSub    â”‚         â”‚
â”‚  â”‚ + CRDTs      â”‚  â”‚ + CRDTs      â”‚  â”‚ + CRDTs      â”‚         â”‚
â”‚  â”‚ Port 3001    â”‚  â”‚ Port 3002    â”‚  â”‚ Port 3003    â”‚         â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜         â”‚
â”‚         â”‚                 â”‚                 â”‚                  â”‚
â”‚         â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                  â”‚
â”‚                   CrossMeshRouter                              â”‚
â”‚              (Deterministic Peering)                           â”‚
â”‚                                                                 â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”       â”‚
â”‚  â”‚              Region Sync Engine                     â”‚       â”‚
â”‚  â”‚  Delta-Encoding | Batch Merge | VersionVector       â”‚       â”‚
â”‚  â”‚  Latency: 50ms | 500ms | 2000ms (satellite)        â”‚       â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜       â”‚
â”‚                                                                 â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

## ðŸ”— Modelo de Peering

### Handshake Determinista

1. **Mesh A** envÃ­a `PeeringRequest { mesh_id, signature, capabilities }`
2. **Mesh B** valida firma criptogrÃ¡fica (`validate_signature`)
3. Si vÃ¡lida, crea `PeerLink` con rate-limiting y backoff exponencial
4. Peering activo â†’ relay de payloads `QLoRAPayload`, `SCTDecision`, `CRDTState`

### ValidaciÃ³n de Firmas

```rust
router.validate_signature("mesh-2", "sig-2")?;
```

- Firma SHA-256 del mesh_id + timestamp
- Rechazo determinista si firma invÃ¡lida (anti-Sybil hopping)
- Cero excepciones, cero backdoors

### Rate Limiting & Backoff

| ParÃ¡metro | Valor Default | DescripciÃ³n |
|-----------|--------------|-------------|
| `rate_limit` | 100 msgs/10s | MÃ¡ximo de mensajes por ventana |
| `backoff_base` | 100ms | Backoff exponencial base |
| `backoff_max` | 2^10 | Tope de backoff (aprox 102s) |
| `max_payload` | 1MB | TamaÃ±o mÃ¡ximo de payload |

## ðŸ”„ Estrategia de Sync Multi-RegiÃ³n

### Delta-Encoding

ReducciÃ³n de payload 60-80% mediante encoding diferencial:

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
- ResoluciÃ³n por `VersionVector` + timestamp determinista

### Latency Awareness

| Latencia | Comportamiento |
|----------|---------------|
| < 100ms | Sync inmediato, delta encoding |
| 100-500ms | Sync con debounce 500ms |
| 500-2000ms | Batch merge, compresiÃ³n agresiva |
| > 5000ms | Fallback a broadcast directo |

## ðŸ›¡ï¸ Threat Model

### Sybil Hopping

**Amenaza:** Nodo malicioso crea mÃºltiples identidades entre regiones.

**MitigaciÃ³n:**
- ValidaciÃ³n criptogrÃ¡fica de firmas de mesh
- Rate limiting por mesh (no por IP)
- CRDT max-registry (reputaciÃ³n no se puede inflar artificialmente)

### Partition Attacks

**Amenaza:** Atacador fuerza particiÃ³n prolongada entre regiones.

**MitigaciÃ³n:**
- CRDTs garantizan convergencia eventual sin coordinaciÃ³n
- Offline cache (`GossipCache`) almacena payloads hasta reconexiÃ³n
- Exponential backoff + auto-recovery en `PeerLink`

### Data Poisoning Cross-Mesh

**Amenaza:** Payload malicioso inyectado en una regiÃ³n se propaga a todas.

**MitigaciÃ³n:**
- SCT Guard (`Z < 0 â†’ REJECTED`) bloquea payloads no Ã©ticos
- BFT Aggregator filtra gradientes bizantinos
- Topological Filter detecta divergencia KL en activations

## ðŸ“‹ Runbook Operativo

### Bootstrap de FederaciÃ³n

```bash
# 1. Validar entorno
./scripts/federate-mesh.sh --dry-run

# 2. Ejecutar bootstrap (3 regiones)
./scripts/federate-mesh.sh

# 3. Verificar reporte
cat docs/federation-test-report-YYYYMMDD.md
```

### DiagnÃ³stico de Sync

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
# Detener federaciÃ³n
pkill -f orchestrator-node

# Limpiar estado
rm -rf /tmp/ed2kia-federation-*

# Re-ejecutar bootstrap
./scripts/federate-mesh.sh
```

## ðŸ”‘ ClÃ¡usula Ã‰tica

Esta federaciÃ³n opera bajo los principios del **Kernel Estuardiano**:

1. **Ley 1 (Diversidad):** Mallas independientes, peering orgÃ¡nico, cero jerarquÃ­a
2. **Ley 2 (Error):** ValidaciÃ³n de firmas, SCT Guard, BFT Aggregator
3. **Ley 3 (HolÃ­stica):** Delta-encoding, cero desperdicio computacional
4. **Ley 4 (Simbiosis):** Hardware modesto, conexiones inestables soportadas
5. **Ley 5 (MÃºltiples Posibilidades):** CRDTs, convergencia eventual, tolerancia a particiones

**Cero lÃ³gica financiera.** **Cero tokens.** **CeroProof of Work extractivo.**

La federaciÃ³n ed2kIA es infraestructura voluntaria global para la interpretabilidad Ã©tica de IA.

---

## ðŸ“š Referencias

- [Kernel Architecture](kernel-architecture.md)
- [GOVERNANCE.md](../GOVERNANCE.md)
- [CRDT Implementation](../src/async_gossip/crdt.rs)
- [Cross-Mesh Router](../src/network/cross_mesh.rs)
- [Region Sync Engine](../src/network/region_sync.rs)
- [Federation Bootstrap Script](../scripts/federate-mesh.sh)

---

*Blueprint generado para ed2kIA v2.1.0-sprint21 â€” FederaciÃ³n OrgÃ¡nica, Cero CentralizaciÃ³n*
