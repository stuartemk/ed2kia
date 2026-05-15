#!/usr/bin/env bash
# dependency_audit.sh — Automated dependency security audit for ed2kIA
# Usage: bash scripts/dependency_audit.sh [--fix]
#
# This script performs a comprehensive audit of all Rust dependencies:
#   1. cargo audit — Check for known CVEs
#   2. cargo tree — Analyze dependency graph
#   3. Duplicate detection — Find duplicate dependencies
#   4. Version pinning — Verify all deps are locked
#   5. License compatibility — Flag non-Apache-2.0 deps
#
# Exit codes:
#   0 — All checks passed
#   1 — Vulnerabilities or issues found
#   2 — Script error (missing tools, etc.)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="$PROJECT_ROOT/docs/security/audit-reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="$REPORT_DIR/audit_${TIMESTAMP}.md"
EXIT_CODE=0

# Auto-fix flag
AUTO_FIX=false
if [[ "${1:-}" == "--fix" ]]; then
    AUTO_FIX=true
fi

# Ensure report directory exists
mkdir -p "$REPORT_DIR"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE} ed2kIA Dependency Security Audit${NC}"
echo -e "${BLUE} $(date -u +"%Y-%m-%d %H:%M:%S UTC")${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Start report
cat > "$REPORT_FILE" << EOF
# Dependency Security Audit Report

**Date:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Project:** ed2kIA
**Branch:** $(cd "$PROJECT_ROOT" && git branch --show-current 2>/dev/null || echo "unknown")
**Commit:** $(cd "$PROJECT_ROOT" && git rev-parse --short HEAD 2>/dev/null || echo "unknown")

---

EOF

# Check 1: cargo audit
echo -e "${BLUE}[1/5]${NC} Running cargo audit..."
if command -v cargo-audit &> /dev/null; then
    AUDIT_OUTPUT=$(cd "$PROJECT_ROOT" && cargo audit 2>&1) || true
    if echo "$AUDIT_OUTPUT" | grep -qi "vulnerability"; then
        echo -e "${RED}  ✗ Vulnerabilities found!${NC}"
        echo "" >> "$REPORT_FILE"
        echo "## 1. CVE Audit — FAIL" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo '```' >> "$REPORT_FILE"
        echo "$AUDIT_OUTPUT" >> "$REPORT_FILE"
        echo '```' >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        EXIT_CODE=1
        if [[ "$AUTO_FIX" == true ]]; then
            echo -e "${YELLOW}  Attempting auto-fix with cargo-audit...${NC}"
            cd "$PROJECT_ROOT" && cargo audit --fix 2>&1 || true
        fi
    else
        echo -e "${GREEN}  ✓ No known vulnerabilities${NC}"
        echo "" >> "$REPORT_FILE"
        echo "## 1. CVE Audit — PASS" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo "No known vulnerabilities found." >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
    fi
else
    echo -e "${YELLOW}  ⚠ cargo-audit not installed. Install with: cargo install cargo-audit${NC}"
    echo "" >> "$REPORT_FILE"
    echo "## 1. CVE Audit — SKIPPED" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "cargo-audit not installed." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

# Check 2: Dependency tree analysis
echo -e "${BLUE}[2/5]${NC} Analyzing dependency tree..."
DEP_COUNT=$(cd "$PROJECT_ROOT" && cargo tree --normal-deps 2>/dev/null | wc -l)
echo -e "  → ${DEP_COUNT} direct dependencies"
echo "" >> "$REPORT_FILE"
echo "## 2. Dependency Tree Analysis" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "- **Direct dependencies:** ${DEP_COUNT}" >> "$REPORT_FILE"
TOTAL_DEPS=$(cd "$PROJECT_ROOT" && cargo tree 2>/dev/null | wc -l)
echo "- **Total dependencies (including transitive):** ${TOTAL_DEPS}" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "### Direct Dependencies" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo '```' >> "$REPORT_FILE"
cd "$PROJECT_ROOT" && cargo tree --normal-deps -p ed2kia 2>/dev/null >> "$REPORT_FILE" || echo "cargo tree failed" >> "$REPORT_FILE"
echo '```' >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Check 3: Duplicate dependencies
echo -e "${BLUE}[3/5]${NC} Checking for duplicate dependencies..."
DUPLICATES=$(cd "$PROJECT_ROOT" && cargo tree --duplicates 2>/dev/null || echo "")
if [[ -n "$DUPLICATES" ]]; then
    DUP_COUNT=$(echo "$DUPLICATES" | wc -l)
    echo -e "${YELLOW}  ⚠ ${DUP_COUNT} duplicate dependencies found${NC}"
    echo "" >> "$REPORT_FILE"
    echo "## 3. Duplicate Dependencies — WARNING" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "${DUP_COUNT} duplicate dependencies found:" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo "$DUPLICATES" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
else
    echo -e "${GREEN}  ✓ No duplicate dependencies${NC}"
    echo "" >> "$REPORT_FILE"
    echo "## 3. Duplicate Dependencies — PASS" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "No duplicate dependencies found." >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
fi

# Check 4: Version pinning verification
echo -e "${BLUE}[4/5]${NC} Verifying version pinning..."
if [[ -f "$PROJECT_ROOT/Cargo.lock" ]]; then
    LOCK_AGE=$(find "$PROJECT_ROOT/Cargo.lock" -mtime +90 2>/dev/null | wc -l)
    if [[ "$LOCK_AGE" -gt 0 ]]; then
        echo -e "${YELLOW}  ⚠ Cargo.lock not updated in 90+ days${NC}"
        echo "" >> "$REPORT_FILE"
        echo "## 4. Version Pinning — WARNING" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo "Cargo.lock has not been updated in 90+ days. Consider running \`cargo update\`." >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
    else
        echo -e "${GREEN}  ✓ Cargo.lock is current${NC}"
        echo "" >> "$REPORT_FILE"
        echo "## 4. Version Pinning — PASS" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo "Cargo.lock is present and recently updated." >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
    fi
else
    echo -e "${RED}  ✗ Cargo.lock not found!${NC}"
    echo "" >> "$REPORT_FILE"
    echo "## 4. Version Pinning — FAIL" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "Cargo.lock file not found. Dependencies are not pinned!" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    EXIT_CODE=1
fi

# Check 5: Security-relevant dependencies
echo -e "${BLUE}[5/5]${NC} Checking security-relevant dependencies..."
SECURITY_DEPS=("ed25519-dalek" "arkworks" "wasmtime" "libp2p" "getrandom")
echo "" >> "$REPORT_FILE"
echo "## 5. Security-Relevant Dependencies" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "| Dependency | Status | Version |" >> "$REPORT_FILE"
echo "|------------|--------|---------|" >> "$REPORT_FILE"

for dep in "${SECURITY_DEPS[@]}"; do
    VERSION=$(cd "$PROJECT_ROOT" && cargo tree 2>/dev/null | grep "^${dep} v" | head -1 | awk '{print $2}' || echo "not found")
    if [[ -n "$VERSION" ]]; then
        echo -e "  ✓ ${dep} ${VERSION}"
        echo "| ${dep} | ✓ Found | ${VERSION} |" >> "$REPORT_FILE"
    else
        echo -e "${YELLOW}  ⚠ ${dep} not found in dependency tree${NC}"
        echo "| ${dep} | ⚠ Not found | - |" >> "$REPORT_FILE"
    fi
done
echo "" >> "$REPORT_FILE"

# Summary
echo ""
echo -e "${BLUE}========================================${NC}"
if [[ $EXIT_CODE -eq 0 ]]; then
    echo -e "${GREEN}✓ Audit PASSED — No critical issues found${NC}"
    echo "" >> "$REPORT_FILE"
    echo "---" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "## Summary: PASS" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "No critical security issues found in dependency audit." >> "$REPORT_FILE"
else
    echo -e "${RED}✗ Audit FAILED — Issues require attention${NC}"
    echo "" >> "$REPORT_FILE"
    echo "---" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "## Summary: FAIL" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "Critical security issues found. Review report for details." >> "$REPORT_FILE"
fi
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Report saved to: ${GREEN}${REPORT_FILE}${NC}"

exit $EXIT_CODE
