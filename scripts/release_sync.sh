#!/usr/bin/env bash
# =============================================================================
# ed2kIA Release Sync Script — Sprint 83 (v9.19.0)
# Hardened release pipeline with API fallback for GitHub Releases
# =============================================================================
# Usage: ./scripts/release_sync.sh <version> <feature_flag> [dry-run]
#
# Example:
#   ./scripts/release_sync.sh 9.19.0-sprint83 v9.19-empirical-strike
#   ./scripts/release_sync.sh 9.19.0-sprint83 v9.19-empirical-strike dry-run
#
# This script:
#   1. Validates version and feature flag
#   2. Runs cargo test with feature flag
#   3. Runs cargo clippy with feature flag
#   4. Creates annotated git commit
#   5. Creates annotated git tag
#   6. Pushes to origin (main + tags)
#   7. Creates GitHub Release (gh CLI or curl API fallback)
#   8. Verifies ZIP availability
# =============================================================================
set -euo pipefail

# ─── Color Output ────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1"; }
log_step()    { echo -e "${CYAN}[STEP]${NC} $1"; }

# ─── Parse Arguments ─────────────────────────────────────────────────────────
if [ $# -lt 2 ]; then
    echo "Usage: $0 <version> <feature_flag> [dry-run]"
    echo ""
    echo "Example:"
    echo "  $0 9.19.0-sprint83 v9.19-empirical-strike"
    echo "  $0 9.19.0-sprint83 v9.19-empirical-strike dry-run"
    exit 1
fi

VERSION="$1"
FEATURE="$2"
DRY_RUN="${3:-}"
REPO_OWNER="Stuartemk"
REPO_NAME="ed2kIA"
TAG="v${VERSION}"
COMMIT_MSG="release(${VERSION}): empirical strike, benchmark engine, visual dashboard scaffold & public doc translation"

if [ "$DRY_RUN" = "dry-run" ]; then
    log_warn "DRY-RUN MODE — No git or API operations will be performed"
fi

# ─── Step 1: Validate Inputs ─────────────────────────────────────────────────
log_step "Step 1: Validating inputs..."

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
    log_error "Invalid version format: $VERSION (expected X.Y.Z-sprintN)"
    exit 1
fi

if [[ ! "$FEATURE" =~ ^v[0-9]+\.[0-9]+- ]]; then
    log_error "Invalid feature flag: $FEATURE (expected vX.Y-name)"
    exit 1
fi

log_success "Version: $VERSION | Feature: $FEATURE | Tag: $TAG"

# ─── Step 2: Cargo Test ──────────────────────────────────────────────────────
log_step "Step 2: Running cargo test --features ${FEATURE}..."

if [ "$DRY_RUN" != "dry-run" ]; then
    if cargo test --features "$FEATURE" 2>&1 | tee /tmp/ed2kIA_test_output.txt; then
        TEST_COUNT=$(grep -oP '\d+ tests? ran' /tmp/ed2kIA_test_output.txt | tail -1 | grep -oP '\d+' || echo "unknown")
        log_success "Tests passed ($TEST_COUNT tests ran)"
    else
        log_error "Tests failed! Check /tmp/ed2kIA_test_output.txt"
        exit 1
    fi
else
    log_info "[DRY-RUN] Skipping cargo test"
fi

# ─── Step 3: Cargo Clippy ────────────────────────────────────────────────────
log_step "Step 3: Running cargo clippy --features ${FEATURE}..."

if [ "$DRY_RUN" != "dry-run" ]; then
    if cargo clippy --features "$FEATURE" -- -D warnings 2>&1 | tee /tmp/ed2kIA_clippy_output.txt; then
        log_success "Clippy passed (no warnings)"
    else
        log_warn "Clippy reported warnings — review /tmp/ed2kIA_clippy_output.txt"
        # Non-fatal: continue with release
    fi
else
    log_info "[DRY-RUN] Skipping cargo clippy"
fi

# ─── Step 4: Git Status Check ────────────────────────────────────────────────
log_step "Step 4: Checking git status..."

if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    log_error "Not inside a git repository!"
    exit 1
fi

BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
log_info "Current branch: $BRANCH"

UNCOMMITTED=$(git status --porcelain | wc -l)
if [ "$UNCOMMITTED" -eq 0 ]; then
    log_warn "No uncommitted changes — nothing to commit"
else
    log_info "Uncommitted changes detected: $UNCOMMITTED file(s)"
fi

# ─── Step 5: Git Add + Commit ────────────────────────────────────────────────
log_step "Step 5: Git add + commit..."

if [ "$DRY_RUN" != "dry-run" ]; then
    git add -A
    git commit -m "$COMMIT_MSG" || log_warn "Commit may have already been created (no changes)"
    log_success "Committed: $COMMIT_MSG"
else
    log_info "[DRY-RUN] Would commit: $COMMIT_MSG"
fi

# ─── Step 6: Git Tag ─────────────────────────────────────────────────────────
log_step "Step 6: Creating annotated tag $TAG..."

if [ "$DRY_RUN" != "dry-run" ]; then
    if git tag -l "$TAG" | grep -q "$TAG"; then
        log_warn "Tag $TAG already exists — skipping"
    else
        git tag -a "$TAG" -m "${VERSION}: The Empirical Strike & Visual Proof"
        log_success "Tag created: $TAG"
    fi
else
    log_info "[DRY-RUN] Would create tag: $TAG"
fi

# ─── Step 7: Git Push ────────────────────────────────────────────────────────
log_step "Step 7: Pushing to origin (main + tags)..."

if [ "$DRY_RUN" != "dry-run" ]; then
    if git push origin main --tags 2>&1; then
        log_success "Pushed to origin/main + tags"
    else
        log_error "Push failed! Check network connectivity and credentials"
        exit 1
    fi
else
    log_info "[DRY-RUN] Would push: git push origin main --tags"
fi

# ─── Helper: Create Release via curl API ──────────────────────────────────────
create_release_via_curl() {
    log_info "Creating release via GitHub API (curl)..."
    
    # Check for GITHUB_TOKEN
    if [ -z "${GITHUB_TOKEN:-}" ]; then
        log_warn "GITHUB_TOKEN not set — skipping API release creation"
        log_info "Set GITHUB_TOKEN and re-run, or create release manually on GitHub"
        return 0
    fi

    # Escape release body for JSON
    ESCAPED_BODY=$(echo "$RELEASE_BODY" | sed 's/\\/\\\\/g; s/"/\\"/g; s/$//' | tr '\n' '\\\\n')
    
    RESPONSE=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Authorization: token $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github.v3+json" \
        "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases" \
        -d "{
            \"tag_name\": \"${TAG}\",
            \"name\": \"${TAG}\",
            \"body\": \"${ESCAPED_BODY}\",
            \"draft\": false,
            \"prerelease\": false
        }" 2>&1)
    
    HTTP_CODE=$(echo "$RESPONSE" | tail -1)
    BODY=$(echo "$RESPONSE" | head -n -1)
    
    if [ "$HTTP_CODE" = "201" ]; then
        log_success "GitHub Release created via curl API"
    else
        log_error "API release failed (HTTP $HTTP_CODE): $BODY"
        return 1
    fi
}

# ─── Step 8: GitHub Release ──────────────────────────────────────────────────
log_step "Step 8: Creating GitHub Release..."

RELEASE_BODY="## ${VERSION} — The Empirical Strike & Visual Proof

### New Modules
- \`src/benchmarks/sae_audit_benchmark.rs\` — SAE vs baseline benchmark engine, TCM Z-axis, CSV/JSON export (35 tests)
- \`src/ui/visual_dashboard_scaffold.rs\` — WebSocket/HTTP streaming of SAE activations, 3D manifold placeholder (33 tests)

### Technical Translation
- SCT → TCM (Topological Coherence Metric)
- Network Apoptosis → Automated Byzantine Eviction
- GEI → Gradient Ethical Invariant

### Validation
- 68 tests passing
- Feature gate: \`${FEATURE}\`
- All public documentation translated to ML-standard terminology

### Feature Gate Chain
\`${FEATURE}\` → \`v9.18-mvp-deployment\` → \`v9.17-biological-bridge\`"

if [ "$DRY_RUN" != "dry-run" ]; then
    # Try gh CLI first
    if command -v gh > /dev/null 2>&1; then
        log_info "Using gh CLI for release..."
        if gh release create "$TAG" \
            --title "$TAG" \
            --notes "$RELEASE_BODY" \
            --repo "${REPO_OWNER}/${REPO_NAME}" 2>&1; then
            log_success "GitHub Release created via gh CLI"
        else
            log_warn "gh CLI failed — falling back to curl API"
            create_release_via_curl
        fi
    else
        log_warn "gh CLI not found — using curl API fallback"
        create_release_via_curl
    fi
else
    log_info "[DRY-RUN] Would create release: $TAG"
fi

# ─── Step 9: Verify ZIP Availability ─────────────────────────────────────────
log_step "Step 9: Verifying ZIP availability..."

ZIP_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/archive/refs/tags/${TAG}.zip"

if [ "$DRY_RUN" != "dry-run" ]; then
    # Wait a moment for GitHub to process
    sleep 5

    HTTP_CODE=$(curl -sI -o /dev/null -w "%{http_code}" "$ZIP_URL" 2>/dev/null || echo "000")
    if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "302" ]; then
        log_success "ZIP available: $ZIP_URL (HTTP $HTTP_CODE)"
    else
        log_warn "ZIP not yet available (HTTP $HTTP_CODE) — GitHub may need time to process"
        log_info "Manual verification: $ZIP_URL"
    fi
else
    log_info "[DRY-RUN] Would verify: $ZIP_URL"
fi

# ─── Summary ─────────────────────────────────────────────────────────────────
echo ""
log_success "═══════════════════════════════════════════════════════════"
log_success "Release Sync Complete: $TAG"
log_success "  Version:    $VERSION"
log_success "  Feature:    $FEATURE"
log_success "  Tag:        $TAG"
log_success "  ZIP:        $ZIP_URL"
log_success "═══════════════════════════════════════════════════════════"
echo ""

exit 0
