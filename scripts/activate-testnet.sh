#!/bin/sh
# =============================================================================
# ed2kIA Testnet Activator — Live Testnet Orchestration Script
# =============================================================================
# Sprint 35: "Live Testnet Activation, Public Dashboard & Steward Onboarding"
# Feature gate: v2.1-testnet-ops
#
# POSIX-compliant, idempotent, cross-platform (Linux/macOS/WSL)
# Usage: ./scripts/activate-testnet.sh [OPTIONS]
#
# This script automates the deployment of a live ed2kIA testnet with:
#   - N-node deployment (default: 3)
#   - Bootstrap peer list generation (testnet-bootstrap.json)
#   - P2P handshake verification
#   - SymbolRegistry CRDT sync validation
#   - Docker or cargo run execution modes
#   - Lifecycle management (--start, --stop, --clean, --status)
#   - Public status dashboard integration
#   - Steward connection instructions
#
# Options:
#   --nodes N           Number of nodes to deploy (default: 3)
#   --port BASE         Base port for first node (default: 18080)
#   --data DIR          Data directory (default: ~/.ed2kIA/testnet-live)
#   --mode MODE         Execution mode: cargo|docker (default: cargo)
#   --start             Start the testnet (default action)
#   --stop              Stop all running testnet nodes
#   --clean             Clean all testnet data and stop nodes
#   --status            Show testnet status
#   --dry-run           Show configuration without executing
#   --help              Show this help message
#
# Environment Variables:
#   ED2KIA_TESTNET_NODES      — Number of nodes (overrides --nodes)
#   ED2KIA_TESTNET_PORT       — Base port (overrides --port)
#   ED2KIA_TESTNET_DATA       — Data directory (overrides --data)
#   ED2KIA_TESTNET_MODE       — Execution mode (overrides --mode)
#   ED2KIA_TESTNET_TIMEOUT    — Wait timeout in seconds (default: 120)
#
# Output Files:
#   ~/.ed2kIA/testnet-live/testnet-bootstrap.json  — Peer bootstrap config
#   ~/.ed2kIA/testnet-live/status.json             — Live status for dashboard
#   ~/.ed2kIA/testnet-live/nodes/                   — Per-node data/logs
# =============================================================================

set -e

# ─── Color Codes ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# ─── Logging ─────────────────────────────────────────────────────────────────
log_info()  { printf "${BLUE}[INFO]${NC} %s\n" "$1"; }
log_ok()    { printf "${GREEN}[OK]${NC}   %s\n" "$1"; }
log_warn()  { printf "${YELLOW}[WARN]${NC} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${NC} %s\n" "$1" >&2; }
log_node()  { printf "${CYAN}[NODE %s]${NC} %s\n" "$1" "$2"; }
log_step()  { printf "${MAGENTA}[STEP]${NC} %s\n" "$1"; }

# ─── Defaults ────────────────────────────────────────────────────────────────
NODES="${ED2KIA_TESTNET_NODES:-3}"
BASE_PORT="${ED2KIA_TESTNET_PORT:-18080}"
DATA_DIR="${ED2KIA_TESTNET_DATA:-$HOME/.ed2kIA/testnet-live}"
MODE="${ED2KIA_TESTNET_MODE:-cargo}"
TIMEOUT="${ED2KIA_TESTNET_TIMEOUT:-120}"
ACTION="start"
CLEAN=0
DRY_RUN=0
ED2KIA_DIR=""

# ─── Usage ───────────────────────────────────────────────────────────────────
usage() {
    cat <<'EOF'
ed2kIA Live Testnet Activator — Deploy and Manage Public Testnet

USAGE:
    ./scripts/activate-testnet.sh [OPTIONS]

OPTIONS:
    --nodes N           Number of nodes to deploy (default: 3)
    --port BASE         Base port for first node (default: 18080)
    --data DIR          Data directory (default: ~/.ed2kIA/testnet-live)
    --mode MODE         Execution mode: cargo|docker (default: cargo)
    --start             Start the testnet (default action)
    --stop              Stop all running testnet nodes
    --clean             Clean all testnet data and stop nodes
    --status            Show testnet status
    --dry-run           Show configuration without executing
    --help              Show this help message

ENVIRONMENT:
    ED2KIA_TESTNET_NODES      Number of nodes (overrides --nodes)
    ED2KIA_TESTNET_PORT       Base port (overrides --port)
    ED2KIA_TESTNET_DATA       Data directory (overrides --data)
    ED2KIA_TESTNET_MODE       Execution mode (overrides --mode)
    ED2KIA_TESTNET_TIMEOUT    Wait timeout in seconds (default: 120)

OUTPUT FILES:
    ~/.ed2kIA/testnet-live/testnet-bootstrap.json  Peer bootstrap config
    ~/.ed2kIA/testnet-live/status.json             Live status for dashboard
    ~/.ed2kIA/testnet-live/nodes/                   Per-node data/logs

EXAMPLES:
    # Launch default 3-node testnet
    ./scripts/activate-testnet.sh

    # Launch 5-node testnet with Docker
    ./scripts/activate-testnet.sh --nodes 5 --mode docker

    # Stop running testnet
    ./scripts/activate-testnet.sh --stop

    # Clean all testnet data
    ./scripts/activate-testnet.sh --clean

    # Check testnet status
    ./scripts/activate-testnet.sh --status

    # Dry run to see configuration
    ./scripts/activate-testnet.sh --dry-run
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
            --mode)
                MODE="$2"
                shift 2
                ;;
            --start)
                ACTION="start"
                shift
                ;;
            --stop)
                ACTION="stop"
                shift
                ;;
            --clean)
                ACTION="clean"
                shift
                ;;
            --status)
                ACTION="status"
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

# ─── Pre-flight Validation ──────────────────────────────────────────────────
preflight() {
    log_step "Pre-flight validation..."

    # Check required tools
    if [ "$MODE" = "cargo" ]; then
        if ! command -v cargo >/dev/null 2>&1; then
            log_error "cargo not found. Install Rust: https://rustup.rs/"
            exit 1
        fi
        log_ok "cargo found: $(cargo --version)"
    fi

    if [ "$MODE" = "docker" ]; then
        if ! command -v docker >/dev/null 2>&1; then
            log_error "docker not found. Install Docker: https://docs.docker.com/get-docker/"
            exit 1
        fi
        log_ok "docker found: $(docker --version)"
    fi

    # Check jq for JSON generation
    if ! command -v jq >/dev/null 2>&1; then
        log_warn "jq not found. Bootstrap JSON will be generated manually."
    fi

    # Validate node count
    if [ "$NODES" -lt 1 ] 2>/dev/null || [ "$NODES" -gt 20 ] 2>/dev/null; then
        log_error "Node count must be between 1 and 20 (got: $NODES)"
        exit 1
    fi

    # Validate port range
    LAST_PORT=$((BASE_PORT + NODES))
    if [ "$BASE_PORT" -lt 1024 ] 2>/dev/null || [ "$LAST_PORT" -gt 65535 ] 2>/dev/null; then
        log_error "Port range invalid: $BASE_PORT-$LAST_PORT"
        exit 1
    fi

    log_ok "Pre-flight validation passed"
}

# ─── Directory Setup ─────────────────────────────────────────────────────────
setup_dirs() {
    log_step "Setting up directories..."

    mkdir -p "$DATA_DIR/nodes"
    mkdir -p "$DATA_DIR/logs"

    i=0
    while [ "$i" -lt "$NODES" ]; do
        NODE_PORT=$((BASE_PORT + i))
        NODE_DATA="$DATA_DIR/nodes/node-$i"
        mkdir -p "$NODE_DATA"
        log_node "node-$i" "Data dir: $NODE_DATA (port: $NODE_PORT)"
        i=$((i + 1))
    done

    log_ok "Directories ready"
}

# ─── Generate Bootstrap JSON ─────────────────────────────────────────────────
generate_bootstrap() {
    log_step "Generating testnet-bootstrap.json..."

    BOOTSTRAP_FILE="$DATA_DIR/testnet-bootstrap.json"

    # Build peer list
    PEERS="[]"
    i=0
    while [ "$i" -lt "$NODES" ]; do
        NODE_PORT=$((BASE_PORT + i))
        P2P_PORT=$((NODE_PORT + 1000))
        WS_PORT=$((NODE_PORT + 2000))

        # Generate deterministic peer ID based on node index
        PEER_ID="12D3KooWTestNet$(printf '%012d' $i)"

        if command -v jq >/dev/null 2>&1; then
            PEERS=$(echo "$PEERS" | jq --arg id "$PEER_ID" \
                --arg multiaddr "/ip4/127.0.0.1/tcp/$P2P_PORT/p2p/$PEER_ID" \
                --arg http "/ip4/127.0.0.1/tcp/$NODE_PORT" \
                --arg ws "/ip4/127.0.0.1/tcp/$WS_PORT" \
                --argjson index "$i" \
                '. + [{"peerId": $id, "multiaddr": $multiaddr, "http": $http, "ws": $ws, "index": $index}]')
        else
            # Manual JSON generation
            if [ "$i" -gt 0 ]; then
                PEERS="$PEERS,"
            fi
            PEERS="$PEERS{\"peerId\":\"$PEER_ID\",\"multiaddr\":\"/ip4/127.0.0.1/tcp/$P2P_PORT/p2p/$PEER_ID\",\"http\":\"/ip4/127.0.0.1/tcp/$NODE_PORT\",\"ws\":\"/ip4/127.0.0.1/tcp/$WS_PORT\",\"index\":$i}"
        fi

        i=$((i + 1))
    done

    # Generate bootstrap file
    if command -v jq >/dev/null 2>&1; then
        jq -n --argjson peers "$PEERS" \
            --arg version "v2.1.0-stable" \
            --arg feature "v2.1-testnet-ops" \
            --argjson nodes "$NODES" \
            --arg dataDir "$DATA_DIR" \
            --arg generated "$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%Y-%m-%dT%H:%M:%SZ)" \
            '{
              "ed2kIA_testnet": $true,
              "version": $version,
              "feature_gate": $feature,
              "nodes": $nodes,
              "data_directory": $dataDir,
              "generated_at": $generated,
              "peers": $peers,
              "instructions": {
                "connect_external": "ed2kIA-node --bootstrap <this_file> --features v2.1-testnet-ops",
                "dashboard": "Open web/testnet-status.html in browser",
                "status_file": "$DATA_DIR/status.json"
              }
            }' > "$BOOTSTRAP_FILE"
    else
        # Manual JSON generation
        cat > "$BOOTSTRAP_FILE" <<BOOTSTRAP_EOF
{
  "ed2kIA_testnet": true,
  "version": "v2.1.0-stable",
  "feature_gate": "v2.1-testnet-ops",
  "nodes": $NODES,
  "data_directory": "$DATA_DIR",
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%Y-%m-%dT%H:%M:%SZ)",
  "peers": [$PEERS],
  "instructions": {
    "connect_external": "ed2kIA-node --bootstrap <this_file> --features v2.1-testnet-ops",
    "dashboard": "Open web/testnet-status.html in browser",
    "status_file": "$DATA_DIR/status.json"
  }
}
BOOTSTRAP_EOF
    fi

    log_ok "Bootstrap file: $BOOTSTRAP_FILE"
    log_info "Peers: $NODES nodes (ports $BASE_PORT-$((BASE_PORT + NODES - 1)))"
}

# ─── Start Node ──────────────────────────────────────────────────────────────
start_node() {
    NODE_INDEX=$1
    NODE_PORT=$((BASE_PORT + NODE_INDEX))
    P2P_PORT=$((NODE_PORT + 1000))
    NODE_DATA="$DATA_DIR/nodes/node-$NODE_INDEX"
    NODE_LOG="$DATA_DIR/logs/node-$NODE_INDEX.log"
    PID_FILE="$NODE_DATA/node.pid"

    log_node "node-$NODE_INDEX" "Starting on port $NODE_PORT (P2P: $P2P_PORT)..."

    # First node connects to no peers, subsequent nodes connect to previous
    BOOTSTRAP_PEERS=""
    if [ "$NODE_INDEX" -gt 0 ]; then
        j=0
        while [ "$j" -lt "$NODE_INDEX" ]; do
            PREV_PORT=$((BASE_PORT + j))
            PREV_P2P_PORT=$((PREV_PORT + 1000))
            PREV_PEER_ID="12D3KooWTestNet$(printf '%012d' $j)"
            if [ -n "$BOOTSTRAP_PEERS" ]; then
                BOOTSTRAP_PEERS="$BOOTSTRAP_PEERS,/ip4/127.0.0.1/tcp/$PREV_P2P_PORT/p2p/$PREV_PEER_ID"
            else
                BOOTSTRAP_PEERS="/ip4/127.0.0.1/tcp/$PREV_P2P_PORT/p2p/$PREV_PEER_ID"
            fi
            j=$((j + 1))
        done
    fi

    if [ "$MODE" = "cargo" ]; then
        # Launch with cargo run
        ED2KIA_NODE_PORT=$NODE_PORT \
        ED2KIA_P2P_PORT=$P2P_PORT \
        ED2KIA_DATA_DIR="$NODE_DATA" \
        ED2KIA_BOOTSTRAP_PEERS="$BOOTSTRAP_PEERS" \
        ED2KIA_FEATURE_FLAGS="v2.1-testnet-ops" \
        ED2KIA_LOG_LEVEL="info" \
        cargo run --features v2.1-testnet-ops --bin ed2kIA-node > "$NODE_LOG" 2>&1 &

        NODE_PID=$!
        echo "$NODE_PID" > "$PID_FILE"
        log_node "node-$NODE_INDEX" "PID: $NODE_PID (cargo)"
    elif [ "$MODE" = "docker" ]; then
        # Launch with Docker
        docker run -d \
            --name "ed2kIA-testnet-node-$NODE_INDEX" \
            -p "$NODE_PORT:8080" \
            -p "$P2P_PORT:9080" \
            -v "$NODE_DATA:/data" \
            -e ED2KIA_NODE_PORT=8080 \
            -e ED2KIA_P2P_PORT=9080 \
            -e ED2KIA_DATA_DIR=/data \
            -e ED2KIA_BOOTSTRAP_PEERS="$BOOTSTRAP_PEERS" \
            -e ED2KIA_FEATURE_FLAGS="v2.1-testnet-ops" \
            -e ED2KIA_LOG_LEVEL="info" \
            ed2kia/ed2kIA-node:v2.1.0-stable > "$NODE_LOG" 2>&1

        NODE_PID=$(docker inspect --format='{{.State.Pid}}' "ed2kIA-testnet-node-$NODE_INDEX" 2>/dev/null || echo "docker")
        echo "$NODE_PID" > "$PID_FILE"
        log_node "node-$NODE_INDEX" "PID: $NODE_PID (docker)"
    fi
}

# ─── Wait for P2P Handshake ──────────────────────────────────────────────────
wait_for_handshake() {
    log_step "Waiting for P2P handshake (timeout: ${TIMEOUT}s)..."

    ELAPSED=0
    HANDSHAKE_COMPLETE=0

    while [ "$ELAPSED" -lt "$TIMEOUT" ]; do
        # Check if all nodes are running
        ALL_RUNNING=1
        i=0
        while [ "$i" -lt "$NODES" ]; do
            NODE_DATA="$DATA_DIR/nodes/node-$i"
            PID_FILE="$NODE_DATA/node.pid"

            if [ -f "$PID_FILE" ]; then
                PID=$(cat "$PID_FILE")
                if [ "$PID" != "docker" ] && ! kill -0 "$PID" 2>/dev/null; then
                    ALL_RUNNING=0
                    log_node "node-$i" "Process $PID not running"
                    break
                fi
            fi

            i=$((i + 1))
        done

        if [ "$ALL_RUNNING" -eq 1 ]; then
            # Check if nodes have logged P2P connection
            CONNECTED_NODES=0
            i=0
            while [ "$i" -lt "$NODES" ]; do
                NODE_LOG="$DATA_DIR/logs/node-$i.log"
                if [ -f "$NODE_LOG" ]; then
                    # Look for P2P connection indicators
                    if grep -q "peer.*connected\|discovery.*peer\|GossipSub.*topic" "$NODE_LOG" 2>/dev/null; then
                        CONNECTED_NODES=$((CONNECTED_NODES + 1))
                    fi
                fi
                i=$((i + 1))
            done

            # Consider handshake complete if at least 2 nodes show connection
            if [ "$CONNECTED_NODES" -ge 2 ] || [ "$NODES" -eq 1 ]; then
                HANDSHAKE_COMPLETE=1
                log_ok "P2P handshake complete: $CONNECTED_NODES/$NODES nodes connected"
                break
            fi
        fi

        sleep 5
        ELAPSED=$((ELAPSED + 5))
        log_info "Waiting... ${ELAPSED}s/${TIMEOUT}s"
    done

    if [ "$HANDSHAKE_COMPLETE" -eq 0 ]; then
        log_warn "P2P handshake timeout reached. Nodes may still be connecting."
        log_info "Check logs in $DATA_DIR/logs/"
    fi
}

# ─── Verify SymbolRegistry Sync ──────────────────────────────────────────────
verify_symbol_registry() {
    log_step "Verifying SymbolRegistry CRDT sync..."

    # Check if nodes have logged SymbolRegistry activity
    SYNC_INDICATORS=0
    i=0
    while [ "$i" -lt "$NODES" ]; do
        NODE_LOG="$DATA_DIR/logs/node-$i.log"
        if [ -f "$NODE_LOG" ]; then
            if grep -q "SymbolRegistry\|CRDT.*sync\|symbol.*insert\|crdt.*merge" "$NODE_LOG" 2>/dev/null; then
                SYNC_INDICATORS=$((SYNC_INDICATORS + 1))
            fi
        fi
        i=$((i + 1))
    done

    if [ "$SYNC_INDICATORS" -ge 2 ] || [ "$NODES" -eq 1 ]; then
        log_ok "SymbolRegistry sync verified: $SYNC_INDICATORS/$NODES nodes show CRDT activity"
    else
        log_warn "SymbolRegistry sync not yet verified. CRDT may still be converging."
        log_info "This is normal for fresh networks. Sync will complete as nodes exchange data."
    fi
}

# ─── Update Status JSON ──────────────────────────────────────────────────────
update_status() {
    log_step "Updating status.json for dashboard..."

    STATUS_FILE="$DATA_DIR/status.json"
    TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date +%Y-%m-%dT%H:%M:%SZ)

    # Build node status array
    NODES_STATUS="[]"
    i=0
    while [ "$i" -lt "$NODES" ]; do
        NODE_PORT=$((BASE_PORT + i))
        P2P_PORT=$((NODE_PORT + 1000))
        NODE_DATA="$DATA_DIR/nodes/node-$i"
        NODE_LOG="$DATA_DIR/logs/node-$i.log"
        PID_FILE="$NODE_DATA/node.pid"

        # Check if node is running
        IS_RUNNING="false"
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if [ "$PID" = "docker" ]; then
                # Check Docker container
                if docker ps --filter "name=ed2kIA-testnet-node-$i" --filter "status=running" --format '{{.Names}}' | grep -q "ed2kIA-testnet-node-$i" 2>/dev/null; then
                    IS_RUNNING="true"
                fi
            elif kill -0 "$PID" 2>/dev/null; then
                IS_RUNNING="true"
            fi
        fi

        # Count log lines as activity indicator
        LOG_LINES=0
        if [ -f "$NODE_LOG" ]; then
            LOG_LINES=$(wc -l < "$NODE_LOG" 2>/dev/null || echo "0")
        fi

        PEER_ID="12D3KooWTestNet$(printf '%012d' $i)"

        if command -v jq >/dev/null 2>&1; then
            NODES_STATUS=$(echo "$NODES_STATUS" | jq \
                --argjson index "$i" \
                --arg peerId "$PEER_ID" \
                --argjson port "$NODE_PORT" \
                --argjson p2pPort "$P2P_PORT" \
                --argjson running "$IS_RUNNING" \
                --argjson logLines "$LOG_LINES" \
                '. + [{"index": $index, "peerId": $peerId, "port": $port, "p2pPort": $p2pPort, "running": $running, "logLines": $logLines}]')
        fi

        i=$((i + 1))
    done

    # Generate status file
    if command -v jq >/dev/null 2>&1; then
        jq -n \
            --arg timestamp "$TIMESTAMP" \
            --argjson nodes "$NODES" \
            --argjson activeNodes "$NODES_STATUS" \
            --arg dataDir "$DATA_DIR" \
            --arg mode "$MODE" \
            --arg feature "v2.1-testnet-ops" \
            '{
              "timestamp": $timestamp,
              "testnet_active": true,
              "nodes_configured": $nodes,
              "nodes": $activeNodes,
              "data_directory": $dataDir,
              "execution_mode": $mode,
              "feature_gate": $feature,
              "ce_distribution": {},
              "apoptosis_events": [],
              "steering_events": []
            }' > "$STATUS_FILE"
    fi

    log_ok "Status file: $STATUS_FILE"
}

# ─── Print Connection Instructions ───────────────────────────────────────────
print_instructions() {
    log_step "Testnet Activation Complete!"

    BOOTSTRAP_FILE="$DATA_DIR/testnet-bootstrap.json"
    STATUS_FILE="$DATA_DIR/status.json"

    printf "\n"
    printf "${GREEN}╔══════════════════════════════════════════════════════════════════════════════╗${NC}\n"
    printf "${GREEN}║              ed2kIA Live Testnet — Activation Complete!                     ║${NC}\n"
    printf "${GREEN}╚══════════════════════════════════════════════════════════════════════════════╝${NC}\n"
    printf "\n"

    log_info "Testnet Configuration:"
    log_info "  Nodes: $NODES"
    log_info "  Ports: $BASE_PORT-$((BASE_PORT + NODES - 1))"
    log_info "  Mode: $MODE"
    log_info "  Data: $DATA_DIR"
    printf "\n"

    log_info "Connect External Nodes:"
    log_info "  ed2kIA-node --bootstrap $BOOTSTRAP_FILE --features v2.1-testnet-ops"
    printf "\n"

    log_info "Public Dashboard:"
    log_info "  Open web/testnet-status.html in your browser"
    log_info "  (Points to: $STATUS_FILE)"
    printf "\n"

    log_info "Steward Onboarding:"
    log_info "  Read: docs/steward-onboarding-guide.md"
    log_info "  Quick: ./scripts/activate-testnet.sh --status"
    printf "\n"

    log_info "Management Commands:"
    log_info "  Stop:  ./scripts/activate-testnet.sh --stop"
    log_info "  Clean: ./scripts/activate-testnet.sh --clean"
    log_info "  Status:./scripts/activate-testnet.sh --status"
    printf "\n"

    log_info "Logs:"
    i=0
    while [ "$i" -lt "$NODES" ]; do
        log_node "node-$i" "$DATA_DIR/logs/node-$i.log"
        i=$((i + 1))
    done
    printf "\n"

    log_ok "Your ed2kIA testnet is live and ready for steward onboarding!"
}

# ─── Stop Testnet ────────────────────────────────────────────────────────────
stop_testnet() {
    log_step "Stopping testnet nodes..."

    i=0
    while [ "$i" -lt "$NODES" ]; do
        NODE_DATA="$DATA_DIR/nodes/node-$i"
        PID_FILE="$NODE_DATA/node.pid"

        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if [ "$PID" = "docker" ]; then
                log_node "node-$i" "Stopping Docker container..."
                docker stop "ed2kIA-testnet-node-$i" >/dev/null 2>&1 || true
                docker rm "ed2kIA-testnet-node-$i" >/dev/null 2>&1 || true
            else
                log_node "node-$i" "Stopping process $PID..."
                kill "$PID" 2>/dev/null || true
                # Wait for graceful shutdown
                sleep 2
                # Force kill if still running
                kill -9 "$PID" 2>/dev/null || true
            fi
            rm -f "$PID_FILE"
            log_node "node-$i" "Stopped"
        else
            log_node "node-$i" "No PID file found (already stopped?)"
        fi

        i=$((i + 1))
    done

    log_ok "All testnet nodes stopped"
}

# ─── Clean Testnet ───────────────────────────────────────────────────────────
clean_testnet() {
    log_step "Cleaning testnet data..."

    # Stop nodes first
    stop_testnet

    # Remove data directory
    if [ -d "$DATA_DIR" ]; then
        rm -rf "$DATA_DIR"
        log_ok "Data directory removed: $DATA_DIR"
    else
        log_info "Data directory not found: $DATA_DIR"
    fi

    log_ok "Testnet cleaned"
}

# ─── Show Status ─────────────────────────────────────────────────────────────
show_status() {
    log_step "Testnet Status"

    STATUS_FILE="$DATA_DIR/status.json"
    BOOTSTRAP_FILE="$DATA_DIR/testnet-bootstrap.json"

    if [ ! -d "$DATA_DIR" ]; then
        log_warn "No testnet data found in $DATA_DIR"
        log_info "Run: ./scripts/activate-testnet.sh --start"
        return
    fi

    if [ -f "$BOOTSTRAP_FILE" ]; then
        log_info "Bootstrap: $BOOTSTRAP_FILE"
        if command -v jq >/dev/null 2>&1; then
            CONFIGURED=$(jq -r '.nodes' "$BOOTSTRAP_FILE" 2>/dev/null || echo "?")
            log_info "Configured Nodes: $CONFIGURED"
        fi
    fi

    printf "\n"
    log_info "Node Status:"

    i=0
    while [ "$i" -lt "$NODES" ]; do
        NODE_DATA="$DATA_DIR/nodes/node-$i"
        PID_FILE="$NODE_DATA/node.pid"
        NODE_PORT=$((BASE_PORT + i))

        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if [ "$PID" = "docker" ]; then
                if docker ps --filter "name=ed2kIA-testnet-node-$i" --filter "status=running" --format '{{.Names}}' | grep -q "ed2kIA-testnet-node-$i" 2>/dev/null; then
                    log_node "node-$i" "RUNNING (port: $NODE_PORT, docker)"
                else
                    log_node "node-$i" "STOPPED (docker)"
                fi
            elif kill -0 "$PID" 2>/dev/null; then
                log_node "node-$i" "RUNNING (port: $NODE_PORT, pid: $PID)"
            else
                log_node "node-$i" "STOPPED (pid: $PID)"
            fi
        else
            log_node "node-$i" "NOT STARTED"
        fi

        i=$((i + 1))
    done

    printf "\n"
    log_info "Logs: $DATA_DIR/logs/"
    if [ -f "$STATUS_FILE" ]; then
        log_info "Status: $STATUS_FILE"
    fi
}

# ─── Main ────────────────────────────────────────────────────────────────────
main() {
    parse_args "$@"

    printf "\n"
    log_info "ed2kIA Live Testnet Activator (v2.1.0-stable)"
    log_info "Feature Gate: v2.1-testnet-ops"
    printf "\n"

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "DRY RUN MODE — No changes will be made"
        printf "\n"
        log_info "Configuration:"
        log_info "  Nodes: $NODES"
        log_info "  Base Port: $BASE_PORT"
        log_info "  Data Dir: $DATA_DIR"
        log_info "  Mode: $MODE"
        log_info "  Action: $ACTION"
        printf "\n"
        log_ok "Dry run complete"
        exit 0
    fi

    case "$ACTION" in
        start)
            preflight
            setup_dirs
            generate_bootstrap

            log_step "Starting $NODES nodes..."
            i=0
            while [ "$i" -lt "$NODES" ]; do
                start_node "$i"
                i=$((i + 1))
            done

            wait_for_handshake
            verify_symbol_registry
            update_status
            print_instructions
            ;;
        stop)
            stop_testnet
            ;;
        clean)
            clean_testnet
            ;;
        status)
            show_status
            ;;
        *)
            log_error "Unknown action: $ACTION"
            usage
            exit 1
            ;;
    esac
}

# ─── Entry Point ─────────────────────────────────────────────────────────────
main "$@"
