#!/usr/bin/env bash
# build-wasm.sh — WASM Browser Node build pipeline for ed2kIA v2.1
# POSIX-compatible, idempotent, zero-assumption preflight.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_ROOT/dist/wasm"

echo "=== ed2kIA WASM Build Pipeline v2.1 ==="
echo "Project root: $PROJECT_ROOT"
echo "Dist target:  $DIST_DIR"

# ------------------------------------------
# Preflight: Ensure wasm32 target installed
# ------------------------------------------
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
  echo "[preflight] Adding wasm32-unknown-unknown target..."
  rustup target add wasm32-unknown-unknown
fi

# ------------------------------------------
# Preflight: Install wasm-pack if missing
# ------------------------------------------
if ! command -v wasm-pack &>/dev/null; then
  echo "[preflight] Installing wasm-pack..."
  cargo install wasm-pack --locked
fi

# ------------------------------------------
# Preflight: Install trunk if missing
# ------------------------------------------
if ! command -v trunk &>/dev/null; then
  echo "[preflight] Installing trunk..."
  cargo install trunk
fi

# ------------------------------------------
# Cleanup previous artifacts
# ------------------------------------------
echo "[cleanup] Removing previous dist/wasm..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# ------------------------------------------
# Build: wasm-pack for library validation
# ------------------------------------------
echo "[build] Running wasm-pack build (lib only)..."
cd "$PROJECT_ROOT"
wasm-pack build \
  --target web \
  --out-dir "$DIST_DIR" \
  --features v2.1-wasm-browser \
  --scope ed2kia \
  2>&1 || {
    echo "[ERROR] wasm-pack build failed. Aborting."
    exit 1
  }

# ------------------------------------------
# Validate: Check .wasm and .js exist
# ------------------------------------------
echo "[validate] Checking artifacts..."
WASM_FOUND=false
JS_FOUND=false

for f in "$DIST_DIR"/*.wasm; do
  [ -e "$f" ] && WASM_FOUND=true && echo "  Found WASM: $(basename "$f")"
done

for f in "$DIST_DIR"/*.js; do
  [ -e "$f" ] && JS_FOUND=true && echo "  Found JS:   $(basename "$f")"
done

if [ "$WASM_FOUND" = false ]; then
  echo "[ERROR] No .wasm artifact found in $DIST_DIR"
  exit 1
fi

if [ "$JS_FOUND" = false ]; then
  echo "[WARN]  No .js glue found — library-only build, expected for scaffold"
fi

# ------------------------------------------
# Checksum: SHA-256 for reproducibility
# ------------------------------------------
echo "[checksum] Generating SHA-256..."
if command -v sha256sum &>/dev/null; then
  sha256sum "$DIST_DIR"/*.wasm "$DIST_DIR"/*.js 2>/dev/null > "$DIST_DIR/checksums.sha256" || true
elif command -v shasum &>/dev/null; then
  shasum -a 256 "$DIST_DIR"/*.wasm "$DIST_DIR"/*.js 2>/dev/null > "$DIST_DIR/checksums.sha256" || true
else
  echo "[WARN]  No sha256 tool available, skipping checksum"
fi

# ------------------------------------------
# Report
# ------------------------------------------
echo ""
echo "=== WASM Build Complete ==="
echo "Artifacts in: $DIST_DIR"
ls -la "$DIST_DIR"
echo ""
echo "SHA-256 checksums:"
cat "$DIST_DIR/checksums.sha256" 2>/dev/null || echo "  (none)"
echo ""
echo "=== Pipeline SUCCESS ==="
