#!/usr/bin/env bash
# simulate_traffic.sh — Demo traffic injection for ed2kIA v2.1.0-sprint10
# Simulates 2 WASM browser nodes + orchestrator stats for visual demo recording
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configurable defaults
PORT="${ED2KIA_PORT:-3030}"
BASE_URL="http://localhost:${PORT}"
DURATION="${DEMO_DURATION:-15}"
NODE_A="wasm-browser-a-$(head -c 4 /dev/urandom | xxd -p)"
NODE_B="wasm-browser-b-$(head -c 4 /dev/urandom | xxd -p)"

cleanup() {
  echo "[cleanup] Demo traffic injection complete."
}
trap cleanup EXIT INT TERM

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
log_node()  { echo -e "${CYAN}[NODE]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_err()   { echo -e "${RED}[ERR]${NC} $*"; }

# ------------------------------------------
# Usage
# ------------------------------------------
usage() {
  cat <<EOF
ed2kIA Demo Traffic Simulator v2.1.0-sprint10

Usage: bash scripts/simulate_traffic.sh [OPTIONS]

Options:
  -p, --port PORT       Orchestrator API port (default: 3030)
  -d, --duration SECS   Demo duration in seconds (default: 15)
  -h, --help            Show this help message

Environment:
  ED2KIA_PORT           Orchestrator API port
  DEMO_DURATION         Demo duration in seconds

Demo Recording Guide:
  1. Start orchestrator: cargo run --release
  2. Open terminal split (left: orchestrator logs, right: this script)
  3. Open browser tabs: http://localhost:3030 (Atlas 3D Visualizer)
  4. Run this script to inject simulated traffic
  5. Record screen for 15-30s demo clip

Example:
  bash scripts/simulate_traffic.sh -p 3030 -d 15
EOF
}

# ------------------------------------------
# Parse arguments
# ------------------------------------------
while [[ $# -gt 0 ]]; do
  case $1 in
    -p|--port) PORT="$2"; shift 2 ;;
    -d|--duration) DURATION="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) log_err "Unknown option: $1"; usage; exit 1 ;;
  esac
done

BASE_URL="http://localhost:${PORT}"

# ------------------------------------------
# Preflight: Check if orchestrator is running
# ------------------------------------------
preflight() {
  log_info "Checking orchestrator at ${BASE_URL}..."
  if ! curl -sf "${BASE_URL}/api/atlas/stats" -o /dev/null 2>&1; then
    log_warn "Orchestrator not responding at ${BASE_URL}"
    log_warn "Starting in offline mode (simulated output only)"
    return 1
  fi
  log_info "Orchestrator connected ✓"
  return 0
}

# ------------------------------------------
# Simulate node connection
# ------------------------------------------
connect_node() {
  local node_id="$1"
  log_node "${node_id} → Connecting to orchestrator..."

  if preflight &>/dev/null; then
    curl -sf -X POST "${BASE_URL}/api/node/connect" \
      -H "Content-Type: application/json" \
      -d "{\"node_id\":\"${node_id}\",\"type\":\"wasm-browser\"}" \
      -o /dev/null 2>&1 && log_node "${node_id} ✓ Connected" || log_node "${node_id} ✗ Connection failed"
  else
    log_node "${node_id} [SIMULATED] Connected via WebRTC relay"
  fi
}

# ------------------------------------------
# Simulate audit task submission
# ------------------------------------------
submit_task() {
  local node_id="$1"
  local task_num="$2"
  log_node "${node_id} → Submitting audit task #${task_num}..."

  if preflight &>/dev/null; then
    local payload
    payload=$(cat <<EOF
{
  "node_id": "${node_id}",
  "task_type": "sae_audit",
  "input": [$(printf '%.1f,' $(seq 1 5) | sed 's/,$//')],
  "k": 10
}
EOF
)
    curl -sf -X POST "${BASE_URL}/api/audit" \
      -H "Content-Type: application/json" \
      -d "${payload}" \
      -o /dev/null 2>&1 && log_node "${node_id} ✓ Task #${task_num} submitted" || log_node "${node_id} ✗ Task #${task_num} failed"
  else
    log_node "${node_id} [SIMULATED] Task #${task_num} → SAE forward → Sparse features [k=10]"
  fi
}

# ------------------------------------------
# Simulate RLHF feedback submission
# ------------------------------------------
submit_feedback() {
  local node_id="$1"
  local token="$2"
  local feature="$3"
  local decision="$4"
  log_node "${node_id} → RLHF feedback: '${token}' → ${feature} (${decision})"

  if preflight &>/dev/null; then
    local payload
    payload=$(cat <<EOF
{
  "node_id": "${node_id}",
  "token": "${token}",
  "feature": "${feature}",
  "decision": "${decision}",
  "note": "Demo feedback injection"
}
EOF
)
    curl -sf -X POST "${BASE_URL}/api/feedback" \
      -H "Content-Type: application/json" \
      -d "${payload}" \
      -o /dev/null 2>&1 && log_node "${node_id} ✓ Feedback submitted" || log_node "${node_id} ✗ Feedback failed"
  else
    log_node "${node_id} [SIMULATED] Feedback stored → ${token}:${feature}=${decision}"
  fi
}

# ------------------------------------------
# Fetch and display stats
# ------------------------------------------
show_stats() {
  log_info "Fetching Atlas stats..."
  if preflight &>/dev/null; then
    curl -sf "${BASE_URL}/api/atlas/stats" 2>/dev/null | python3 -m json.tool 2>/dev/null || \
      curl -sf "${BASE_URL}/api/atlas/stats" 2>/dev/null
  else
    cat <<EOF
{
  "node_count": 42,
  "edge_count": 128,
  "active_peers": 2,
  "tasks_completed": 15,
  "feedback_submissions": 3
}
EOF
  fi
}

# ------------------------------------------
# Main demo loop
# ------------------------------------------
main() {
  echo "============================================"
  echo "  ed2kIA Demo Traffic Simulator v2.1.0"
  echo "  Duration: ${DURATION}s | Port: ${PORT}"
  echo "  Nodes: ${NODE_A}, ${NODE_B}"
  echo "============================================"
  echo ""

  local elapsed=0
  local task_counter=0

  # Phase 1: Node connections (0-3s)
  log_info "=== Phase 1: Node Connections ==="
  connect_node "${NODE_A}"
  sleep 1
  connect_node "${NODE_B}"
  sleep 1

  # Phase 2: Audit tasks (3-10s)
  log_info "=== Phase 2: Audit Tasks ==="
  while [ $elapsed -lt $DURATION ]; do
    task_counter=$((task_counter + 1))
    submit_task "${NODE_A}" "${task_counter}"
    sleep 1
    elapsed=$((elapsed + 1))

    if [ $elapsed -lt $DURATION ]; then
      task_counter=$((task_counter + 1))
      submit_task "${NODE_B}" "${task_counter}"
      sleep 1
      elapsed=$((elapsed + 1))
    fi
  done

  # Phase 3: RLHF Feedback (final seconds)
  log_info "=== Phase 3: RLHF Feedback ==="
  submit_feedback "${NODE_A}" "justicia" "feat-42" "correct"
  sleep 1
  submit_feedback "${NODE_B}" "libertad" "feat-17" "correct"
  sleep 1
  submit_feedback "${NODE_A}" "equidad" "feat-88" "correct"

  # Phase 4: Final stats
  log_info "=== Phase 4: Final Stats ==="
  show_stats

  echo ""
  echo "============================================"
  echo "  Demo Complete!"
  echo "  Total tasks: $((task_counter))"
  echo "  Feedback: 3 submissions"
  echo "  Nodes: ${NODE_A}, ${NODE_B}"
  echo "============================================"
}

main "$@"
