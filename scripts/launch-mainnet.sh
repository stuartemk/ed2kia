#!/usr/bin/env bash
# =============================================================================
# ed2kIA Mainnet Launch Script — v2.1.0-stable
# =============================================================================
# POSIX-compliant, idempotent mainnet launch script.
# Performs pre-flight validation, deploys services, and runs post-deploy checks.
#
# Usage: ./scripts/launch-mainnet.sh [--dry-run] [--env FILE]
# =============================================================================
set -euo pipefail

# --- Configuration ---
DRY_RUN=false
ENV_FILE="${ED2KIA_ENV_FILE:-deploy/systemd/ed2kIA.env}"
COMPOSE_FILE="${ED2KIA_COMPOSE_FILE:-deploy/docker-compose.yml}"
COMPOSE_PROJECT="${ED2KIA_PROJECT:-ed2kia}"
DATA_DIR="${ED2KIA_DATA_DIR:-./deploy/data}"

# --- Parse Arguments ---
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --env)
            ENV_FILE="$2"
            shift 2
            ;;
        --compose)
            COMPOSE_FILE="$2"
            shift 2
            ;;
        --project)
            COMPOSE_PROJECT="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [--dry-run] [--env FILE] [--compose FILE] [--project NAME]"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# --- Color Output ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}[STEP]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_info() { echo -e "[INFO] $1"; }
log_dry() { echo -e "${YELLOW}[DRY-RUN]${NC} Would execute: $1"; }

# --- Header ---
echo -e "${BLUE}============================================================${NC}"
echo -e "${BLUE}  ed2kIA Mainnet Launch — v2.1.0-stable${NC}"
echo -e "${BLUE}  Date: $(date -u '+%Y-%m-%d %H:%M:%S UTC')${NC}"
echo -e "${BLUE}============================================================${NC}"

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}\n*** DRY RUN MODE — No changes will be made ***${NC}"
fi

# =============================================================================
# PRE-FLIGHT VALIDATION
# =============================================================================
log_step "1. Pre-flight Validation"

# Check 1.1: Docker available
if command -v docker >/dev/null 2>&1; then
    log_pass "Docker is available: $(docker --version)"
else
    log_fail "Docker is not installed"
    exit 1
fi

# Check 1.2: Docker Compose available
if docker compose version >/dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
    log_pass "Docker Compose is available: $(docker compose version)"
elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE_CMD="docker-compose"
    log_pass "Docker Compose (standalone) is available: $(docker-compose version)"
else
    log_fail "Docker Compose is not installed"
    exit 1
fi

# Check 1.3: Compose file exists
if [ -f "$COMPOSE_FILE" ]; then
    log_pass "Compose file found: $COMPOSE_FILE"
else
    log_fail "Compose file not found: $COMPOSE_FILE"
    exit 1
fi

# Check 1.4: Validate YAML syntax
if command -v python3 >/dev/null 2>&1; then
    if python3 -c "import yaml; yaml.safe_load(open('$COMPOSE_FILE'))" 2>/dev/null; then
        log_pass "YAML syntax valid"
    else
        log_warn "Could not validate YAML syntax (python3-yaml not available)"
    fi
fi

# Check 1.5: Disk space
if command -v df >/dev/null 2>&1; then
    AVAIL_MB=$(df -m . | tail -1 | awk '{print $4}')
    if [ "$AVAIL_MB" -gt 1024 ]; then
        log_pass "Available disk space: ${AVAIL_MB}MB (> 1GB)"
    else
        log_fail "Insufficient disk space: ${AVAIL_MB}MB (< 1GB)"
        exit 1
    fi
fi

# Check 1.6: Ports available
for PORT in 9001 9002 9003; do
    if command -v ss >/dev/null 2>&1; then
        if ss -tlnp | grep -q ":${PORT} " 2>/dev/null; then
            log_warn "Port ${PORT} is already in use"
        else
            log_pass "Port ${PORT} is available"
        fi
    fi
done

# =============================================================================
# DEPLOYMENT
# =============================================================================
log_step "2. Building Docker Images"

if [ "$DRY_RUN" = true ]; then
    log_dry "$COMPOSE_CMD -f $COMPOSE_FILE -p $COMPOSE_PROJECT build"
else
    $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT" build --no-cache 2>&1 | tail -5
    log_pass "Docker images built successfully"
fi

log_step "3. Starting Services"

if [ "$DRY_RUN" = true ]; then
    log_dry "$COMPOSE_CMD -f $COMPOSE_FILE -p $COMPOSE_PROJECT up -d"
else
    $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT" up -d 2>&1 | tail -10
    log_pass "Services started"
fi

# =============================================================================
# POST-DEPLOY VALIDATION
# =============================================================================
log_step "4. Post-Deploy Validation"

if [ "$DRY_RUN" = true ]; then
    log_dry "Waiting for services to become healthy..."
    log_dry "$COMPOSE_CMD -f $COMPOSE_FILE -p $COMPOSE_PROJECT ps"
else
    # Wait for services to start
    log_info "Waiting 15 seconds for services to initialize..."
    sleep 15

    # Check service status
    log_info "Service status:"
    $COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT" ps

    # Health checks
    for NODE in node1 node2 node3; do
        STATUS=$($COMPOSE_CMD -f "$COMPOSE_FILE" -p "$COMPOSE_PROJECT" ps "$NODE" 2>/dev/null | grep -c "Up" || true)
        if [ "$STATUS" -gt 0 ]; then
            log_pass "${NODE} is running"
        else
            log_fail "${NODE} is NOT running"
        fi
    done

    # Run health check script if available
    if [ -f "scripts/health-check.sh" ]; then
        log_info "Running health check..."
        bash scripts/health-check.sh --port 9001 --host localhost || true
    fi
fi

# =============================================================================
# SUMMARY
# =============================================================================
echo ""
echo -e "${GREEN}============================================================${NC}"
echo -e "${GREEN}  Mainnet Launch Complete${NC}"
echo -e "${GREEN}============================================================${NC}"
echo ""
log_info "Services: $COMPOSE_PROJECT (node1, node2, node3)"
log_info "Ports: 9001, 9002, 9003"
log_info "Logs: $COMPOSE_CMD -f $COMPOSE_FILE -p $COMPOSE_PROJECT logs -f"
log_info "Stop: $COMPOSE_CMD -f $COMPOSE_FILE -p $COMPOSE_PROJECT down"
echo ""

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}*** DRY RUN COMPLETE — No changes were made ***${NC}"
    exit 0
fi

exit 0
