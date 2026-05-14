#!/bin/bash
# monitor_setup.sh - Setup Prometheus, Grafana, and Loki monitoring stack for ed2kIA
# Usage: ./scripts/monitor_setup.sh [install|start|stop|status|uninstall]
#
# This script deploys a complete monitoring stack using Docker Compose:
# - Prometheus: Metrics collection and storage
# - Grafana: Visualization and dashboards
# - Loki: Log aggregation
# - Alertmanager: Alert routing and notifications
#
# Prerequisites: Docker, Docker Compose

set -euo pipefail

# Configuration
MONITOR_DIR="${MONITOR_DIR:-/opt/ed2k-monitoring}"
GRAFANA_PORT="${GRAFANA_PORT:-3000}"
PROMETHEUS_PORT="${PROMETHEUS_PORT:-9090}"
LOKI_PORT="${LOKI_PORT:-3100}"
ALERTMANAGER_PORT="${ALERTMANAGER_PORT:-9093}"
ED2K_METRICS_PORT="${ED2K_METRICS_PORT:-9091}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    for cmd in docker docker-compose; do
        if ! command -v "$cmd" &> /dev/null; then
            log_error "$cmd not found. Please install Docker and Docker Compose."
            exit 1
        fi
    done

    log_success "Prerequisites satisfied"
}

# Create directory structure
create_directories() {
    log_info "Creating directory structure..."

    mkdir -p "$MONITOR_DIR"/{prometheus,grafana,loki,alertmanager,data/{prometheus,grafana,loki}}

    log_success "Directories created in $MONITOR_DIR"
}

# Generate Prometheus configuration
generate_prometheus_config() {
    log_info "Generating Prometheus configuration..."

    cat > "$MONITOR_DIR/prometheus/prometheus.yml" << 'EOF'
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  scrape_timeout: 10s

rule_files:
  - "alerts.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  # ed2kIA nodes
  - job_name: 'ed2kia'
    scrape_interval: 10s
    static_configs:
      - targets:
        - localhost:9091  # Local node metrics
        # Add more nodes here:
        # - node1.ed2kIA:9091
        # - node2.ed2kIA:9091
    metrics_path: /metrics
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
        regex: '([^:]+):\d+'
        replacement: '${1}'

  # Prometheus self-monitoring
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  # Node exporter (system metrics)
  - job_name: 'node'
    static_configs:
      - targets: ['node-exporter:9100']
EOF

    # Generate alerts configuration
    cat > "$MONITOR_DIR/prometheus/alerts.yml" << 'EOF'
groups:
  - name: ed2kia_alerts
    rules:
      # SAE Latency
      - alert: SAEHighLatency
        expr: histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m])) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "SAE latency high (instance {{ $labels.instance }})"
          description: "95th percentile SAE latency is {{ $value }}s (threshold: 0.5s)"

      - alert: SAECriticalLatency
        expr: histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m])) > 1.0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "SAE latency critical (instance {{ $labels.instance }})"
          description: "95th percentile SAE latency is {{ $value }}s (threshold: 1.0s)"

      # Consensus Rate
      - alert: ConsensusRateLow
        expr: sum(rate(consensus_votes_total{result="agree"}[5m])) / sum(rate(consensus_votes_total[5m])) < 0.7
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Consensus rate low"
          description: "Consensus agreement rate is {{ $value | humanizePercentage }} (threshold: 70%)"

      - alert: ConsensusRateCritical
        expr: sum(rate(consensus_votes_total{result="agree"}[5m])) / sum(rate(consensus_votes_total[5m])) < 0.5
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Consensus rate critical"
          description: "Consensus agreement rate is {{ $value | humanizePercentage }} (threshold: 50%)"

      # WASM Memory
      - alert: WASMMemoryHigh
        expr: wasm_memory_bytes > 200 * 1024 * 1024
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "WASM memory high (instance {{ $labels.instance }})"
          description: "WASM memory usage is {{ $value | humanize1024 }} (threshold: 200MB)"

      - alert: WASMMemoryCritical
        expr: wasm_memory_bytes > 500 * 1024 * 1024
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "WASM memory critical (instance {{ $labels.instance }})"
          description: "WASM memory usage is {{ $value | humanize1024 }} (threshold: 500MB)"

      # Node Reputation
      - alert: NodeReputationLow
        expr: node_reputation_score < 0.4
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "Node reputation low (instance {{ $labels.instance }})"
          description: "Node reputation is {{ $value }} (threshold: 0.4)"

      # Peer Count
      - alert: LowPeerCount
        expr: peer_count < 5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count (instance {{ $labels.instance }})"
          description: "Connected peers: {{ $value }} (threshold: 5)"

      - alert: NoPeersConnected
        expr: peer_count < 2
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "No peers connected (instance {{ $labels.instance }})"
          description: "Connected peers: {{ $value }} - Node may be isolated"

      # System Resources
      - alert: HighCPUUsage
        expr: 100 - (avg by(instance) (rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100) > 80
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage (instance {{ $labels.instance }})"
          description: "CPU usage is {{ $value }}% (threshold: 80%)"

      - alert: HighMemoryUsage
        expr: (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100 > 85
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage (instance {{ $labels.instance }})"
          description: "Memory usage is {{ $value }}% (threshold: 85%)"

      - alert: DiskSpaceLow
        expr: (1 - (node_filesystem_avail_bytes / node_filesystem_size_bytes)) * 100 > 90
        for: 15m
        labels:
          severity: critical
        annotations:
          summary: "Disk space low (instance {{ $labels.instance }})"
          description: "Disk usage is {{ $value }}% (threshold: 90%)"
EOF

    log_success "Prometheus configuration generated"
}

# Generate Alertmanager configuration
generate_alertmanager_config() {
    log_info "Generating Alertmanager configuration..."

    cat > "$MONITOR_DIR/alertmanager/alertmanager.yml" << 'EOF'
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'instance']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'default'
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
      continue: true
    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'default'
    email_configs:
      - to: 'ops@ed2kIA.org'
        from: 'alertmanager@ed2kIA.org'
        smarthost: 'smtp.example.com:587'
        auth_username: 'alertmanager@ed2kIA.org'
        auth_password: '${SMTP_PASSWORD:-change-me}'

  - name: 'slack'
    slack_configs:
      - api_url: '${SLACK_WEBHOOK_URL:-https://hooks.slack.com/services/CHANGE/ME}'
        channel: '#ed2k-ops'
        title: '[ed2kIA] {{ .GroupLabels.alertname }}'
        text: >-
          {{ range .Alerts }}
          *Alert:* {{ .Labels.alertname }}
          *Severity:* {{ .Labels.severity }}
          *Instance:* {{ .Labels.instance }}
          *Description:* {{ .Annotations.description }}
          {{ end }}

  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: '${PAGERDUTY_SERVICE_KEY:-change-me}'
        severity: '{{ .CommonLabels.severity }}'
        description: '{{ .CommonAnnotations.summary }}'

inhibit_rules:
  - source_match:
      severity: 'critical'
    target_match:
      severity: 'warning'
    equal: ['alertname', 'instance']
EOF

    log_success "Alertmanager configuration generated"
}

# Generate Grafana configuration
generate_grafana_config() {
    log_info "Generating Grafana configuration..."

    mkdir -p "$MONITOR_DIR/grafana/provisioning/{datasources,dashboards}"

    # Datasource provisioning
    cat > "$MONITOR_DIR/grafana/provisioning/datasources/datasources.yml" << 'EOF'
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: true

  - name: Loki
    type: loki
    access: proxy
    url: http://loki:3100
    editable: true
EOF

    # Dashboard provisioning
    cat > "$MONITOR_DIR/grafana/provisioning/dashboards/dashboards.yml" << 'EOF'
apiVersion: 1

providers:
  - name: 'ed2kIA'
    orgId: 1
    folder: 'ed2kIA'
    type: file
    disableDeletion: false
    editable: true
    options:
      path: /var/lib/grafana/dashboards
      foldersFromFilesStructure: false
EOF

    # Create dashboards directory
    mkdir -p "$MONITOR_DIR/grafana/dashboards"

    # Node Overview Dashboard
    cat > "$MONITOR_DIR/grafana/dashboards/node_overview.json" << 'EOF'
{
  "annotations": { "list": [] },
  "editable": true,
  "fiscalYearStartMonth": 0,
  "graphTooltip": 0,
  "id": null,
  "links": [],
  "liveNow": false,
  "panels": [
    {
      "datasource": {"type": "prometheus", "uid": "prometheus"},
      "fieldConfig": {
        "defaults": {
          "color": {"mode": "palette-classic"},
          "unit": "short"
        }
      },
      "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0},
      "id": 1,
      "title": "Peer Count",
      "type": "timeseries",
      "targets": [
        {"expr": "peer_count", "legendFormat": "{{instance}}"}
      ]
    },
    {
      "datasource": {"type": "prometheus", "uid": "prometheus"},
      "fieldConfig": {
        "defaults": {
          "color": {"mode": "palette-classic"},
          "unit": "percentunit"
        }
      },
      "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0},
      "id": 2,
      "title": "Consensus Agreement Rate",
      "type": "timeseries",
      "targets": [
        {"expr": "sum(rate(consensus_votes_total{result=\"agree\"}[5m])) / sum(rate(consensus_votes_total[5m]))"}
      ]
    },
    {
      "datasource": {"type": "prometheus", "uid": "prometheus"},
      "fieldConfig": {
        "defaults": {
          "color": {"mode": "palette-classic"},
          "unit": "s"
        }
      },
      "gridPos": {"h": 8, "w": 12, "x": 0, "y": 8},
      "id": 3,
      "title": "SAE Latency (p95)",
      "type": "timeseries",
      "targets": [
        {"expr": "histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m]))"}
      ]
    },
    {
      "datasource": {"type": "prometheus", "uid": "prometheus"},
      "fieldConfig": {
        "defaults": {
          "color": {"mode": "palette-classic"},
          "unit": "bytes"
        }
      },
      "gridPos": {"h": 8, "w": 12, "x": 12, "y": 8},
      "id": 4,
      "title": "WASM Memory Usage",
      "type": "timeseries",
      "targets": [
        {"expr": "wasm_memory_bytes", "legendFormat": "{{instance}}"}
      ]
    }
  ],
  "schemaVersion": 38,
  "style": "dark",
  "tags": ["ed2kIA"],
  "templating": {"list": []},
  "time": {"from": "now-6h", "to": "now"},
  "title": "ed2kIA Node Overview",
  "uid": "ed2kia-overview",
  "version": 1
}
EOF

    log_success "Grafana configuration generated"
}

# Generate Loki configuration
generate_loki_config() {
    log_info "Generating Loki configuration..."

    cat > "$MONITOR_DIR/loki/loki.yml" << 'EOF'
auth_enabled: false

server:
  http_listen_port: 3100

common:
  path_prefix: /loki
  storage:
    type: filesystem
  replication_factor: 1
  ring:
    kvstore:
      store: inmemory

schema_config:
  configs:
    - from: 2020-10-24
      store: tsdb
      object_store: filesystem
      schema: v13
      index:
        prefix: index_
        period: 24h

storage_config:
  filesystem:
    directory: /loki/chunks

limits_config:
  reject_old_samples: true
  reject_old_samples_max_age: 168h
  max_entries_limit_per_query: 5000

query_range:
  results_cache:
    cache:
      embedded_cache:
        enabled: true
        max_size_mb: 100
EOF

    log_success "Loki configuration generated"
}

# Generate Docker Compose file
generate_docker_compose() {
    log_info "Generating Docker Compose configuration..."

    cat > "$MONITOR_DIR/docker-compose.yml" << EOF
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:v2.48.0
    container_name: ed2k-prometheus
    ports:
      - "${PROMETHEUS_PORT}:9090"
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./prometheus/alerts.yml:/etc/prometheus/alerts.yml:ro
      - data/prometheus:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=30d'
      - '--web.enable-lifecycle'
    restart: unless-stopped
    networks:
      - monitoring

  grafana:
    image: grafana/grafana:10.2.0
    container_name: ed2k-grafana
    ports:
      - "${GRAFANA_PORT}:3000"
    volumes:
      - ./grafana/provisioning:/etc/grafana/provisioning:ro
      - ./grafana/dashboards:/var/lib/grafana/dashboards:ro
      - data/grafana:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_USER=admin
      - GF_SECURITY_ADMIN_PASSWORD=\${GRAFANA_PASSWORD:-ed2kIA2024}
      - GF_USERS_ALLOW_SIGN_UP=false
    restart: unless-stopped
    networks:
      - monitoring
    depends_on:
      - prometheus

  loki:
    image: grafana/loki:2.9.0
    container_name: ed2k-loki
    ports:
      - "${LOKI_PORT}:3100"
    volumes:
      - ./loki/loki.yml:/etc/loki/loki.yml:ro
      - data/loki:/loki
    command: -config.file=/etc/loki/loki.yml
    restart: unless-stopped
    networks:
      - monitoring

  alertmanager:
    image: prom/alertmanager:v0.26.0
    container_name: ed2k-alertmanager
    ports:
      - "${ALERTMANAGER_PORT}:9093"
    volumes:
      - ./alertmanager/alertmanager.yml:/etc/alertmanager/alertmanager.yml:ro
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
      - '--log.level=info'
    restart: unless-stopped
    networks:
      - monitoring

  node-exporter:
    image: prom/node-exporter:v1.7.0
    container_name: ed2k-node-exporter
    ports:
      - "9100:9100"
    volumes:
      - /proc:/host/proc:ro
      - /sys:/host/sys:ro
      - /:/rootfs:ro
    command:
      - '--path.procfs=/host/proc'
      - '--path.sysfs=/host/sys'
      - '--path.rootfs=/rootfs'
    restart: unless-stopped
    networks:
      - monitoring

  promtail:
    image: grafana/promtail:2.9.0
    container_name: ed2k-promtail
    volumes:
      - /var/log:/var/log:ro
      - /var/lib/docker/containers:/var/lib/docker/containers:ro
      - ./promtail/promtail.yml:/etc/promtail/promtail.yml:ro
    command: -config.file=/etc/promtail/promtail.yml
    restart: unless-stopped
    networks:
      - monitoring
    depends_on:
      - loki

networks:
  monitoring:
    driver: bridge

volumes:
  data:
EOF

    # Generate Promtail configuration
    mkdir -p "$MONITOR_DIR/promtail"
    cat > "$MONITOR_DIR/promtail/promtail.yml" << 'EOF'
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: ed2kia
    static_configs:
      - targets:
          - localhost
        labels:
          job: ed2kia
          __path__: /var/log/ed2kia/*.log

  - job_name: docker
    static_configs:
      - targets:
          - localhost
        labels:
          job: docker
          __path__: /var/lib/docker/containers/*/*-json.log
EOF

    log_success "Docker Compose configuration generated"
}

# Install monitoring stack
install() {
    log_info "=== Installing ed2kIA Monitoring Stack ==="

    check_prerequisites
    create_directories
    generate_prometheus_config
    generate_alertmanager_config
    generate_grafana_config
    generate_loki_config
    generate_docker_compose

    log_success "Monitoring stack installed in $MONITOR_DIR"
    echo ""
    log_info "To start the stack:"
    echo "  cd $MONITOR_DIR && docker-compose up -d"
    echo ""
    log_info "Access URLs:"
    echo "  Grafana:      http://localhost:$GRAFANA_PORT (admin/ed2kIA2024)"
    echo "  Prometheus:   http://localhost:$PROMETHEUS_PORT"
    echo "  Loki:         http://localhost:$LOKI_PORT"
    echo "  Alertmanager: http://localhost:$ALERTMANAGER_PORT"
}

# Start monitoring stack
start() {
    log_info "Starting monitoring stack..."

    if [ ! -f "$MONITOR_DIR/docker-compose.yml" ]; then
        log_error "Not installed. Run: $0 install"
        exit 1
    fi

    cd "$MONITOR_DIR" && docker-compose up -d
    log_success "Monitoring stack started"

    echo ""
    log_info "Access URLs:"
    echo "  Grafana:      http://localhost:$GRAFANA_PORT (admin/ed2kIA2024)"
    echo "  Prometheus:   http://localhost:$PROMETHEUS_PORT"
    echo "  Loki:         http://localhost:$LOKI_PORT"
    echo "  Alertmanager: http://localhost:$ALERTMANAGER_PORT"
}

# Stop monitoring stack
stop() {
    log_info "Stopping monitoring stack..."

    if [ ! -f "$MONITOR_DIR/docker-compose.yml" ]; then
        log_error "Not installed."
        exit 1
    fi

    cd "$MONITOR_DIR" && docker-compose down
    log_success "Monitoring stack stopped"
}

# Check status
status() {
    log_info "Monitoring stack status:"

    if [ ! -f "$MONITOR_DIR/docker-compose.yml" ]; then
        log_error "Not installed."
        exit 1
    fi

    cd "$MONITOR_DIR" && docker-compose ps
}

# Uninstall monitoring stack
uninstall() {
    log_info "Uninstalling monitoring stack..."

    if [ ! -f "$MONITOR_DIR/docker-compose.yml" ]; then
        log_error "Not installed."
        exit 1
    fi

    cd "$MONITOR_DIR" && docker-compose down -v

    log_warn "Data files remain in $MONITOR_DIR/data/"
    log_info "To remove everything: rm -rf $MONITOR_DIR"

    log_success "Monitoring stack uninstalled"
}

# Main
case "${1:-install}" in
    install)
        install
        ;;
    start)
        start
        ;;
    stop)
        stop
        ;;
    status)
        status
        ;;
    uninstall)
        uninstall
        ;;
    *)
        echo "Usage: $0 {install|start|stop|status|uninstall}"
        exit 1
        ;;
esac
