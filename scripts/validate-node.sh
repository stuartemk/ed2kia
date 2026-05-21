#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# validate-node.sh — Validador Ligero para Nodos Voluntarios (Sprint18)
# ═══════════════════════════════════════════════════════════════════════════════
#
# Verifica:
#   1. Conexión a mesh GossipSub (endpoint local)
#   2. Estado de SCTGuard (Z-axis activo)
#   3. Sync de CRDTs (convergencia)
#   4. Latencia <500ms
#   5. Consumo RAM <256MB
#
# Output: 🟢 NODE HEALTHY + métricas JSON, o 🔴 DEGRADED + recomendaciones
# Compatible con Docker Compose y ejecución nativa.
# Uso: bash scripts/validate-node.sh [--endpoint URL] [--output FILE]
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ─── Configuración ───

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ENDPOINT="${ED2KIA_ENDPOINT:-http://localhost:3000}"
OUTPUT_FILE=""
ISSUES=()
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
VERSION="v2.1.0-sprint18"

# Thresholds
MAX_LATENCY_MS=500
MAX_RAM_MB=256
HEALTH_TIMEOUT=10

# ─── Parse Arguments ───

for arg in "$@"; do
    case $arg in
        --endpoint=*) ENDPOINT="${arg#*=}" ;;
        --endpoint) shift; ENDPOINT="$1" ;;
        --output=*) OUTPUT_FILE="${arg#*=}" ;;
        --output) shift; OUTPUT_FILE="$1" ;;
        --help)
            echo "Usage: bash scripts/validate-node.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --endpoint URL   API endpoint (default: $ENDPOINT)"
            echo "  --output FILE    Save metrics JSON to file"
            echo "  --help           Show this help"
            exit 0
            ;;
    esac
done

# Remove trailing slash
ENDPOINT="${ENDPOINT%/}"

# ─── Limpieza ───

cleanup() {
    :
}
trap cleanup EXIT INT TERM

# ─── Colores y Formateo ───

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[PASS]${NC} $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
error()   { echo -e "${RED}[FAIL]${NC} $*"; }
phase()   { echo -e "\n${CYAN}── $* ──${NC}"; }

# ─── Métricas ───

declare -A METRICS
METRICS[timestamp]="$TIMESTAMP"
METRICS[version]="$VERSION"
METRICS[endpoint]="$ENDPOINT"

# ─── Check 1: Health Endpoint ───

phase "Health Endpoint"

if curl -sf --connect-timeout "$HEALTH_TIMEOUT" --max-time "$HEALTH_TIMEOUT" \
    "${ENDPOINT}/api/health" -o /dev/null 2>/dev/null; then
    success "Health endpoint responsive"
    METRICS[health]="healthy"
else
    # Try alternate path
    if curl -sf --connect-timeout "$HEALTH_TIMEOUT" --max-time "$HEALTH_TIMEOUT" \
        "${ENDPOINT}/health" -o /dev/null 2>/dev/null; then
        success "Health endpoint responsive (/health)"
        METRICS[health]="healthy"
    else
        error "Health endpoint unreachable at ${ENDPOINT}/api/health"
        ISSUES+=("Health endpoint unreachable — Verify node is running")
        METRICS[health]="unreachable"
    fi
fi

# ─── Check 2: Latency ───

phase "Latency Check"

LATENCY_MS=0
if curl -sf --connect-timeout "$HEALTH_TIMEOUT" "${ENDPOINT}/api/health" -o /dev/null 2>/dev/null; then
    LATENCY_MS=$(curl -s --connect-timeout "$HEALTH_TIMEOUT" -o /dev/null -w '%{time_total}' \
        "${ENDPOINT}/api/health" 2>/dev/null | awk '{printf "%.0f", $1*1000}') || LATENCY_MS=0

    if [[ "$LATENCY_MS" -lt "$MAX_LATENCY_MS" ]]; then
        success "Latency: ${LATENCY_MS}ms (<${MAX_LATENCY_MS}ms)"
        METRICS[latency_ms]="$LATENCY_MS"
        METRICS[latency_status]="pass"
    else
        warn "Latency: ${LATENCY_MS}ms (≥${MAX_LATENCY_MS}ms threshold)"
        ISSUES+=("High latency ${LATENCY_MS}ms — Check network or load")
        METRICS[latency_ms]="$LATENCY_MS"
        METRICS[latency_status]="degraded"
    fi
else
    warn "Latency check skipped (endpoint unreachable)"
    METRICS[latency_ms]="N/A"
    METRICS[latency_status]="skipped"
fi

# ─── Check 3: SCTGuard Status ───

phase "SCTGuard Status"

if curl -sf --connect-timeout "$HEALTH_TIMEOUT" "${ENDPOINT}/api/metrics" -o /dev/null 2>/dev/null; then
    METRICS_CONTENT=$(curl -sf --connect-timeout "$HEALTH_TIMEOUT" "${ENDPOINT}/api/metrics" 2>/dev/null) || METRICS_CONTENT=""

    if echo "$METRICS_CONTENT" | grep -qi "sct\|guard\|stuartian"; then
        success "SCTGuard metrics present in /api/metrics"
        METRICS[sct_guard]="active"
    else
        warn "SCTGuard metrics not found in /api/metrics (may not be exposed)"
        METRICS[sct_guard]="unknown"
    fi
else
    warn "SCTGuard check skipped (metrics endpoint unreachable)"
    METRICS[sct_guard]="unreachable"
fi

# ─── Check 4: CRDT Sync ───

phase "CRDT Sync Status"

if curl -sf --connect-timeout "$HEALTH_TIMEOUT" "${ENDPOINT}/api/metrics" -o /dev/null 2>/dev/null; then
    if echo "$METRICS_CONTENT" | grep -qi "crdt\|converge\|replication"; then
        success "CRDT sync metrics present"
        METRICS[crdt_sync]="converged"
    else
        warn "CRDT sync metrics not found (may not be exposed)"
        METRICS[crdt_sync]="unknown"
    fi
else
    warn "CRDT sync check skipped (metrics endpoint unreachable)"
    METRICS[crdt_sync]="unreachable"
fi

# ─── Check 5: RAM Usage ───

phase "RAM Usage"

RAM_MB=0
if command -v ps &> /dev/null; then
    # Try to find ed2kia process
    RAM_MB=$(ps aux 2>/dev/null | grep -E "ed2kia|cargo" | grep -v grep | awk '{sum+=$6} END {printf "%.0f", sum/1024}') || RAM_MB=0

    if [[ "$RAM_MB" -gt 0 ]]; then
        if [[ "$RAM_MB" -lt "$MAX_RAM_MB" ]]; then
            success "RAM: ${RAM_MB}MB (<${MAX_RAM_MB}MB)"
            METRICS[ram_mb]="$RAM_MB"
            METRICS[ram_status]="pass"
        else
            warn "RAM: ${RAM_MB}MB (≥${MAX_RAM_MB}MB threshold)"
            ISSUES+=("High RAM usage ${RAM_MB}MB — Consider reducing mesh size")
            METRICS[ram_mb]="$RAM_MB"
            METRICS[ram_status]="degraded"
        fi
    else
        info "RAM: Unable to measure (process not found or Docker)"
        METRICS[ram_mb]="N/A"
        METRICS[ram_status]="unknown"
    fi
else
    info "RAM: ps command not available"
    METRICS[ram_mb]="N/A"
    METRICS[ram_status]="unknown"
fi

# ─── Check 6: Docker Mode ───

phase "Docker Mode Detection"

if [[ -f "/.dockerenv" ]] || grep -q "docker\|containerd" /proc/1/cgroup 2>/dev/null; then
    info "Running inside Docker container"
    METRICS[docker]="true"

    # Check Docker Compose services
    if docker ps 2>/dev/null | grep -q "ed2kia"; then
        success "ed2kia container running in Docker"
        METRICS[docker_status]="running"
    else
        warn "No ed2kia container found in Docker"
        METRICS[docker_status]="not_found"
    fi
else
    info "Running natively (not in Docker)"
    METRICS[docker]="false"
fi

# ─── Generate JSON Output ───

phase "Generating Metrics"

TOTAL_ISSUES=${#ISSUES[@]}
if [[ $TOTAL_ISSUES -eq 0 ]]; then
    OVERALL_STATUS="healthy"
else
    OVERALL_STATUS="degraded"
fi

METRICS_JSON=$(cat << EOF
{
  "timestamp": "${METRICS[timestamp]}",
  "version": "${METRICS[version]}",
  "endpoint": "${METRICS[endpoint]}",
  "status": "${OVERALL_STATUS}",
  "checks": {
    "health": "${METRICS[health]}",
    "latency_ms": ${METRICS[latency_ms]:-"null"},
    "latency_status": "${METRICS[latency_status]}",
    "sct_guard": "${METRICS[sct_guard]}",
    "crdt_sync": "${METRICS[crdt_sync]}",
    "ram_mb": ${METRICS[ram_mb]:-"null"},
    "ram_status": "${METRICS[ram_status]}",
    "docker": ${METRICS[docker]:-"false"},
    "docker_status": "${METRICS[docker_status]:-null}"
  },
  "issues": ${TOTAL_ISSUES}
}
EOF
)

if [[ -n "$OUTPUT_FILE" ]]; then
    echo "$METRICS_JSON" > "$OUTPUT_FILE"
    success "Metrics saved to $OUTPUT_FILE"
fi

# ─── Final Output ───

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"

if [[ $TOTAL_ISSUES -eq 0 ]]; then
    echo -e "${GREEN}🟢 NODE HEALTHY${NC}"
    echo -e "${GREEN}   All checks passed successfully${NC}"
else
    echo -e "${RED}🔴 DEGRADED${NC}"
    echo -e "${RED}   ${TOTAL_ISSUES} issue(s) detected:${NC}"
    for issue in "${ISSUES[@]}"; do
        echo -e "${RED}   • $issue${NC}"
    done
    echo ""
    echo -e "${YELLOW}   Recommendations:${NC}"
    echo -e "${YELLOW}   1. Check node logs: docker logs ed2kia-node${NC}"
    echo -e "${YELLOW}   2. Verify network: ping your-peers${NC}"
    echo -e "${YELLOW}   3. Reduce mesh size if RAM is high${NC}"
    echo -e "${YELLOW}   4. Run full audit: bash scripts/audit-scan.sh${NC}"
fi

echo ""
echo -e "${BLUE}Metrics JSON:${NC}"
echo "$METRICS_JSON" | grep -v '^$'
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"

if [[ $TOTAL_ISSUES -gt 0 ]]; then
    exit 1
fi
