#!/bin/sh
# =============================================================================
# ed2kIA v1.6.0-stable — Quickstart Local Node
# =============================================================================
# POSIX-compliant script to launch a local ed2kIA node for development/testing.
# No external dependencies beyond Rust + Cargo.
#
# Usage:
#   ./examples/quickstart/run_local.sh [OPTIONS]
#
# Options:
#   --port <PORT>       P2P port (default: 9000)
#   --http-port <PORT>  HTTP API port (default: 3000)
#   --data-dir <PATH>   Data directory (default: ./ed2k_data)
#   --features <FLAGS>  Cargo features (default: stable)
#   --help              Show this help
#
# Example:
#   ./examples/quickstart/run_local.sh --port 9000 --http-port 3000
# =============================================================================

set -e

# ─── Defaults ────────────────────────────────────────────────────────────────
P2P_PORT=9000
HTTP_PORT=3000
DATA_DIR="./ed2k_data"
FEATURES="stable"
MODE="run"

# ─── Parse arguments ─────────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
  case "$1" in
    --port)
      P2P_PORT="$2"
      shift 2
      ;;
    --http-port)
      HTTP_PORT="$2"
      shift 2
      ;;
    --data-dir)
      DATA_DIR="$2"
      shift 2
      ;;
    --features)
      FEATURES="$2"
      shift 2
      ;;
    --help|-h)
      echo "ed2kIA v1.6.0-stable — Quickstart Local Node"
      echo ""
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --port <PORT>       P2P port (default: 9000)"
      echo "  --http-port <PORT>  HTTP API port (default: 3000)"
      echo "  --data-dir <PATH>   Data directory (default: ./ed2k_data)"
      echo "  --features <FLAGS>  Cargo features (default: stable)"
      echo "  --help              Show this help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
done

# ─── Pre-flight checks ───────────────────────────────────────────────────────
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  ed2kIA v1.6.0-stable — Quickstart Local Node              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check Rust installation
if ! command -v cargo > /dev/null 2>&1; then
  echo "ERROR: Cargo not found. Install Rust first:"
  echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.sh | sh"
  exit 1
fi

RUST_VERSION=$(rustc --version | awk '{print $2}')
echo "✓ Rust: $RUST_VERSION"

# Create data directory
mkdir -p "$DATA_DIR"
echo "✓ Data dir: $DATA_DIR"

# ─── Build ───────────────────────────────────────────────────────────────────
echo ""
echo "→ Building with features: $FEATURES ..."
cargo build --features "$FEATURES" 2>&1

echo "✓ Build complete"

# ─── Run ─────────────────────────────────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Starting ed2kIA node                                      ║"
echo "║  P2P Port:    $P2P_PORT                                     ║"
echo "║  HTTP Port:   $HTTP_PORT                                    ║"
echo "║  Data Dir:    $DATA_DIR                                     ║"
echo "║  Features:    $FEATURES                                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "Press Ctrl+C to stop."
echo ""

# Execute
exec cargo run --features "$FEATURES" -- \
  --data-dir "$DATA_DIR" \
  --port "$P2P_PORT" \
  bootstrap genesis
