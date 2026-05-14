# Arquitectura ed2kIA v1.2.0

## Vista General

ed2kIA v1.2.0 es una red descentralizada de interpretabilidad para LLMs usando Sparse Autoencoders (SAEs). La arquitectura se organiza en capas modulares con feature flags para control de despliegue.

## Capas Arquitectónicas

```
┌─────────────────────────────────────────────────────────┐
│                   Capa de Aplicación                     │
│  Web Dashboard (SSE/WS) │ CLI │ API REST               │
├─────────────────────────────────────────────────────────┤
│                   Capa de Negocio                        │
│  Marketplace v3 │ Alignment v3 │ Federation v3          │
│  Governance v3  │ SLO/SLA v3   │ Staking                │
├─────────────────────────────────────────────────────────┤
│                   Capa de Consenso                       │
│  Cross-Chain Consensus │ DAO Governance │ ZKP Proofs    │
├─────────────────────────────────────────────────────────┤
│                   Capa de Red                            │
│  libp2p (KAD + mDNS) │ P2P Sharding │ Trust Sync       │
├─────────────────────────────────────────────────────────┤
│                   Capa de ML                             │
│  candle-core │ candle-nn │ SAE Engine │ Gradient Sync   │
├─────────────────────────────────────────────────────────┤
│                   Capa de Almacenamiento                 │
│  redb (Escrow) │prost (Serialization) │ FlatBuffers     │
└─────────────────────────────────────────────────────────┘
```

## Módulos Principales v1.2.0

### Marketplace v3 (`src/marketplace/`)
- **marketplace_v3.rs:** Matching descentralizado con scoring ponderado
- **cross_chain_settlement.rs:** Liquidación multi-chain con compromisos ZKP
- **reputation_matcher.rs:** Matching por reputación criptográfica
- **escrow_ledger.rs:** Ledger inmutable en `redb` con firmas ed25519

### Alignment v3 (`src/alignment/`)
- **loop_v3.rs:** Loop de alineación con verificación ZKP
- **steering_verifier.rs:** Verificación de señales con ZKP + integridad
- **bias_mitigator.rs:** Detección y mitigación de sesgos

### Federation v3 (`src/federation/`)
- **scaling_v3.rs:** Escalado adaptativo de federación
- **adaptive_sharder.rs:** Particionamiento adaptativo con balanceo dinámico
- **gradient_sync_v3.rs:** Sync tolerante a partición ≥99.5%
- **gradient_aggregator_v3.rs:** Agregación FedAvg con compresión

## Feature Flags

| Flag | Descripción | Incluye |
|------|-------------|---------|
| `stable` | Production baseline | Todos los módulos validados v1.0-v1.2 |
| `v1.2-sprint4` | Sprint 4 modules | marketplace_v3, alignment_v3, federation_scaling_v3 |

## Principios de Diseño

1. **Modularidad:** Cada módulo es independiente con APIs bien definidas
2. **Zero Trust:** Verificación criptográfica en todas las capas
3. **Sin Telemetría:** Cero datos salientes, código auditable
4. **Eficiencia Energética:** Algoritmos optimizados para bajo consumo
5. **Gobernanza Meritocrática:** Reputación técnica como único incentivo

## Decisiones Técnicas

| Decisión | Rationale |
|----------|-----------|
| `redb` para escrow | Embedded DB ligera, sin dependencias externas |
| `ark-bn254` para ZKP | Curva estándar, bien auditada |
| `ed25519-dalek` para firmas | Firmas rápidas, seguras, determinísticas |
| `candle-core` para ML | Rust-native, sin dependencias Python |
| `libp2p` para red | Estándar P2P, multi-transport |

## Métricas de Rendimiento

| Métrica | v1.1.0 | v1.2.0 | Mejora |
|---------|--------|--------|--------|
| Tests E2E | 12 | 15 | +25% |
| Tests Stress | 15 | 19 | +27% |
| Clippy Warnings | 3 | 0 | -100% |
| Feature Flags | 15+ | 1 (stable) | Simplificado |
| Módulos Federation | 6 | 10 | +67% |
