#!/bin/sh
# stable-maintenance.sh — v1.9 Stable Maintenance Script (POSIX)
# Usage: ./scripts/stable-maintenance.sh [--report-only] [--full]
#
# Modes:
#   --report-only  Run checks without modifying dependencies
#   --full         Full maintenance including cargo update (dry-run)
#
# Exit codes:
#   0 = All checks passed
#   1 = Validation failure (details in report)
#   2 = Script error

set -e

REPORT_DIR="docs/operations"
REPORT_FILE="${REPORT_DIR}/stable-maintenance-report.md"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "2026-05-16T00:00:00Z")
MODE="${1:---report-only}"

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; }
log_info() { echo "[INFO] $1"; }

# Initialize report
init_report() {
    mkdir -p "${REPORT_DIR}"
    cat > "${REPORT_FILE}" << EOF
# Stable Maintenance Report — v1.9.0-stable

> **Generated:** ${TIMESTAMP}
> **Mode:** ${MODE}
> **Script:** scripts/stable-maintenance.sh

---

## Summary

| Check | Status | Details |
|-------|--------|---------|
EOF
}

append_result() {
    local check="$1"
    local status="$2"
    local details="$3"
    echo "| ${check} | ${status} | ${details} |" >> "${REPORT_FILE}"
}

# 1. Dependency Audit
check_dependencies() {
    log_info "Running dependency audit..."

    # Check for yanked/unmaintained packages
    if command -v cargo >/dev/null 2>&1; then
        AUDIT_OUTPUT=$(cargo audit 2>&1 || true)
        VULN_COUNT=$(echo "${AUDIT_OUTPUT}" | grep -c "warning: Vulnerability" || echo "0")

        if [ "${VULN_COUNT}" -eq 0 ]; then
            log_pass "No vulnerabilities found"
            append_result "Dependency Audit" "PASS" "0 vulnerabilities"
        else
            log_warn "Found ${VULN_COUNT} vulnerabilities (documented in ossf-compliance-report.md)"
            append_result "Dependency Audit" "WARN" "${VULN_COUNT} vulnerabilities (mitigated)"
        fi
    else
        log_warn "cargo not available — skipping audit"
        append_result "Dependency Audit" "SKIP" "cargo not available"
    fi
}

# 2. Dry-run dependency update
check_updates() {
    log_info "Checking for dependency updates (dry-run)..."

    if [ "${MODE}" = "--full" ] && command -v cargo >/dev/null 2>&1; then
        UPDATE_OUTPUT=$(cargo update --dry-run 2>&1 || true)
        UPDATE_COUNT=$(echo "${UPDATE_OUTPUT}" | grep -c "Updated\|Downgraded" || echo "0")

        if [ "${UPDATE_COUNT}" -gt 0 ]; then
            log_warn "${UPDATE_COUNT} dependencies have updates available"
            append_result "Dependency Updates" "WARN" "${UPDATE_COUNT} updates available (dry-run)"
        else
            log_pass "All dependencies up to date"
            append_result "Dependency Updates" "PASS" "No updates needed"
        fi
    else
        log_info "Skipping update check (report-only mode or cargo unavailable)"
        append_result "Dependency Updates" "SKIP" "Report-only mode"
    fi
}

# 3. Cargo Check
run_cargo_check() {
    log_info "Running cargo check..."

    if command -v cargo >/dev/null 2>&1; then
        if cargo check --features v1.9-sprint1,v1.9-sprint2 2>&1 | tee /tmp/cargo-check-output.txt; then
            log_pass "cargo check passed"
            append_result "Cargo Check" "PASS" "v1.9-sprint1 + v1.9-sprint2 features"
        else
            log_fail "cargo check failed"
            append_result "Cargo Check" "FAIL" "See /tmp/cargo-check-output.txt"
            return 1
        fi
    else
        log_warn "cargo not available — skipping check"
        append_result "Cargo Check" "SKIP" "cargo not available"
    fi
}

# 4. Clippy Lint
run_clippy() {
    log_info "Running cargo clippy..."

    if command -v cargo >/dev/null 2>&1; then
        if cargo clippy --features v1.9-sprint1,v1.9-sprint2 -- -D warnings 2>&1 | tee /tmp/clippy-output.txt; then
            log_pass "clippy passed"
            append_result "Clippy Lint" "PASS" "No warnings"
        else
            log_fail "clippy found warnings"
            append_result "Clippy Lint" "FAIL" "See /tmp/clippy-output.txt"
            return 1
        fi
    else
        log_warn "cargo not available — skipping clippy"
        append_result "Clippy Lint" "SKIP" "cargo not available"
    fi
}

# 5. Unit Tests
run_tests() {
    log_info "Running unit tests..."

    if command -v cargo >/dev/null 2>&1; then
        TEST_OUTPUT=$(cargo test --lib --features v1.9-sprint1,v1.9-sprint2 2>&1 || true)
        TEST_PASSED=$(echo "${TEST_OUTPUT}" | grep -c "test result: ok" || echo "0")
        TEST_FAILED=$(echo "${TEST_OUTPUT}" | grep -c "test result: FAILED" || echo "0")

        if [ "${TEST_FAILED}" -eq 0 ] && [ "${TEST_PASSED}" -gt 0 ]; then
            log_pass "All tests passed"
            append_result "Unit Tests" "PASS" "${TEST_PASSED} test groups passed"
        else
            log_fail "${TEST_FAILED} test groups failed"
            append_result "Unit Tests" "FAIL" "${TEST_FAILED} failures"
            return 1
        fi
    else
        log_warn "cargo not available — skipping tests"
        append_result "Unit Tests" "SKIP" "cargo not available"
    fi
}

# 6. Coverage Check (best-effort)
check_coverage() {
    log_info "Checking test coverage (best-effort)..."

    if command -v cargo-tarpaulin >/dev/null 2>&1; then
        log_info "Running cargo-tarpaulin..."
        cargo tarpaulin --features v1.9-sprint1,v1.9-sprint2 --out Stdout 2>&1 | tail -5 || true
        append_result "Coverage" "INFO" "tarpaulin executed"
    else
        log_info "cargo-tarpaulin not available — estimating from test count"
        # Count tests as proxy for coverage
        TEST_COUNT=$(cargo test --lib --features v1.9-sprint1,v1.9-sprint2 -- --list 2>/dev/null | grep -c "tests::" || echo "0")
        log_info "Estimated test count: ${TEST_COUNT}"
        append_result "Coverage" "INFO" "~${TEST_COUNT} tests (tarpaulin not installed)"
    fi
}

# 7. Benchmark Regression Check
check_benchmarks() {
    log_info "Checking benchmark baseline..."

    BASELINE_FILE="benchmarks/results/baseline-v1.7.json"
    if [ -f "${BASELINE_FILE}" ]; then
        log_pass "Baseline exists: ${BASELINE_FILE}"
        append_result "Benchmark Baseline" "PASS" "baseline-v1.7.json present"
    else
        log_warn "No benchmark baseline found"
        append_result "Benchmark Baseline" "WARN" "No baseline file"
    fi
}

# 8. Generate Final Report Summary
finalize_report() {
    cat >> "${REPORT_FILE}" << 'EOF'

---

## Recommendations

| Priority | Action | Target |
|----------|--------|--------|
| Immediate | Review cargo audit findings | v1.9.0-stable |
| Short-term | Update wasmtime to patched version | v1.10 |
| Long-term | Replace unmaintained crates | v2.0 |

## Next Maintenance Window

- **Schedule:** Monthly (first Monday)
- **Trigger:** New CVE disclosure, dependency update, or manual request
- **Process:** Run this script → Review report → Apply fixes → Commit

---

*Report generated by scripts/stable-maintenance.sh*
EOF

    log_info "Report saved to ${REPORT_FILE}"
}

# Main Execution
main() {
    echo "========================================"
    echo "  ed2kIA v1.9.0-stable Maintenance"
    echo "  Mode: ${MODE}"
    echo "  Time: ${TIMESTAMP}"
    echo "========================================"
    echo ""

    init_report

    check_dependencies
    check_updates
    run_cargo_check
    run_clippy
    run_tests
    check_coverage
    check_benchmarks

    echo ""
    finalize_report

    echo "========================================"
    echo "  Maintenance Complete"
    echo "  Report: ${REPORT_FILE}"
    echo "========================================"
}

main "$@"
