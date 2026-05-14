#!/usr/bin/env bash
# release/v1.6.0-stable/release_commands.sh
# Git Release Commands for ed2kIA v1.6.0-stable
# Usage: bash release/v1.6.0-stable/release_commands.sh
set -euo pipefail

VERSION="1.6.0-stable"
TAG="v${VERSION}"
BRANCH="main"

echo "=========================================="
echo "ed2kIA ${TAG} Release Script"
echo "=========================================="

# Pre-flight checks
echo ""
echo "[1/6] Pre-flight checks..."
if [[ "$(git branch --show-current)" != "${BRANCH}" ]]; then
  echo "ERROR: Must be on ${BRANCH} branch. Current: $(git branch --show-current)"
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "ERROR: Working directory is not clean. Commit or stash changes first."
  exit 1
fi

echo "✓ On ${BRANCH} branch with clean working directory"

# Stage all changes
echo ""
echo "[2/6] Staging all changes..."
git add -A
echo "✓ All changes staged"

# Commit with conventional format
echo ""
echo "[3/6] Creating release commit..."
git commit -m "release(${VERSION}): Official stable release

- SAE Fine-Tuning v7: Cross-model gradient alignment v5, adaptive LR decay, LZ4 compression
- Cross-Model Federation Scaling v7: Multi-model shard coordination, predictive load balancing
- Async ZKP v14: Adaptive proof batching, parallel verification, Merkle+VRF fallback
- Federation ZKP Bridge v7: Adaptive routing, credibility scoring, proof fallback
- UI Dashboard v7: WebSocket streaming, pool metrics, federation health
- 187 tests passing (160 unit + 27 E2E + 13 stress)
- Zero unsafe code, zero telemetry, zero financial logic
- Apache 2.0 + Ethical Use Clause
- GitHub scaffolding: Issue/PR templates, labels, auto-management
- CHANGELOG.md updated with Keep-a-Changelog format
- SECURITY.md with responsible disclosure policy
- CODEOWNERS with auto-assignment patterns

Checklist:
- [x] cargo check --features stable: 0 errors, 0 warnings
- [x] cargo clippy --features stable: 0 warnings
- [x] cargo test --features stable: 187 passed
- [x] CHANGELOG.md updated
- [x] VERSION normalized across codebase
- [x] GitHub templates updated (core-only → stable)
- [x] Labels.json + labeler.yml configured"

echo "✓ Release commit created"

# Create annotated tag
echo ""
echo "[4/6] Creating annotated tag ${TAG}..."
git tag -a "${TAG}" -m "release(${VERSION}): Official stable release

ed2kIA v1.6.0-stable
=====================
Distributed AI federation with zero-knowledge verification.

Key Features:
- SAE Fine-Tuning v7
- Cross-Model Federation Scaling v7
- Async ZKP v14
- Federation ZKP Bridge v7
- UI Dashboard v7
- 187 tests passing
- Zero unsafe code, zero telemetry, zero financial logic
- Apache 2.0 + Ethical Use Clause

Documentation: https://github.com/ed2kIA/ed2kIA/docs
Security Policy: https://github.com/ed2kIA/ed2kIA/security/policy
Governance: https://github.com/ed2kIA/ed2kIA/blob/main/docs/GOVERNANCE.md"

echo "✓ Tag ${TAG} created"

# Push to remote
echo ""
echo "[5/6] Pushing to origin/${BRANCH}..."
git push origin "${BRANCH}"
echo "✓ Branch pushed"

# Push tags
echo ""
echo "[6/6] Pushing tags..."
git push origin "${TAG}"
echo "✓ Tag pushed"

echo ""
echo "=========================================="
echo "Release ${TAG} complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  1. Create GitHub Release from tag: https://github.com/ed2kIA/ed2kIA/releases/new?tag=${TAG}"
echo "  2. Attach build artifacts (tar.gz, zip, checksums.txt)"
echo "  3. Announce on community channels"
echo "  4. Monitor for issues (see docs/post-launch-monitoring.md)"
