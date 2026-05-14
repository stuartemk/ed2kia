#!/bin/sh
# =============================================================================
# Telemetry Simulator for ed2kIA v0.6.0-RC Canary Validation
# =============================================================================
# Usage: ./rollout/validation/telemetry_simulator.sh [OPTIONS]
#
# Generates simulated telemetry data (JSON) every N seconds to validate
# canary deployment thresholds before production rollout.
#
# Options:
#   --interval SECS     Output interval in seconds (default: 60)
#   --cycles N          Number of cycles to generate (default: 20)
#   --output FILE       Output file (default: /tmp/ed2kia_telemetry.jsonl)
#   --scenario MODE     Scenario mode: normal, degradation, failure
#   --seed N            Random seed for reproducibility (default: timestamp)
#   --help              Show this help
#
# Metrics Generated:
#   - sae_latency_p95   : SAE inference latency p95 (ms)
#   - consensus_pct     : Consensus participation (%)
#   - reputation_avg    : Average node reputation [0-1]
#   - api_error_rate    : API v2 error rate (%)
#   - wasm_memory_pct   : WASM sandbox memory usage (%)
#   - federation_rounds : Federation rounds completed/total
#   - staking_active    : Active staking nodes count
#
# Output Format: JSONL (one JSON object per line)
# =============================================================================

set -euo pipefail

# --- Defaults ---
INTERVAL=60
CYCLES=20
OUTPUT="/tmp/ed2kia_telemetry.jsonl"
SCENARIO="normal"
SEED=""

# --- Parse Arguments ---
while [ $# -gt 0 ]; do
    case "$1" in
        --interval) INTERVAL="$2"; shift 2 ;;
        --cycles) CYCLES="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        --scenario) SCENARIO="$2"; shift 2 ;;
        --seed) SEED="$2"; shift 2 ;;
        --help)
            head -30 "$0" | tail -25
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# --- Pseudo-Random Number Generator (POSIX) ---
if [ -z "$SEED" ]; then
    SEED=$(date +%s)
fi

rand_next() {
    SEED=$(( (SEED * 1103515245 + 12345) % 2147483648 ))
    echo $SEED
}

# Generate random number in range [min, max]
rand_range() {
    _min=$1
    _max=$2
    _range=$(( _max - _min + 1 ))
    _val=$(rand_next)
    echo $(( (_val % _range) + _min ))
}

# Generate random float with 2 decimal places [min*100, max*100] / 100
rand_float() {
    _min_int=$(echo "$1" | awk '{printf "%d", $1 * 100}')
    _max_int=$(echo "$2" | awk '{printf "%d", $1 * 100}')
    _val=$(rand_range $_min_int $_max_int)
    echo "scale=2; $_val / 100" | bc
}

# --- Scenario Configuration ---
# Format: sae_latency consensus reputation api_error wasm_memory federation staking
case "$SCENARIO" in
    normal)
        SAE_LAT_MIN=150; SAE_LAT_MAX=350
        CONSENSUS_MIN=88; CONSENSUS_MAX=99
        REP_MIN=75; REP_MAX=95
        API_ERR_MIN=0; API_ERR_MAX=3   # 0.00 - 0.03
        WASM_MEM_MIN=30; WASM_MEM_MAX=65
        FED_SUCCESS_MIN=90; FED_SUCCESS_MAX=100
        STAKING_MIN=180; STAKING_MAX=195
        ;;
    degradation)
        SAE_LAT_MIN=300; SAE_LAT_MAX=600
        CONSENSUS_MIN=75; CONSENSUS_MAX=92
        REP_MIN=60; REP_MAX=85
        API_ERR_MIN=1; API_ERR_MAX=15  # 0.01 - 0.15
        WASM_MEM_MIN=55; WASM_MEM_MAX=85
        FED_SUCCESS_MIN=70; FED_SUCCESS_MAX=92
        STAKING_MIN=160; STAKING_MAX=190
        ;;
    failure)
        SAE_LAT_MIN=500; SAE_LAT_MAX=1200
        CONSENSUS_MIN=50; CONSENSUS_MAX=82
        REP_MIN=40; REP_MAX=70
        API_ERR_MIN=5; API_ERR_MAX=50  # 0.05 - 0.50
        WASM_MEM_MIN=75; WASM_MEM_MAX=98
        FED_SUCCESS_MIN=40; FED_SUCCESS_MAX=70
        STAKING_MIN=100; STAKING_MAX=160
        ;;
    *)
        echo "ERROR: Unknown scenario: $SCENARIO (use: normal, degradation, failure)"
        exit 1
        ;;
esac

# --- Header ---
echo "# ed2kIA v0.6.0-RC Telemetry Simulator" > "$OUTPUT"
echo "# Scenario: $SCENARIO | Interval: ${INTERVAL}s | Cycles: $CYCLES | Seed: $SEED" >> "$OUTPUT"
echo "# Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> "$OUTPUT"
echo "# Format: JSONL" >> "$OUTPUT"

# --- Generate Telemetry ---
CYCLE=0
TOTAL_FED_ROUNDS=100

while [ "$CYCLE" -lt "$CYCLES" ]; do
    CYCLE=$((CYCLE + 1))
    TIMESTAMP=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
    EPOCH=$(date +%s)

    # Generate metrics based on scenario
    SAE_LAT=$(rand_float $SAE_LAT_MIN $SAE_LAT_MAX)
    CONSENSUS=$(rand_float $CONSENSUS_MIN $CONSENSUS_MAX)
    REP_AVG=$(rand_float 0.$REP_MIN 0.$REP_MAX)
    API_ERR=$(rand_float 0.$API_ERR_MIN 0.$API_ERR_MAX)
    WASM_MEM=$(rand_float $WASM_MEM_MIN $WASM_MEM_MAX)
    FED_COMPLETED=$(rand_range $FED_SUCCESS_MIN $FED_SUCCESS_MAX)
    STAKING_ACTIVE=$(rand_range $STAKING_MIN $STAKING_MAX)

    # Output JSON line
    cat >> "$OUTPUT" <<EOF
{"timestamp":"${TIMESTAMP}","epoch":${EPOCH},"cycle":${CYCLE},"scenario":"${SCENARIO}","sae_latency_p95":${SAE_LAT},"consensus_pct":${CONSENSUS},"reputation_avg":${REP_AVG},"api_error_rate":${API_ERR},"wasm_memory_pct":${WASM_MEM},"federation_rounds_completed":${FED_COMPLETED},"federation_rounds_total":${TOTAL_FED_ROUNDS},"staking_active_nodes":${STAKING_ACTIVE}}
EOF

    echo "[Cycle $CYCLE/$CYCLES] $TIMESTAMP | Latency: ${SAE_LAT}ms | Consensus: ${CONSENSUS}% | Errors: ${API_ERR}%"

    if [ "$CYCLE" -lt "$CYCLES" ]; then
        sleep "$INTERVAL"
    fi
done

echo ""
echo "Telemetry generation complete: $OUTPUT"
echo "Total cycles: $CYCLES | Scenario: $SCENARIO"
echo "Run threshold_checker.sh to validate against canary thresholds."
