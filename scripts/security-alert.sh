#!/bin/bash
# security-alert.sh — Parse security monitor report and generate alerts
# Usage: bash scripts/security-alert.sh [report_file]
# License: Apache 2.0 + Ethical Use Clause

set -euo pipefail

# Configuration
REPORT_FILE="${1:-docs/reports/security-monitor-weekly.md}"
SLACK_WEBHOOK="${SLACK_WEBHOOK_URL:-}"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"

# Colors for terminal output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

usage() {
    echo "Usage: $0 [report_file]"
    echo ""
    echo "Arguments:"
    echo "  report_file  Path to security-monitor-weekly.md (default: docs/reports/security-monitor-weekly.md)"
    echo ""
    echo "Environment Variables:"
    echo "  SLACK_WEBHOOK_URL  Slack incoming webhook URL (optional)"
    echo "  GITHUB_TOKEN       GitHub token for creating issues (optional)"
    exit 1
}

validate_report() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo -e "${RED}ERROR: Report file '$file' not found${NC}"
        exit 1
    fi
}

extract_cves() {
    local file="$1"
    # Extract CVE table entries (lines with | ID | pattern)
    grep -E "^\| RUSTSEC" "$file" 2>/dev/null || echo "No CVEs found in report"
}

extract_critical_high() {
    local file="$1"
    # Extract CRITICAL and HIGH severity entries
    grep -E "CRITICAL|HIGH" "$file" 2>/dev/null || echo "No CRITICAL/HIGH alerts"
}

generate_alert_message() {
    local file="$1"
    local cves
    cves=$(extract_critical_high "$file")

    local alert_count
    alert_count=$(echo "$cves" | grep -c "CRITICAL\|HIGH" 2>/dev/null || echo "0")

    echo "=========================================="
    echo "  Security Alert Summary"
    echo "=========================================="
    echo ""
    echo "Report: $file"
    echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "Critical/High Alerts: $alert_count"
    echo ""

    if [[ "$alert_count" -gt 0 ]]; then
        echo -e "${RED}⚠️  ALERTS DETECTED:${NC}"
        echo ""
        echo "$cves"
        echo ""
        echo -e "${YELLOW}ACTION REQUIRED:${NC}"
        echo "1. Review CVE details in: $file"
        echo "2. Update remediation plan: docs/reports/dependency-remediation-plan-Q1-2027.md"
        echo "3. Escalate to stewards if no mitigation available"
    else
        echo -e "${GREEN}✅ No critical/high alerts detected${NC}"
    fi
    echo ""
    echo "=========================================="
}

send_slack_alert() {
    local message="$1"
    if [[ -z "$SLACK_WEBHOOK" ]]; then
        echo "SLACK_WEBHOOK_URL not set, skipping Slack notification"
        return
    fi

    curl -s -X POST "$SLACK_WEBHOOK" \
        -H 'Content-type: application/json' \
        --data "{\"text\":\"${message}\"}" \
        || echo "Failed to send Slack alert"
}

create_github_issue() {
    local title="$1"
    local body="$2"
    if [[ -z "$GITHUB_TOKEN" ]]; then
        echo "GITHUB_TOKEN not set, skipping GitHub issue creation"
        return
    fi

    # Placeholder: GitHub issue creation via API
    echo "GitHub issue placeholder: $title"
    echo "Body: $body"
}

# Main
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    usage
fi

validate_report "$REPORT_FILE"
generate_alert_message "$REPORT_FILE"

# Send alerts if configured
ALERT_MSG="ed2kIA Security Monitor: Alerts detected in $(date -u +%Y-%m-%d) scan. Review: docs/reports/security-monitor-weekly.md"
send_slack_alert "$ALERT_MSG"
