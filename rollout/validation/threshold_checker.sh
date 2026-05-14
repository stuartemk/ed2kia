#!/bin/sh
# =============================================================================
# Threshold Checker for ed2kIA v0.6.0-RC Canary Validation
# =============================================================================
# Usage: ./rollout/validation/threshold_checker.sh [OPTIONS]
#
# Reads telemetry data (JSONL) and validates against canary deployment
# thresholds defined in release/v0.6.0-rc/rollout_plan.md.
#
# Options:
#   --input FILE        Telemetry JSONL file (default: /tmp/ed2kia_telemetry.jsonl)
#   --report FILE       Output report file (default: stdout)
#   --strict            Fail on ANY threshold breach (not just consecutive)
#   --help              Show this help
#
# Thresholds (from rollout_plan.md):
#   CRITICAL (triggers rollback):
#     - Consensus < 70% for 3 consecutive cycles
#     - SAE latency p95 > 800ms for 3 consecutive cycles
#     - API error rate > 2% for 3 consecutive cycles
#   WARNING (requires attention):
#     - Consensus < 85%
#     - SAE latency p95 > 400ms
#     - API error rate > 0.5%
#     - WASM memory > 80%
#
# Exit Codes:
#   0 - ALL CLEAR (all thresholds within limits)
#   1 - WARNING (some warnings, no critical breaches)
#   2 - ROLLBACK_TRIGGERED (critical threshold breached)
#   3 - ERROR (input file missing or malformed)
# =============================================================================

set -euo pipefail

# --- Defaults ---
INPUT="/tmp/ed2kia_telemetry.jsonl"
REPORT=""
STRICT=0

# --- Thresholds ---
CRIT_CONSENSUS=70
CRIT_LATENCY=800
CRIT_ERROR_RATE=200    # Stored as *100 to avoid floats (2.00% = 200)
WARN_CONSENSUS=85
WARN_LATENCY=400
WARN_ERROR_RATE=50     # 0.50% = 50
WARN_WASM_MEM=80
CONSECUTIVE_LIMIT=3

# --- Parse Arguments ---
while [ $# -gt 0 ]; do
    case "$1" in
        --input) INPUT="$2"; shift 2 ;;
        --report) REPORT="$2"; shift 2 ;;
        --strict) STRICT=1; shift ;;
        --help)
            head -30 "$0" | tail -25
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# --- Output Helper ---
output() {
    if [ -n "$REPORT" ]; then
        echo "$1" >> "$REPORT"
    fi
    echo "$1"
}

# --- Validate Input ---
if [ ! -f "$INPUT" ]; then
    echo "ERROR: Telemetry file not found: $INPUT"
    echo "Run telemetry_simulator.sh first."
    exit 3
fi

# --- Header ---
output "============================================================"
output "ed2kIA v0.6.0-RC Canary Threshold Validation Report"
output "============================================================"
output "Input: $INPUT"
output "Date: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
output "Strict Mode: $([ $STRICT -eq 1 ] && echo 'YES' || echo 'NO')"
output ""
output "Thresholds:"
output "  CRITICAL (rollback trigger):"
output "    Consensus < ${CRIT_CONSENSUS}% for $CONSECUTIVE_LIMIT consecutive cycles"
output "    SAE Latency > ${CRIT_LATENCY}ms for $CONSECUTIVE_LIMIT consecutive cycles"
output "    API Error Rate > ${CRIT_ERROR_RATE}% for $CONSECUTIVE_LIMIT consecutive cycles"
output "  WARNING:"
output "    Consensus < ${WARN_CONSENSUS}%"
output "    SAE Latency > ${WARN_LATENCY}ms"
output "    API Error Rate > 0.${WARN_ERROR_RATE}%"
output "    WASM Memory > ${WARN_WASM_MEM}%"
output ""
output "------------------------------------------------------------"

# --- Counters ---
TOTAL_CYCLES=0
WARN_COUNT=0
CRIT_COUNT=0
CONSEC_CONSENSUS=0
CONSEC_LATENCY=0
CONSEC_ERROR=0
ROLLBACK_STATUS="CLEAR"

# --- Process Each Line ---
while IFS= read -r line; do
    # Skip comments and empty lines
    case "$line" in
        \#*|"") continue ;;
    esac

    TOTAL_CYCLES=$((TOTAL_CYCLES + 1))

    # Extract fields using grep/sed (POSIX compatible, no jq dependency)
    CYCLE=$(echo "$line" | grep -o '"cycle":[0-9]*' | cut -d: -f2)
    TIMESTAMP=$(echo "$line" | grep -o '"timestamp":"[^"]*"' | cut -d'"' -f4)
    SAE_LATENCY=$(echo "$line" | grep -o '"sae_latency_p95":[0-9.]*' | cut -d: -f2)
    CONSENSUS=$(echo "$line" | grep -o '"consensus_pct":[0-9.]*' | cut -d: -f2)
    API_ERROR=$(echo "$line" | grep -o '"api_error_rate":[0-9.]*' | cut -d: -f2)
    WASM_MEM=$(echo "$line" | grep -o '"wasm_memory_pct":[0-9.]*' | cut -d: -f2)

    # Convert to integers for comparison (multiply by 100)
    LAT_INT=$(echo "$SAE_LATENCY" | awk '{printf "%d", $1}')
    CONSENSUS_INT=$(echo "$CONSENSUS" | awk '{printf "%d", $1 * 100}')
    ERROR_INT=$(echo "$API_ERROR" | awk '{printf "%d", $1 * 10000}')
    MEM_INT=$(echo "$WASM_MEM" | awk '{printf "%d", $1}')

    # --- Check Critical Thresholds ---
    CRIT_CYCLE=0

    # Consensus critical
    if [ "$CONSENSUS_INT" -lt "$((CRIT_CONSENSUS * 100))" ]; then
        CONSEC_CONSENSUS=$((CONSEC_CONSENSUS + 1))
    else
        CONSEC_CONSENSUS=0
    fi

    # Latency critical
    if [ "$LAT_INT" -gt "$CRIT_LATENCY" ]; then
        CONSEC_LATENCY=$((CONSEC_LATENCY + 1))
    else
        CONSEC_LATENCY=0
    fi

    # Error rate critical
    if [ "$ERROR_INT" -gt "$((CRIT_ERROR_RATE * 100))" ]; then
        CONSEC_ERROR=$((CONSEC_ERROR + 1))
    else
        CONSEC_ERROR=0
    fi

    # Check if any critical threshold hit consecutive limit
    if [ "$CONSEC_CONSENSUS" -ge "$CONSECUTIVE_LIMIT" ] || \
       [ "$CONSEC_LATENCY" -ge "$CONSECUTIVE_LIMIT" ] || \
       [ "$CONSEC_ERROR" -ge "$CONSECUTIVE_LIMIT" ]; then
        CRIT_CYCLE=1
        CRIT_COUNT=$((CRIT_COUNT + 1))
        ROLLBACK_STATUS="ROLLBACK_TRIGGERED"
    fi

    # --- Check Warning Thresholds ---
    WARN_CYCLE=0
    WARN_REASONS=""

    if [ "$CONSENSUS_INT" -lt "$((WARN_CONSENSUS * 100))" ]; then
        WARN_CYCLE=1
        WARN_REASONS="${WARN_REASONS}consensus=${CONSENSUS}%"
    fi

    if [ "$LAT_INT" -gt "$WARN_LATENCY" ]; then
        WARN_CYCLE=1
        WARN_REASONS="${WARN_REASONS:+$WARN_REASONS, }latency=${SAE_LATENCY}ms"
    fi

    if [ "$ERROR_INT" -gt "$((WARN_ERROR_RATE * 100))" ]; then
        WARN_CYCLE=1
        WARN_REASONS="${WARN_REASONS:+$WARN_REASONS, }errors=${API_ERROR}%"
    fi

    if [ "$MEM_INT" -gt "$WARN_WASM_MEM" ]; then
        WARN_CYCLE=1
        WARN_REASONS="${WARN_REASONS:+$WARN_REASONS, }wasm_mem=${WASM_MEM}%"
    fi

    if [ "$WARN_CYCLE" -eq 1 ]; then
        WARN_COUNT=$((WARN_COUNT + 1))
    fi

    # --- Strict mode: any breach = critical ---
    if [ "$STRICT" -eq 1 ] && [ "$WARN_CYCLE" -eq 1 ]; then
        CRIT_COUNT=$((CRIT_COUNT + 1))
        ROLLBACK_STATUS="ROLLBACK_TRIGGERED"
        CRIT_CYCLE=1
    fi

    # --- Output Cycle Result ---
    if [ "$CRIT_CYCLE" -eq 1 ]; then
        output "[CRITICAL] Cycle $CYCLE ($TIMESTAMP) - ROLLBACK THRESHOLD BREACHED"
        output "  Latency: ${SAE_LATENCY}ms | Consensus: ${CONSENSUS}% | Errors: ${API_ERROR}%"
        output "  Consecutive breaches: consensus=$CONSEC_CONSENSUS latency=$CONSEC_LATENCY errors=$CONSEC_ERROR"
    elif [ "$WARN_CYCLE" -eq 1 ]; then
        output "[WARNING]  Cycle $CYCLE ($TIMESTAMP) - $WARN_REASONS"
    else
        output "[OK]       Cycle $CYCLE ($TIMESTAMP) - All thresholds within limits"
    fi

done < "$INPUT"

# --- Summary ---
output ""
output "============================================================"
output "VALIDATION SUMMARY"
output "============================================================"
output "Total Cycles Analyzed: $TOTAL_CYCLES"
output "Warnings: $WARN_COUNT"
output "Critical Breaches: $CRIT_COUNT"
output ""
output "STATUS: $ROLLBACK_STATUS"
output ""

case "$ROLLBACK_STATUS" in
    CLEAR)
        output "RESULT: ✅ CANARY VALIDATION PASSED"
        output "All metrics within acceptable thresholds."
        output "Proceed to next rollout phase."
        exit 0
        ;;
    ROLLBACK_TRIGGERED)
        output "RESULT: ❌ CANARY VALIDATION FAILED"
        output "Critical thresholds breached. ROLLBACK REQUIRED."
        output ""
        output "Immediate Actions:"
        output "  1. Execute: ./ops/rollback_v0.6.0.sh --auto"
        output "  2. Notify governance channel"
        output "  3. File incident report"
        output "  4. Review telemetry: $INPUT"
        exit 2
        ;;
esac
