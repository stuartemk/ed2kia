# ed2kIA Monitoring Guide

> **Configuración de Prometheus/Grafana, alertas críticas y dashboards recomendados.**

---

## 📋 Tabla de Contenidos

1. [Arquitectura de Monitoreo](#-arquitectura-de-monitoreo)
2. [Métricas Expuestas](#-métricas-expuestas)
3. [Configuración de Prometheus](#-configuración-de-prometheus)
4. [Configuración de Grafana](#-configuración-de-grafana)
5. [Alertas Críticas](#-alertas-críticas)
6. [Dashboards Recomendados](#-dashboards-recomendados)
7. [Log Aggregation](#-log-aggregation)
8. [Troubleshooting](#-troubleshooting)

---

## 🏗️ Arquitectura de Monitoreo

```
┌─────────────────────────────────────────────────────────────┐
│                    Monitoring Architecture                    │
└─────────────────────────────────────────────────────────────┘

┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  ed2kIA Node │    │  ed2kIA Node │    │  ed2kIA Node │
│  :3000/metrics│    │  :3001/metrics│    │  :3002/metrics│
└──────┬───────┘    └──────┬───────┘    └──────┬───────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │ HTTP Scrape
                    ┌──────▼───────┐
                    │  Prometheus  │
                    │  :9090       │
                    └──────┬───────┘
                           │ Query
                    ┌──────▼───────┐
                    │    Grafana   │
                    │  :3000       │
                    └─────────────┘
```

---

## 📊 Métricas Expuestas

### Endpoint de Métricas

```
GET http://<node>:<http_port>/api/metrics
```

### Métricas Disponibles

#### P2P Network

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `ed2kia_p2p_peers_total` | Gauge | Total de peers conectados |
| `ed2kia_p2p_messages_received_total` | Counter | Mensajes GossipSub recibidos |
| `ed2kia_p2p_messages_sent_total` | Counter | Mensajes GossipSub enviados |
| `ed2kia_p2p_bytes_received_total` | Counter | Bytes recibidos |
| `ed2kia_p2p_bytes_sent_total` | Counter | Bytes enviados |
| `ed2kia_p2p_latency_ms` | Histogram | Latencia P2P (ms) |

#### SAE Processing

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `ed2kia_sae_forwards_total` | Counter | Forward passes SAE totales |
| `ed2kia_sae_inference_time_ms` | Histogram | Tiempo de inferencia (ms) |
| `ed2kia_sae_layers_active` | Gauge | Capas SAE activas |
| `ed2kia_sae_features_extracted_total` | Counter | Features extraídas |

#### Consensus

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `ed2kia_consensus_votes_total` | Counter | Votos de consenso totales |
| `ed2kia_consensus_approved_total` | Counter | Batches aprobados |
| `ed2kia_consensus_rejected_total` | Counter | Batches rechazados |
| `ed2kia_consensus_pending_batches` | Gauge | Batches pendientes |
| `ed2kia_consensus_agreement_ratio` | Gauge | Ratio de acuerdo (0-1) |

#### Governance

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `ed2kia_governance_proposals_total` | Counter | Propuestas creadas |
| `ed2kia_governance_votes_cast_total` | Counter | Votos emitidos |
| `ed2kia_governance_proposals_approved` | Counter | Propuestas aprobadas |
| `ed2kia_governance_proposals_rejected` | Counter | Propuestas rechazadas |

#### Reputation

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `ed2kia_reputation_total_credits` | Gauge | Créditos totales del nodo |
| `ed2kia_reputation_contributions_total` | Counter | Contribuciones registradas |
| `ed2kia_reputation_decay_applied_total` | Counter | Decays aplicados |

#### System

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `ed2kia_uptime_seconds` | Gauge | Uptime del nodo (s) |
| `ed2kia_memory_usage_bytes` | Gauge | Memoria usada (bytes) |
| `ed2kia_cpu_usage_percent` | Gauge | CPU usada (%) |
| `ed2kia_disk_usage_bytes` | Gauge | Disco usado (bytes) |

---

## 🔧 Configuración de Prometheus

### prometheus.yml

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "alert_rules.yml"

scrape_configs:
  - job_name: "ed2kia"
    static_configs:
      - targets:
          - "seed-node-1:3000"
          - "seed-node-2:3001"
          - "seed-node-3:3002"
        labels:
          cluster: "production"
          environment: "prod"

  - job_name: "ed2kia_dev"
    static_configs:
      - targets:
          - "localhost:3000"
          - "localhost:3001"
          - "localhost:3002"
        labels:
          cluster: "development"
          environment: "dev"
```

### alert_rules.yml

```yaml
groups:
  - name: ed2kia_critical
    rules:
      # Node down
      - alert: Ed2kIANodeDown
        expr: up{job="ed2kia"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "ed2kIA node {{ $labels.instance }} is down"
          description: "Node has been unreachable for > 5 minutes"

      # Low peer count
      - alert: Ed2kIALowPeers
        expr: ed2kia_p2p_peers_total < 2
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count on {{ $labels.instance }}"
          description: "Node has < 2 peers for > 10 minutes"

      # High latency
      - alert: Ed2kIAHighLatency
        expr: histogram_quantile(0.95, ed2kia_p2p_latency_ms) > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High P2P latency on {{ $labels.instance }}"
          description: "95th percentile latency > 1s"

      # Consensus failure
      - alert: Ed2kIAConsensusFailure
        expr: ed2kia_consensus_agreement_ratio < 0.4
        for: 15m
        labels:
          severity: critical
        annotations:
          summary: "Consensus agreement ratio low"
          description: "Agreement ratio < 40% for > 15 minutes"

      # High memory usage
      - alert: Ed2kIAHighMemory
        expr: ed2kia_memory_usage_bytes / 1024 / 1024 / 1024 > 14
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage on {{ $labels.instance }}"
          description: "Memory usage > 14GB for > 10 minutes"

      # Disk space low
      - alert: Ed2kIALowDiskSpace
        expr: (ed2kia_disk_usage_bytes / 1024 / 1024 / 1024) > 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low disk space on {{ $labels.instance }}"
          description: "Disk usage > 90GB"
```

### Docker Compose (Monitoring Stack)

```yaml
version: "3.8"
services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - ./alert_rules.yml:/etc/prometheus/alert_rules.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.retention.time=30d'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3001:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false

volumes:
  prometheus_data:
  grafana_data:
```

---

## 📈 Configuración de Grafana

### Dashboard: Network Overview

```json
{
  "dashboard": {
    "title": "ed2kIA Network Overview",
    "panels": [
      {
        "title": "Connected Peers",
        "type": "graph",
        "targets": [
          {
            "expr": "ed2kia_p2p_peers_total",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Messages/sec",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(ed2kia_p2p_messages_received_total[5m])",
            "legendFormat": "{{ instance }} - received"
          },
          {
            "expr": "rate(ed2kia_p2p_messages_sent_total[5m])",
            "legendFormat": "{{ instance }} - sent"
          }
        ]
      },
      {
        "title": "P2P Latency (p95)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(ed2kia_p2p_latency_ms_bucket[5m]))",
            "legendFormat": "{{ instance }}"
          }
        ]
      }
    ]
  }
}
```

### Dashboard: SAE Processing

```json
{
  "dashboard": {
    "title": "ed2kIA SAE Processing",
    "panels": [
      {
        "title": "SAE Forwards/sec",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(ed2kia_sae_forwards_total[5m])",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Inference Time (p50/p95/p99)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(ed2kia_sae_inference_time_ms_bucket[5m]))",
            "legendFormat": "{{ instance }} - p50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(ed2kia_sae_inference_time_ms_bucket[5m]))",
            "legendFormat": "{{ instance }} - p95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(ed2kia_sae_inference_time_ms_bucket[5m]))",
            "legendFormat": "{{ instance }} - p99"
          }
        ]
      },
      {
        "title": "Active Layers",
        "type": "stat",
        "targets": [
          {
            "expr": "ed2kia_sae_layers_active",
            "legendFormat": "{{ instance }}"
          }
        ]
      }
    ]
  }
}
```

### Dashboard: Consensus & Governance

```json
{
  "dashboard": {
    "title": "ed2kIA Consensus & Governance",
    "panels": [
      {
        "title": "Consensus Agreement Ratio",
        "type": "gauge",
        "targets": [
          {
            "expr": "ed2kia_consensus_agreement_ratio",
            "legendFormat": "{{ instance }}"
          }
        ],
        "options": {
          "min": 0,
          "max": 1,
          "thresholds": [
            { "value": 0, "color": "red" },
            { "value": 0.4, "color": "orange" },
            { "value": 0.6, "color": "green" }
          ]
        }
      },
      {
        "title": "Pending Batches",
        "type": "stat",
        "targets": [
          {
            "expr": "ed2kia_consensus_pending_batches",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Proposals Status",
        "type": "stat",
        "targets": [
          {
            "expr": "ed2kia_governance_proposals_approved",
            "legendFormat": "Approved"
          },
          {
            "expr": "ed2kia_governance_proposals_rejected",
            "legendFormat": "Rejected"
          }
        ]
      }
    ]
  }
}
```

---

## 🚨 Alertas Críticas

### Canales de Notificación

#### Alertmanager Configuration

```yaml
# alertmanager.yml
global:
  smtp_smarthost: 'smtp.example.com:587'
  smtp_from: 'alerts@ed2kia.network'
  smtp_auth_username: 'alerts@ed2kia.network'
  smtp_auth_password: '${SMTP_PASSWORD}'

route:
  group_by: ['alertname', 'cluster']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'pagerduty'

receivers:
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: '${PAGERDUTY_KEY}'
        severity: 'critical'

  - name: 'slack'
    slack_configs:
      - api_url: '${SLACK_WEBHOOK}'
        channel: '#ed2kia-alerts'
        title: '{{ .GroupLabels.alertname }}'
        text: '{{ .CommonAnnotations.description }}'

  - name: 'email'
    email_configs:
      - to: 'ops@ed2kia.network'
```

### Matriz de Alertas

| Alerta | Severidad | Canal | SLA Respuesta |
|--------|-----------|-------|---------------|
| Node Down | Critical | PagerDuty + Slack | 15 min |
| Consensus Failure | Critical | PagerDuty + Slack | 15 min |
| Low Peers | Warning | Slack | 1 hour |
| High Latency | Warning | Slack | 1 hour |
| High Memory | Warning | Slack | 2 hours |
| Low Disk | Warning | Slack + Email | 4 hours |

---

## 📋 Log Aggregation

### Structured Logging

ed2kIA usa `tracing` con formato JSON:

```bash
# Configurar logging JSON
export RUST_LOG=info
export RUST_LOG_FORMAT=json

./target/release/ed2kia join
```

### Example Log Entry

```json
{
    "timestamp": "2024-01-15T10:30:00Z",
    "level": "INFO",
    "target": "ed2kia::p2p::swarm",
    "message": "Peer connected",
    "peer_id": "12D3Koo...",
    "total_peers": 5
}
```

### Loki Configuration (Opcional)

```yaml
# loki-config.yml
auth_enabled: false

server:
  http_listen_port: 3100

ingester:
  lifecycler:
    address: 127.0.0.1
    ring:
      kvstore:
        store: inmemory

schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb-shipper
      object_store: filesystem
      schema: v11
      index:
        prefix: index_
        period: 24h

storage_config:
  boltdb_shipper:
    active_index_directory: /tmp/loki/boltdb-shipper-active
    cache_location: /tmp/loki/boltdb-shipper-cache
    shared_store: filesystem
  filesystem:
    directory: /tmp/loki/chunks
```

---

## 🔧 Troubleshooting

### Prometheus no scrapes métricas

**Síntoma:** Target muestra "DOWN" en Prometheus

**Solución:**
```bash
# 1. Verificar que el endpoint está accesible
curl http://<node>:3000/api/metrics

# 2. Verificar conectividad desde Prometheus
docker exec -it prometheus wget -q -O- http://<node>:3000/api/metrics

# 3. Verificar firewall
sudo ufw allow from <prometheus_ip> to any port 3000
```

### Grafana no muestra datos

**Síntoma:** Paneles vacíos

**Solución:**
```bash
# 1. Verificar datasource en Grafana
#    Admin → Data Sources → Prometheus → Test

# 2. Verificar que Prometheus tiene datos
curl "http://localhost:9090/api/v1/query?query=up"

# 3. Verificar tiempo de retención
#    prometheus.yml → storage.tsdb.retention.time
```

### Alertas no se disparan

**Síntoma:** Condición cumplida pero sin notificación

**Solución:**
```bash
# 1. Verificar reglas en Prometheus
#    Prometheus UI → Status → Rules

# 2. Verificar Alertmanager
curl http://localhost:9093/api/v2/status

# 3. Verificar logs de Alertmanager
docker logs alertmanager
```

---

## 📞 Soporte

- **Issue Tracker:** [GitHub Issues](https://github.com/ed2kia/ed2kIA/issues)
- **Discusión:** [GitHub Discussions](https://github.com/ed2kia/ed2kIA/discussions)

---

**ed2kIA** - Descentralizando la interpretabilidad de IA para el beneficio humano.
