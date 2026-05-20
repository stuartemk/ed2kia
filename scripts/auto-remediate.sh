#!/usr/bin/env bash
###############################################################################
# auto-remediate.sh — Auto-remediation & Rollback Pipeline
#
# Sprint15 - Resiliencia Operativa & Automatización de Respuesta
#
# Automated incident response with:
# - Active monitoring (health, metrics, consensus, slashing/partition detection)
# - Auto actions (graceful restart, rollback to checkpoint, incident reports)
# - Optional webhook notifications
#
# Usage:
#   ./scripts/auto-remediate.sh [--monitor] [--rollback] [--report] [--webhook URL]
#
# Safety:
#   set -euo pipefail
#   trap cleanup EXIT INT TERM
###############################################################################

set -euo pipefail
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

ED2KIA_API="${ED2KIA_API:-http://localhost:3000}"
ED2KIA_DATA_DIR="${ED2KIA_DATA_DIR:-./data}"
ED2KIA_CHECKPOINT_DIR="${ED2KIA_CHECKPOINT_DIR:-${ED2KIA_DATA_DIR}/checkpoints}"
ED2KIA_LOG_DIR="${ED2KIA_LOG_DIR:-./logs}"
ED2KIA_REPORT_DIR="${ED2KIA_REPORT_DIR:-./reports}"
ED2KIA_WEBHOOK_URL="${ED2KIA_WEBHOOK_URL:-}"
ED2KIA_MAX_RESTARTS="${ED2KIA_MAX_RESTARTS:-3}"
ED2KIA_HEALTH_INTERVAL="${ED2KIA_HEALTH_INTERVAL:-10}"  # seconds
ED2KIA_METRICS_INTERVAL="${ED2KIA_METRICS_INTERVAL:-30}" # seconds
ED2KIA_CONSENSUS_TIMEOUT="${ED2KIA_CONSENSUS_TIMEOUT:-60}" # seconds

RESTART_COUNT=0
INCIDENT_ID=""
INCIDENT_START=""

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ---------------------------------------------------------------------------
# Utility Functions
# ---------------------------------------------------------------------------

cleanup() {
    local exit_code=$?
    log_info "Cleanup triggered (exit code: ${exit_code})"
    if [[ -n "${INCIDENT_ID}" ]]; then
        log_info "Incident ${INCIDENT_ID} cleanup complete"
    fi
    exit ${exit_code}
}

log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_debug() {
    echo -e "${BLUE}[DEBUG]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

generate_incident_id() {
    INCIDENT_ID="incident-$(date '+%Y%m%d%H%M%S')-$$"
    INCIDENT_START="$(date '+%Y-%m-%d %H:%M:%S')"
    log_info "Incident ID: ${INCIDENT_ID}"
}

# ---------------------------------------------------------------------------
# Monitoring Functions
# ---------------------------------------------------------------------------

# Check API health endpoint.
check_health() {
    log_debug "Checking health: ${ED2KIA_API}/api/health"

    local http_code
    http_code=$(curl -s -o /dev/null -w "%{http_code}" "${ED2KIA_API}/api/health" 2>/dev/null || echo "000")

    if [[ "${http_code}" == "200" ]]; then
        log_info "Health check passed (HTTP ${http_code})"
        return 0
    else
        log_error "Health check failed (HTTP ${http_code})"
        return 1
    fi
}

# Check API metrics endpoint.
check_metrics() {
    log_debug "Checking metrics: ${ED2KIA_API}/api/metrics"

    local http_code
    http_code=$(curl -s -o /dev/null -w "%{http_code}" "${ED2KIA_API}/api/metrics" 2>/dev/null || echo "000")

    if [[ "${http_code}" == "200" ]]; then
        log_info "Metrics check passed (HTTP ${http_code})"
        return 0
    else
        log_warn "Metrics check failed (HTTP ${http_code})"
        return 1
    fi
}

# Verify consensus is active.
check_consensus() {
    log_debug "Checking consensus status"

    local consensus_data
    consensus_data=$(curl -s "${ED2KIA_API}/api/consensus/status" 2>/dev/null || echo "{}")

    local active
    active=$(echo "${consensus_data}" | jq -r '.active // false' 2>/dev/null || echo "false")

    if [[ "${active}" == "true" ]]; then
        log_info "Consensus is active"
        return 0
    else
        log_warn "Consensus is not active"
        return 1
    fi
}

# Detect slashing events.
detect_slashing() {
    log_debug "Checking for slashing events"

    local slashing_data
    slashing_data=$(curl -s "${ED2KIA_API}/api/reputation/slashing" 2>/dev/null || echo "[]")

    local count
    count=$(echo "${slashing_data}" | jq 'length' 2>/dev/null || echo "0")

    if [[ "${count}" -gt 0 ]]; then
        log_error "Slashing detected: ${count} event(s)"
        echo "${slashing_data}"
        return 1
    else
        log_info "No slashing events detected"
        return 0
    fi
}

# Detect network partitions.
detect_partition() {
    log_debug "Checking for network partitions"

    local network_data
    network_data=$(curl -s "${ED2KIA_API}/api/network/status" 2>/dev/null || echo "{}")

    local peer_count
    peer_count=$(echo "${network_data}" | jq '.peer_count // 0' 2>/dev/null || echo "0")

    if [[ "${peer_count}" -eq 0 ]]; then
        log_error "Network partition detected: 0 peers connected"
        return 1
    else
        log_info "Network healthy: ${peer_count} peers connected"
        return 0
    fi
}

# ---------------------------------------------------------------------------
# Remediation Functions
# ---------------------------------------------------------------------------

# Graceful restart of ed2kIA service.
graceful_restart() {
    log_warn "Initiating graceful restart (attempt $((RESTART_COUNT + 1))/${ED2KIA_MAX_RESTARTS})"

    if [[ ${RESTART_COUNT} -ge ${ED2KIA_MAX_RESTARTS} ]]; then
        log_error "Maximum restart attempts (${ED2KIA_MAX_RESTARTS}) reached. Aborting."
        return 1
    fi

    RESTART_COUNT=$((RESTART_COUNT + 1))

    # Attempt graceful shutdown via API.
    log_info "Sending graceful shutdown signal..."
    curl -s -X POST "${ED2KIA_API}/api/shutdown" 2>/dev/null || true

    # Wait for shutdown.
    sleep 5

    # Verify process stopped.
    if pgrep -f "ed2kia" > /dev/null 2>&1; then
        log_warn "Process still running. Force killing..."
        pkill -f "ed2kia" || true
        sleep 2
    fi

    # Restart service.
    log_info "Restarting ed2kIA service..."
    if command -v systemctl &> /dev/null && systemctl is-active ed2kia.service &> /dev/null; then
        systemctl restart ed2kia.service
    else
        log_warn "systemd service not found. Manual restart required."
    fi

    log_info "Restart attempt ${RESTART_COUNT} complete"
    return 0
}

# Rollback to latest checkpoint.
rollback_to_checkpoint() {
    log_warn "Initiating rollback to latest checkpoint"

    if [[ ! -d "${ED2KIA_CHECKPOINT_DIR}" ]]; then
        log_error "Checkpoint directory not found: ${ED2KIA_CHECKPOINT_DIR}"
        return 1
    fi

    # Find latest checkpoint.
    local latest_checkpoint
    latest_checkpoint=$(ls -t "${ED2KIA_CHECKPOINT_DIR}"/*.checkpoint 2>/dev/null | head -1 || echo "")

    if [[ -z "${latest_checkpoint}" ]]; then
        log_error "No checkpoints found in ${ED2KIA_CHECKPOINT_DIR}"
        return 1
    fi

    log_info "Rolling back to: ${latest_checkpoint}"

    # Stop service.
    log_info "Stopping service for rollback..."
    curl -s -X POST "${ED2KIA_API}/api/shutdown" 2>/dev/null || true
    sleep 5

    # Restore checkpoint.
    log_info "Restoring checkpoint..."
    cp "${latest_checkpoint}" "${ED2KIA_DATA_DIR}/state.backup" 2>/dev/null || {
        log_error "Failed to restore checkpoint"
        return 1
    }

    # Restart service.
    log_info "Restarting service after rollback..."
    if command -v systemctl &> /dev/null && systemctl is-active ed2kia.service &> /dev/null; then
        systemctl restart ed2kia.service
    fi

    log_info "Rollback complete"
    return 0
}

# ---------------------------------------------------------------------------
# Reporting Functions
# ---------------------------------------------------------------------------

# Generate incident report.
generate_incident_report() {
    log_info "Generating incident report"

    mkdir -p "${ED2KIA_REPORT_DIR}"

    local report_file="${ED2KIA_REPORT_DIR}/${INCIDENT_ID}.md"

    cat > "${report_file}" <<EOF
# Incident Report: ${INCIDENT_ID}

## Summary
- **Incident ID:** ${INCIDENT_ID}
- **Start Time:** ${INCIDENT_START}
- **End Time:** $(date '+%Y-%m-%d %H:%M:%S')
- **Duration:** $(( $(date +%s) - $(date -d "${INCIDENT_START}" +%s 2>/dev/null || date +%s) )) seconds
- **Restart Attempts:** ${RESTART_COUNT}

## Environment
- **API Endpoint:** ${ED2KIA_API}
- **Data Directory:** ${ED2KIA_DATA_DIR}
- **Checkpoint Directory:** ${ED2KIA_CHECKPOINT_DIR}

## Timeline
| Time | Event |
|------|-------|
| ${INCIDENT_START} | Incident detected |
EOF

    # Add recent logs if available.
    if [[ -d "${ED2KIA_LOG_DIR}" ]]; then
        local latest_log
        latest_log=$(ls -t "${ED2KIA_LOG_DIR}"/*.log 2>/dev/null | head -1 || echo "")
        if [[ -n "${latest_log}" ]]; then
            echo "" >> "${report_file}"
            echo "## Recent Logs (last 50 lines)" >> "${report_file}"
            echo '```' >> "${report_file}"
            tail -50 "${latest_log}" >> "${report_file}" 2>/dev/null || true
            echo '```' >> "${report_file}"
        fi
    fi

    log_info "Incident report saved: ${report_file}"
    echo "${report_file}"
}

# Send webhook notification.
send_webhook() {
    local message="$1"

    if [[ -z "${ED2KIA_WEBHOOK_URL}" ]]; then
        log_debug "No webhook URL configured. Skipping notification."
        return 0
    fi

    log_info "Sending webhook notification..."

    local payload
    payload=$(cat <<EOF
{
    "incident_id": "${INCIDENT_ID}",
    "message": "${message}",
    "timestamp": "$(date -u '+%Y-%m-%dT%H:%M:%SZ')",
    "restart_count": ${RESTART_COUNT},
    "api_endpoint": "${ED2KIA_API}"
}
EOF
)

    curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "${payload}" \
        "${ED2KIA_WEBHOOK_URL}" 2>/dev/null || {
        log_warn "Failed to send webhook notification"
        return 1
    }

    log_info "Webhook notification sent"
    return 0
}

# ---------------------------------------------------------------------------
# Main Monitoring Loop
# ---------------------------------------------------------------------------

run_monitoring_loop() {
    log_info "Starting monitoring loop"
    log_info "Health interval: ${ED2KIA_HEALTH_INTERVAL}s"
    log_info "Metrics interval: ${ED2KIA_METRICS_INTERVAL}s"

    local health_counter=0
    local metrics_counter=0

    while true; do
        health_counter=$((health_counter + 1))
        metrics_counter=$((metrics_counter + 1))

        # Health check every interval.
        if ! check_health; then
            log_error "Health check failed!"
            generate_incident_id

            # Send notification.
            send_webhook "Health check failed. Initiating remediation."

            # Attempt graceful restart.
            if graceful_restart; then
                log_info "Restart successful. Monitoring..."
                sleep 10

                # Verify recovery.
                if check_health; then
                    log_info "Service recovered after restart"
                    generate_incident_report
                    RESTART_COUNT=0
                else
                    log_error "Service still down after restart. Attempting rollback..."
                    rollback_to_checkpoint
                    generate_incident_report
                fi
            else
                log_error "Remediation failed. Generating report..."
                generate_incident_report
            fi
        fi

        # Metrics check less frequently.
        if [[ $((metrics_counter % 3)) -eq 0 ]]; then
            check_metrics || true
        fi

        # Consensus check.
        if [[ $((health_counter % 6)) -eq 0 ]]; then
            check_consensus || {
                log_warn "Consensus check failed"
                send_webhook "Consensus check failed"
            }
        fi

        # Slashing detection.
        if [[ $((health_counter % 10)) -eq 0 ]]; then
            detect_slashing || {
                log_error "Slashing detected!"
                send_webhook "Slashing event detected"
            }
        fi

        # Partition detection.
        if [[ $((health_counter % 5)) -eq 0 ]]; then
            detect_partition || {
                log_error "Network partition detected!"
                send_webhook "Network partition detected"
            }
        fi

        sleep "${ED2KIA_HEALTH_INTERVAL}"
    done
}

# ---------------------------------------------------------------------------
# CLI Entry Point
# ---------------------------------------------------------------------------

usage() {
    cat <<EOF
ed2kIA Auto-Remediation Script v2.1.0-sprint15

Usage: $(basename "$0") [OPTIONS]

Options:
  --monitor          Start continuous monitoring loop
  --rollback         Rollback to latest checkpoint
  --report           Generate incident report
  --webhook URL      Set webhook notification URL
  --health           Run single health check
  --help             Show this help message

Environment Variables:
  ED2KIA_API              API endpoint (default: http://localhost:3000)
  ED2KIA_DATA_DIR         Data directory (default: ./data)
  ED2KIA_CHECKPOINT_DIR   Checkpoint directory (default: ./data/checkpoints)
  ED2KIA_LOG_DIR          Log directory (default: ./logs)
  ED2KIA_REPORT_DIR       Report directory (default: ./reports)
  ED2KIA_WEBHOOK_URL      Webhook URL for notifications
  ED2KIA_MAX_RESTARTS     Maximum restart attempts (default: 3)
  ED2KIA_HEALTH_INTERVAL  Health check interval in seconds (default: 10)

Examples:
  $(basename "$0") --monitor
  $(basename "$0") --rollback
  $(basename "$0") --health
  ED2KIA_WEBHOOK_URL=https://hooks.slack.com/xxx $(basename "$0") --monitor
EOF
}

main() {
    local mode=""
    local webhook_url=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --monitor)
                mode="monitor"
                shift
                ;;
            --rollback)
                mode="rollback"
                shift
                ;;
            --report)
                mode="report"
                shift
                ;;
            --health)
                mode="health"
                shift
                ;;
            --webhook)
                webhook_url="$2"
                ED2KIA_WEBHOOK_URL="$2"
                shift 2
                ;;
            --help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    if [[ -z "${mode}" ]]; then
        usage
        exit 1
    fi

    log_info "ed2kIA Auto-Remediation v2.1.0-sprint15"
    log_info "Mode: ${mode}"
    log_info "API: ${ED2KIA_API}"

    case "${mode}" in
        monitor)
            generate_incident_id
            run_monitoring_loop
            ;;
        rollback)
            generate_incident_id
            rollback_to_checkpoint
            generate_incident_report
            send_webhook "Rollback completed"
            ;;
        report)
            generate_incident_id
            generate_incident_report
            ;;
        health)
            if check_health; then
                log_info "✓ Health check passed"
                exit 0
            else
                log_error "✗ Health check failed"
                exit 1
            fi
            ;;
    esac
}

# Run main with all arguments.
main "$@"
