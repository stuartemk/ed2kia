#!/usr/bin/env bash
# ignite-local-testnet.sh — Local Testnet Bootstrap for ed2kIA v2.1.0-sprint7
# License: Apache 2.0 + Ethical Use Clause
#
# Purpose: Bootstrap a local testnet environment for dry-run end-to-end
# validation of the "Secuencia de Ignición":
#   Relay → Orchestrator → WASM Nodes → Consensus/Reputation → Atlas 3D
#
# Guardrails:
#   - set -e for immediate exit on error
#   - trap for cleanup on EXIT/INT/TERM
#   - No external network calls
#   - All processes run locally
#
# Usage:
#   bash scripts/ignite-local-testnet.sh
#   bash scripts/ignite-local-testnet.sh --clean
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - Python 3 (for generate_dummy_sae.py)
#   - Target wasm32-unknown-unknown installed

set -euo pipefail

# ─── Configuration ───
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$PROJECT_ROOT/tmp/testnet-logs"
MODELS_DIR="$PROJECT_ROOT/models"
CLEAN=false
PIDS=()

# ─── Colors ───
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  CYAN='\033[0;36m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  CYAN=''
  NC=''
fi

log_info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_step()  { echo -e "${CYAN}[STEP]${NC} $*"; }

# ─── Cleanup Handler ───
cleanup() {
  log_info "Cleaning up testnet processes..."
  for pid in "${PIDS[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      log_info "  Stopping process $pid"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  log_info "Cleanup complete."
}

trap cleanup EXIT INT TERM

# ─── Argument Parsing ───
for arg in "$@"; do
  case $arg in
    --clean) CLEAN=true ;;
    --help|-h)
      echo "Usage: $0 [--clean]"
      echo ""
      echo "Options:"
      echo "  --clean  Remove tmp/ and models/ before starting"
      exit 0
      ;;
    *)
      log_error "Unknown option: $arg"
      exit 1
      ;;
  esac
done

# ─── Step 0: Pre-flight ───
log_step "=== Step 0: Pre-flight Checks ==="

# Check cargo
if ! command -v cargo &>/dev/null; then
  log_error "cargo not found. Install Rust toolchain first."
  exit 1
fi
log_info "  cargo: $(cargo --version)"

# Check python
PYTHON_CMD=""
if command -v python3 &>/dev/null; then
  PYTHON_CMD="python3"
elif command -v python &>/dev/null; then
  PYTHON_CMD="python"
else
  log_warn "  Python not found — skipping dummy SAE generation"
fi

# Check WASM target
if ! rustup target list --installed | grep -q wasm32-unknown-unknown 2>/dev/null; then
  log_warn "  wasm32-unknown-unknown target not installed — skipping WASM build"
fi

# ─── Step 1: Clean (optional) ───
if [ "$CLEAN" = true ]; then
  log_step "=== Step 1: Cleaning Previous State ==="
  rm -rf "$LOG_DIR"
  rm -f "$MODELS_DIR/dummy_qwen_scope.safetensors"
  log_info "  Cleaned tmp/testnet-logs/ and models/dummy_qwen_scope.safetensors"
fi

# ─── Step 2: Create Directories ───
log_step "=== Step 2: Creating Directories ==="
mkdir -p "$LOG_DIR"
mkdir -p "$MODELS_DIR"
log_info "  Created $LOG_DIR"
log_info "  Created $MODELS_DIR"

# ─── Step 3: Generate Dummy SAE ───
log_step "=== Step 3: Generating Dummy SAE Model ==="
if [ -n "$PYTHON_CMD" ]; then
  if [ ! -f "$MODELS_DIR/dummy_qwen_scope.safetensors" ]; then
    $PYTHON_CMD "$SCRIPT_DIR/generate_dummy_sae.py" \
      --d-model 64 \
      --d-sae 256 \
      --output "$MODELS_DIR/dummy_qwen_scope.safetensors" \
      >> "$LOG_DIR/sae_generation.log" 2>&1
    log_info "  Generated $MODELS_DIR/dummy_qwen_scope.safetensors"
  else
    log_info "  Dummy SAE already exists — skipping"
  fi
else
  log_warn "  Skipping SAE generation (Python not available)"
fi

# ─── Step 4: Build WASM ───
log_step "=== Step 4: Building WASM Target ==="
if rustup target list --installed | grep -q wasm32-unknown-unknown 2>/dev/null; then
  log_info "  Running: cargo build --target wasm32-unknown-unknown"
  cargo build --target wasm32-unknown-unknown \
    >> "$LOG_DIR/wasm_build.log" 2>&1
  log_info "  WASM build complete."
else
  log_warn "  Skipping WASM build (target not installed)"
  log_warn "  Install with: rustup target add wasm32-unknown-unknown"
fi

# ─── Step 5: Run Relay Server (simulated) ───
log_step "=== Step 5: Starting Relay Server (simulated) ==="
# Simulated relay — logs heartbeat every 5s
(
  while true; do
    echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] RELAY: Heartbeat — 0 active circuits" \
      >> "$LOG_DIR/relay-server.log"
    sleep 5
  done
) &
PIDS+=($!)
log_info "  Relay server PID: ${PIDS[-1]}"

# ─── Step 6: Run Orchestrator Node (simulated) ───
log_step "=== Step 6: Starting Orchestrator Node (simulated) ==="
# Simulated orchestrator — logs task dispatch every 3s
(
  while true; do
    echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] ORCHESTRATOR: Task dispatch — 0 pending, 0 active" \
      >> "$LOG_DIR/orchestrator-node.log"
    sleep 3
  done
) &
PIDS+=($!)
log_info "  Orchestrator PID: ${PIDS[-1]}"

# ─── Step 7: Run E2E Consensus Test ───
log_step "=== Step 7: Running E2E Consensus Test ==="
if cargo test --features "v2.1-reputation-system v2.1-task-manager" \
  --test e2e_consensus_test \
  >> "$LOG_DIR/e2e_consensus.log" 2>&1; then
  log_info "  E2E consensus test PASSED"
else
  log_error "  E2E consensus test FAILED — check $LOG_DIR/e2e_consensus.log"
  exit 1
fi

# ─── Step 8: Status Report ───
log_step "=== Step 8: Testnet Status Report ==="
echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║           ed2kIA v2.1.0-sprint7 Local Testnet            ║${NC}"
echo -e "${GREEN}╠═══════════════════════════════════════════════════════════╣${NC}"
echo -e "${GREEN}║${NC} Status: ${YELLOW}RUNNING${NC}                                        ${GREEN}║${NC}"
echo -e "${GREEN}║${NC} Relay:    PID ${PIDS[0]:-N/A}                                      ${GREEN}║${NC}"
echo -e "${GREEN}║${NC} Orchestr: PID ${PIDS[1]:-N/A}                                      ${GREEN}║${NC}"
echo -e "${GREEN}║${NC} Logs:     ${LOG_DIR}                        ${GREEN}║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""
log_info "Testnet bootstrap complete."
log_info "Processes will run until you press Ctrl+C."
log_info "Logs available in: $LOG_DIR"
echo ""

# ─── Wait for Processes ───
# Keep script alive so background processes run
wait
