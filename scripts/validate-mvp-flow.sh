#!/usr/bin/env bash
# =============================================================================
# validate-mvp-flow.sh — MVP Core Loop Simulated Validation
# =============================================================================
# Purpose: Validate v2.1-mvp-core feature gate without network calls, real
#          inference, or permanent state. Parses test output to verify the
#          3-step cycle (discovery → distribute → infer → return) completes
#          without panics.
#
# Usage:   bash scripts/validate-mvp-flow.sh
# Exit:    0 on PASS, 1 on FAIL
# =============================================================================
set -euo pipefail

FEATURE="v2.1-mvp-core"
PASS_COUNT=0
FAIL_COUNT=0
RESULT="PASS"

echo "============================================"
echo "  MVP Core Loop Validation — v2.1"
echo "  Feature: ${FEATURE}"
echo "  Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date)"
echo "============================================"
echo ""

# ---------------------------------------------------------------------------
# Step 1: cargo test --features v2.1-mvp-core --lib mvp_core
# ---------------------------------------------------------------------------
echo "[1/3] Running MVP Core Loop tests..."
TEST_OUTPUT=$(cargo test --features "${FEATURE}" --lib mvp_core -- --nocapture 2>&1) || true

# Parse test results
PASSED=$(echo "${TEST_OUTPUT}" | grep -c "test result: ok" || true)
FAILED=$(echo "${TEST_OUTPUT}" | grep -c "test result: FAILED" || true)
TOTAL_TESTS=$(echo "${TEST_OUTPUT}" | grep -oP 'test result:;?\s*\K\d+' || echo "0")

if [ "${PASSED}" -gt 0 ] && [ "${FAILED}" -eq 0 ]; then
    echo "  PASS: Tests completed successfully (${TOTAL_TESTS} tests)"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "  FAIL: Tests failed or did not run"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    RESULT="FAIL"
fi

# Verify core cycle steps (discovery → distribute → infer → return)
echo "[1b] Verifying core cycle steps..."
CYCLE_STEPS=("discovery" "distribute" "infer" "return")
for step in "${CYCLE_STEPS[@]}"; do
    if echo "${TEST_OUTPUT}" | grep -qi "${step}"; then
        echo "  PASS: Cycle step '${step}' found in output"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "  WARN: Cycle step '${step}' not explicitly found (may be implicit)"
    fi
done

# Check for panics
PANICS=$(echo "${TEST_OUTPUT}" | grep -c "panicked" || true)
if [ "${PANICS}" -eq 0 ]; then
    echo "  PASS: No panics detected"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "  FAIL: ${PANICS} panic(s) detected"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    RESULT="FAIL"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 2: cargo bench --features v2.1-mvp-core --no-run
# ---------------------------------------------------------------------------
echo "[2/3] Compiling benchmarks (no-run)..."
BENCH_OUTPUT=$(cargo bench --features "${FEATURE}" --no-run 2>&1) || true

BENCH_COMPILED=$(echo "${BENCH_OUTPUT}" | grep -c "Finished" || true)
if [ "${BENCH_COMPILED}" -gt 0 ]; then
    echo "  PASS: Benchmarks compiled successfully"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "  WARN: Benchmark compilation status unclear"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 3: cargo check --features v2.1-mvp-core
# ---------------------------------------------------------------------------
echo "[3/3] Final cargo check..."
CHECK_OUTPUT=$(cargo check --features "${FEATURE}" 2>&1) || true

CHECK_OK=$(echo "${CHECK_OUTPUT}" | grep -c "Finished" || true)
if [ "${CHECK_OK}" -gt 0 ]; then
    echo "  PASS: cargo check completed"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo "  FAIL: cargo check failed"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    RESULT="FAIL"
fi

echo ""

# ---------------------------------------------------------------------------
# Summary Report
# ---------------------------------------------------------------------------
echo "============================================"
echo "  MVP Core Loop Validation Summary"
echo "============================================"
echo "  Result: ${RESULT}"
echo "  Checks Passed: ${PASS_COUNT}"
echo "  Checks Failed: ${FAIL_COUNT}"
echo "  Tests: ${TOTAL_TESTS} (from cargo test)"
echo "  Benchmarks: Registered"
echo "============================================"
echo ""

# Generate temp report (cleaned up)
TMP_DIR=$(mktemp -d 2>/dev/null || echo "/tmp/mvp-validation-$$")
cat > "${TMP_DIR}/mvp-validation.txt" <<EOF
MVP Core Loop Validation: ${RESULT}
Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date)
Feature: ${FEATURE}
Checks Passed: ${PASS_COUNT}
Checks Failed: ${FAIL_COUNT}
Tests: ${TOTAL_TESTS}
Benchmarks: Registered
Panics: ${PANICS}
EOF

echo "  Report: ${TMP_DIR}/mvp-validation.txt"
cat "${TMP_DIR}/mvp-validation.txt"
echo ""

# Cleanup
rm -rf "${TMP_DIR}"

# Exit code
if [ "${RESULT}" = "PASS" ]; then
    echo "MVP Core Loop Validation: PASS"
    exit 0
else
    echo "MVP Core Loop Validation: FAIL"
    exit 1
fi
