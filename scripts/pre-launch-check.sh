#!/usr/bin/env bash
# pre-launch-check.sh — Automated Pre-Launch Validation for ed2kIA v2.1.0-sprint11
# Validates build, tests, critical files, documentation links & generates readiness report.
# Usage: bash scripts/pre-launch-check.sh
set -euo pipefail
trap cleanup EXIT INT TERM

REPORT_FILE="docs/launch-readiness-report.md"
PASS_COUNT=0
FAIL_COUNT=0
FAILURES=()

cleanup() {
    # Cleanup temp files if any
    rm -f /tmp/ed2kia-pre-launch-* 2>/dev/null || true
}

log_pass() {
    PASS_COUNT=$((PASS_COUNT + 1))
    echo "  PASS: $1"
}

log_fail() {
    FAIL_COUNT=$((FAIL_COUNT + 1))
    FAILURES+=("$1")
    echo "  FAIL: $1"
}

init_report() {
    cat > "$REPORT_FILE" <<EOF
# Launch Readiness Report — ed2kIA v2.1.0-sprint11

**Generated:** $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Script:** scripts/pre-launch-check.sh

---

## Results Summary

| Status | Count |
|--------|-------|
| PASS | $PASS_COUNT |
| FAIL | $FAIL_COUNT |

EOF
}

append_report() {
    echo "$1" >> "$REPORT_FILE"
}

echo "============================================"
echo " ed2kIA Pre-Launch Validation"
echo " Sprint11 — Operational Readiness & Mainnet Prep"
echo "============================================"
echo ""

# Initialize report
init_report

# ──────────────────────────────────────────────
# 1. Cargo Check (all targets, key feature gates)
# ──────────────────────────────────────────────
echo "[1/5] cargo check --all-targets..."
if cargo check --all-targets --features "stable,v2.1-observability" > /tmp/ed2kia-pre-launch-check.log 2>&1; then
    log_pass "cargo check --all-targets (stable + v2.1-observability)"
    append_report "- [x] cargo check --all-targets (stable + v2.1-observability)"
else
    log_fail "cargo check --all-targets"
    append_report "- [ ] cargo check --all-targets — FAILED"
    cat /tmp/ed2kia-pre-launch-check.log >> "$REPORT_FILE"
fi

# ──────────────────────────────────────────────
# 2. Cargo Test (lib tests)
# ──────────────────────────────────────────────
echo "[2/5] cargo test --lib..."
if cargo test --lib --features "stable,v2.1-observability" > /tmp/ed2kia-pre-launch-test.log 2>&1; then
    TEST_COUNT=$(grep -c "test result:" /tmp/ed2kia-pre-launch-test.log 2>/dev/null || echo "0")
    log_pass "cargo test --lib ($TEST_COUNT test suites)"
    append_report "- [x] cargo test --lib ($TEST_COUNT test suites)"
else
    log_fail "cargo test --lib"
    append_report "- [ ] cargo test --lib — FAILED"
fi

# ──────────────────────────────────────────────
# 3. Critical Files Exist
# ──────────────────────────────────────────────
echo "[3/5] Verifying critical files..."
CRITICAL_FILES=(
    "CODEOWNERS"
    "CONTRIBUTING.md"
    "GOVERNANCE.md"
    "SECURITY.md"
    ".github/workflows/deploy-pages.yml"
    "scripts/ignite-local-testnet.sh"
    "scripts/build-wasm.sh"
    "prometheus/grafana-dashboard.json"
)

for f in "${CRITICAL_FILES[@]}"; do
    if [ -f "$f" ]; then
        log_pass "File exists: $f"
        append_report "- [x] File exists: $f"
    else
        log_fail "File missing: $f"
        append_report "- [ ] File missing: $f"
    fi
done

# ──────────────────────────────────────────────
# 4. Validate JSON (Grafana Dashboard)
# ──────────────────────────────────────────────
echo "[4/5] Validating JSON artifacts..."
if python -c "import json; json.load(open('prometheus/grafana-dashboard.json'))" 2>/dev/null; then
    log_pass "prometheus/grafana-dashboard.json — valid JSON"
    append_report "- [x] prometheus/grafana-dashboard.json — valid JSON"
else
    log_fail "prometheus/grafana-dashboard.json — invalid JSON"
    append_report "- [ ] prometheus/grafana-dashboard.json — invalid JSON"
fi

# ──────────────────────────────────────────────
# 5. Documentation Link Check (basic)
# ──────────────────────────────────────────────
echo "[5/5] Checking documentation integrity..."
# Verify CHANGELOG.md has sprint11 entry
if grep -q "v2.1.0-sprint11" CHANGELOG.md 2>/dev/null; then
    log_pass "CHANGELOG.md contains v2.1.0-sprint11 entry"
    append_report "- [x] CHANGELOG.md contains v2.1.0-sprint11 entry"
else
    log_fail "CHANGELOG.md missing v2.1.0-sprint11 entry"
    append_report "- [ ] CHANGELOG.md missing v2.1.0-sprint11 entry"
fi

# Verify README.md has observability section
if grep -q "Observabilidad\|Observability\|observability" README.md 2>/dev/null; then
    log_pass "README.md contains observability section"
    append_report "- [x] README.md contains observability section"
else
    log_fail "README.md missing observability section"
    append_report "- [ ] README.md missing observability section"
fi

# ──────────────────────────────────────────────
# Final Report
# ──────────────────────────────────────────────
echo ""
echo "============================================"
echo " Results: $PASS_COUNT PASS, $FAIL_COUNT FAIL"
echo "============================================"

if [ "$FAIL_COUNT" -eq 0 ]; then
    echo ""
    echo "GREEN READY FOR MAINNET"
    echo ""
    append_report "## Status: GREEN READY FOR MAINNET"
    append_report ""
    append_report "All pre-launch validations passed. The network is technically ready for public deployment."
else
    echo ""
    echo "RED BLOCKED"
    echo ""
    append_report "## Status: RED BLOCKED"
    append_report ""
    append_report "### Failures:"
    append_report ""
    for failure in "${FAILURES[@]}"; do
        append_report "- $failure"
    done
    append_report ""
    append_report "Resolve all failures before proceeding with mainnet deployment."
fi

echo "Report saved to: $REPORT_FILE"
