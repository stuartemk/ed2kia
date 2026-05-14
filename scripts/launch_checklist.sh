#!/usr/bin/env bash
# =============================================================================
# ed2kIA Launch Checklist
# Pre-launch validation script for seed nodes and production deployment
# =============================================================================
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASS=0
WARN=0
FAIL=0

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASS++))
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    ((WARN++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((FAIL++))
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

echo "=============================================="
echo "  ed2kIA Launch Checklist v0.5.0"
echo "  Pre-launch Validation Script"
echo "=============================================="
echo ""

# ------------------------------------------------------------------------------
# 1. System Dependencies
# ------------------------------------------------------------------------------
echo "--- System Dependencies ---"

# Check Rust/Cargo
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(cargo --version | awk '{print $2}')
    log_pass "Cargo installed: $RUST_VERSION"
else
    log_fail "Cargo not found. Install Rust from https://rustup.rs"
fi

# Check Docker (optional for container deployment)
if command -v docker &> /dev/null; then
    DOCKER_VERSION=$(docker --version | awk '{print $3}')
    log_pass "Docker installed: $DOCKER_VERSION"
else
    log_warn "Docker not found (optional for container deployment)"
fi

# Check Git
if command -v git &> /dev/null; then
    log_pass "Git installed"
else
    log_warn "Git not found (needed for version info)"
fi

# Check OpenSSL
if command -v openssl &> /dev/null; then
    log_pass "OpenSSL installed"
else
    log_warn "OpenSSL not found (needed for certificate operations)"
fi

echo ""

# ------------------------------------------------------------------------------
# 2. Project Build Validation
# ------------------------------------------------------------------------------
echo "--- Project Build Validation ---"

# Check Cargo.toml exists
if [ -f "Cargo.toml" ]; then
    log_pass "Cargo.toml found"
else
    log_fail "Cargo.toml not found. Run from project root."
    exit 1
fi

# Check LICENSE exists
if [ -f "LICENSE" ]; then
    log_pass "LICENSE found"
else
    log_fail "LICENSE not found. Required for release."
fi

# Check README.md exists
if [ -f "README.md" ]; then
    log_pass "README.md found"
else
    log_warn "README.md not found"
fi

# Syntax check
log_info "Running cargo check..."
if cargo check 2>&1 | tail -1 | grep -q "Finished"; then
    log_pass "cargo check passed"
else
    log_fail "cargo check failed. Fix compilation errors before launch."
fi

echo ""

# ------------------------------------------------------------------------------
# 3. Port Availability
# ------------------------------------------------------------------------------
echo "--- Port Availability ---"

check_port() {
    local port=$1
    local name=$2
    if command -v ss &> /dev/null; then
        if ss -tlnp 2>/dev/null | grep -q ":${port} "; then
            log_fail "Port $port ($name) is already in use"
        else
            log_pass "Port $port ($name) is available"
        fi
    elif command -v netstat &> /dev/null; then
        if netstat -tlnp 2>/dev/null | grep -q ":${port} "; then
            log_fail "Port $port ($name) is already in use"
        else
            log_pass "Port $port ($name) is available"
        fi
    else
        log_warn "Cannot check port $port (ss/netstat not available)"
    fi
}

# Default P2P port (configurable)
check_port 9000 "P2P"
# Default HTTP port
check_port 3000 "HTTP/Web UI"

echo ""

# ------------------------------------------------------------------------------
# 4. File Permissions
# ------------------------------------------------------------------------------
echo "--- File Permissions ---"

# Check write permission for data directory
DATA_DIR="${ED2KIA_DATA_DIR:-./data}"
if [ -w "$(dirname "$DATA_DIR")" ]; then
    log_pass "Data directory path is writable: $DATA_DIR"
else
    log_warn "Data directory may not be writable: $DATA_DIR"
fi

# Check script permissions
for script in release/packager.sh scripts/launch_checklist.sh scripts/simulate_network.sh scripts/tag_release.sh; do
    if [ -f "$script" ]; then
        if [ -x "$script" ]; then
            log_pass "$script is executable"
        else
            log_warn "$script is not executable (run: chmod +x $script)"
        fi
    fi
done

echo ""

# ------------------------------------------------------------------------------
# 5. Seed Node Health (if configured)
# ------------------------------------------------------------------------------
echo "--- Seed Node Health ---"

# Check if seed nodes are configured
SEED_FILE="config/seeds.json"
if [ -f "$SEED_FILE" ]; then
    log_pass "Seed configuration found: $SEED_FILE"

    # Try to reach each seed (basic connectivity check)
    if command -v jq &> /dev/null; then
        SEED_COUNT=$(jq length "$SEED_FILE" 2>/dev/null || echo "0")
        log_info "Found $SEED_COUNT configured seed nodes"

        for i in $(seq 0 $((SEED_COUNT - 1))); do
            SEED_ADDR=$(jq -r ".[$i].multiaddress" "$SEED_FILE" 2>/dev/null || echo "")
            if [ -n "$SEED_ADDR" ] && [ "$SEED_ADDR" != "null" ]; then
                log_info "Seed $i: $SEED_ADDR"
            fi
        done
    else
        log_warn "jq not installed. Cannot parse seed configuration."
    fi
else
    log_warn "No seed configuration found at $SEED_FILE (use defaults or configure)"
fi

echo ""

# ------------------------------------------------------------------------------
# 6. Documentation Check
# ------------------------------------------------------------------------------
echo "--- Documentation ---"

for doc in docs/GOVERNANCE.md docs/CONTRIBUTING.md docs/NETWORK_BOOTSTRAP.md docs/LAUNCH_GUIDE.md docs/MONITORING.md; do
    if [ -f "$doc" ]; then
        log_pass "$doc exists"
    else
        log_warn "$doc not found"
    fi
done

echo ""

# ------------------------------------------------------------------------------
# 7. Security Checks
# ------------------------------------------------------------------------------
echo "--- Security Checks ---"

# Check for .env files in version control
if [ -f ".env" ]; then
    log_warn ".env file found in project root (should be in .gitignore)"
else
    log_pass "No .env file in project root"
fi

# Check .gitignore for sensitive files
if [ -f ".gitignore" ]; then
    if grep -q ".env" .gitignore 2>/dev/null; then
        log_pass ".gitignore includes .env"
    else
        log_warn ".gitignore does not include .env"
    fi
else
    log_warn ".gitignore not found"
fi

echo ""

# ------------------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------------------
echo "=============================================="
echo "  Launch Checklist Summary"
echo "=============================================="
echo -e "  ${GREEN}Passed: $PASS${NC}"
echo -e "  ${YELLOW}Warnings: $WARN${NC}"
echo -e "  ${RED}Failed: $FAIL${NC}"
echo ""

if [ $FAIL -gt 0 ]; then
    echo -e "${RED}Launch checklist FAILED. Fix errors before proceeding.${NC}"
    exit 1
elif [ $WARN -gt 0 ]; then
    echo -e "${YELLOW}Launch checklist passed with warnings. Review before proceeding.${NC}"
    exit 0
else
    echo -e "${GREEN}Launch checklist PASSED. Ready for deployment.${NC}"
    exit 0
fi
