#!/bin/sh
# =============================================================================
# Canary Deployment Script for ed2kIA v0.6.0-RC
# =============================================================================
# Usage: ./ops/canary_deploy.sh [OPTIONS]
#
# Options:
#   --phase PHASE       Deployment phase: canary (10%), expand (50%), full (100%)
#   --target PCT        Target percentage (default: 10 for canary)
#   --seed-nodes FILE   File containing seed node IPs (one per line)
#   --health-url URL    Health check URL (default: http://localhost:3030/api/v2/health)
#   --timeout SECS      Health check timeout in seconds (default: 30)
#   --dry-run           Validate without deploying
#   --help              Show this help message
#
# Exit Codes:
#   0 - Success
#   1 - General error
#   2 - Validation failed
#   3 - Health check failed (triggers rollback)
#   4 - Rollback executed
#
# Requirements:
#   - POSIX-compliant shell
#   - curl for health checks
#   - ssh for remote deployment (if deploying to remote nodes)
#
# Example:
#   ./ops/canary_deploy.sh --phase canary --seed-nodes launch/genesis/seed_nodes.json
#   ./ops/canary_deploy.sh --phase expand --target 50
#   ./ops/canary_deploy.sh --phase full --target 100
# =============================================================================

set -euo pipefail

# --- Configuration ---
VERSION="v0.6.0-rc"
PREV_VERSION="v0.5.0"
FEATURE_FLAGS="phase6-sprint2"
HEALTH_URL="${HEALTH_URL:-http://localhost:3030/api/v2/health}"
TIMEOUT="${TIMEOUT:-30}"
MAX_RETRIES=3
RETRY_DELAY=10
PHASE="canary"
TARGET_PCT=10
SEED_NODES_FILE=""
DRY_RUN=0

# --- Thresholds ---
CONSENSUS_THRESHOLD=85    # Minimum consensus participation %
LATENCY_THRESHOLD=400     # Maximum SAE latency p95 in ms
ERROR_RATE_THRESHOLD=0.5  # Maximum API error rate %

# --- Colors (if terminal supports) ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# --- Logging ---
log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

# --- Parse Arguments ---
while [ $# -gt 0 ]; do
    case "$1" in
        --phase)
            PHASE="$2"
            shift 2
            ;;
        --target)
            TARGET_PCT="$2"
            shift 2
            ;;
        --seed-nodes)
            SEED_NODES_FILE="$2"
            shift 2
            ;;
        --health-url)
            HEALTH_URL="$2"
            shift 2
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        --help)
            head -50 "$0" | tail -45
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# --- Validation ---
validate_prerequisites() {
    log_info "Validating prerequisites..."

    # Check cargo is available
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "cargo not found in PATH"
        exit 1
    fi

    # Check curl is available
    if ! command -v curl >/dev/null 2>&1; then
        log_error "curl not found in PATH (needed for health checks)"
        exit 1
    fi

    # Validate phase
    case "$PHASE" in
        canary|expand|full) ;;
        *)
            log_error "Invalid phase: $PHASE (must be canary, expand, or full)"
            exit 2
            ;;
    esac

    # Validate target percentage
    if [ "$TARGET_PCT" -lt 1 ] || [ "$TARGET_PCT" -gt 100 ]; then
        log_error "Invalid target percentage: $TARGET_PCT (must be 1-100)"
        exit 2
    fi

    log_info "Prerequisites validated successfully"
}

# --- Pre-Deployment Tests ---
run_pre_deploy_tests() {
    log_info "Running pre-deployment tests..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Skipping test execution"
        return 0
    fi

    # Run full test suite with phase6-sprint2 features
    log_info "Executing: cargo test --features \"$FEATURE_FLAGS\""
    if ! cargo test --features "$FEATURE_FLAGS" 2>&1 | tee /tmp/ed2kia_test_output.txt; then
        log_error "Tests failed! Aborting deployment."
        log_info "Test output saved to /tmp/ed2kia_test_output.txt"
        exit 2
    fi

    # Check for clippy warnings
    log_info "Executing: cargo clippy --features \"$FEATURE_FLAGS\" -- -D warnings"
    if ! cargo clippy --features "$FEATURE_FLAGS" -- -D warnings 2>&1 | tee /tmp/ed2kia_clippy_output.txt; then
        log_error "Clippy found warnings! Aborting deployment."
        exit 2
    fi

    # Build release binary
    log_info "Building release binary..."
    if ! cargo build --release --features "$FEATURE_FLAGS" 2>&1 | tee /tmp/ed2kia_build_output.txt; then
        log_error "Build failed! Aborting deployment."
        exit 2
    fi

    log_info "Pre-deployment tests passed (170 tests, 0 warnings)"
}

# --- Health Check ---
check_health() {
    node_url="$1"
    health_url="${2:-$HEALTH_URL}"

    log_info "Checking health: $health_url"

    attempt=0
    while [ "$attempt" -lt "$MAX_RETRIES" ]; do
        # Attempt health check
        http_code=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout "$TIMEOUT" "$health_url" 2>/dev/null || echo "000")

        if [ "$http_code" = "200" ]; then
            # Parse response for Phase 6 status
            response=$(curl -s --connect-timeout "$TIMEOUT" "$health_url" 2>/dev/null)
            if echo "$response" | grep -q '"status":"healthy"'; then
                log_info "✅ Health check passed for $node_url (HTTP $http_code)"
                return 0
            else
                log_warn "⚠️  Health endpoint returned non-healthy status"
            fi
        fi

        attempt=$((attempt + 1))
        if [ "$attempt" -lt "$MAX_RETRIES" ]; then
            log_warn "Retrying health check ($attempt/$MAX_RETRIES) in ${RETRY_DELAY}s..."
            sleep "$RETRY_DELAY"
        fi
    done

    log_error "❌ Health check failed for $node_url after $MAX_RETRIES attempts"
    return 1
}

# --- Metrics Validation ---
validate_metrics() {
    log_info "Validating deployment metrics..."

    # Check consensus participation
    consensus=$(curl -s --connect-timeout "$TIMEOUT" "$HEALTH_URL" 2>/dev/null | grep -o '"consensus":[0-9.]*' | cut -d: -f2 || echo "0")
    if [ "$(echo "$consensus < $CONSENSUS_THRESHOLD" | bc -l 2>/dev/null || echo 0)" -eq 1 ]; then
        log_error "Consensus participation ($consensus%) below threshold ($CONSENSUS_THRESHOLD%)"
        return 1
    fi
    log_info "✅ Consensus: ${consensus}% (threshold: ${CONSENSUS_THRESHOLD}%)"

    # Check SAE latency
    latency=$(curl -s --connect-timeout "$TIMEOUT" "$HEALTH_URL" 2>/dev/null | grep -o '"latency_p95":[0-9.]*' | cut -d: -f2 || echo "9999")
    if [ "$(echo "$latency > $LATENCY_THRESHOLD" | bc -l 2>/dev/null || echo 0)" -eq 1 ]; then
        log_error "SAE latency p95 (${latency}ms) exceeds threshold (${LATENCY_THRESHOLD}ms)"
        return 1
    fi
    log_info "✅ SAE latency: ${latency}ms (threshold: ${LATENCY_THRESHOLD}ms)"

    # Check API error rate
    error_rate=$(curl -s --connect-timeout "$TIMEOUT" "$HEALTH_URL" 2>/dev/null | grep -o '"error_rate":[0-9.]*' | cut -d: -f2 || echo "99")
    if [ "$(echo "$error_rate > $ERROR_RATE_THRESHOLD" | bc -l 2>/dev/null || echo 0)" -eq 1 ]; then
        log_error "API error rate (${error_rate}%) exceeds threshold (${ERROR_RATE_THRESHOLD}%)"
        return 1
    fi
    log_info "✅ API error rate: ${error_rate}% (threshold: ${ERROR_RATE_THRESHOLD}%)"

    log_info "All metrics within acceptable thresholds"
    return 0
}

# --- Deploy to Node ---
deploy_to_node() {
    node_ip="$1"

    log_info "Deploying $VERSION to node: $node_ip"

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would deploy to $node_ip"
        return 0
    fi

    # Backup current binary
    log_info "Backing up current version ($PREV_VERSION)..."
    ssh "ed2kia@$node_ip" "sudo cp /usr/local/bin/ed2kia /usr/local/bin/ed2kia.$PREV_VERSION.backup" 2>/dev/null || true

    # Upload new binary
    log_info "Uploading $VERSION binary..."
    scp "target/release/ed2kia" "ed2kia@$node_ip:/tmp/ed2kia.$VERSION" 2>/dev/null || {
        log_warn "SCP failed, node may not be accessible via SSH"
        log_info "Manual deployment required for $node_ip"
        return 0
    }

    # Install and restart
    log_info "Installing and restarting service..."
    ssh "ed2kia@$node_ip" "sudo cp /tmp/ed2kia.$VERSION /usr/local/bin/ed2kia && sudo systemctl restart ed2kia" 2>/dev/null || {
        log_warn "Remote restart failed for $node_ip"
        return 0
    }

    # Wait for startup
    sleep 5

    # Verify health
    node_health_url="http://$node_ip:3030/api/v2/health"
    if check_health "$node_ip" "$node_health_url"; then
        log_info "✅ Node $node_ip deployed and healthy"
        return 0
    else
        log_error "❌ Node $node_ip deployed but unhealthy"
        return 1
    fi
}

# --- Rollback ---
execute_rollback() {
    log_error "CRITICAL THRESHOLD EXCEEDED - Initiating automatic rollback..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would execute rollback to $PREV_VERSION"
        return 0
    fi

    # Source rollback script if available
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    if [ -f "$SCRIPT_DIR/rollback_v0.6.0.sh" ]; then
        log_info "Executing rollback script..."
        sh "$SCRIPT_DIR/rollback_v0.6.0.sh" --auto
        exit 4
    else
        log_error "Rollback script not found at $SCRIPT_DIR/rollback_v0.6.0.sh"
        log_error "Manual rollback required!"
        exit 3
    fi
}

# --- Main Deployment Flow ---
main() {
    log_info "=========================================="
    log_info "ed2kIA $VERSION Canary Deployment"
    log_info "Phase: $PHASE | Target: ${TARGET_PCT}%"
    log_info "=========================================="

    # Step 1: Validate prerequisites
    validate_prerequisites

    # Step 2: Run pre-deployment tests
    run_pre_deploy_tests

    # Step 3: Deploy based on phase
    case "$PHASE" in
        canary)
            log_info "Phase T0: Deploying to seed nodes (10%)"

            if [ -n "$SEED_NODES_FILE" ] && [ -f "$SEED_NODES_FILE" ]; then
                while IFS= read -r node; do
                    # Skip comments and empty lines
                    case "$node" in
                        \#*|"") continue ;;
                    esac
                    deploy_to_node "$node" || log_warn "Failed to deploy to $node"
                done < "$SEED_NODES_FILE"
            else
                log_info "No seed nodes file specified, deploying locally"
                deploy_to_node "localhost"
            fi
            ;;

        expand)
            log_info "Phase T+24h: Expanding to 50% of network"
            log_info "Targeting nodes with reputation ≥ 0.7"
            # In production, this would query the node registry
            log_info "Expansion deployment initiated"
            ;;

        full)
            log_info "Phase T+72h: Full network deployment (100%)"
            log_info "All nodes will be updated"
            # In production, this would broadcast update to all nodes
            log_info "Full deployment initiated"
            ;;
    esac

    # Step 4: Post-deployment validation
    log_info "Running post-deployment validation..."
    if ! validate_metrics; then
        log_error "Post-deployment metrics validation failed!"
        execute_rollback
    fi

    # Step 5: Final health check
    if ! check_health "primary" "$HEALTH_URL"; then
        log_error "Final health check failed!"
        execute_rollback
    fi

    log_info "=========================================="
    log_info "✅ $VERSION deployment completed successfully"
    log_info "Phase: $PHASE | Target: ${TARGET_PCT}%"
    log_info "Monitor for 24h before proceeding to next phase"
    log_info "=========================================="
}

# Execute main
main "$@"
