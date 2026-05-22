#!/bin/sh
# =============================================================================
# ed2kIA Testnet Mode Script — Isolated Testnet Configuration
# =============================================================================
# POSIX-compliant, idempotent
# Usage: ./scripts/testnet-mode.sh [OPTIONS]
#
# This script configures and launches an isolated ed2kIA testnet for:
#   - Local development and testing
#   - Contributor onboarding
#   - Feature validation before mainnet
#   - Educational demonstrations
#
# Testnet characteristics:
#   - Isolated from mainnet (separate peer table, data directory)
#   - Reduced consensus thresholds for faster validation
#   - Debug logging enabled by default
#   - Auto-generated bootstrap peers
#   - Clean state (no mainnet data contamination)
#
# Options:
#   --nodes N         Number of nodes to launch (default: 3)
#   --port BASE       Base port for first node (default: 18080)
#   --data DIR        Data directory (default: ~/.ed2kIA/testnet)
#   --clean           Clean testnet data before starting
#   --dry-run         Show configuration without launching
#   --help            Show this help message
#
# Environment Variables:
#   ED2KIA_TESTNET_NODES    — Number of nodes (overrides --nodes)
#   ED2KIA_TESTNET_PORT     — Base port (overrides --port)
#   ED2KIA_TESTNET_DATA     — Data directory (overrides --data)
# =============================================================================

set -e

# ─── Color Codes ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# ─── Logging ─────────────────────────────────────────────────────────────────
log_info()  { printf "${BLUE}[INFO]${NC} %s\n" "$1"; }
log_ok()    { printf "${GREEN}[OK]${NC}   %s\n" "$1"; }
log_warn()  { printf "${YELLOW}[WARN]${NC} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${NC} %s\n" "$1" >&2; }
log_node()  { printf "${CYAN}[NODE %s]${NC} %s\n" "$1" "$2"; }

# ─── Defaults ────────────────────────────────────────────────────────────────
NODES="${ED2KIA_TESTNET_NODES:-3}"
BASE_PORT="${ED2KIA_TESTNET_PORT:-18080}"
DATA_DIR="${ED2KIA_TESTNET_DATA:-$HOME/.ed2kIA/testnet}"
CLEAN=0
DRY_RUN=0
ED2KIA_DIR=""
ED2KIA_FEATURES="stable"

# ─── Usage ───────────────────────────────────────────────────────────────────
usage() {
    cat <<'EOF'
ed2kIA Testnet Mode — Isolated Testnet Configuration

USAGE:
    ./scripts/testnet-mode.sh [OPTIONS]

OPTIONS:
    --nodes N         Number of nodes to launch (default: 3)
    --port BASE       Base port for first node (default: 18080)
    --data DIR        Data directory (default: ~/.ed2kIA/testnet)
    --clean           Clean testnet data before starting
    --dry-run         Show configuration without launching
    --help            Show this help message

ENVIRONMENT:
    ED2KIA_TESTNET_NODES    Number of nodes (overrides --nodes)
    ED2KIA_TESTNET_PORT     Base port (overrides --port)
    ED2KIA_TESTNET_DATA     Data directory (overrides --data)

EXAMPLES:
    # Launch default 3-node testnet
    ./scripts/testnet-mode.sh

    # Launch 5-node testnet with clean state
    ./scripts/testnet-mode.sh --nodes 5 --clean

    # Dry run to see configuration
    ./scripts/testnet-mode.sh --dry-run

    # Custom ports and data directory
    ./scripts/testnet-mode.sh --port 20000 --data /tmp/ed2k-testnet
EOF
}

# ─── Parse Arguments ─────────────────────────────────────────────────────────
parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --nodes)
                NODES="$2"
                shift 2
                ;;
            --port)
                BASE_PORT="$2"
                shift 2
                ;;
            --data)
                DATA_DIR="$2"
                shift 2
                ;;
            --clean)
                CLEAN=1
                shift
                ;;
            --dry-run)
                DRY_RUN=1
                shift
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# ─── Pre-Flight ──────────────────────────────────────────────────────────────
preflight() {
    log_info "=== ed2kIA Testnet Mode ==="
    log_info "Configuration: ${NODES} nodes, base port ${BASE_PORT}, data ${DATA_DIR}"

    # Find ed2kIA directory
    if [ -d "../../Cargo.toml" ]; then
        ED2KIA_DIR="$(cd ../.. && pwd)"
    elif [ -d "../Cargo.toml" ]; then
        ED2KIA_DIR="$(cd .. && pwd)"
    elif [ -d "Cargo.toml" ]; then
        ED2KIA_DIR="$(pwd)"
    else
        log_error "Cannot find ed2kIA project root (Cargo.toml not found)"
        exit 1
    fi

    log_ok "Project root: ${ED2KIA_DIR}"

    # Check cargo
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo not found. Install Rust first."
        exit 1
    fi
}

# ─── Clean Testnet Data ──────────────────────────────────────────────────────
clean_testnet() {
    if [ "$CLEAN" -eq 1 ]; then
        log_info "Cleaning testnet data at ${DATA_DIR}..."
        rm -rf "${DATA_DIR}"
        log_ok "Testnet data cleaned"
    fi
}

# ─── Generate Node Configuration ─────────────────────────────────────────────
generate_config() {
    node_index=$1
    node_port=$((BASE_PORT + node_index))
    metrics_port=$((BASE_PORT + node_index + 1000))
    node_dir="${DATA_DIR}/node${node_index}"

    mkdir -p "${node_dir}/data"
    mkdir -p "${node_dir}/logs"

    # Generate peer ID for this node
    PEER_ID="testnet-node${node_index}"

    # Collect bootstrap peers (all other nodes)
    BOOTSTRAP_PEERS=""
    i=0
    while [ $i -lt $NODES ]; do
        if [ $i -ne $node_index ]; then
            PEER_PORT=$((BASE_PORT + i))
            if [ -n "$BOOTSTRAP_PEERS" ]; then
                BOOTSTRAP_PEERS="${BOOTSTRAP_PEERS}, "
            fi
            BOOTSTRAP_PEERS="${BOOTSTRAP_PEERS}\"/ip4/127.0.0.1/tcp/${PEER_PORT}/p2p/testnet-node${i}\""
        fi
        i=$((i + 1))
    done

    cat > "${node_dir}/config.toml" <<EOF
# ed2kIA Testnet Node ${node_index} Configuration
# Generated by testnet-mode.sh

[node]
id = "${PEER_ID}"
listen_addr = "127.0.0.1:${node_port}"
public_addr = "127.0.0.1:${node_port}"
role = "contributor"

[network]
testnet = true
bootstrap_peers = [${BOOTSTRAP_PEERS}]
max_connections = 10
message_size_limit = 1048576  # 1MB for testnet
discovery = "mdns"

[sae]
features = "${ED2KIA_FEATURES}"
latent_dim = 1024
top_k = 64
mock_inference = true

[consensus]
min_verifiers = 1
quorum_threshold = 0.5
timeout_ms = 5000

[economics]
ce_initial_balance = 10.0
ce_emit_rate = 1.0
ce_burn_rate = 0.5

[logging]
level = "debug"
file = "${node_dir}/logs/node.log"
console = true

[metrics]
enabled = true
port = ${metrics_port}
path = "/metrics"

[storage]
data_dir = "${node_dir}/data"
max_size_mb = 100
EOF

    log_node "node${node_index}" "Config: ${node_dir}/config.toml (port ${node_port}, metrics ${metrics_port})"
}

# ─── Generate Testnet Manifest ───────────────────────────────────────────────
generate_manifest() {
    cat > "${DATA_DIR}/testnet-manifest.json" <<EOF
{
    "version": "v2.1.0-stable",
    "testnet_id": "ed2kIA-testnet-$(date +%s)",
    "created_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date)",
    "nodes": ${NODES},
    "base_port": ${BASE_PORT},
    "data_dir": "${DATA_DIR}",
    "features": "${ED2KIA_FEATURES}",
    "nodes_config": [
EOF

    i=0
    while [ $i -lt $NODES ]; do
        node_port=$((BASE_PORT + i))
        metrics_port=$((BASE_PORT + i + 1000))
        COMMA=","
        if [ $i -eq $((NODES - 1)) ]; then
            COMMA=""
        fi
        cat >> "${DATA_DIR}/testnet-manifest.json" <<EOF
        {
            "index": ${i},
            "id": "testnet-node${i}",
            "port": ${node_port},
            "metrics_port": ${metrics_port},
            "config": "${DATA_DIR}/node${i}/config.toml"
        }${COMMA}
EOF
        i=$((i + 1))
    done

    cat >> "${DATA_DIR}/testnet-manifest.json" <<EOF
    ]
}
EOF

    log_ok "Testnet manifest: ${DATA_DIR}/testnet-manifest.json"
}

# ─── Launch Nodes ────────────────────────────────────────────────────────────
launch_nodes() {
    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "DRY RUN: Nodes configured but not launched"
        return
    fi

    log_info "Launching ${NODES} testnet nodes..."

    # Build first if needed
    if [ ! -f "${ED2KIA_DIR}/target/release/ed2kIA" ]; then
        log_info "Building release binary..."
        cd "${ED2KIA_DIR}"
        cargo build --release --features "${ED2KIA_FEATURES}" 2>&1 | tail -3
        log_ok "Build complete"
    fi

    # Launch each node
    i=0
    PIDS=""
    while [ $i -lt $NODES ]; do
        node_dir="${DATA_DIR}/node${i}"
        log_node "node${i}" "Starting..."

        cd "${ED2KIA_DIR}"
        nohup cargo run --release --features "${ED2KIA_FEATURES}" -- \
            --config "${node_dir}/config.toml" \
            > "${node_dir}/logs/stdout.log" 2>&1 &

        PID=$!
        PIDS="${PIDS} ${PID}"
        log_node "node${i}" "Started (PID: ${PID})"

        i=$((i + 1))

        # Stagger startup
        sleep 1
    done

    # Save PIDs
    echo "${PIDS}" > "${DATA_DIR}/node_pids.txt"

    log_ok "All ${NODES} nodes launched"
}

# ─── Show Status ─────────────────────────────────────────────────────────────
show_status() {
    log_info "=== Testnet Status ==="
    log_info "Data Directory: ${DATA_DIR}"
    log_info "Nodes: ${NODES}"

    i=0
    while [ $i -lt $NODES ]; do
        node_port=$((BASE_PORT + i))
        metrics_port=$((BASE_PORT + i + 1000))
        log_node "node${i}" "Port: ${node_port} | Metrics: http://localhost:${metrics_port}/metrics | Logs: ${DATA_DIR}/node${i}/logs/node.log"
        i=$((i + 1))
    done

    log_info ""
    log_info "Management Commands:"
    log_info "  # View all node logs"
    log_info "  tail -f ${DATA_DIR}/node*/logs/node.log"
    log_info ""
    log_info "  # Stop all nodes"
    log_info "  for pid in $(cat ${DATA_DIR}/node_pids.txt); do kill \$pid; done"
    log_info ""
    log_info "  # Clean testnet"
    log_info "  ./scripts/testnet-mode.sh --clean"
    log_info ""
    log_ok "Testnet is running!"
}

# ─── Main ────────────────────────────────────────────────────────────────────
main() {
    parse_args "$@"
    preflight
    clean_testnet
    mkdir -p "${DATA_DIR}"

    i=0
    while [ $i -lt $NODES ]; do
        generate_config $i
        i=$((i + 1))
    done

    generate_manifest
    launch_nodes
    show_status
}

main "$@"
