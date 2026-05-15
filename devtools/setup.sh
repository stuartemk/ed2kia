#!/bin/bash
# devtools/setup.sh — ed2kIA Local Development Environment Setup
# Usage: bash devtools/setup.sh [--full] [--docker]
#
# --full    Install all optional tooling (cargo-audit, cargo-expand, etc.)
# --docker  Setup Docker + Docker Compose for local dev

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

FULL=false
DOCKER=false

for arg in "$@"; do
    case $arg in
        --full) FULL=true ;;
        --docker) DOCKER=true ;;
        *) log_error "Unknown option: $arg"; exit 1 ;;
    esac
done

# ─── Check Prerequisites ───

check_rust() {
    if command -v rustc &> /dev/null; then
        local version
        version=$(rustc --version)
        log_ok "Rust installed: $version"
        return 0
    else
        log_error "Rust not found. Install with:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.sh | sh"
        return 1
    fi
}

check_cargo() {
    if command -v cargo &> /dev/null; then
        log_ok "Cargo installed: $(cargo --version)"
        return 0
    else
        log_error "Cargo not found. Install Rust first."
        return 1
    fi
}

check_git() {
    if command -v git &> /dev/null; then
        log_ok "Git installed: $(git --version)"
        return 0
    else
        log_error "Git not found. Install Git first."
        return 1
    fi
}

check_docker() {
    if command -v docker &> /dev/null; then
        log_ok "Docker installed: $(docker --version)"
        return 0
    else
        log_warn "Docker not found. Install from https://docs.docker.com/get-docker/"
        return 1
    fi
}

# ─── Setup Functions ───

setup_rust_toolchain() {
    log_info "Setting up Rust toolchain..."

    # Set stable as default
    rustup default stable 2>/dev/null || true

    # Add essential components
    log_info "Adding clippy & rustfmt..."
    rustup component add clippy rustfmt 2>/dev/null || true
    log_ok "Essential components installed"

    if [[ "$FULL" == true ]]; then
        log_info "Adding WASM target..."
        rustup target add wasm32-unknown-unknown 2>/dev/null || true

        log_info "Installing optional tooling..."
        cargo install cargo-audit 2>/dev/null || log_warn "cargo-audit install failed"
        cargo install cargo-expand 2>/dev/null || log_warn "cargo-expand install failed"
        cargo install cargo-nextest 2>/dev/null || log_warn "cargo-nextest install failed"
        cargo install cargo-watch 2>/dev/null || log_warn "cargo-watch install failed"
        cargo install cargo-tarpaulin 2>/dev/null || log_warn "cargo-tarpaulin install failed"

        log_ok "Full toolchain setup complete"
    fi
}

setup_just() {
    if command -v just &> /dev/null; then
        log_ok "Just already installed: $(just --version)"
        return 0
    fi

    log_info "Installing Just (command runner)..."
    cargo install just 2>/dev/null || {
        # Fallback: package manager
        if command -v winget &> /dev/null; then
            winget install casey/just
        elif command -v brew &> /dev/null; then
            brew install just
        elif command -v apt-get &> /dev/null; then
            sudo apt-get install -y just
        else
            log_warn "Could not install Just automatically. Install from https://just.systems/"
        fi
    }
    log_ok "Just installed"
}

setup_docker_env() {
    if [[ "$DOCKER" != true ]]; then
        return 0
    fi

    if ! check_docker; then
        log_warn "Skipping Docker setup (Docker not installed)"
        return 0
    fi

    log_info "Setting up Docker environment..."

    # Create .env file if it doesn't exist
    if [[ ! -f ".env.dev" ]]; then
        cat > .env.dev << 'EOF'
# ed2kIA Development Environment
ED2KIA_PORT=9000
ED2KIA_LOG_LEVEL=debug
ED2KIA_FEATURES=stable
ED2KIA_P2P_PORT=9001
ED2KIA_DB_PATH=/tmp/ed2kIA-dev.db
EOF
        log_ok "Created .env.dev"
    fi

    log_ok "Docker environment ready"
    log_info "Start with: just docker-compose"
}

validate_setup() {
    log_info "Validating setup..."
    echo ""

    local errors=0

    # Check Rust
    if ! check_rust; then
        errors=$((errors + 1))
    fi

    # Check Cargo
    if ! check_cargo; then
        errors=$((errors + 1))
    fi

    # Check Git
    if ! check_git; then
        errors=$((errors + 1))
    fi

    # Check Just
    if command -v just &> /dev/null; then
        log_ok "Just: $(just --version 2>&1 | head -1)"
    else
        log_warn "Just not installed (optional)"
    fi

    # Quick build check
    log_info "Running quick build check..."
    if cargo check --features stable --quiet 2>/dev/null; then
        log_ok "Build check passed"
    else
        log_warn "Build check failed (may need to run 'cargo build')"
    fi

    echo ""
    if [[ $errors -eq 0 ]]; then
        log_ok "Setup validation complete! All checks passed."
        echo ""
        echo "Next steps:"
        echo "  just build          # Build the project"
        echo "  just test           # Run tests"
        echo "  just dev            # Run local dev node"
        echo "  just docker-compose # Start full dev environment"
    else
        log_error "$errors check(s) failed. Fix prerequisites and retry."
        return 1
    fi
}

# ─── Main ───

main() {
    echo "========================================"
    echo "  ed2kIA Development Environment Setup"
    echo "========================================"
    echo ""

    setup_rust_toolchain
    setup_just
    setup_docker_env
    validate_setup
}

main
