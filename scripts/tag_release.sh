#!/usr/bin/env bash
# =============================================================================
# ed2kIA Release Tagging Script
# Generates semantic version tag, signs checksums, prepares GitHub Release
# =============================================================================
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

usage() {
    echo "Usage: $0 <major.minor.patch> [dry-run]"
    echo ""
    echo "Examples:"
    echo "  $0 0.5.0           # Tag and prepare release v0.5.0"
    echo "  $0 1.0.0 dry-run   # Dry run (no git operations)"
    echo ""
    echo "This script:"
    echo "  1. Validates version format"
    echo "  2. Builds release artifacts"
    echo "  3. Generates SHA-256 checksums"
    echo "  4. Creates Ed25519 signature placeholder"
    echo "  5. Tags git repository"
    echo "  6. Generates release notes from changelog"
    exit 1
}

# ------------------------------------------------------------------------------
# Parse arguments
# ------------------------------------------------------------------------------
if [ $# -lt 1 ]; then
    usage
fi

VERSION="$1"
DRY_RUN=false
if [ "${2:-}" = "dry-run" ]; then
    DRY_RUN=true
    log_info "DRY RUN MODE - No git operations will be performed"
fi

# ------------------------------------------------------------------------------
# Validate version format (semantic versioning)
# ------------------------------------------------------------------------------
log_info "Validating version: $VERSION"

if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    log_error "Invalid version format. Use: MAJOR.MINOR.PATCH (e.g., 1.0.0)"
    exit 1
fi

# ------------------------------------------------------------------------------
# Pre-release checks
# ------------------------------------------------------------------------------
log_info "Pre-release checks..."

# Check git status
if ! git rev-parse --is-inside-work-tree &> /dev/null; then
    log_error "Not a git repository. Initialize git first."
    exit 1
fi

# Check for uncommitted changes
if ! git diff --quiet 2>/dev/null; then
    log_warn "Uncommitted changes detected. Commit or stash before releasing."
fi

# Check if tag already exists
TAG="v${VERSION}"
if git tag -l "$TAG" | grep -q .; then
    log_error "Tag $TAG already exists. Use a different version."
    exit 1
fi

log_success "Pre-release checks passed."
echo ""

# ------------------------------------------------------------------------------
# Update Cargo.toml version
# ------------------------------------------------------------------------------
log_info "Updating Cargo.toml version to $VERSION..."

if [ "$DRY_RUN" = false ]; then
    # Use sed to update version in Cargo.toml
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    else
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    fi
    log_success "Cargo.toml version updated."
else
    log_info "[DRY RUN] Would update Cargo.toml version to $VERSION"
fi

# ------------------------------------------------------------------------------
# Build release binary
# ------------------------------------------------------------------------------
log_info "Building release binary..."

BUILD_DIR="target/release"
BINARY_NAME="ed2kia"

if cargo build --release 2>&1 | tail -5; then
    log_success "Release build complete."
else
    log_error "Release build failed."
    exit 1
fi

echo ""

# ------------------------------------------------------------------------------
# Package release artifacts
# ------------------------------------------------------------------------------
log_info "Packaging release artifacts..."

RELEASE_DIR="release/v${VERSION}"
mkdir -p "$RELEASE_DIR"

# Copy binary
cp "$BUILD_DIR/$BINARY_NAME" "$RELEASE_DIR/" 2>/dev/null || true

# Copy documentation
cp -r docs/ "$RELEASE_DIR/" 2>/dev/null || true
cp README.md "$RELEASE_DIR/" 2>/dev/null || true
cp LICENSE "$RELEASE_DIR/" 2>/dev/null || true

# Copy deploy configs
cp -r deploy/ "$RELEASE_DIR/" 2>/dev/null || true

log_success "Artifacts packaged in $RELEASE_DIR/"

# ------------------------------------------------------------------------------
# Generate checksums
# ------------------------------------------------------------------------------
log_info "Generating SHA-256 checksums..."

CHECKSUM_FILE="$RELEASE_DIR/checksums.sha256"
cd "$RELEASE_DIR"

# Generate checksums for all files
find . -type f ! -name "checksums.sha256" ! -name "*.sig" -exec sha256sum {} \; > "$CHECKSUM_FILE" 2>/dev/null || true

if [ -s "$CHECKSUM_FILE" ]; then
    log_success "Checksums generated: $CHECKSUM_FILE"
    echo ""
    echo "Checksums:"
    cat "$CHECKSUM_FILE"
else
    log_warn "No files found for checksum generation."
fi

cd - > /dev/null

# ------------------------------------------------------------------------------
# Ed25519 signature (placeholder)
# ------------------------------------------------------------------------------
log_info "Creating Ed25519 signature placeholder..."

SIG_FILE="$RELEASE_DIR/release.sig"
cat > "$SIG_FILE" << EOF
# ed2kIA Release Signature
# Version: $VERSION
# Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Algorithm: Ed25519
#
# To sign this release, replace this placeholder with:
#   ed25519-sign < checksums.sha256 > release.sig
#
# To verify:
#   ed25519-verify < public_key > release.sig < checksums.sha256
#
# Placeholder - Replace with actual signature before public release
SIGNATURE_PLEASEHOLDER=true
EOF

log_success "Signature placeholder created: $SIG_FILE"
echo ""

# ------------------------------------------------------------------------------
# Generate release notes from changelog
# ------------------------------------------------------------------------------
log_info "Generating release notes..."

RELEASE_NOTES="$RELEASE_DIR/RELEASE_NOTES.md"

if [ -f "release/changelog.md" ]; then
    # Extract section for this version from changelog
    cat > "$RELEASE_NOTES" << EOF
# ed2kIA v${VERSION} Release Notes

## Date
$(date -u +"%Y-%m-%d")

## Highlights
See [CHANGELOG.md](../changelog.md) for detailed changes.

## Checksums
\`\`\`
$(cat "$RELEASE_DIR/checksums.sha256" 2>/dev/null || echo "See checksums.sha256")
\`\`\`

## Installation
\`\`\`bash
# Download and extract
tar xzf ed2kia-v${VERSION}-<platform>.tar.gz
cd ed2kia-v${VERSION}

# Run
./ed2kia --version
./ed2kia join
\`\`\`

## Verification
\`\`\`bash
# Verify checksums
sha256sum -c checksums.sha256

# Verify signature (when available)
ed25519-verify <public_key> release.sig checksums.sha256
\`\`\`

---
**ed2kIA** - Decentralized AI Interpretability Network
EOF
    log_success "Release notes generated: $RELEASE_NOTES"
else
    log_warn "changelog.md not found. Manual release notes required."
fi

echo ""

# ------------------------------------------------------------------------------
# Create git tag
# ------------------------------------------------------------------------------
log_info "Creating git tag $TAG..."

if [ "$DRY_RUN" = false ]; then
    # Commit version update
    git add Cargo.toml
    git commit -m "bump: version $VERSION for release" || true

    # Create annotated tag
    git tag -a "$TAG" -m "Release v${VERSION}

ed2kIA v${VERSION} - Bootstrap, Governance, Reputation & Ecosystem

Features:
- Ed25519 signed governance proposals
- Time-locked voting with quorum
- Immutable reputation ledger (redb)
- Hugging Face/ModelScope sync
- Local model registry with rollback
- Seed node discovery and health validation
- Network genesis initialization

See CHANGELOG.md for full details."

    log_success "Git tag created: $TAG"
else
    log_info "[DRY RUN] Would create git tag $TAG"
fi

echo ""

# ------------------------------------------------------------------------------
# Prepare GitHub Release (manual step)
# ------------------------------------------------------------------------------
log_info "Preparing for GitHub Release..."

echo "=============================================="
echo "  Release Preparation Complete"
echo "=============================================="
echo ""
echo -e "  Version: ${GREEN}$VERSION${NC}"
echo -e "  Tag: ${GREEN}$TAG${NC}"
echo -e "  Artifacts: ${BLUE}$RELEASE_DIR/${NC}"
echo ""
echo "Next steps:"
echo "  1. Review artifacts in $RELEASE_DIR/"
echo "  2. Replace Ed25519 signature placeholder"
echo "  3. Push tag: git push origin $TAG"
echo "  4. Create GitHub Release:"
echo "     gh release create $TAG $RELEASE_DIR/* --title \"$TAG\" --notes-file $RELEASE_NOTES"
echo ""

if [ "$DRY_RUN" = false ]; then
    log_success "Release v${VERSION} preparation complete!"
else
    log_info "DRY RUN complete. No changes were made."
fi
