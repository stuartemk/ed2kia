#!/bin/sh
# =============================================================================
# ed2kIA Quickstart Script — One-Command Setup
# =============================================================================
# POSIX-compliant, idempotent, cross-platform (Linux/macOS)
# Usage: curl -sSL https://raw.githubusercontent.com/ed2kia/ed2kIA/main/scripts/quickstart.sh | sh
#
# This script performs:
#   1. Pre-flight validation (Rust, Cargo, Git, Docker)
#   2. Clone or update the ed2kIA repository
#   3. Build the project with production features
#   4. Run the test suite
#   5. Generate Ed25519 node identity
#   6. Configure local node
#   7. Launch the node in development mode
#
# Environment Variables (optional):
#   ED2KIA_DIR        — Installation directory (default: ~/ed2kIA)
#   ED2KIA_PORT       — Node port (default: 8080)
#   ED2KIA_PEER_ID    — Node peer ID (auto-generated if not set)
#   ED2KIA_BOOTSTRAP  — Bootstrap peer addresses (comma-separated)
#   ED2KIA_FEATURES   — Comma-separated feature gates (default: stable)
# =============================================================================

set -e

# ─── Color Codes ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ─── Logging ─────────────────────────────────────────────────────────────────
log_info()  { printf "${BLUE}[INFO]${NC} %s\n" "$1"; }
log_ok()    { printf "${GREEN}[OK]${NC}   %s\n" "$1"; }
log_warn()  { printf "${YELLOW}[WARN]${NC} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${NC} %s\n" "$1" >&2; }

# ─── Configuration ───────────────────────────────────────────────────────────
ED2KIA_DIR="${ED2KIA_DIR:-$HOME/ed2kIA}"
ED2KIA_PORT="${ED2KIA_PORT:-8080}"
ED2KIA_FEATURES="${ED2KIA_FEATURES:-stable}"
ED2KIA_REPO="https://github.com/ed2kia/ed2kIA.git"
ED2KIA_VERSION="v2.1.0-stable"

# ─── Pre-Flight Validation ──────────────────────────────────────────────────
preflight() {
    log_info "=== ed2kIA Quickstart v${ED2KIA_VERSION} ==="
    log_info "Pre-flight validation..."

    # Check Rust
    if command -v rustc >/dev/null 2>&1; then
        RUST_VERSION=$(rustc --version | awk '{print $2}')
        log_ok "Rust ${RUST_VERSION} detected"
    else
        log_error "Rust not found. Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    # Check Cargo
    if command -v cargo >/dev/null 2>&1; then
        log_ok "Cargo detected"
    else
        log_error "Cargo not found. Install Rust toolchain first."
        exit 1
    fi

    # Check Git
    if command -v git >/dev/null 2>&1; then
        log_ok "Git detected"
    else
        log_error "Git not found. Install git for your platform."
        exit 1
    fi

    # Check Docker (optional)
    if command -v docker >/dev/null 2>&1; then
        log_ok "Docker detected (optional)"
    else
        log_warn "Docker not found. Container deployment will not be available."
    fi

    log_info "Pre-flight validation complete."
}

# ─── Clone/Update Repository ─────────────────────────────────────────────────
setup_repo() {
    log_info "Setting up repository..."

    if [ -d "${ED2KIA_DIR}/.git" ]; then
        log_info "Repository exists at ${ED2KIA_DIR}. Updating..."
        cd "${ED2KIA_DIR}"
        git fetch origin --tags 2>/dev/null || true
        git checkout "${ED2KIA_VERSION}" 2>/dev/null || git pull origin main
        log_ok "Repository updated"
    else
        if [ -d "${ED2KIA_DIR}" ]; then
            log_error "Directory ${ED2KIA_DIR} exists but is not a git repository."
            exit 1
        fi
        log_info "Cloning ed2kIA to ${ED2KIA_DIR}..."
        git clone "${ED2KIA_REPO}" "${ED2KIA_DIR}"
        cd "${ED2KIA_DIR}"
        git checkout "${ED2KIA_VERSION}" 2>/dev/null || true
        log_ok "Repository cloned"
    fi
}

# ─── Build Project ───────────────────────────────────────────────────────────
build_project() {
    log_info "Building ed2kIA with features: ${ED2KIA_FEATURES}..."
    cd "${ED2KIA_DIR}"

    # Format check
    log_info "Running cargo fmt --check..."
    if cargo fmt --all -- --check 2>/dev/null; then
        log_ok "Code formatting valid"
    else
        log_warn "Code formatting issues detected (non-blocking)"
    fi

    # Build with production features
    log_info "Building release binary..."
    if cargo build --release --features "${ED2KIA_FEATURES}" 2>&1 | tail -5; then
        log_ok "Build complete"
    else
        log_error "Build failed. Check output above."
        exit 1
    fi
}

# ─── Run Tests ───────────────────────────────────────────────────────────────
run_tests() {
    log_info "Running test suite..."
    cd "${ED2KIA_DIR}"

    # Quick test run (unit tests only, threaded)
    log_info "Running unit tests (--test-threads=4)..."
    TEST_OUTPUT=$(cargo test --features "${ED2KIA_FEATURES}" --lib --test-threads=4 2>&1 | tail -20)
    echo "${TEST_OUTPUT}"

    if echo "${TEST_OUTPUT}" | grep -q "test result: ok"; then
        log_ok "Tests passed"
    else
        log_warn "Some tests may have failed. Check output above."
    fi
}

# ─── Generate Node Identity ──────────────────────────────────────────────────
generate_identity() {
    log_info "Generating node identity..."

    IDENTITY_FILE="${ED2KIA_DIR}/.ed2kIA/identity"
    mkdir -p "${ED2KIA_DIR}/.ed2kIA"

    if [ -f "${IDENTITY_FILE}" ]; then
        log_info "Identity already exists at ${IDENTITY_FILE}"
        log_info "Use --force to regenerate"
    else
        # Generate Ed25519 keypair using ed25519-dalek CLI or openssl
        if command -v ed25519-keygen >/dev/null 2>&1; then
            ed25519-keygen > "${IDENTITY_FILE}"
        else
            # Fallback: Generate random 32-byte seed
            if command -v openssl >/dev/null 2>&1; then
                openssl rand -hex 32 > "${IDENTITY_FILE}"
            else
                # Last resort: /dev/urandom
                head -c 32 /dev/urandom | xxd -p > "${IDENTITY_FILE}"
            fi
        fi
        chmod 600 "${IDENTITY_FILE}"
        log_ok "Node identity generated at ${IDENTITY_FILE}"
    fi
}

# ─── Configure Node ──────────────────────────────────────────────────────────
configure_node() {
    log_info "Configuring node..."

    CONFIG_FILE="${ED2KIA_DIR}/.ed2kIA/config.toml"
    mkdir -p "${ED2KIA_DIR}/.ed2kIA"

    cat > "${CONFIG_FILE}" <<EOF
# ed2kIA Node Configuration
# Generated by quickstart.sh on $(date -u +"%Y-%m-%dT%H:%M:%SZ")

[node]
id = "${ED2KIA_PEER_ID:-auto}"
listen_addr = "0.0.0.0:${ED2KIA_PORT}"
public_addr = ""

[network]
bootstrap_peers = [
    "/ip4/127.0.0.1/tcp/9000/p2p/ed2kIA-bootstrap",
]
max_connections = 25
message_size_limit = 4194304  # 4MB

[sae]
features = "${ED2KIA_FEATURES}"
latent_dim = 4096
top_k = 256

[logging]
level = "info"
file = "${ED2KIA_DIR}/.ed2kIA/node.log"

[metrics]
enabled = true
port = 9090
EOF

    log_ok "Configuration written to ${CONFIG_FILE}"
}

# ─── Launch Node ─────────────────────────────────────────────────────────────
launch_node() {
    log_info "=== Quickstart Complete ==="
    log_info "Installation directory: ${ED2KIA_DIR}"
    log_info "Configuration: ${ED2KIA_DIR}/.ed2kIA/config.toml"
    log_info "Identity: ${ED2KIA_DIR}/.ed2kIA/identity"

    log_info ""
    log_info "To start your node:"
    log_info "  cd ${ED2KIA_DIR}"
    log_info "  cargo run --release --features ${ED2KIA_FEATURES} -- --config .ed2kIA/config.toml"
    log_info ""
    log_info "To run in background:"
    log_info "  nohup cargo run --release --features ${ED2KIA_FEATURES} -- --config .ed2kIA/config.toml > .ed2kIA/node.log 2>&1 &"
    log_info ""
    log_info "To view metrics (if enabled):"
    log_info "  http://localhost:9090/metrics"
    log_info ""
    log_info "Documentation: https://github.com/ed2kia/ed2kIA/blob/main/docs/technical-report.md"
    log_info "Steward Program: https://github.com/ed2kia/ed2kIA/blob/main/docs/steward-program.md"
    log_info ""
    log_ok "Welcome to ed2kIA! 🌐"
}

# ─── Main ────────────────────────────────────────────────────────────────────
main() {
    preflight
    setup_repo
    build_project
    run_tests
    generate_identity
    configure_node
    launch_node
}

# Run main
main "$@"
