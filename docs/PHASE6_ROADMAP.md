# ed2kIA Phase 6 Roadmap

> **Especificación técnica: Interoperabilidad cross-model, Federated Alignment, Staking de Recursos y API Pública.**

---

## 📋 Tabla de Contenidos

1. [Visión de Fase 6](#-visión-de-fase-6)
2. [Interoperabilidad Cross-Model](#-interoperabilidad-cross-model)
3. [Federated Alignment](#-federated-alignment)
4. [Staking de Recursos](#-staking-de-recursos)
5. [API Pública](#-api-pública)
6. [Timeline y Hitos](#-timeline-y-hitos)
7. [Riesgos y Mitigación](#-riesgos-y-mitigación)
8. [Dependencias Técnicas](#-dependencias-técnicas)

---

## 🎯 Visión de Fase 6

Fase 6 transforma ed2kIA de una red de interpretación SAE a una **plataforma federada de alineación de IA** capaz de:

1. **Interoperar** con múltiples modelos LLM (Qwen, Llama, Mistral, etc.)
2. **Alinear** modelos de forma federada usando feedback humano distribuido
3. **Incentivar** contribuciones mediante staking de recursos computacionales
4. **Exponer** capacidades mediante API pública para desarrolladores

### Objetivos Cuantificables

| Objetivo | Métrica | Target |
|----------|---------|--------|
| Modelos soportados | Count | 5+ |
| Throughput API | req/s | 1000+ |
| Latencia p95 | ms | < 200 |
| Nodos federados | Count | 100+ |
| Uptime | % | 99.9% |

---

## 🔗 Interoperabilidad Cross-Model

### Arquitectura

```
┌─────────────────────────────────────────────────────────────┐
│              Cross-Model Interoperability Layer              │
└─────────────────────────────────────────────────────────────┘

┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   Qwen-SAE   │  │  Llama-SAE   │  │ Mistral-SAE  │
│   (7B)       │  │   (8B)       │  │   (7B)       │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                  │
       └─────────────────┼──────────────────┘
                         │ Model Adapter
                    ┌────▼─────┐
                    │  Unified │
                    │  SAE API │
                    └────┬─────┘
                         │
                    ┌────▼─────┐
                    │ Feature  │
                    │ Normalizer│
                    └────┬─────┘
                         │
                    ┌────▼─────┐
                    │  P2P     │
                    │ Network  │
                    └──────────┘
```

### Especificación: Model Adapter

```rust
/// Adapter trait para normalizar diferentes modelos SAE
pub trait ModelAdapter: Send + Sync {
    /// Identificador del modelo (e.g., "qwen2-7b-sae")
    fn model_id(&self) -> &str;

    /// Dimensionalidad del hidden state
    fn hidden_dim(&self) -> usize;

    /// Número de capas
    fn num_layers(&self) -> usize;

    /// Forward pass normalizado
    async fn forward(
        &self,
        hidden_state: &[f32],
        layer_id: u32,
    ) -> Result<NormalizedFeatures, AdapterError>;

    /// Validar compatibilidad de weights
    fn validate_weights(&self, path: &Path) -> Result<(), AdapterError>;
}

/// Features normalizadas para interoperabilidad
pub struct NormalizedFeatures {
    /// Features sparse normalizadas
    pub features: Vec<SparseFeature>,
    /// Modelo de origen
    pub source_model: String,
    /// Layer de origen
    pub source_layer: u32,
    /// Timestamp
    pub timestamp: u64,
}
```

### Modelos Soportados (Fase 6)

| Modelo | SAE Provider | Hidden Dim | Status |
|--------|-------------|------------|--------|
| Qwen2-7B | Qwen-Scope | 3584 | ✅ Fase 1-5 |
| Llama-3-8B | Anthropic SAE | 4096 | 🔄 Fase 6 |
| Mistral-7B | TransformerLens | 4096 | 🔄 Fase 6 |
| Gemma-7B | Google SAE | 3072 | 📋 Fase 6 |
| Mixtral-8x7B | Multi-head SAE | 4096 | 📋 Fase 6 |

---

## 🤝 Federated Alignment

### Arquitectura

```
┌─────────────────────────────────────────────────────────────┐
│              Federated Alignment Pipeline                    │
└─────────────────────────────────────────────────────────────┘

┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  Human Node  │  │  Human Node  │  │  Human Node  │
│   (Region A) │  │  (Region B)  │  │  (Region C)  │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │ Feedback        │ Feedback        │ Feedback
       └─────────────────┼──────────────────┘
                         │ Aggregation
                    ┌────▼─────┐
                    │ Federated│
                    │  Averager│
                    └────┬─────┘
                         │ Gradient Update
                    ┌────▼─────┐
                    │  SAE     │
                    │  Weights │
                    └────┬─────┘
                         │ Distribution
                    ┌────▼─────┐
                    │  P2P     │
                    │ Network  │
                    └──────────┘
```

### Especificación: Federated Averager

```rust
/// Agregador federado para updates de weights SAE
pub struct FederatedAverager {
    /// Participantes en el round actual
    participants: HashMap<String, WeightUpdate>,
    /// Round actual
    current_round: u64,
    /// Mínimo de participantes para agregar
    min_participants: usize,
    /// Security: detección de poisoning
    poison_detector: PoisonDetector,
}

impl FederatedAverager {
    /// Recibir update de un participante
    pub async fn receive_update(
        &mut self,
        node_id: String,
        update: WeightUpdate,
    ) -> Result<(), FederatedError> {
        // Verificar integridad del update
        self.poison_detector.validate(&update)?;

        // Almacenar
        self.participants.insert(node_id, update);

        // Si hay suficientes participantes, agregar
        if self.participants.len() >= self.min_participants {
            self.aggregate_and_distribute().await
        } else {
            Ok(())
        }
    }

    /// Agregar updates usando FedAvg
    async fn aggregate_and_distribute(&mut self) -> Result<AggregatedUpdate, FederatedError> {
        let updates: Vec<_> = self.participants.values().collect();

        // FedAvg: weighted average por cantidad de samples
        let total_samples: usize = updates.iter()
            .map(|u| u.sample_count)
            .sum();

        let aggregated = updates.iter()
            .map(|u| {
                let weight = u.sample_count as f32 / total_samples as f32;
                &u.delta * weight
            })
            .sum();

        Ok(AggregatedUpdate {
            round: self.current_round,
            delta: aggregated,
            participant_count: updates.len(),
            total_samples,
        })
    }
}
```

### Seguridad: Detección de Poisoning

```rust
/// Detector de gradient poisoning
pub struct PoisonDetector {
    /// Umbral de desviación (std dev)
    threshold_std: f64,
    /// Historial de updates por nodo
    history: HashMap<String, Vec<WeightUpdate>>,
}

impl PoisonDetector {
    /// Validar update contra poisoning
    pub fn validate(&mut self, update: &WeightUpdate) -> Result<(), PoisonError> {
        // Krum-like filtering
        let distances = self.compute_distances(update);
        let krum_score = self.krum_score(&distances);

        if krum_score > self.threshold_std {
            return Err(PoisonError::SuspiciousUpdate {
                node_id: update.node_id.clone(),
                score: krum_score,
            });
        }

        Ok(())
    }
}
```

---

## 💰 Staking de Recursos

### Arquitectura

```
┌─────────────────────────────────────────────────────────────┐
│              Resource Staking System                         │
└─────────────────────────────────────────────────────────────┘

┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   Node A     │  │   Node B     │  │   Node C     │
│ Stake: 1000  │  │ Stake: 500   │  │ Stake: 2000  │
│ CPU: 16 core │  │ CPU: 8 core  │  │ CPU: 32 core │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                  │
       └─────────────────┼──────────────────┘
                         │ Staking Registry
                    ┌────▼─────┐
                    │  Smart   │
                    │ Contract │  (on-chain o ligera)
                    └────┬─────┘
                         │ Rewards
                    ┌────▼─────┐
                    │ Reward   │
                    │ Distributor│
                    └──────────┘
```

### Especificación: Staking Registry

```rust
/// Registro de staking de recursos
pub struct StakingRegistry {
    /// Stakes activos
    stakes: HashMap<String, ResourceStake>,
    /// Total stake en la red
    total_stake: u64,
    /// Mínimo para participar
    minimum_stake: u64,
}

/// Stake de recursos de un nodo
pub struct ResourceStake {
    /// ID del nodo
    pub node_id: String,
    /// Cantidad stakeada
    pub amount: u64,
    /// Recursos comprometidos
    pub resources: CommittedResources,
    /// Timestamp de stake
    pub staked_at: u64,
    /// Epoch actual
    pub epoch: u64,
}

/// Recursos comprometidos
pub struct CommittedResources {
    /// Núcleos de CPU
    pub cpu_cores: usize,
    /// Memoria RAM (bytes)
    pub ram_bytes: u64,
    /// Almacenamiento (bytes)
    pub storage_bytes: u64,
    /// Ancho de banda (Mbps)
    pub bandwidth_mbps: u64,
}

impl StakingRegistry {
    /// Stakear recursos
    pub fn stake(
        &mut self,
        node_id: String,
        amount: u64,
        resources: CommittedResources,
    ) -> Result<(), StakingError> {
        if amount < self.minimum_stake {
            return Err(StakingError::InsufficientStake {
                provided: amount,
                required: self.minimum_stake,
            });
        }

        self.stakes.insert(node_id.clone(), ResourceStake {
            node_id,
            amount,
            resources,
            staked_at: current_timestamp(),
            epoch: self.current_epoch(),
        });

        self.total_stake += amount;
        Ok(())
    }

    /// Calcular rewards para un nodo
    pub fn calculate_rewards(
        &self,
        node_id: &str,
        epoch_contributions: u64,
    ) -> Result<u64, StakingError> {
        let stake = self.stakes.get(node_id)
            .ok_or(StakingError::NoStakeFound)?;

        // Reward proporcional al stake y contribuciones
        let stake_ratio = stake.amount as f64 / self.total_stake as f64;
        let base_reward = 10000; // Tokens por epoch
        let contribution_multiplier = 1.0 + (epoch_contributions as f64 * 0.01);

        Ok((base_reward as f64 * stake_ratio * contribution_multiplier) as u64)
    }

    /// Slash por mala conducta
    pub fn slash(&mut self, node_id: &str, percentage: f64) -> Result<u64, StakingError> {
        let stake = self.stakes.get_mut(node_id)
            .ok_or(StakingError::NoStakeFound)?;

        let slash_amount = (stake.amount as f64 * percentage) as u64;
        stake.amount -= slash_amount;
        self.total_stake -= slash_amount;

        if stake.amount < self.minimum_stake {
            self.stakes.remove(node_id);
        }

        Ok(slash_amount)
    }
}
```

### Modelo de Rewards

| Contribución | Base Reward | Multiplicador |
|-------------|-------------|---------------|
| SAE Forward | 10 tokens | 1.0x |
| ZKP Verified | 10 tokens | 1.5x |
| Anomaly Detection | 5 tokens | 1.2x |
| Human Feedback | 20 tokens | 2.0x |
| Seed Node | 50 tokens/epoch | 1.0x |

### Slashing Conditions

| Infracción | Slash % | Acción |
|------------|---------|--------|
| Double voting | 10% | Warning |
| Malicious batch | 25% | Temp ban |
| Sybil detection | 50% | Permanent ban |
| Downtime > 24h | 5% | Auto-unstake |

---

## 🌐 API Pública

### Arquitectura

```
┌─────────────────────────────────────────────────────────────┐
│                    Public API Gateway                        │
└─────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │   API Gateway    │
                    │   (Rate Limited) │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
        ┌─────▼─────┐  ┌────▼─────┐  ┌────▼─────┐
        │ SAE API   │  │ Query    │  │ Admin    │
        │ /v1/sae   │  │ /v1/query│  │ /v1/admin│
        └───────────┘  └──────────┘  └──────────┘
```

### Endpoints

#### SAE Forward

```
POST /v1/sae/forward
Authorization: Bearer <api_key>

Request:
{
    "model_id": "qwen2-7b-sae",
    "layer_id": 0,
    "hidden_state": [0.1, 0.2, ..., 0.9],  // [hidden_dim]
    "options": {
        "top_k": 512,
        "normalize": true
    }
}

Response:
{
    "features": [
        { "index": 42, "value": 0.95 },
        { "index": 128, "value": 0.87 }
    ],
    "confidence": 0.92,
    "processing_time_ms": 15
}
```

#### Query Network

```
GET /v1/network/status
Authorization: Bearer <api_key>

Response:
{
    "total_nodes": 150,
    "active_nodes": 142,
    "total_stake": 500000,
    "consensus_ratio": 0.87,
    "models_supported": ["qwen2-7b", "llama-3-8b", "mistral-7b"]
}
```

#### Submit Feedback

```
POST /v1/feedback
Authorization: Bearer <api_key>

Request:
{
    "layer_id": "layer_0",
    "feature_idx": 42,
    "feature_value": 0.95,
    "decision": "approved",
    "concept": "sentiment_positive",
    "annotator_id": "user_123"
}

Response:
{
    "feedback_id": "fb_abc123",
    "credits_earned": 20,
    "status": "recorded"
}
```

### Rate Limiting

| Plan | Requests/min | Daily Limit | Features |
|------|-------------|-------------|----------|
| Free | 10 | 1,000 | SAE forward, status |
| Developer | 100 | 50,000 | + Feedback, Query |
| Enterprise | 1000 | Unlimited | + Admin, Priority |

### API Key Management

```rust
/// Gestor de API keys
pub struct ApiKeyManager {
    /// Keys activas
    keys: HashMap<String, ApiKey>,
    /// Rate limiter
    rate_limiter: RateLimiter,
}

pub struct ApiKey {
    pub key: String,
    pub owner: String,
    pub plan: ApiPlan,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub permissions: Vec<ApiPermission>,
}
```

---

## 📅 Timeline y Hitos

### M1: Foundation (Semanas 1-4)

- [ ] Model Adapter trait + Qwen implementation
- [ ] Feature Normalizer
- [ ] API Gateway skeleton
- [ ] Rate limiter

### M2: Cross-Model (Semanas 5-8)

- [ ] Llama-3 SAE adapter
- [ ] Mistral-7B SAE adapter
- [ ] Unified SAE API
- [ ] Cross-model benchmark

### M3: Federated (Semanas 9-12)

- [ ] FederatedAverager implementation
- [ ] Poison detector (Krum)
- [ ] Secure aggregation
- [ ] Federated training loop

### M4: Staking (Semanas 13-16)

- [ ] StakingRegistry implementation
- [ ] Reward calculator
- [ ] Slash mechanism
- [ ] Epoch management

### M5: API (Semanas 17-20)

- [ ] Public API endpoints
- [ ] API key management
- [ ] Rate limiting
- [ ] API documentation (OpenAPI)

### M6: Launch (Semanas 21-24)

- [ ] Load testing
- [ ] Security audit
- [ ] Public beta
- [ ] v1.0.0 release

---

## ⚠️ Riesgos y Mitigación

| Riesgo | Impacto | Probabilidad | Mitigación |
|--------|---------|-------------|------------|
| Model incompatibility | Alto | Media | Adapter pattern + validation |
| Gradient poisoning | Crítico | Baja | Krum + multi-sig verification |
| Sybil attacks | Alto | Media | Staking + reputation + ZKP |
| API abuse | Medio | Alta | Rate limiting + API keys |
| Regulatory | Crítico | Baja | Ethical clause + transparency |

---

## 🔧 Dependencias Técnicas

### Nuevas Dependencias (Fase 6)

```toml
# Cross-model
safetensors = { version = "0.4", features = ["http"] }

# Federated learning
ndarray = { version = "0.15", features = ["blas"] }

# API
utoipa = "4.0"  # OpenAPI
tower = { version = "0.4", features = ["limit"] }  # Rate limiting

# Staking (optional blockchain)
ethers = { version = "2.0", optional = true }
```

### Infraestructura

| Component | Provider | Purpose |
|-----------|----------|---------|
| API Gateway | Cloudflare / AWS | Rate limiting, CDN |
| Database | PostgreSQL | API keys, usage |
| Redis | Upstash / AWS ElastiCache | Rate limit counters |
| Monitoring | Prometheus + Grafana | Metrics, alerts |

---

## 📞 Contribuir

Para contribuir a Fase 6:

1. Revisa los issues etiquetados con `phase-6`
2. Lee [`CONTRIBUTING.md`](CONTRIBUTING.md)
3. Únete a [GitHub Discussions](https://github.com/ed2kia/ed2kIA/discussions)

---

**ed2kIA** - Descentralizando la interpretabilidad de IA para el beneficio humano.
