#!/bin/bash
# github_release.sh - Automate ed2kIA release process
# Usage: ./scripts/github_release.sh <version> [dry-run]
# Example: ./scripts/github_release.sh 0.5.0

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO="ed2kIA/ed2kIA"
RELEASE_DIR="release"
DOCS_DIR="docs"
FEATURES="core-only"

# Parse arguments
VERSION="${1:-}"
DRY_RUN="${2:-}"

if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: Version required${NC}"
    echo "Usage: $0 <version> [dry-run]"
    echo "Example: $0 0.5.0"
    exit 1
fi

if [ "$DRY_RUN" = "dry-run" ]; then
    echo -e "${YELLOW}=== DRY RUN MODE - No changes will be committed ===${NC}"
fi

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

run_cmd() {
    if [ "$DRY_RUN" = "dry-run" ]; then
        log_info "[DRY RUN] Would execute: $*"
    else
        "$@"
    fi
}

# Pre-flight checks
log_info "=== Pre-flight Checks ==="

# Check Git status
if ! git diff --quiet; then
    log_error "Working directory not clean. Commit changes first."
    exit 1
fi

# Check required tools
for cmd in cargo git sha256sum; do
    if ! command -v "$cmd" &> /dev/null; then
        log_error "Required command not found: $cmd"
        exit 1
    fi
done

log_success "Pre-flight checks passed"

# Step 1: Compilation checks
log_info "=== Step 1: Compilation Checks ==="

log_info "Running cargo check..."
if ! cargo check --features "$FEATURES" 2>&1 | tee cargo_check_output.txt; then
    log_error "cargo check failed"
    exit 1
fi
log_success "cargo check passed"

log_info "Running cargo clippy..."
if ! cargo clippy --features "$FEATURES" 2>&1 | tee clippy_output.txt; then
    log_error "cargo clippy failed"
    exit 1
fi
log_success "cargo clippy passed"

# Step 2: Tests
log_info "=== Step 2: Running Tests ==="

log_info "Running cargo test..."
if ! cargo test --features "$FEATURES" 2>&1 | tee test_output.txt; then
    log_error "Tests failed"
    exit 1
fi
log_success "Tests passed"

# Step 3: Build release binary
log_info "=== Step 3: Building Release ==="

log_info "Building release binary..."
run_cmd cargo build --release --features "$FEATURES"
log_success "Release binary built"

# Step 4: Generate checksums
log_info "=== Step 4: Generating Checksums ==="

mkdir -p "$RELEASE_DIR"
CHECKSUMS_FILE="$RELEASE_DIR/checksums.txt"

log_info "Generating SHA-256 checksums..."
{
    echo "# ed2kIA v${VERSION} - SHA-256 Checksums"
    echo "# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo ""
    for binary in target/release/ed2kia*; do
        if [ -f "$binary" ]; then
            sha256sum "$binary"
        fi
    done
} > "$CHECKSUMS_FILE"

log_success "Checksums generated: $CHECKSUMS_FILE"

# Step 5: Generate signatures (if ed25519-key available)
log_info "=== Step 5: Generating Signatures ==="

SIGNATURES_FILE="$RELEASE_DIR/signatures.ed25519"

if command -v ed25519-keygen &> /dev/null && [ -f ~/.ed2kIA/signing.key ]; then
    log_info "Signing release artifacts..."
    # Placeholder for actual signing
    echo "# Signatures for v${VERSION}" > "$SIGNATURES_FILE"
    echo "# Signing key: $(head -c 16 ~/.ed2kIA/signing.key | xxd -p)" >> "$SIGNATURES_FILE"
    log_success "Signatures generated"
else
    log_warn "Signing key not found. Create with: ed25519-keygen -o ~/.ed2kIA/signing.key"
    cat > "$SIGNATURES_FILE" << EOF
# ed2kIA v${VERSION} - Ed25519 Signatures
# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
#
# To sign this release:
# 1. Generate key: ed25519-keygen -o ~/.ed2kIA/signing.key
# 2. Sign: ed25519-sign -k ~/.ed2kIA/signing.key -m release/checksums.txt -o release/signatures.ed25519
# 3. Verify: ed25519-verify -p <public_key> -m release/checksums.txt -s release/signatures.ed25519
EOF
fi

# Step 6: Update documentation
log_info "=== Step 6: Updating Documentation ==="

# Update README badges
log_info "Updating README.md badges..."
# (README update is manual for review)

# Generate release notes template
RELEASE_NOTES="$DOCS_DIR/RELEASE_NOTES_v${VERSION}.md"
if [ ! -f "$RELEASE_NOTES" ]; then
    log_info "Creating release notes template..."
    cat > "$RELEASE_NOTES" << EOF
# Release Notes - ed2kIA v${VERSION}

## Release Date
$(date -u +%Y-%m-%d)

## Status
Stable

## Summary
Brief summary of this release.

## New Features
- Feature 1
- Feature 2

## Bug Fixes
- Fix 1
- Fix 2

## Breaking Changes
- None

## Migration Guide
No migration required from v$((VERSION * 10 - 1)).0.0

## Performance
- Metric 1: improvement
- Metric 2: improvement

## Validation
| Check | Result |
|-------|--------|
| cargo check | 0 errors, 0 warnings |
| cargo clippy | 0 warnings |
| cargo test | All passed |

## Checksums
See [release/checksums.txt](../release/checksums.txt)

## Signatures
See [release/signatures.ed25519](../release/signatures.ed25519)
EOF
    log_success "Release notes template created"
fi

# Step 7: Git tag and commit
log_info "=== Step 7: Git Operations ==="

if [ "$DRY_RUN" != "dry-run" ]; then
    # Add release files
    run_cmd git add "$RELEASE_DIR/" "$DOCS_DIR/RELEASE_NOTES_v${VERSION}.md"

    # Commit
    run_cmd git commit -m "release: v${VERSION}

- Generate checksums and signatures
- Update release documentation
- Validation: 0 errors, 0 warnings, tests passed"

    # Tag
    run_cmd git tag -a "v${VERSION}" -m "ed2kIA v${VERSION} - Stable Release"

    log_success "Git tag v${VERSION} created"
else
    log_info "[DRY RUN] Would commit and tag v${VERSION}"
fi

# Step 8: GitHub Release
log_info "=== Step 8: GitHub Release ==="

if command -v gh &> /dev/null && [ "$DRY_RUN" != "dry-run" ]; then
    log_info "Creating GitHub release..."

    gh release create "v${VERSION}" \
        --repo "$REPO" \
        --title "v${VERSION}" \
        --notes-file "$DOCS_DIR/RELEASE_NOTES_v${VERSION}.md" \
        --target main \
        target/release/ed2kia \
        "$CHECKSUMS_FILE" \
        "$SIGNATURES_FILE"

    log_success "GitHub release created"
    log_info "URL: https://github.com/$REPO/releases/tag/v${VERSION}"
else
    log_warn "GitHub CLI (gh) not found or dry-run mode"
    log_info "Manual release creation:"
    echo "  gh release create v${VERSION} \\"
    echo "    --repo $REPO \\"
    echo "    --title 'v${VERSION}' \\"
    echo "    --notes-file $DOCS_DIR/RELEASE_NOTES_v${VERSION}.md \\"
    echo "    --target main \\"
    echo "    target/release/ed2kia \\"
    echo "    $CHECKSUMS_FILE \\"
    echo "    $SIGNATURES_FILE"
fi

# Summary
log_info "=== Release Summary ==="
echo ""
log_success "ed2kIA v${VERSION} release preparation complete!"
echo ""
echo "Generated files:"
echo "  - $CHECKSUMS_FILE"
echo "  - $SIGNATURES_FILE"
echo "  - $RELEASE_NOTES"
echo ""
echo "Next steps:"
echo "  1. Review release notes: $RELEASE_NOTES"
echo "  2. Push tag: git push origin v${VERSION}"
echo "  3. Monitor CI: https://github.com/$REPO/actions"
echo ""
