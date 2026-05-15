#!/bin/bash
# auto_triage_prs.sh — Automated PR Triage Script for ed2kIA
# Usage: ./scripts/auto_triage_prs.sh [repo] [token]
#
# This script automates the initial PR triage process:
# 1. Fetches open PRs
# 2. Checks CI status
# 3. Applies labels based on file changes
# 4. Generates triage report

set -euo pipefail

REPO="${1:-Stuartemk/ed2kIA}"
TOKEN="${2:-$GITHUB_TOKEN}"
OUTPUT_DIR="./triage-reports"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
REPORT="${OUTPUT_DIR}/triage-${TIMESTAMP}.md"

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() { echo -e "${BLUE}[TRIAGE]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Validate prerequisites
check_prerequisites() {
    if ! command -v gh &> /dev/null; then
        error "GitHub CLI (gh) is not installed. Install from https://cli.github.com/"
        exit 1
    fi

    if [ -z "$TOKEN" ]; then
        warn "No GITHUB_TOKEN provided. Some features may be rate-limited."
    fi

    mkdir -p "$OUTPUT_DIR"
    log "Output directory: $OUTPUT_DIR"
}

# Fetch open PRs
fetch_prs() {
    log "Fetching open PRs from $REPO..."
    gh pr list --repo "$REPO" --state open --json \
        number,title,author,body,labels,files,statusCheckRollup,createdAt,updatedAt \
        --jq '.[] | {
            number: .number,
            title: .title,
            author: .author.login,
            files: [.files[].path],
            labels: [.labels[].name],
            createdAt: .createdAt
        }' > "${OUTPUT_DIR}/prs.json" 2>/dev/null || true

    local count
    count=$(jq 'length' "${OUTPUT_DIR}/prs.json" 2>/dev/null || echo "0")
    log "Found $count open PR(s)"
}

# Analyze PR and suggest labels
analyze_pr() {
    local pr_number="$1"
    local pr_title="$2"
    local pr_files="$3"
    local labels=""

    # Check for conventional commit format
    if echo "$pr_title" | grep -qE '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)\(.+\):'; then
        labels="${labels}conventional-commit "
    fi

    # Categorize by file changes
    local has_src=false has_docs=false has_tests=false has_ci=false has_security=false

    for file in $pr_files; do
        case "$file" in
            src/*) has_src=true ;;
            docs/*|*.md) has_docs=true ;;
            tests/*|*test*) has_tests=true ;;
            .github/*|*.yml|*.yaml) has_ci=true ;;
            security/*|*security*) has_security=true ;;
        esac
    done

    if $has_security; then
        labels="${labels}security "
    fi
    if $has_src && $has_tests; then
        labels="${labels}feature "
    elif $has_src; then
        labels="${labels}code-change "
    fi
    if $has_docs; then
        labels="${labels}documentation "
    fi
    if $has_ci; then
        labels="${labels}ci "
    fi

    echo "$labels"
}

# Generate triage report
generate_report() {
    log "Generating triage report..."

    cat > "$REPORT" << 'HEADER'
# PR Triage Report

**Generated:** TIMESTAMP_PLACEHOLDER
**Repository:** REPO_PLACEHOLDER

## Summary

| Metric | Value |
|--------|-------|
HEADER

    # Count PRs by category
    local total=0 docs=0 code=0 ci=0 needs_attention=0

    while IFS= read -r line; do
        local number title files labels
        number=$(echo "$line" | jq -r '.number')
        title=$(echo "$line" | jq -r '.title')
        files=$(echo "$line" | jq -r '.files | join(" ")')

        total=$((total + 1))

        local suggested
        suggested=$(analyze_pr "$number" "$title" "$files")

        case "$suggested" in
            *documentation*) docs=$((docs + 1)) ;;
            *code-change*|*feature*) code=$((code + 1)) ;;
            *ci*) ci=$((ci + 1)) ;;
        esac

        # Check if PR needs attention (no labels, old, etc.)
        local age_days
        age_days=$(( ( $(date +%s) - $(date -d "$(echo "$line" | jq -r '.createdAt')" +%s 2>/dev/null || echo "$(date +%s)") ) / 86400 ))

        if [ "$age_days" -gt 7 ]; then
            needs_attention=$((needs_attention + 1))
        fi

        cat >> "$REPORT" << PR_ENTRY

### PR #${number}: ${title}

- **Author:** $(echo "$line" | jq -r '.author')
- **Files:** $(echo "$line" | jq -r '.files | length') changed
- **Suggested Labels:** ${suggested:-none}
- **Age:** ${age_days} days
- **Status:** $( [ "$age_days" -gt 7 ] && echo "⚠️ Needs Attention" || echo "✅ Recent" )

---
PR_ENTRY

    done < <(jq -c '.[]' "${OUTPUT_DIR}/prs.json" 2>/dev/null)

    # Update summary
    sed -i "s/TIMESTAMP_PLACEHOLDER/$(date -Iseconds)/" "$REPORT"
    sed -i "s/REPO_PLACEHOLDER/$REPO/" "$REPORT"

    cat >> "$REPORT" << SUMMARY

## Statistics

| Category | Count |
|----------|-------|
| Total Open PRs | $total |
| Documentation | $docs |
| Code Changes | $code |
| CI/Build | $ci |
| Needs Attention (>7 days) | $needs_attention |

## Actions Required

$([ $needs_attention -gt 0 ] && echo "- [ ] Review $needs_attention PR(s) older than 7 days" || echo "- [x] All PRs are recent")
- [ ] Apply suggested labels
- [ ] Assign reviewers based on file ownership
- [ ] Update PR descriptions if incomplete

SUMMARY

    success "Report saved to: $REPORT"
}

# Main execution
main() {
    log "ed2kIA PR Triage Tool v1.0"
    log "========================="

    check_prerequisites
    fetch_prs

    local count
    count=$(jq 'length' "${OUTPUT_DIR}/prs.json" 2>/dev/null || echo "0")

    if [ "$count" -eq 0 ]; then
        warn "No open PRs found. Nothing to triage."
        return 0
    fi

    generate_report
    log "Triage complete!"
}

main "$@"
