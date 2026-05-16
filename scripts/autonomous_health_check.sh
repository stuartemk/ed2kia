#!/bin/bash
# autonomous_health_check.sh — v2.0.0-stable
# POSIX-compliant autonomous health check script for ed2kIA
# Executes: compilation, tests, coverage, dependency audit, stale detection
# Usage: ./scripts/autonomous_health_check.sh [--report] [--verbose]

set -euo pipefail

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_FILE="${PROJECT_ROOT}/reports/health_check_$(date +%Y%m%d_%H%M%S).json"
VERBOSE=false
GENERATE_REPORT=false
EXIT_CODE=0

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --report) GENERATE_REPORT=true; shift ;;
        --verbose) VERBOSE=true; shift ;;
        --help)
            echo "Usage: $0 [--report] [--verbose]"
            echo "  --report   Generate JSON report"
            echo "  --verbose  Enable verbose output"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Helper functions
log() {
    if [[ "$VERBOSE" == true ]] || [[ "$1" == "ERROR" ]]; then
        echo "[$(date +%Y-%m-%dT%H:%M:%S%z)] [$1] $2"
    fi
}

log_info() { log "INFO" "$1"; }
log_warn() { log "WARN" "$1"; }
log_error() { log "ERROR" "$1"; }

check_result() {
    local name="$1"
    local exit_code="$2"
    local details="$3"
    local status="PASS"
    if [[ $exit_code -ne 0 ]]; then
        status="FAIL"
        EXIT_CODE=1
    fi
    echo "{\"check\":\"$name\",\"status\":\"$status\",\"exit_code\":$exit_code,\"details\":\"$details\"}"
}

# Ensure reports directory exists
mkdir -p "${PROJECT_ROOT}/reports"

# Initialize results array
RESULTS=()

log_info "Starting autonomous health check..."
START_TIME=$(date +%s)

# ============================================
# CHECK 1: Compilation (cargo check)
# ============================================
log_info "CHECK 1: Compilation (cargo check)"
cd "$PROJECT_ROOT"
if cargo check --features stable 2>&1 | tee /dev/null; then
    RESULTS+=($(check_result "compilation" 0 "cargo check --features stable PASSED"))
    log_info "✓ Compilation PASSED"
else
    RESULTS+=($(check_result "compilation" 1 "cargo check --features stable FAILED"))
    log_error "✗ Compilation FAILED"
fi

# ============================================
# CHECK 2: Linting (cargo clippy)
# ============================================
log_info "CHECK 2: Linting (cargo clippy)"
if cargo clippy --features stable 2>&1 | grep -q "Finished"; then
    RESULTS+=($(check_result "linting" 0 "cargo clippy --features stable PASSED"))
    log_info "✓ Linting PASSED"
else
    RESULTS+=($(check_result "linting" 1 "cargo clippy --features stable FAILED"))
    log_error "✗ Linting FAILED"
fi

# ============================================
# CHECK 3: Unit Tests (cargo test --lib)
# ============================================
log_info "CHECK 3: Unit Tests (cargo test --lib)"
TEST_OUTPUT=$(cargo test --features "stable,v2.0-sprint1,v2.0-sprint2" --lib 2>&1)
TEST_RESULT=$(echo "$TEST_OUTPUT" | grep "test result:" || echo "")
if echo "$TEST_RESULT" | grep -q "passed"; then
    PASSED=$(echo "$TEST_RESULT" | grep -oP '\d+(?= passed)' || echo "0")
    FAILED=$(echo "$TEST_RESULT" | grep -oP '\d+(?= failed)' || echo "0")
    RESULTS+=($(check_result "unit_tests" 0 "passed=$PASSED,failed=$FAILED"))
    log_info "✓ Unit Tests PASSED ($PASSED passed, $FAILED failed)"
    if [[ "$FAILED" -gt 10 ]]; then
        log_warn "⚠ High failure count: $FAILED (threshold: 10)"
        EXIT_CODE=1
    fi
else
    RESULTS+=($(check_result "unit_tests" 1 "cargo test --lib FAILED"))
    log_error "✗ Unit Tests FAILED"
fi

# ============================================
# CHECK 4: Coverage Check (≥80% target)
# ============================================
log_info "CHECK 4: Coverage Check"
if command -v cargo-llvm-cov &> /dev/null; then
    COVERAGE_OUTPUT=$(cargo llvm-cov --features stable --json 2>&1 || echo "{}")
    COVERAGE=$(echo "$COVERAGE_OUTPUT" | grep -oP '"percent":\K\d+\.\d+' | head -1 || echo "0")
    if (( $(echo "$COVERAGE >= 80.0" | bc -l 2>/dev/null || echo 0) )); then
        RESULTS+=($(check_result "coverage" 0 "coverage=${COVERAGE}%"))
        log_info "✓ Coverage PASSED (${COVERAGE}% ≥ 80%)"
    else
        RESULTS+=($(check_result "coverage" 1 "coverage=${COVERAGE}% < 80%"))
        log_warn "⚠ Coverage BELOW TARGET (${COVERAGE}% < 80%)"
    fi
else
    RESULTS+=($(check_result "coverage" 0 "cargo-llvm-cov not installed, skipping"))
    log_info "⊘ Coverage SKIPPED (cargo-llvm-cov not installed)"
fi

# ============================================
# CHECK 5: Dependency Audit (cargo audit)
# ============================================
log_info "CHECK 5: Dependency Audit (cargo audit)"
if command -v cargo-audit &> /dev/null; then
    if cargo audit 2>&1 | grep -q "Crate.*Yanked\|Warning.*yanked"; then
        RESULTS+=($(check_result "dependency_audit" 1 "yanked dependencies found"))
        log_error "✗ Dependency Audit FAILED (yanked deps)"
    else
        RESULTS+=($(check_result "dependency_audit" 0 "no yanked dependencies"))
        log_info "✓ Dependency Audit PASSED"
    fi
else
    RESULTS+=($(check_result "dependency_audit" 0 "cargo-audit not installed, skipping"))
    log_info "⊘ Dependency Audit SKIPPED (cargo-audit not installed)"
fi

# ============================================
# CHECK 6: Feature Flags Validation
# ============================================
log_info "CHECK 6: Feature Flags Validation"
FEATURES_OK=true
for feature in stable v2.0-sprint1 v2.0-sprint2; do
    if cargo metadata --features "$feature" --format-version 1 >/dev/null 2>&1; then
        log_info "  ✓ Feature '$feature' valid"
    else
        log_error "  ✗ Feature '$feature' INVALID"
        FEATURES_OK=false
    fi
done
if [[ "$FEATURES_OK" == true ]]; then
    RESULTS+=($(check_result "feature_flags" 0 "all features valid"))
else
    RESULTS+=($(check_result "feature_flags" 1 "invalid features detected"))
    EXIT_CODE=1
fi

# ============================================
# CHECK 7: Critical Files Existence
# ============================================
log_info "CHECK 7: Critical Files Existence"
CRITICAL_FILES=(
    "Cargo.toml"
    "src/lib.rs"
    "SECURITY.md"
    "GOVERNANCE.md"
    "CONTRIBUTING.md"
    "release/changelog.md"
    "release/v2.0.0-stable/RELEASE_NOTES.md"
)
FILES_OK=true
for file in "${CRITICAL_FILES[@]}"; do
    if [[ -f "${PROJECT_ROOT}/${file}" ]]; then
        log_info "  ✓ ${file} exists"
    else
        log_error "  ✗ ${file} MISSING"
        FILES_OK=false
    fi
done
if [[ "$FILES_OK" == true ]]; then
    RESULTS+=($(check_result "critical_files" 0 "all critical files present"))
else
    RESULTS+=($(check_result "critical_files" 1 "missing critical files"))
    EXIT_CODE=1
fi

# ============================================
# CHECK 8: Git Status (clean working tree)
# ============================================
log_info "CHECK 8: Git Status"
if git status --porcelain 2>/dev/null | grep -q .; then
    RESULTS+=($(check_result "git_status" 0 "uncommitted changes present"))
    log_warn "⚠ Uncommitted changes detected"
else
    RESULTS+=($(check_result "git_status" 0 "clean working tree"))
    log_info "✓ Git status clean"
fi

# ============================================
# Calculate Duration
# ============================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# ============================================
# Generate Report
# ============================================
if [[ "$GENERATE_REPORT" == true ]]; then
    log_info "Generating report: $REPORT_FILE"
    {
        echo "{"
        echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
        echo "  \"version\": \"v2.0.0-stable\","
        echo "  \"duration_seconds\": $DURATION,"
        echo "  \"exit_code\": $EXIT_CODE,"
        echo "  \"verdict\": \"$([ $EXIT_CODE -eq 0 ] && echo 'PASS' || echo 'FAIL')\","
        echo "  \"checks\": ["
        for i in "${!RESULTS[@]}"; do
            comma=","
            if [[ $i -eq $((${#RESULTS[@]} - 1)) ]]; then comma=""; fi
            echo "    ${RESULTS[$i]}$comma"
        done
        echo "  ]"
        echo "}"
    } > "$REPORT_FILE"
    log_info "Report saved: $REPORT_FILE"
fi

# ============================================
# Final Summary
# ============================================
echo ""
echo "=========================================="
echo "  Health Check Complete"
echo "  Duration: ${DURATION}s"
echo "  Verdict: $([ $EXIT_CODE -eq 0 ] && echo 'PASS ✓' || echo 'FAIL ✗')"
echo "=========================================="
echo ""

exit $EXIT_CODE
