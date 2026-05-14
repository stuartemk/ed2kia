# Gobernanza de ed2kIA

> Sistema de gobernanza descentralizada, ligera y transparente para la red ed2kIA.

## 📐 Principios Fundamentales

1. **Apertura:** Todas las propuestas, votos y decisiones son públicas y auditables.
2. **Meritocracia verificada:** La reputación se gana con cómputo verificado (ZKP), no con capital.
3. **Transparencia:** Código abierto, decisiones documentadas, ejecución automática.
4. **Ética:** Apache 2.0 + Cláusula de Uso Ético. Cero centralización, cero explotación.
5. **Ligereza:** Sin blockchain pesada. Solo firmas Ed25519 + hashes SHA-256 + redb local.

## 🏛️ Estructura de Gobernanza

### Componentes

| Componente | Archivo | Descripción |
|------------|---------|-------------|
| Propuestas | `src/governance/proposal.rs` | Creación, firma y validación de propuestas |
| Votación | `src/governance/voting.rs` | Votación P2P, time-lock, quórum, ejecución |
| Reputación | `src/reputation/scoring.rs` | Créditos por cómputo verificado, decay, anti-Sybil |
| Ledger | `src/reputation/ledger.rs` | Registro inmutable de contribuciones |

### Flujo de Gobernanza

```
┌─────────────────────────────────────────────────────────────────┐
│                    FLUJO DE GOBERNANZA ed2kIA                    │
└─────────────────────────────────────────────────────────────────┘

  1. Propuesta
     └── Nodo con reputación ≥ 0.7 crea propuesta firmada (Ed25519)
     └── Proposal { id, author, title, payload, signature }
     └── Propagada vía GossipSub a toda la red

  2. Votación (Time-lock: 72h)
     └── Nodos con reputación ≥ 0.7 pueden votar
     └── Voto: For / Against / Abstain
     └── Votos propagados vía GossipSub

  3. Resolución
     └── Quórum: ≥30% de nodos activos con reputación ≥0.7
     └── Aprobación: ≥51% de votos ponderados por reputación
     └── Si pasa → Approved → Ejecución automática
     └── Si falla → Rejected → Archivado

  4. Ejecución
     └── AutoExecutor ejecuta callback registrado
     └── Resultado registrado en ledger
     └── Estado → Executed
```

## 📝 Tipos de Propuesta

| Tipo | Descripción | Ejemplo |
|------|-------------|---------|
| `NetworkParam` | Cambios en parámetros de red | Ajustar mesh_n de GossipSub |
| `ModelUpdate` | Actualización de modelo SAE | Migrar a nueva versión Qwen-Scope |
| `ReputationPolicy` | Cambios en política de reputación | Ajustar decay period |
| `Security` | Parches o mejoras de seguridad | Actualizar libp2p, rotar claves |
| `Governance` | Cambios en el sistema de gobernanza | Modificar quórum, time-lock |
| `Ecosystem` | Integraciones externas | Agregar soporte ModelScope |
| `Custom` | Propuestas custom | Cualquier otra propuesta válida |

## 🗳️ Mecánica de Votación

### Parámetros

| Parámetro | Valor Default | Descripción |
|-----------|--------------|-------------|
| Time-lock | 72 horas | Duración del período de votación |
| Quórum | ≥30% | Porcentaje mínimo de nodos activos |
| Reputación mínima | ≥0.7 | Score mínimo para votar |
| Umbral aprobación | ≥51% | Mayoría ponderada por reputación |

### Cálculo de Resultado

```
quorum_required = ceil(active_nodes * 0.30)
total_valid_votes = votes_for + votes_against + votes_abstain

reputation_weight_for = sum(voter_reputation for each "For" vote)
reputation_weight_against = sum(voter_reputation for each "Against" vote)

approval_rate = reputation_weight_for / (reputation_weight_for + reputation_weight_against)

approved = (total_valid_votes >= quorum_required) AND (approval_rate >= 0.51)
```

## 📊 Reputación y Ponderación

### Sistema de Créditos

Los créditos se otorgan por contribuciones verificadas:

| Contribución | Créditos Base | Multiplicador ZKP |
|--------------|--------------|-------------------|
| SAE Forward | 10.0 | x1.5 |
| Consensus Batch | 15.0 | x1.5 |
| Human Feedback | 5.0 | x1.0 |
| Concept Learned | 8.0 | x1.0 |
| Governance Proposal | 20.0 | x1.0 |
| Governance Vote | 2.0 | x1.0 |
| Model Sync | 12.0 | x1.0 |

### Decay Exponencial

- **Período:** 50% cada 30 días
- **Fórmula:** `credits *= 2^(-elapsed_days / 30)`
- **Propósito:** Incentivar contribución continua

### Anti-Sybil

- **Límite:** 1000 créditos por IP/ASN cada 24 horas
- **Verificación:** Contribución única por batch (hash deduplicación)
- **Detección:** Tracking por dirección IP + ASN

## 🔐 Seguridad Criptográfica

### Firmas

- **Algoritmo:** Ed25519 (vía `ed25519-dalek`)
- **Uso:** Firma de propuestas, verificación de autoría
- **Generación:** `Proposal::generate_keypair()`

### Hashes

- **Algoritmo:** SHA-256 (vía `sha2`)
- **Uso:** Integridad de propuestas, cadena de ledger, deduplicación

### Almacenamiento

- **Base de datos:** redb (embedded key-value)
- **Ubicación:** `~/.ed2kIA/governance/`
- **Formato:** JSON serializado en tablas redb

## 📋 CLI de Gobernanza

```bash
# Crear propuesta firmada
cargo run -- govern --propose "Actualizar mesh_n a 8" --type NetworkParam

# Ver propuestas activas
cargo run -- govern --list

# Votar en propuesta
cargo run -- govern --vote <proposal_id> --direction for

# Ver resultado de votación
cargo run -- govern --result <proposal_id>

# Ver estadísticas de gobernanza
cargo run -- govern --stats
```

## 🔄 Ejecución Automática

Las propuestas aprobadas se ejecutan automáticamente vía `AutoExecutor`:

```rust
let mut executor = AutoExecutor::new();
executor.with_callback(|proposal| {
    // Ejecutar acción basada en proposal.payload
    match proposal.proposal_type {
        ProposalType::NetworkParam => update_network_params(&proposal.payload),
        ProposalType::ModelUpdate => download_and_activate_model(&proposal.payload),
        // ...
    }
    Ok(())
});
```

## 📜 Historial y Auditoría

- Todas las propuestas se almacenan permanentemente en redb.
- El ledger de reputación mantiene cadena inmutable de contribuciones.
- Export JSON disponible para auditoría externa.
- TODO: Phase 6 - Dashboard web de gobernanza.

## 🌍 Mandato Ético

Este sistema de gobernanza es:
- **Transparente:** Código abierto, decisiones públicas.
- **Equitativo:** Reputación por trabajo verificado, no por capital.
- **Resiliente:** Anti-Sybil, decay, quórum ponderado.
- **Ético:** Apache 2.0 + Cláusula de Uso Ético.

---

**ed2kIA** - Gobernanza descentralizada para el beneficio humano.
