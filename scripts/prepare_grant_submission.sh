#!/bin/bash
# prepare_grant_submission.sh — Grant Submission Package Generator
# Usage: bash scripts/prepare_grant_submission.sh
# Output: grants-submission-v1.8.tar.gz + grants-submission-v1.8.sha256

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/grants-submission"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "============================================"
echo "  Grant Submission Package Generator v1.8"
echo "  Timestamp: $TIMESTAMP"
echo "============================================"
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Track errors
ERRORS=0
WARNINGS=0

# --- Step 1: Verify Required Files ---
echo "[1/6] Verifying required files..."
echo ""

REQUIRED_FILES=(
    "docs/grants/nsf-ai-safety-draft.md"
    "docs/grants/gitcoin-quadratic-funding-draft.md"
    "docs/grants/ossf-draft.md"
    "docs/grants/submission-tracker.md"
    "README.md"
    "LICENSE"
    "SECURITY.md"
    "CONTRIBUTING.md"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$PROJECT_ROOT/$file" ]; then
        echo -e "  ${GREEN}✓${NC} $file"
    else
        echo -e "  ${RED}✗${NC} $file (MISSING)"
        ERRORS=$((ERRORS + 1))
    fi
done
echo ""

# --- Step 2: Check for Placeholders ---
echo "[2/6] Checking for unresolved placeholders..."
echo ""

PLACEHOLDER_COUNT=0
for draft in docs/grants/*.md; do
    if [ -f "$PROJECT_ROOT/$draft" ]; then
        count=$(grep -c "\[PLACEHOLDER" "$PROJECT_ROOT/$draft" 2>/dev/null || true)
        if [ "$count" -gt 0 ]; then
            echo -e "  ${YELLOW}⚠${NC} $draft: $count placeholder(s) found"
            PLACEHOLDER_COUNT=$((PLACEHOLDER_COUNT + count))
        else
            echo -e "  ${GREEN}✓${NC} $draft: No placeholders"
        fi
    fi
done

if [ "$PLACEHOLDER_COUNT" -gt 0 ]; then
    echo ""
    echo -e "  ${YELLOW}WARNING: $PLACEHOLDER_COUNT unresolved placeholder(s) found.${NC}"
    echo "  Resolve all placeholders before submission."
    WARNINGS=$((WARNINGS + 1))
fi
echo ""

# --- Step 3: Copy Files to Package ---
echo "[3/6] Building submission package..."
echo ""

# Copy grant drafts
mkdir -p "$OUTPUT_DIR/grants"
for draft in docs/grants/*.md; do
    if [ -f "$PROJECT_ROOT/$draft" ]; then
        cp "$PROJECT_ROOT/$draft" "$OUTPUT_DIR/grants/"
        echo -e "  ${GREEN}✓${NC} Added: $draft"
    fi
done

# Copy required project files
for file in README.md LICENSE SECURITY.md CONTRIBUTING.md; do
    if [ -f "$PROJECT_ROOT/$file" ]; then
        cp "$PROJECT_ROOT/$file" "$OUTPUT_DIR/"
        echo -e "  ${GREEN}✓${NC} Added: $file"
    fi
done

# Copy Cargo.toml for dependency verification
if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
    cp "$PROJECT_ROOT/Cargo.toml" "$OUTPUT_DIR/"
    echo -e "  ${GREEN}✓${NC} Added: Cargo.toml"
fi
echo ""

# --- Step 4: Generate Checksums ---
echo "[4/6] Generating SHA256 checksums..."
echo ""

cd "$OUTPUT_DIR"
find . -type f -not -name "*.sha256" | sort | xargs sha256sum > "checksums.sha256" 2>/dev/null || true

if [ -f "checksums.sha256" ]; then
    echo -e "  ${GREEN}✓${NC} Checksums generated: checksums.sha256"
    echo ""
    echo "  Checksum contents:"
    while IFS= read -r line; do
        echo "    $line"
    done < "checksums.sha256"
else
    echo -e "  ${RED}✗${NC} Failed to generate checksums"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# --- Step 5: Create Archive ---
echo "[5/6] Creating archive..."
echo ""

cd "$PROJECT_ROOT"
ARCHIVE_NAME="grants-submission-v1.8.tar.gz"
tar -czf "$OUTPUT_DIR/$ARCHIVE_NAME" -C "$OUTPUT_DIR" . 2>/dev/null || true

if [ -f "$OUTPUT_DIR/$ARCHIVE_NAME" ]; then
    ARCHIVE_SIZE=$(du -h "$OUTPUT_DIR/$ARCHIVE_NAME" | cut -f1)
    echo -e "  ${GREEN}✓${NC} Archive created: $ARCHIVE_NAME ($ARCHIVE_SIZE)"
else
    echo -e "  ${RED}✗${NC} Failed to create archive"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# --- Step 6: Pre-Submission Checklist ---
echo "[6/6] Pre-submission checklist..."
echo ""

echo "  === NSF AI Safety ==="
echo "  [ ] Draft reviewed by PI and Co-PI"
echo "  [ ] Budget justification updated"
echo "  [ ] Institutional letter obtained"
echo "  [ ] NSF_APP_ID registered in FastLane"
echo "  [ ] Digital signatures from all PIs"
echo "  [ ] References verified (DOIs, URLs)"
echo ""

echo "  === Gitcoin Quadratic Funding ==="
echo "  [ ] Wallet address verified (ETH mainnet)"
echo "  [ ] Discord server URL active"
echo "  [ ] Twitter handle verified"
echo "  [ ] Signal post published on Gitcoin Forum"
echo "  [ ] Community narrative with real metrics"
echo "  [ ] Gitcoin_APP_ID registered"
echo ""

echo "  === OSSF Security Grant ==="
echo "  [ ] Cryptography expert identified"
echo "  [ ] Security auditor contacted"
echo "  [ ] Threat model updated"
echo "  [ ] SBOM generated (CycloneDX/SPDX)"
echo "  [ ] ZKP circuit audit scope defined"
echo "  [ ] OSSF_APP_ID registered"
echo ""

# --- Summary ---
echo "============================================"
echo "  Summary"
echo "============================================"
echo ""
echo "  Errors:   $ERRORS"
echo "  Warnings: $WARNINGS"
echo "  Placeholders: $PLACEHOLDER_COUNT"
echo ""

if [ "$ERRORS" -eq 0 ]; then
    echo -e "  ${GREEN}✓ Package ready for review${NC}"
    echo ""
    echo "  Archive: $OUTPUT_DIR/$ARCHIVE_NAME"
    echo "  Checksums: $OUTPUT_DIR/checksums.sha256"
    echo ""
    echo "  Next steps:"
    echo "  1. Review all drafts for accuracy"
    echo "  2. Resolve all placeholders"
    echo "  3. Obtain required signatures"
    echo "  4. Verify submission links"
    echo "  5. Submit through official portals"
else
    echo -e "  ${RED}✗ Package has $ERRORS error(s). Fix before proceeding.${NC}"
fi
echo ""
echo "============================================"

# Exit with error if critical files missing
if [ "$ERRORS" -gt 0 ]; then
    exit 1
fi

exit 0
