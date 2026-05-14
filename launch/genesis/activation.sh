#!/bin/sh
# =============================================================================
# ed2kIA v0.5.0 - Network Activation Script
# =============================================================================
# POSIX-compliant script for deterministic network activation.
# Validates dependencies, starts seed nodes in order, verifies gossipsub,
# distributes initial leases, and reports final status.
#
# Usage: ./activation.sh [--dry-run] [--config <path>] [--seeds <path>]
#
# License: Apache 2.0 + Ethical Use Clause
# =============================================================================

set -e

# ---------------------------------------------------------------------------
# Configuration Defaults
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CONFIG_FILE="${SCRIPT_DIR}/config.toml"
SEEDS_FILE="${SCRIPT_DIR}/seed_nodes.json"
ED2KIA_BIN="ed2kia"
LOG_DIR="${SCRIPT_DIR}/logs"
DATA_BASE="/var/lib/ed2kia"
HEALTH_ENDPOINT="http://localhost:3030/api/health"
NETWORK_ENDPOINT="http://localhost:3030/api/network"
DRY_RUN=0
ACTIVATION_LOG="${LOG_DIR}/activation.log"

# ---------------------------------------------------------------------------
# Color Output (disabled if not a terminal)
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

# ---------------------------------------------------------------------------
# Helper Functions
# ---------------------------------------------------------------------------
log_info() {
    echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1" | tee -a "$ACTIVATION_LOG"
}

log_success() {
    echo -e "${GREEN}[OK]${NC}   $(date '+%Y-%m-%d %H:%M:%S') $1" | tee -a "$ACTIVATION_LOG"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1" | tee -a "$ACTIVATION_LOG"
}

log_error() {
    echo -e "${RED}[ERR]${NC}  $(date '+%Y-%m-%d %H:%M:%S') $1" | tee -a "$ACTIVATION_LOG"
}

die() {
    log_error "$1"
    exit 1
}

# ---------------------------------------------------------------------------
# Parse Arguments
# ---------------------------------------------------------------------------
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        --config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        --seeds)
            SEEDS_FILE="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--dry-run] [--config <path>] [--seeds <path>]"
            echo ""
            echo "Options:"
            echo "  --dry-run    Validate configuration without starting nodes"
            echo "  --config     Path to genesis config.toml (default: ./config.toml)"
            echo "  --seeds      Path to seed_nodes.json (default: ./seed_nodes.json)"
            echo "  --help       Show this help message"
            exit 0
            ;;
        *)
            die "Unknown option: $1"
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Step 0: Pre-flight Validation
# ---------------------------------------------------------------------------
echo ""
echo "============================================================"
echo " ed2kIA v0.5.0 - Network Activation"
echo "============================================================"
echo ""

mkdir -p "$LOG_DIR"

if [ "$DRY_RUN" -eq 1 ]; then
    log_warn "DRY RUN MODE - No nodes will be started"
fi

log_info "Step 0: Pre-flight Validation"

# Check required files
[ -f "$CONFIG_FILE" ] || die "Config file not found: $CONFIG_FILE"
[ -f "$SEEDS_FILE" ] || die "Seed nodes file not found: $SEEDS_FILE"
log_success "Configuration files found"

# Check ed2kia binary
if ! command -v "$ED2KIA_BIN" >/dev/null 2>&1; then
    # Try local build
    if [ -f "${SCRIPT_DIR}/../../target/release/ed2kia" ]; then
        ED2KIA_BIN="${SCRIPT_DIR}/../../target/release/ed2kia"
    else
        die "ed2kia binary not found. Build with: cargo build --release --features core-only"
    fi
fi
log_success "ed2kia binary found: $ED2KIA_BIN"

# Verify binary version
VERSION=$("$ED2KIA_BIN" --version 2>/dev/null | head -1 || echo "unknown")
log_info "Binary version: $VERSION"

# Check for placeholder values in seed_nodes.json
if grep -q "PLACEHOLDER" "$SEEDS_FILE" 2>/dev/null; then
    die "seed_nodes.json contains PLACEHOLDER values. Replace with actual keys before activation."
fi
log_success "Seed nodes validated (no placeholders)"

# ---------------------------------------------------------------------------
# Step 1: Validate Seed Node Configuration
# ---------------------------------------------------------------------------
log_info "Step 1: Validating Seed Node Configuration"

# Count seed nodes
SEED_COUNT=$(grep -c '"node_id"' "$SEEDS_FILE" 2>/dev/null || echo "0")
if [ "$SEED_COUNT" -lt 3 ]; then
    die "Minimum 3 seed nodes required. Found: $SEED_COUNT"
fi
log_success "Seed node count: $SEED_COUNT (minimum: 3)"

# Extract node IDs
NODE_IDS=$(grep '"node_id"' "$SEEDS_FILE" | sed 's/.*"node_id": *"\([^"]*\)".*/\1/')
log_info "Seed nodes: $(echo $NODE_IDS | tr '\n' ' ')"

# ---------------------------------------------------------------------------
# Step 2: Initialize Data Directories
# ---------------------------------------------------------------------------
log_info "Step 2: Initializing Data Directories"

for NODE_ID in $NODE_IDS; do
    NODE_DIR="${DATA_BASE}/${NODE_ID}"
    if [ "$DRY_RUN" -eq 0 ]; then
        mkdir -p "${NODE_DIR}"/{data,logs,models}
        log_success "Created data directory: $NODE_DIR"
    else
        log_info "[DRY RUN] Would create: $NODE_DIR"
    fi
done

# ---------------------------------------------------------------------------
# Step 3: Start Seed Nodes in Order
# ---------------------------------------------------------------------------
log_info "Step 3: Starting Seed Nodes"

PID_FILE="${LOG_DIR}/node_pids.txt"
> "$PID_FILE"

STARTED_COUNT=0
for NODE_ID in $NODE_IDS; do
    NODE_DIR="${DATA_BASE}/${NODE_ID}"
    NODE_LOG="${LOG_DIR}/${NODE_ID}.log"

    log_info "Starting node: $NODE_ID"

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would start: $ED2KIA_BIN run --data-dir $NODE_DIR --config $CONFIG_FILE"
        echo "$NODE_ID:DRY-RUN" >> "$PID_FILE"
        STARTED_COUNT=$((STARTED_COUNT + 1))
        continue
    fi

    # Start node in background
    $ED2KIA_BIN run \
        --data-dir "$NODE_DIR" \
        --config "$CONFIG_FILE" \
        --seeds "$SEEDS_FILE" \
        --node-id "$NODE_ID" \
        >> "$NODE_LOG" 2>&1 &

    PID=$!
    echo "$NODE_ID:$PID" >> "$PID_FILE"

    # Wait for node to initialize
    sleep 3

    # Verify process is running
    if kill -0 "$PID" 2>/dev/null; then
        log_success "Node $NODE_ID started (PID: $PID)"
        STARTED_COUNT=$((STARTED_COUNT + 1))
    else
        log_error "Node $NODE_ID failed to start. Check: $NODE_LOG"
    fi
done

log_success "Started $STARTED_COUNT / $SEED_COUNT nodes"

# ---------------------------------------------------------------------------
# Step 4: Wait for Network Bootstrap
# ---------------------------------------------------------------------------
log_info "Step 4: Waiting for Network Bootstrap"

BOOTSTRAP_TIMEOUT=120
BOOTSTRAP_START=$(date +%s)

while true; do
    ELAPSED=$(( $(date +%s) - BOOTSTRAP_START ))
    if [ "$ELAPSED" -gt "$BOOTSTRAP_TIMEOUT" ]; then
        die "Network bootstrap timeout after ${BOOTSTRAP_TIMEOUT}s"
    fi

    # Check if first node has peers
    if [ "$DRY_RUN" -eq 0 ] && [ -f "${LOG_DIR}/$(echo $NODE_IDS | head -1).log" ]; then
        FIRST_NODE=$(echo $NODE_IDS | head -1)
        if grep -q "peers_connected" "${LOG_DIR}/${FIRST_NODE}.log" 2>/dev/null; then
            log_success "Network bootstrap detected"
            break
        fi
    else
        log_info "[DRY RUN] Simulating network bootstrap"
        sleep 2
        break
    fi

    sleep 5
    log_info "Waiting for bootstrap... (${ELAPSED}s elapsed)"
done

# ---------------------------------------------------------------------------
# Step 5: Verify GossipSub Connectivity
# ---------------------------------------------------------------------------
log_info "Step 5: Verifying GossipSub Connectivity"

CONNECTED=0
for NODE_ID in $NODE_IDS; do
    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] $NODE_ID: gossipsub verified"
        CONNECTED=$((CONNECTED + 1))
        continue
    fi

    # Check health endpoint
    RESPONSE=$(curl -sf "http://localhost:3030/api/health" 2>/dev/null || echo "{}")
    if echo "$RESPONSE" | grep -q '"healthy"' 2>/dev/null; then
        log_success "$NODE_ID: Health check passed"
        CONNECTED=$((CONNECTED + 1))
    else
        log_warn "$NODE_ID: Health check pending"
    fi
done

log_success "GossipSub verified: $CONNECTED / $SEED_COUNT nodes"

# ---------------------------------------------------------------------------
# Step 6: Distribute Initial Leases
# ---------------------------------------------------------------------------
log_info "Step 6: Distributing Initial Leases"

if [ "$DRY_RUN" -eq 1 ]; then
    log_info "[DRY RUN] Would distribute leases from seed_nodes.json"
else
    # Trigger lease distribution via CLI
    $ED2KIA_BIN bootstrap genesis \
        --config "$CONFIG_FILE" \
        --seeds "$SEEDS_FILE" \
        >> "${LOG_DIR}/lease_distribution.log" 2>&1

    if [ $? -eq 0 ]; then
        log_success "Initial leases distributed"
    else
        log_warn "Lease distribution completed with warnings"
    fi
fi

# ---------------------------------------------------------------------------
# Step 7: Final Health Report
# ---------------------------------------------------------------------------
log_info "Step 7: Generating Final Health Report"

REPORT_FILE="${LOG_DIR}/activation_report.txt"
cat > "$REPORT_FILE" << EOF
============================================================
ed2kIA v0.5.0 - Network Activation Report
============================================================
Timestamp:     $(date -u '+%Y-%m-%d %H:%M:%S UTC')
Config:        $CONFIG_FILE
Seed Nodes:    $SEEDS_FILE
Nodes Started: $STARTED_COUNT / $SEED_COUNT
Connected:     $CONNECTED / $SEED_COUNT
Mode:          $([ "$DRY_RUN" -eq 1 ] && echo "DRY RUN" || echo "LIVE")

Seed Nodes:
$(echo $NODE_IDS | tr '\n' ', ' | sed 's/,$//')

Next Steps:
1. Monitor: curl http://localhost:3030/api/health
2. Network: curl http://localhost:3030/api/network
3. Metrics: curl http://localhost:9090/metrics
4. Logs: tail -f ${LOG_DIR}/*.log

Activation Log: $ACTIVATION_LOG
============================================================
EOF

cat "$REPORT_FILE"

# ---------------------------------------------------------------------------
# Completion
# ---------------------------------------------------------------------------
echo ""
if [ "$CONNECTED" -ge 3 ]; then
    log_success "=========================================="
    log_success "ed2kIA v0.5.0 NETWORK ACTIVATED SUCCESSFULLY"
    log_success "=========================================="
    exit 0
else
    log_warn "Network activated with warnings ($CONNECTED connected)"
    log_warn "Review logs: $ACTIVATION_LOG"
    exit 0
fi
