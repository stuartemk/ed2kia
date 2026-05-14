#!/usr/bin/env bash
# release_v1.6.0.sh
# Main release entry point for ed2kIA v1.6.0-stable
# Usage: bash release_v1.6.0.sh
set -euo pipefail

VERSION="1.6.0-stable"
TAG="v${VERSION}"

echo "=========================================="
echo "ed2kIA ${TAG} Release"
echo "=========================================="
echo ""
echo "This script will:"
echo "  1. Run pre-flight validation"
echo "  2. Execute git release (add, commit, tag, push)"
echo "  3. Generate release artifacts"
echo "  4. Provide post-launch checklist"
echo ""
read -p "Continue? [y/N] " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
  echo "Aborted."
  exit 0
fi

# Step 1: Pre-flight validation
echo ""
echo "=========================================="
echo "[STEP 1] Pre-flight Validation"
echo "=========================================="

echo "[1.1] Running cargo check..."
cargo check --features "stable" || { echo "FAILED: cargo check"; exit 1; }
echo "✓ cargo check passed"

echo "[1.2] Running cargo clippy..."
cargo clippy --features "stable" || { echo "FAILED: cargo clippy"; exit 1; }
echo "✓ cargo clippy passed"

echo "[1.3] Running cargo test..."
cargo test --features "stable" || { echo "FAILED: cargo test"; exit 1; }
echo "✓ cargo test passed"

# Step 2: Git release
echo ""
echo "=========================================="
echo "[STEP 2] Git Release"
echo "=========================================="

bash release/v1.6.0-stable/release_commands.sh

# Step 3: Generate artifacts
echo ""
echo "=========================================="
echo "[STEP 3] Release Artifacts"
echo "=========================================="

echo "[3.1] Building release binary..."
cargo build --release --features "stable" || { echo "FAILED: cargo build --release"; exit 1; }
echo "✓ Release binary built"

echo "[3.2] Packaging release..."
bash release/packager.sh "${VERSION}" || { echo "FAILED: packager.sh"; exit 1; }
echo "✓ Release packaged"

echo "[3.3] Generating checksums..."
cd release
sha256sum * > checksums.txt
cd ..
echo "✓ Checksums generated"

# Step 4: Post-launch checklist
echo ""
echo "=========================================="
echo "[STEP 4] Post-Launch Checklist"
echo "=========================================="
echo ""
echo "Release ${TAG} artifacts ready in: release/"
echo ""
echo "NEXT STEPS:"
echo "  1. Create GitHub Release:"
echo "     https://github.com/ed2kIA/ed2kIA/releases/new?tag=${TAG}"
echo ""
echo "  2. Attach artifacts:"
echo "     - release/ed2kIA-${VERSION}-*.tar.gz"
echo "     - release/ed2kIA-${VERSION}-*.zip"
echo "     - release/checksums.txt"
echo ""
echo "  3. Announce:"
echo "     - Discord #announcements"
echo "     - GitHub Discussions"
echo "     - Community mailing list"
echo ""
echo "  4. Monitor:"
echo "     - See docs/post-launch-monitoring.md"
echo "     - Critical window: 0-48h"
echo ""
echo "  5. Roadmap:"
echo "     - See docs/v1.7-roadmap-placeholder.md"
echo "     - Community input opens after 7-day stabilization"
echo ""
echo "=========================================="
echo "Release ${TAG} COMPLETE"
echo "=========================================="
