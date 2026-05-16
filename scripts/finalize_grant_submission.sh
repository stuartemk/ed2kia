#!/bin/sh
# finalize_grant_submission.sh — Package grant drafts for submission
# POSIX-compliant shell script for ed2kIA v1.9 grant package generation
#
# Usage: sh scripts/finalize_grant_submission.sh [output_dir]
#
# This script:
#   1. Collects all grant drafts from docs/grants/
#   2. Generates SHA256 checksums
#   3. Creates grants-submission-v1.9.tar.gz
#   4. Outputs execution checklist
#
# Exit codes:
#   0 — Success
#   1 — Missing grant files
#   2 — Packaging failed

set -e

# Configuration
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GRANTS_DIR="$PROJECT_ROOT/docs/grants"
OUTPUT_DIR="${1:-$PROJECT_ROOT/grants-submission}"
TIMESTAMP=$(date -u +"%Y%m%d_%H%M%S" 2>/dev/null || echo "20260516_000000")
TAR_FILE="grants-submission-v1.9.tar.gz"
CHECKSUM_FILE="grants-submission-v1.9.sha256"

echo "========================================"
echo " ed2kIA Grant Submission Package"
echo " $(date -u +"%Y-%m-%d %H:%M:%S UTC" 2>/dev/null || date)"
echo "========================================"
echo ""

# Verify grant files exist
echo "[1/5] Verifying grant files..."
GRANT_FILES="gitcoin-quadratic-funding-draft.md nsf-ai-safety-draft.md ossf-draft.md submission-tracker.md follow-up-tracker.md"
MISSING=0
for file in $GRANT_FILES; do
    if [ -f "$GRANTS_DIR/$file" ]; then
        echo "  ✓ $file"
    else
        echo "  ✗ MISSING: $file"
        MISSING=1
    fi
done

if [ "$MISSING" -eq 1 ]; then
    echo ""
    echo "ERROR: Some grant files are missing. Aborting."
    exit 1
fi
echo ""

# Create output directory
echo "[2/5] Creating output directory..."
mkdir -p "$OUTPUT_DIR"
echo "  → $OUTPUT_DIR"
echo ""

# Copy grant files
echo "[3/5] Packaging grant files..."
cp "$GRANTS_DIR"/gitcoin-quadratic-funding-draft.md "$OUTPUT_DIR/"
cp "$GRANTS_DIR"/nsf-ai-safety-draft.md "$OUTPUT_DIR/"
cp "$GRANTS_DIR"/ossf-draft.md "$OUTPUT_DIR/"
cp "$GRANTS_DIR"/submission-tracker.md "$OUTPUT_DIR/"
cp "$GRANTS_DIR"/follow-up-tracker.md "$OUTPUT_DIR/"

# Add execution checklist
cat > "$OUTPUT_DIR/EXECUTION_CHECKLIST.md" << 'EOF'
# Grant Submission Execution Checklist

## Pre-Submission

- [ ] Review all grant drafts for completeness
- [ ] Verify budget calculations
- [ ] Confirm team member approvals
- [ ] Check deadline dates
- [ ] Prepare supporting documents (GitHub stats, metrics)

## Gitcoin Quadratic Funding

- [ ] Apply at https://gitcoin.co/grants
- [ ] Fill application form
- [ ] Attach: gitcoin-quadratic-funding-draft.md
- [ ] Set matching pool targets
- [ ] Share campaign with community

## NSF AI Safety

- [ ] Register in NSF FastLane system
- [ ] Prepare abstract (≤ 3,000 characters)
- [ ] Attach: nsf-ai-safety-draft.md
- [ ] Budget justification document
- [ ] Letters of support from partners

## OSSF (Open Source Security Foundation)

- [ ] Apply at https://openssf.org
- [ ] Complete security audit section
- [ ] Attach: ossf-draft.md
- [ ] Reference OSSF compliance report
- [ ] Security team contact info

## Post-Submission

- [ ] Update submission-tracker.md with dates
- [ ] Set follow-up reminders (2 weeks, 1 month)
- [ ] Notify community (without disclosing sensitive info)
- [ ] Archive submission copies

## Contact Information

| Grant | Contact | Email |
|-------|---------|-------|
| Gitcoin | Community Lead | community@ed2kia.org |
| NSF | PI Lead | research@ed2kia.org |
| OSSF | Security Lead | security@ed2kia.org |
EOF

echo "  ✓ Grant drafts copied"
echo "  ✓ Execution checklist generated"
echo ""

# Generate SHA256 checksums
echo "[4/5] Generating SHA256 checksums..."
if command -v sha256sum >/dev/null 2>&1; then
    cd "$OUTPUT_DIR" && sha256sum *.md > "$CHECKSUM_FILE"
elif command -v shasum >/dev/null 2>&1; then
    cd "$OUTPUT_DIR" && shasum -a 256 *.md > "$CHECKSUM_FILE"
else
    echo "  ⚠ No SHA256 tool found. Skipping checksum generation."
    CHECKSUM_FILE=""
fi
echo "  ✓ Checksums: $CHECKSUM_FILE"
echo ""

# Create tar.gz archive
echo "[5/5] Creating archive..."
cd "$PROJECT_ROOT"
if tar -czf "$OUTPUT_DIR/$TAR_FILE" -C "$OUTPUT_DIR" .; then
    echo "  ✓ Archive created: $OUTPUT_DIR/$TAR_FILE"
    TAR_SIZE=$(du -h "$OUTPUT_DIR/$TAR_FILE" 2>/dev/null | cut -f1 || echo "unknown")
    echo "  → Size: $TAR_SIZE"
else
    echo "  ✗ ERROR: Failed to create archive"
    exit 2
fi
echo ""

# Summary
echo "========================================"
echo " Package Summary"
echo "========================================"
echo " Output directory: $OUTPUT_DIR"
echo " Archive: $TAR_FILE"
echo " Checksums: $CHECKSUM_FILE"
echo " Files packaged: 6 grant drafts + checklist"
echo ""
echo " Next Steps:"
echo "  1. Review: cat $OUTPUT_DIR/EXECUTION_CHECKLIST.md"
echo "  2. Verify: cd $OUTPUT_DIR && sha256sum -c $CHECKSUM_FILE"
echo "  3. Submit: Follow checklist for each grant"
echo "========================================"
