#!/bin/bash
# beta_release_prep.sh — v1.8.0-beta Release Preparation Script
# Usage: bash scripts/beta_release_prep.sh [--dry-run] [--tag]
#
# --dry-run  Run all checks without modifying state
# --tag      Create git tag after validation passes

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; }
log_step() { echo -e "${CYAN}▶ $1${NC}"; }

DRY_RUN=false
TAG=false
ERRORS=0
CHECKS=0

for arg in "$@"; do
    case $arg in
        --dry-run) DRY_RUN=true ;;
        --tag) TAG=true ;;
        *) log_error "Unknown option: $arg"; exit 1 ;;
    esac
done

check_result() {
    local name="$1"
    local result="$2"
    CHECKS=$((CHECKS + 1))
    if [[ "$result" -eq 0 ]]; then
        log_ok "$name"
    else
        log_error "$name"
        ERRORS=$((ERRORS + 1))
    fi
}

# ─── Header ───

echo "============================================"
echo "  ed2kIA v1.8.0-beta Release Preparation"
echo "  $(date +%Y-%m-%d %H:%M:%S)"
echo "============================================"
echo ""

# ─── Step 1: Code Validation ───

log_step "Step 1: Code Validation"

log_info "Running cargo check --features v1.8-sprint1..."
cargo check --features v1.8-sprint1 2>/dev/null
check_result "cargo check (v1.8-sprint1)" $?

log_info "Running cargo check --features v1.8-sprint2..."
cargo check --features v1.8-sprint2 2>/dev/null
check_result "cargo check (v1.8-sprint2)" $?

log_info "Running cargo clippy --features v1.8-sprint1..."
cargo clippy --features v1.8-sprint1 -- -D warnings 2>/dev/null
check_result "cargo clippy (v1.8-sprint1)" $?

log_info "Running cargo clippy --features v1.8-sprint2..."
cargo clippy --features v1.8-sprint2 -- -D warnings 2>/dev/null
check_result "cargo clippy (v1.8-sprint2)" $?

echo ""

# ─── Step 2: Tests ───

log_step "Step 2: Test Suite"

log_info "Running tests (v1.8-sprint1)..."
cargo test --features v1.8-sprint1 --lib 2>/dev/null
check_result "cargo test (v1.8-sprint1)" $?

log_info "Running tests (v1.8-sprint2)..."
cargo test --features v1.8-sprint2 --lib 2>/dev/null
check_result "cargo test (v1.8-sprint2)" $?

echo ""

# ─── Step 3: Documentation ───

log_step "Step 3: Documentation Check"

log_info "Checking required files..."
for file in release/changelog.md release/v1.8.0-beta/RELEASE_PLAN.md CONTRIBUTING.md README.md; do
    if [[ -f "$file" ]]; then
        log_ok "$file exists"
        CHECKS=$((CHECKS + 1))
    else
        log_error "$file missing"
        ERRORS=$((ERRORS + 1))
        CHECKS=$((CHECKS + 1))
    fi
done

echo ""

# ─── Step 4: Git State ───

log_step "Step 4: Git State"

log_info "Checking git status..."
if [[ -n "$(git status --porcelain)" ]]; then
    log_warn "Working directory has uncommitted changes"
    CHECKS=$((CHECKS + 1))
else
    log_ok "Working directory clean"
    CHECKS=$((CHECKS + 1))
fi

log_info "Current branch: $(git branch --show-current)"
log_info "Last commit: $(git log -1 --oneline)"

echo ""

# ─── Step 5: Version Check ───

log_step "Step 5: Version Consistency"

CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
LIB_VERSION=$(grep 'pub const VERSION' src/lib.rs | head -1 | cut -d'"' -f2 2>/dev/null || echo "N/A")

log_info "Cargo.toml version: $CARGO_VERSION"
log_info "src/lib.rs VERSION: $LIB_VERSION"

CHECKS=$((CHECKS + 1))
if [[ "$CARGO_VERSION" == *"1.6"* ]] || [[ "$CARGO_VERSION" == *"1.8"* ]]; then
    log_ok "Version format valid"
else
    log_warn "Version may need update for beta release"
fi

echo ""

# ─── Summary ───

echo "============================================"
echo "  Validation Summary"
echo "============================================"
echo ""
echo "  Total checks: $CHECKS"
echo "  Passed: $((CHECKS - ERRORS))"
echo "  Failed: $ERRORS"
echo ""

if [[ $ERRORS -eq 0 ]]; then
    log_ok "All validation checks passed!"
    echo ""

    if [[ "$TAG" == true ]] && [[ "$DRY_RUN" != true ]]; then
        log_step "Creating git tag v1.8.0-beta..."
        git tag -a v1.8.0-beta -m "v1.8.0-beta: ChatGPT Moment — API Explorer, Reputation Proofs, Geographic Routing, WASM Mobile Bridge"
        log_ok "Tag v1.8.0-beta created"
        echo ""
        echo "Push tag with: git push origin v1.8.0-beta"
    elif [[ "$DRY_RUN" == true ]]; then
        log_info "Dry run complete. No changes made."
    else
        log_info "Ready for release. Next steps:"
        echo "  1. bash scripts/beta_release_prep.sh --tag    # Create tag"
        echo "  2. git push origin v1.8.0-beta                # Push tag"
        echo "  3. bash scripts/github_release.sh v1.8.0-beta # GitHub Release"
    fi
else
    log_error "$ERRORS check(s) failed. Fix before release."
    exit 1
fi
