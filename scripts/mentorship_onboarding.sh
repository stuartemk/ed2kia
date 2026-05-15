#!/bin/bash
# mentorship_onboarding.sh — ed2kIA Community Mentorship Automation
# Usage: bash scripts/mentorship_onboarding.sh [command]
# Commands: grants-status, grants-report, grants-update, mentorship-list, mentorship-assign, onboarding-check

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
GRANTS_TRACKER="$PROJECT_ROOT/docs/grants/follow-up-tracker.md"
SUBMISSION_TRACKER="$PROJECT_ROOT/docs/grants/submission-tracker.md"
CONTRIBUTING="$PROJECT_ROOT/CONTRIBUTING.md"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# ─── Grants Commands ───

cmd_grants_status() {
    log_info "Grant Follow-up Status — $(date +%Y-%m-%d)"
    echo "============================================"

    if [[ ! -f "$GRANTS_TRACKER" ]]; then
        log_error "follow-up-tracker.md not found at $GRANTS_TRACKER"
        exit 1
    fi

    # Extract grant names and statuses from tracker
    echo ""
    echo "Active Grants:"
    grep -E "^\| \*\*" "$GRANTS_TRACKER" | head -5 || true
    echo ""

    # Check for PLACEHOLDERs
    local placeholders
    placeholders=$(grep -c "PLACEHOLDER" "$GRANTS_TRACKER" 2>/dev/null || echo "0")
    if [[ "$placeholders" -gt 0 ]]; then
        log_warn "$placeholders PLACEHOLDER(s) remaining in follow-up tracker"
    else
        log_ok "All placeholders resolved"
    fi

    # Check submission tracker
    if [[ -f "$SUBMISSION_TRACKER" ]]; then
        local submission_placeholders
        submission_placeholders=$(grep -c "PLACEHOLDER" "$SUBMISSION_TRACKER" 2>/dev/null || echo "0")
        log_info "Submission tracker: $submission_placeholders PLACEHOLDER(s) remaining"
    fi

    echo ""
    log_info "Next actions:"
    echo "  1. Replace all PLACEHOLDER values with real data"
    echo "  2. Update submission dates after sending"
    echo "  3. Schedule follow-up reminders"
}

cmd_grants_report() {
    log_info "Generating Weekly Grant Follow-up Report..."
    local report_date
    report_date=$(date +%Y-%m-%d)

    echo ""
    echo "# Weekly Grant Follow-up Report — $report_date"
    echo ""
    echo "## Summary"
    echo "| Grant | Status | Next Action |"
    echo "|-------|--------|-------------|"

    if [[ -f "$GRANTS_TRACKER" ]]; then
        grep -E "^\| \*\*" "$GRANTS_TRACKER" | head -5 | while read -r line; do
            echo "$line"
        done
    fi

    echo ""
    echo "## Metrics"
    echo "| Target | Current | Progress |"
    echo "|--------|---------|----------|"
    echo "| \$165,000 | \$0 | 0% |"
    echo ""
    echo "## Action Items"
    echo "- [ ] Replace PLACEHOLDER values"
    echo "- [ ] Schedule follow-up reminders"
    echo "- [ ] Coordinate with community team"
    echo ""
    log_ok "Report generated to stdout"
}

cmd_grants_update() {
    log_info "Updating grant tracker dates..."
    local today
    today=$(date +%Y-%m-%d)

    if [[ -f "$GRANTS_TRACKER" ]]; then
        # Update last updated date
        sed -i.bak "s/Última actualización:.*/Última actualización: $today/" "$GRANTS_TRACKER"
        rm -f "${GRANTS_TRACKER}.bak"
        log_ok "Updated last_updated to $today"
    else
        log_error "follow-up-tracker.md not found"
        exit 1
    fi
}

# ─── Mentorship Commands ───

cmd_mentorship_list() {
    log_info "Mentorship Program — Active Mentors & Mentees"
    echo "============================================"
    echo ""
    echo "## Mentorship Tiers"
    echo ""
    echo "| Tier | Requirements | Active | Capacity |"
    echo "|------|-------------|--------|----------|"
    echo "| 🌱 Seed | First PR | Open | Unlimited |"
    echo "| 🌿 Sprout | 2+ merged PRs | Open | 10 |"
    echo "| 🌳 Tree | Module owner | Select | 5 |"
    echo ""
    echo "## Current Mentors"
    echo "| Mentor | Expertise | Active Mentees | Status |"
    echo "|--------|-----------|----------------|--------|"
    echo "| [TBD] | P2P/Mesh | 0 | Available |"
    echo "| [TBD] | SAE/ML | 0 | Available |"
    echo "| [TBD] | ZKP/Crypto | 0 | Available |"
    echo "| [TBD] | Governance | 0 | Available |"
    echo ""
    log_info "Mentorship program ready for onboarding"
}

cmd_mentorship_assign() {
    local mentee="${1:-}"
    local area="${2:-}"

    if [[ -z "$mentee" ]]; then
        log_error "Usage: $0 mentorship-assign <github_username> <area>"
        log_info "Areas: p2p, sae, zkp, governance, bridge, ui"
        exit 1
    fi

    log_info "Assigning mentor for @$mentee (area: $area)"
    echo ""
    echo "## Mentorship Assignment"
    echo "- **Mentee:** @$mentee"
    echo "- **Area:** $area"
    echo "- **Start Date:** $(date +%Y-%m-%d)"
    echo "- **Check-in Schedule:** Weekly"
    echo "- **Duration:** 4 weeks (extendable)"
    echo ""
    echo "## First Week Goals"
    echo "1. [ ] Complete onboarding checklist"
    echo "2. [ ] Set up local dev environment"
    echo "3. [ ] Identify first issue to work on"
    echo "4. [ ] Schedule first check-in with mentor"
    echo ""
    log_ok "Assignment template generated"
}

cmd_onboarding_check() {
    log_info "Onboarding Checklist Verification"
    echo "============================================"
    echo ""

    local checks=0
    local passed=0

    # Check 1: CONTRIBUTING.md exists
    checks=$((checks + 1))
    if [[ -f "$CONTRIBUTING" ]]; then
        log_ok "CONTRIBUTING.md exists"
        passed=$((passed + 1))
    else
        log_error "CONTRIBUTING.md missing"
    fi

    # Check 2: First contributor guide exists
    checks=$((checks + 1))
    if [[ -f "$PROJECT_ROOT/docs/community/first-contributor-guide.md" ]]; then
        log_ok "first-contributor-guide.md exists"
        passed=$((passed + 1))
    else
        log_error "first-contributor-guide.md missing"
    fi

    # Check 3: CI pipeline exists
    checks=$((checks + 1))
    if [[ -f "$PROJECT_ROOT/.github/workflows/ci.yml" ]]; then
        log_ok "CI pipeline configured"
        passed=$((passed + 1))
    else
        log_error "CI pipeline missing"
    fi

    # Check 4: Good first issues exist
    checks=$((checks + 1))
    local mentorship_section
    mentorship_section=$(grep -c "Mentorship" "$CONTRIBUTING" 2>/dev/null || echo "0")
    if [[ "$mentorship_section" -gt 0 ]]; then
        log_ok "Mentorship section in CONTRIBUTING.md"
        passed=$((passed + 1))
    else
        log_warn "Mentorship section not found in CONTRIBUTING.md"
    fi

    # Check 5: Follow-up tracker exists
    checks=$((checks + 1))
    if [[ -f "$GRANTS_TRACKER" ]]; then
        log_ok "Grant follow-up tracker exists"
        passed=$((passed + 1))
    else
        log_error "Grant follow-up tracker missing"
    fi

    echo ""
    echo "Results: $passed/$checks checks passed"
    echo ""

    if [[ "$passed" -eq "$checks" ]]; then
        log_ok "Onboarding checklist complete!"
    else
        log_warn "Some checks failed. Review and fix before onboarding."
    fi
}

# ─── Main ───

main() {
    local command="${1:-help}"

    case "$command" in
        grants-status)
            cmd_grants_status
            ;;
        grants-report)
            cmd_grants_report
            ;;
        grants-update)
            cmd_grants_update
            ;;
        mentorship-list)
            cmd_mentorship_list
            ;;
        mentorship-assign)
            cmd_mentorship_assign "${2:-}" "${3:-}"
            ;;
        onboarding-check)
            cmd_onboarding_check
            ;;
        help|*)
            echo "ed2kIA Mentorship & Grants Automation"
            echo ""
            echo "Usage: $0 <command> [args]"
            echo ""
            echo "Commands:"
            echo "  grants-status       Show current grant follow-up status"
            echo "  grants-report       Generate weekly follow-up report"
            echo "  grants-update       Update tracker dates"
            echo "  mentorship-list     List active mentors and tiers"
            echo "  mentorship-assign   Assign mentor to new contributor"
            echo "  onboarding-check    Verify onboarding checklist"
            echo "  help                Show this help message"
            ;;
    esac
}

main "$@"
