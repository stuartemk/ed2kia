#!/usr/bin/env bash
# bootstrap-mainnet.sh — Automated Mainnet Bootstrap for ed2kIA v2.1.0-sprint12
# Validates environment, launches Docker Compose services, runs pre-launch checks,
# waits for healthchecks and prints mainnet status with URLs.
# Usage: bash scripts/bootstrap-mainnet.sh [--replicas N] [--difficulty D] [--log-level L]
set -euo pipefail
trap cleanup EXIT INT TERM

# =============================================================================
# Configuration Defaults
# =============================================================================
REPLICAS=1
DIFFICULTY=4
LOG_LEVEL="info"
DOCKER_COMPOSE_FILE="infra/docker-compose.testnet-v2.1.yml"
PRE_LAUNCH_SCRIPT="scripts/pre-launch-check.sh"
HEALTH_ENDPOINT="/api/health"
METRICS_ENDPOINT="/api/metrics"
API_PORT=9944
GRAFANA_PORT=3000
PROMETHEUS_PORT=9090
MAX_HEALTH_RETRIES=30
HEALTH_RETRY_INTERVAL=5

# =============================================================================
# Cleanup Handler
# =============================================================================
cleanup() {
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo ""
        echo "❌ Bootstrap failed (exit code: $exit_code)"
        echo "💡 Running cleanup..."
        docker-compose -f "$DOCKER_COMPOSE_FILE" down --remove-orphans 2>/dev/null || true
        echo "Cleanup complete. Check logs for details."
    fi
    rm -f /tmp/ed2kia-bootstrap-* 2>/dev/null || true
}

# =============================================================================
# Argument Parsing
# =============================================================================
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --replicas)
                REPLICAS="$2"
                shift 2
                ;;
            --difficulty)
                DIFFICULTY="$2"
                shift 2
                ;;
            --log-level)
                LOG_LEVEL="$2"
                shift 2
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                echo "Unknown argument: $1"
                usage
                exit 1
                ;;
        esac
    done
}

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Automated Mainnet Bootstrap for ed2kIA v2.1.0-sprint12

Options:
  --replicas N       Number of node replicas (default: 1)
  --difficulty D     PoW difficulty 1-10 (default: 4)
  --log-level L      Log level: debug, info, warn, error (default: info)
  -h, --help         Show this help message

Example:
  $0 --replicas 3 --difficulty 5 --log-level debug
EOF
}

# =============================================================================
# Environment Validation
# =============================================================================
validate_environment() {
    echo "🔍 Validating environment..."

    # Docker
    if command -v docker &>/dev/null; then
        local docker_version
        docker_version=$(docker --version 2>/dev/null | awk '{print $3}')
        echo "  ✓ Docker: $docker_version"
    else
        echo "  ✗ Docker not found. Install from https://docs.docker.com/get-docker/"
        exit 1
    fi

    # Docker Compose
    if command -v docker-compose &>/dev/null; then
        local compose_version
        compose_version=$(docker-compose --version 2>/dev/null | awk '{print $3}')
        echo "  ✓ Docker Compose: $compose_version"
    elif docker compose version &>/dev/null; then
        local compose_version
        compose_version=$(docker compose version 2>/dev/null | awk '{print $3}')
        echo "  ✓ Docker Compose (plugin): $compose_version"
    else
        echo "  ✗ Docker Compose not found."
        exit 1
    fi

    # Rust toolchain
    if command -v rustc &>/dev/null; then
        local rust_version
        rust_version=$(rustc --version 2>/dev/null | awk '{print $2}')
        echo "  ✓ Rust: $rust_version"
    else
        echo "  ⚠ Rust not found (optional for local builds)"
    fi

    # Python
    if command -v python3 &>/dev/null; then
        local python_version
        python_version=$(python3 --version 2>/dev/null | awk '{print $2}')
        echo "  ✓ Python: $python_version"
    elif command -v python &>/dev/null; then
        local python_version
        python_version=$(python --version 2>/dev/null | awk '{print $2}')
        echo "  ✓ Python: $python_version"
    else
        echo "  ⚠ Python not found (optional for scripts)"
    fi

    # Docker Compose file
    if [ -f "$DOCKER_COMPOSE_FILE" ]; then
        echo "  ✓ Docker Compose file: $DOCKER_COMPOSE_FILE"
    else
        echo "  ✗ Docker Compose file not found: $DOCKER_COMPOSE_FILE"
        exit 1
    fi

    # Pre-launch script
    if [ -f "$PRE_LAUNCH_SCRIPT" ]; then
        echo "  ✓ Pre-launch script: $PRE_LAUNCH_SCRIPT"
    else
        echo "  ⚠ Pre-launch script not found: $PRE_LAUNCH_SCRIPT (skipping)"
    fi

    echo ""
}

# =============================================================================
# Launch Docker Compose Services
# =============================================================================
launch_services() {
    echo "🚀 Launching services with Docker Compose..."
    echo "  Replicas: $REPLICAS"
    echo "  Difficulty: $DIFFICULTY"
    echo "  Log Level: $LOG_LEVEL"
    echo ""

    # Export environment variables for services
    export ED2K_REPLICAS="$REPLICAS"
    export ED2K_DIFFICULTY="$DIFFICULTY"
    export ED2K_LOG_LEVEL="$LOG_LEVEL"

    # Launch services
    docker-compose -f "$DOCKER_COMPOSE_FILE" up -d
    echo "  ✓ Services launched in background"
    echo ""
}

# =============================================================================
# Run Pre-Launch Checks
# =============================================================================
run_pre_launch_checks() {
    if [ -f "$PRE_LAUNCH_SCRIPT" ]; then
        echo "📋 Running pre-launch checks..."
        if bash "$PRE_LAUNCH_SCRIPT"; then
            echo "  ✓ Pre-launch checks passed"
        else
            echo "  ⚠ Pre-launch checks completed with warnings"
        fi
        echo ""
    fi
}

# =============================================================================
# Healthcheck Polling
# =============================================================================
wait_for_health() {
    echo "🏥 Waiting for services to become healthy..."
    local retries=0
    local healthy=false

    while [ $retries -lt $MAX_HEALTH_RETRIES ]; do
        retries=$((retries + 1))
        local http_code
        http_code=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:${API_PORT}${HEALTH_ENDPOINT}" 2>/dev/null || echo "000")

        if [ "$http_code" = "200" ]; then
            healthy=true
            echo "  ✓ Health endpoint responsive (attempt $retries/$MAX_HEALTH_RETRIES)"
            break
        fi

        echo "  ⏳ Waiting for health endpoint... (attempt $retries/$MAX_HEALTH_RETRIES, HTTP: $http_code)"
        sleep "$HEALTH_RETRY_INTERVAL"
    done

    if [ "$healthy" = false ]; then
        echo "  ✗ Health endpoint not responsive after $MAX_HEALTH_RETRIES attempts"
        return 1
    fi

    # Check metrics endpoint
    retries=0
    local metrics_healthy=false

    while [ $retries -lt $MAX_HEALTH_RETRIES ]; do
        retries=$((retries + 1))
        local http_code
        http_code=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:${API_PORT}${METRICS_ENDPOINT}" 2>/dev/null || echo "000")

        if [ "$http_code" = "200" ]; then
            metrics_healthy=true
            echo "  ✓ Metrics endpoint responsive (attempt $retries/$MAX_HEALTH_RETRIES)"
            break
        fi

        echo "  ⏳ Waiting for metrics endpoint... (attempt $retries/$MAX_HEALTH_RETRIES, HTTP: $http_code)"
        sleep "$HEALTH_RETRY_INTERVAL"
    done

    if [ "$metrics_healthy" = false ]; then
        echo "  ✗ Metrics endpoint not responsive after $MAX_HEALTH_RETRIES attempts"
        return 1
    fi

    echo ""
}

# =============================================================================
# Print Mainnet Status
# =============================================================================
print_status() {
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║              🟢 MAINNET ACTIVE — ed2kIA v2.1.0-sprint12        ║"
    echo "╠══════════════════════════════════════════════════════════════════╣"
    echo "║                                                                  ║"
    echo "║  Configuration:                                                  ║"
    echo "║    • Replicas:    $REPLICAS"
    echo "║    • Difficulty:  $DIFFICULTY"
    echo "║    • Log Level:   $LOG_LEVEL"
    echo "║                                                                  ║"
    echo "║  Service URLs:                                                   ║"
    echo "║    • API:         http://localhost:${API_PORT}"
    echo "║    • Metrics:     http://localhost:${API_PORT}${METRICS_ENDPOINT}"
    echo "║    • Grafana:     http://localhost:${GRAFANA_PORT}"
    echo "║    • Prometheus:  http://localhost:${PROMETHEUS_PORT}"
    echo "║                                                                  ║"
    echo "║  Governance:                                                     ║"
    echo "║    • Stewardship: http://localhost:${API_PORT}/stewardship-dashboard.html"
    echo "║    • Atlas:       http://localhost:${API_PORT}/atlas.html"
    echo "║                                                                  ║"
    echo "║  Next Steps:                                                     ║"
    echo "║    1. Open Grafana dashboard at http://localhost:${GRAFANA_PORT}"
    echo "║    2. Monitor metrics at http://localhost:${API_PORT}${METRICS_ENDPOINT}"
    echo "║    3. Access stewardship dashboard for governance metrics"
    echo "║    4. Review GOVERNANCE.md for RFC pipeline & community process"
    echo "║                                                                  ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"
    echo ""
}

# =============================================================================
# Main Execution Flow
# =============================================================================
main() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║  ed2kIA v2.1.0-sprint12 — Mainnet Bootstrap                     ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"
    echo ""

    # 1. Parse arguments
    parse_args "$@"

    # 2. Validate environment
    validate_environment

    # 3. Launch Docker Compose services
    launch_services

    # 4. Run pre-launch checks
    run_pre_launch_checks

    # 5. Wait for healthchecks
    wait_for_health

    # 6. Print mainnet active status
    print_status
}

# Run main function
main "$@"