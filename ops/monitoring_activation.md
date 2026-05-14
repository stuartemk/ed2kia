# Monitoring Activation Guide - ed2kIA v0.5.0

## Overview

This guide provides exact commands for deploying the Prometheus/Grafana/Loki monitoring stack, configuring dashboards, setting up alert rules, and validating critical metrics for ed2kIA network operations.

## Prerequisites

- Docker 20.10+ and Docker Compose v2+
- 4GB RAM minimum (8GB recommended for monitoring stack)
- Ports available: 3000 (Grafana), 9090 (Prometheus), 3100 (Loki), 9093 (Alertmanager)

---

## Step 1: Deploy Monitoring Stack

### Quick Deploy

```bash
# Run the setup script
./scripts/monitor_setup.sh install

# Start all services
./scripts/monitor_setup.sh start
```

### Manual Deploy

```bash
# Create monitoring directory
mkdir -p /opt/ed2k-monitoring
cd /opt/ed2k-monitoring

# Generate configuration
# (Use scripts/monitor_setup.sh or copy from deploy/monitoring/)

# Start stack
docker compose up -d

# Verify all containers running
docker compose ps
# Expected: 5/5 services running
```

### Expected Output

```
NAME                    STATUS
ed2k-prometheus         Up
ed2k-grafana            Up
ed2k-loki               Up
ed2k-alertmanager       Up
ed2k-node-exporter      Up
ed2k-promtail           Up
```

---

## Step 2: Validate Prometheus Scraping

### Check Targets

```bash
# Query Prometheus targets
curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | {job, health}'

# Expected output:
# [
#   {"job": "ed2kia", "health": "up"},
#   {"job": "prometheus", "health": "up"},
#   {"job": "node", "health": "up"}
# ]
```

### Add ed2kIA Nodes

Edit `/opt/ed2k-monitoring/prometheus/prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'ed2kia'
    scrape_interval: 10s
    static_configs:
      - targets:
        - seed-alpha.ed2kIA:9090
        - seed-bravo.ed2kIA:9090
        - seed-charlie.ed2kIA:9090
        - seed-delta.ed2kIA:9090
        - seed-echo.ed2kIA:9090
```

Reload Prometheus:
```bash
curl -X POST http://localhost:9090/-/reload
```

---

## Step 3: Configure Grafana Dashboards

### Initial Login

- **URL**: http://localhost:3000
- **Username**: admin
- **Password**: ed2kIA2024 (change immediately!)

```bash
# Verify Grafana is accessible
curl -s -u admin:ed2kIA2024 http://localhost:3000/api/health
# Expected: {"commit":"...","database":"ok","version":"10.2.0"}
```

### Import Dashboards

Dashboards are auto-provisioned from `/opt/ed2k-monitoring/grafana/dashboards/`. Verify:

```bash
# List provisioned dashboards
curl -s -u admin:ed2kIA2024 \
  -H "Authorization: Bearer $(curl -s -u admin:ed2kIA2024 http://localhost:3000/api/auth/temporary/login | jq -r '.token')" \
  http://localhost:3000/api/search | jq '.[] | select(.title=~"ed2k")'
```

### Create Additional Dashboards

#### Node Overview Dashboard

```json
{
  "panels": [
    {
      "title": "Peer Count",
      "targets": [{"expr": "peer_count", "legendFormat": "{{instance}}"}],
      "type": "timeseries",
      "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
    },
    {
      "title": "Consensus Agreement Rate",
      "targets": [{"expr": "sum(rate(consensus_votes_total{result=\"agree\"}[5m])) / sum(rate(consensus_votes_total[5m]))"}],
      "type": "timeseries",
      "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
    },
    {
      "title": "SAE Latency p95",
      "targets": [{"expr": "histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m]))"}],
      "type": "timeseries",
      "gridPos": {"h": 8, "w": 12, "x": 0, "y": 8}
    },
    {
      "title": "WASM Memory",
      "targets": [{"expr": "wasm_memory_bytes / 1024 / 1024", "legendFormat": "{{instance}} MB"}],
      "type": "timeseries",
      "gridPos": {"h": 8, "w": 12, "x": 12, "y": 8}
    }
  ]
}
```

Import via: Dashboard → Import → Upload JSON

---

## Step 4: Configure Alert Rules

### Prometheus Alert Rules

Alerts are pre-configured in `/opt/ed2k-monitoring/prometheus/alerts.yml`. Verify:

```bash
# Check alert rules loaded
curl -s http://localhost:9090/api/v1/rules | jq '.data.groups[0].rules[] | {name, state}'
```

### Critical Alerts Summary

| Alert | Condition | Severity | Action |
|-------|-----------|----------|--------|
| SAECriticalLatency | p95 > 1s for 2m | Critical | Page on-call |
| ConsensusRateCritical | < 50% for 5m | Critical | Page on-call |
| WASMMemoryCritical | > 500MB for 2m | Critical | Investigate leak |
| NoPeersConnected | < 2 peers for 5m | Critical | Check network |
| SAEHighLatency | p95 > 500ms for 5m | Warning | Monitor |
| ConsensusRateLow | < 70% for 10m | Warning | Investigate |
| NodeReputationLow | < 0.4 for 15m | Warning | Check node |

### Test Alerts

```bash
# Send test alert to Alertmanager
curl -X POST http://localhost:9093/api/v1/alerts \
  -H "Content-Type: application/json" \
  -d '[{
    "labels": {
      "alertname": "TestAlert",
      "severity": "warning",
      "instance": "test"
    },
    "annotations": {
      "summary": "Test alert from monitoring activation"
    }
  }]'

# Verify alert received
curl -s http://localhost:9093/api/v1/alerts | jq '.data.alerts[] | select(.labels.alertname=="TestAlert")'
```

---

## Step 5: Configure Log Aggregation (Loki + Promtail)

### Verify Loki

```bash
# Check Loki is running
curl -s http://localhost:3100/loki/api/v1/label | jq '.'
# Expected: {"data":["filename","job","level"]}
```

### Query ed2kIA Logs

```bash
# Search for errors
curl -s 'http://localhost:3100/loki/api/v1/query' \
  --data-urlencode 'query={job="ed2kia"} |= level="error"' | jq '.'

# Count warnings in last hour
curl -s 'http://localhost:3100/loki/api/v1/query_range' \
  --data-urlencode 'query=count_over_time({job="ed2kia"} |= level="warn" [1h])' \
  --data-urlencode 'start=-3600' \
  --data-urlencode 'end=0' \
  --data-urlencode 'step=60' | jq '.'
```

### Add Log Dashboard to Grafana

1. Add Loki as datasource (auto-provisioned)
2. Create panel with Loki query: `{job="ed2kia"}`
3. Add filters for level: error, warn, info

---

## Step 6: Validate Critical Metrics

### Metric Validation Checklist

Run these queries and verify expected values:

```bash
# 1. Peer count (expect: ≥3 per node)
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=peer_count' | jq '.data.result[] | {instance, value}'

# 2. Consensus rate (expect: > 0.7)
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=sum(rate(consensus_votes_total{result="agree"}[5m])) / sum(rate(consensus_votes_total[5m]))' | jq '.data.result[0].value'

# 3. SAE latency p95 (expect: < 0.5)
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m]))' | jq '.data.result[0].value'

# 4. WASM memory (expect: < 200MB = 209715200)
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=wasm_memory_bytes' | jq '.data.result[] | {instance, value: (.value[1] | tonumber / 1024 / 1024 | round)}'

# 5. Node reputation (expect: ≥ 0.4)
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=node_reputation_score' | jq '.data.result[] | {instance, value}'

# 6. Health check status (expect: all "healthy")
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=health_check_status' | jq '.data.result[] | {instance, value}'
```

---

## Step 7: Configure Alert Notifications

### Slack Integration

Edit `/opt/ed2k-monitoring/alertmanager/alertmanager.yml`:

```yaml
receivers:
  - name: 'slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
        channel: '#ed2k-ops'
        title: '[ed2kIA] {{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}*{{ .Labels.alertname }}* ({{ .Labels.severity }})\n{{ .Annotations.description }}\n{{ end }}'
```

### Email Integration

```yaml
  - name: 'email'
    email_configs:
      - to: 'ops@ed2kIA.org'
        from: 'alertmanager@ed2kIA.org'
        smarthost: 'smtp.example.com:587'
        auth_username: 'alertmanager@ed2kIA.org'
        auth_password: '${SMTP_PASSWORD}'
```

Restart Alertmanager:
```bash
docker compose restart alertmanager
```

---

## Troubleshooting

### Prometheus Not Scraping

```bash
# Check Prometheus logs
docker logs ed2k-prometheus | tail -20

# Verify targets reachable
curl -sf http://seed-alpha.ed2kIA:9090/metrics | head -5
```

### Grafana Dashboard Empty

```bash
# Check datasource connection
curl -s -u admin:ed2kIA2024 http://localhost:3000/api/datasources | jq '.[] | {name, type, jsonData.url}'

# Verify Prometheus has data
curl -s http://localhost:9090/api/v1/series --data-urlencode 'match[]={job="ed2kia"}' | jq '.data | length'
```

### Alerts Not Firing

```bash
# Check rule evaluation
curl -s http://localhost:9090/api/v1/rules | jq '.data.groups[] | {name, rules: [.rules[] | {name, state, lastError}]}'

# Check Alertmanager connectivity
curl -s http://localhost:9093/api/v1/status | jq '.cluster.status'
```
