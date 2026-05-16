#!/bin/bash
# process_feedback.sh — Triages and processes Early Access v2.0 feedback
# Usage: bash scripts/process_feedback.sh [--since WEEKS] [--module MODULE]

set -euo pipefail

SINCE_WEEKS="${1:-2}"
MODULE="${2:-all}"
OUTPUT_DIR="docs/feedback-reports"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
REPORT_FILE="${OUTPUT_DIR}/feedback_report_${TIMESTAMP}.md"

mkdir -p "${OUTPUT_DIR}"

echo "=== ed2kIA v2.0 Early Access Feedback Processor ==="
echo "Since: ${SINCE_WEEKS} weeks ago"
echo "Module: ${MODULE}"
echo "Output: ${REPORT_FILE}"
echo ""

# Count issues by label and module
echo "## Feedback Summary" > "${REPORT_FILE}"
echo "" >> "${REPORT_FILE}"
echo "> Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "${REPORT_FILE}"
echo "> Period: Last ${SINCE_WEEKS} weeks" >> "${REPORT_FILE}"
echo "> Module: ${MODULE}" >> "${REPORT_FILE}"
echo "" >> "${REPORT_FILE}"

# Bug reports
echo "### Bug Reports" >> "${REPORT_FILE}"
echo "" >> "${REPORT_FILE}"
echo "| Severity | Count | Status |" >> "${REPORT_FILE}"
echo "|----------|-------|--------|" >> "${REPORT_FILE}"

# Use GitHub CLI if available
if command -v gh &> /dev/null; then
    BUGS=$(gh issue list --label "early-access,bug" --state all --json title,state,labels --jq length 2>/dev/null || echo "0")
    FEEDBACK=$(gh issue list --label "early-access,feedback" --state all --json title,state,labels --jq length 2>/dev/null || echo "0")

    echo "| All | ${BUGS} | Tracked |" >> "${REPORT_FILE}"
    echo "" >> "${REPORT_FILE}"

    echo "### Feature Feedback" >> "${REPORT_FILE}"
    echo "" >> "${REPORT_FILE}"
    echo "| Type | Count | Status |" >> "${REPORT_FILE}"
    echo "|------|-------|--------|" >> "${REPORT_FILE}"
    echo "| All | ${FEEDBACK} | Tracked |" >> "${REPORT_FILE}"
    echo "" >> "${REPORT_FILE}"
else
    echo "⚠️  GitHub CLI (gh) not found. Install with: brew install gh / sudo apt install gh"
    echo "" > "${REPORT_FILE}"
    echo "GitHub CLI not available. Manual triage required." >> "${REPORT_FILE}"
fi

# Module breakdown
echo "### Module Breakdown" >> "${REPORT_FILE}"
echo "" >> "${REPORT_FILE}"
echo "| Module | Bugs | Feedback | Priority |" >> "${REPORT_FILE}"
echo "|--------|------|----------|----------|" >> "${REPORT_FILE}"

for mod in neural_steer_ui neural_tauri_bridge tauri_scaffold commitment_pool mobile_hardening k8s_manifests; do
    if [ "${MODULE}" = "all" ] || [ "${MODULE}" = "${mod}" ]; then
        echo "| ${mod} | — | — | Tracking |" >> "${REPORT_FILE}"
    fi
done

echo "" >> "${REPORT_FILE}"

# Action items
echo "### Action Items" >> "${REPORT_FILE}"
echo "" >> "${REPORT_FILE}"
echo "1. [ ] Review all Critical/High severity bugs" >> "${REPORT_FILE}"
echo "2. [ ] Triage feature feedback for sprint backlog" >> "${REPORT_FILE}"
echo "3. [ ] Update threat model if new security findings" >> "${REPORT_FILE}"
echo "4. [ ] Acknowledge all participant submissions" >> "${REPORT_FILE}"
echo "5. [ ] Update program metrics dashboard" >> "${REPORT_FILE}"
echo "" >> "${REPORT_FILE}"

echo "=== Report generated: ${REPORT_FILE} ==="
echo ""
echo "Next steps:"
echo "1. Review report: cat ${REPORT_FILE}"
echo "2. Triage issues: gh issue list --label early-access"
echo "3. Update metrics: docs/early_access_program_v2.0.md"
