#!/bin/sh
# =============================================================================
# ed2kIA Weekly Metrics Dashboard Generator
# =============================================================================
# POSIX-compliant metrics report generator
# Usage: ./scripts/metrics-dashboard.sh [OPTIONS]
#
# Generates a weekly metrics report from:
#   - Git commit history
#   - Test suite results
#   - Benchmark data (if available)
#   - GitHub API (stars, forks, issues — optional)
#   - Cargo audit status
#
# Output: Markdown report suitable for community transparency reports
#
# Options:
#   --output FILE     Output file (default: stdout)
#   --weeks N         Look back N weeks (default: 1)
#   --json            Output as JSON instead of Markdown
#   --github-token T  GitHub token for API access (optional)
#   --help            Show this help
#
# Environment:
#   ED2KIA_DIR        Project root (auto-detected if not set)
#   GITHUB_TOKEN      GitHub token (alternative to --github-token)
# =============================================================================

set -e

# ─── Color Codes ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ─── Defaults ────────────────────────────────────────────────────────────────
OUTPUT=""
WEEKS=1
FORMAT="markdown"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"
ED2KIA_DIR=""

# ─── Usage ───────────────────────────────────────────────────────────────────
usage() {
    cat <<'EOF'
ed2kIA Weekly Metrics Dashboard Generator

USAGE:
    ./scripts/metrics-dashboard.sh [OPTIONS]

OPTIONS:
    --output FILE     Output file (default: stdout)
    --weeks N         Look back N weeks (default: 1)
    --json            Output as JSON instead of Markdown
    --github-token T  GitHub token for API access (optional)
    --help            Show this help

ENVIRONMENT:
    ED2KIA_DIR        Project root directory
    GITHUB_TOKEN      GitHub API token

EXAMPLES:
    # Generate weekly report to stdout
    ./scripts/metrics-dashboard.sh

    # Generate 4-week report to file
    ./scripts/metrics-dashboard.sh --weeks 4 --output weekly-report.md

    # JSON output with GitHub metrics
    ./scripts/metrics-dashboard.sh --json --github-token $GITHUB_TOKEN
EOF
}

# ─── Parse Arguments ─────────────────────────────────────────────────────────
parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --output)
                OUTPUT="$2"
                shift 2
                ;;
            --weeks)
                WEEKS="$2"
                shift 2
                ;;
            --json)
                FORMAT="json"
                shift
                ;;
            --github-token)
                GITHUB_TOKEN="$2"
                shift 2
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                echo "Unknown option: $1" >&2
                usage
                exit 1
                ;;
        esac
    done
}

# ─── Find Project Root ──────────────────────────────────────────────────────
find_project_root() {
    if [ -n "$ED2KIA_DIR" ] && [ -d "$ED2KIA_DIR" ]; then
        return
    fi

    # Search upward from script location
    DIR="$(cd "$(dirname "$0")/.." 2>/dev/null || echo "." && pwd)"
    if [ -d "${DIR}/Cargo.toml" ]; then
        ED2KIA_DIR="$DIR"
        return
    fi

    # Current directory
    if [ -d "Cargo.toml" ]; then
        ED2KIA_DIR="$(pwd)"
        return
    fi

    echo "ERROR: Cannot find ed2kIA project root" >&2
    exit 1
}

# ─── Git Metrics ─────────────────────────────────────────────────────────────
get_git_metrics() {
    cd "${ED2KIA_DIR}"

    DATE_CUTOFF="$(date -d "${WEEKS} weeks ago" '+%Y-%m-%d' 2>/dev/null || date -v-${WEEKS}w '+%Y-%m-%d' 2>/dev/null || echo "")"

    # Commits
    if [ -n "$DATE_CUTOFF" ]; then
        COMMITS=$(git log --oneline --since "${DATE_CUTOFF}" 2>/dev/null | wc -l | tr -d ' ')
    else
        COMMITS=$(git log --oneline -10 2>/dev/null | wc -l | tr -d ' ')
    fi

    # Contributors
    if [ -n "$DATE_CUTOFF" ]; then
        CONTRIBUTORS=$(git log --since "${DATE_CUTOFF}" --format='%aN' 2>/dev/null | sort -u | wc -l | tr -d ' ')
    else
        CONTRIBUTORS=$(git log --format='%aN' -20 2>/dev/null | sort -u | wc -l | tr -d ' ')
    fi

    # Lines changed (approximate)
    if [ -n "$DATE_CUTOFF" ]; then
        CHANGES=$(git log --since "${DATE_CUTOFF}" --numstat --format='' 2>/dev/null | awk '{+=$1; +=$2} END {print $1+$2}' | tr -d ' ')
    else
        CHANGES="N/A"
    fi

    # Last commit
    LAST_COMMIT=$(git log -1 --format='%h (%s, %ar)' 2>/dev/null || echo "N/A")

    # Branch
    BRANCH=$(git branch --show-current 2>/dev/null || echo "N/A")

    # Tags
    LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "N/A")
}

# ─── Test Metrics ────────────────────────────────────────────────────────────
get_test_metrics() {
    cd "${ED2KIA_DIR}"

    # Run quick test count
    TEST_OUTPUT=$(cargo test --features "stable" --lib 2>&1 | tail -5)

    PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= passed)' | head -1 || echo "N/A")
    FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | head -1 || echo "0")
    IGNORED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= ignored)' | head -1 || echo "N/A")

    if [ -z "$PASSED" ]; then PASSED="N/A"; fi
    if [ -z "$FAILED" ]; then FAILED="0"; fi
    if [ -z "$IGNORED" ]; then IGNORED="N/A"; fi
}

# ─── Code Metrics ────────────────────────────────────────────────────────────
get_code_metrics() {
    cd "${ED2KIA_DIR}"

    # Rust lines of code
    RUST_LOC=$(find src -name '*.rs' -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')

    # Test lines of code
    TEST_LOC=$(find tests -name '*.rs' -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')

    # Total files
    RUST_FILES=$(find src -name '*.rs' 2>/dev/null | wc -l | tr -d ' ')
    TEST_FILES=$(find tests -name '*.rs' 2>/dev/null | wc -l | tr -d ' ')

    # Feature gates count
    FEATURE_GATES=$(grep -c 'feature.*=' Cargo.toml 2>/dev/null || echo "N/A")
}

# ─── GitHub Metrics (Optional) ───────────────────────────────────────────────
get_github_metrics() {
    if [ -z "$GITHUB_TOKEN" ]; then
        STARS="N/A"
        FORKS="N/A"
        ISSUES_OPEN="N/A"
        ISSUES_CLOSED="N/A"
        STARGAZERS_WEEK="N/A"
        return
    fi

    HEADER="Authorization: token ${GITHUB_TOKEN}"

    # Stars
    STARS=$(curl -s -H "$HEADER" "https://api.github.com/repos/ed2kia/ed2kIA" 2>/dev/null | grep '"stargazers_count"' | grep -oP '\d+' || echo "N/A")

    # Forks
    FORKS=$(curl -s -H "$HEADER" "https://api.github.com/repos/ed2kia/ed2kIA" 2>/dev/null | grep '"forks_count"' | grep -oP '\d+' || echo "N/A")

    # Open issues
    ISSUES_OPEN=$(curl -s -H "$HEADER" "https://api.github.com/repos/ed2kia/ed2kIA/issues?state=open&per_page=1" 2>/dev/null | grep -c '"number"' || echo "N/A")

    # Closed issues (total)
    ISSUES_CLOSED=$(curl -s -H "$HEADER" "https://api.github.com/repos/ed2kia/ed2kIA/issues?state=closed&per_page=1" 2>/dev/null | grep -c '"number"' || echo "N/A")
}

# ─── Security Metrics ────────────────────────────────────────────────────────
get_security_metrics() {
    cd "${ED2KIA_DIR}"

    # Cargo audit (if available)
    if command -v cargo-audit >/dev/null 2>&1; then
        AUDIT_OUTPUT=$(cargo audit 2>&1 || true)
        VULN_COUNT=$(echo "$AUDIT_OUTPUT" | grep -c 'warning: Vulnerability' || echo "0")
    else
        VULN_COUNT="N/A (cargo-audit not installed)"
    fi

    # Clippy warnings
    CLIPPY_OUTPUT=$(cargo clippy --features "stable" 2>&1 | grep -c 'warning:' || echo "0")

    # Format check
    if cargo fmt --all -- --check 2>/dev/null; then
        FMT_STATUS="PASS"
    else
        FMT_STATUS="WARN"
    fi
}

# ─── Generate Markdown Report ────────────────────────────────────────────────
generate_markdown() {
    REPORT_DATE=$(date -u +"%Y-%m-%d %H:%M UTC" 2>/dev/null || date)

    cat <<EOF
# ed2kIA Weekly Metrics Report

**Period:** Last ${WEEKS} week(s)  
**Generated:** ${REPORT_DATE}  
**Version:** ${LATEST_TAG}  
**Branch:** ${BRANCH}

---

## Commit Activity

| Metric | Value |
|--------|-------|
| Commits | ${COMMITS} |
| Contributors | ${CONTRIBUTORS} |
| Lines Changed | ${CHANGES} |
| Last Commit | ${LAST_COMMIT} |

## Codebase

| Metric | Value |
|--------|-------|
| Rust Source Files | ${RUST_FILES} |
| Rust Source Lines | ${RUST_LOC} |
| Test Files | ${TEST_FILES} |
| Test Lines | ${TEST_LOC} |
| Feature Gates | ${FEATURE_GATES} |

## Test Suite

| Metric | Value |
|--------|-------|
| Passed | ${PASSED} |
| Failed | ${FAILED} |
| Ignored | ${IGNORED} |

## Security

| Metric | Value |
|--------|-------|
| Known Vulnerabilities | ${VULN_COUNT} |
| Clippy Warnings | ${CLIPPY_OUTPUT} |
| Format Check | ${FMT_STATUS} |

## GitHub (Community)

| Metric | Value |
|--------|-------|
| Stars | ${STARS} |
| Forks | ${FORKS} |
| Open Issues | ${ISSUES_OPEN} |
| Closed Issues | ${ISSUES_CLOSED} |

---

## Health Score

| Category | Status |
|----------|--------|
| Tests | $([ "$FAILED" = "0" ] && echo "✅ PASS" || echo "❌ FAIL") |
| Format | $([ "$FMT_STATUS" = "PASS" ] && echo "✅ PASS" || echo "⚠️ WARN") |
| Security | $([ "$VULN_COUNT" = "0" ] && echo "✅ PASS" || echo "⚠️ REVIEW") |
| Commits | $([ "$COMMITS" -gt 0 ] 2>/dev/null && echo "✅ ACTIVE" || echo "⚠️ QUIET") |

---

*Report generated by \`scripts/metrics-dashboard.sh\`*  
*For questions, contact the Steward Council.*
EOF
}

# ─── Generate JSON Report ────────────────────────────────────────────────────
generate_json() {
    REPORT_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date)

    cat <<EOF
{
  "report": {
    "period": "last_${WEEKS}_weeks",
    "generated_at": "${REPORT_DATE}",
    "version": "${LATEST_TAG}",
    "branch": "${BRANCH}"
  },
  "git": {
    "commits": ${COMMITS:-0},
    "contributors": ${CONTRIBUTORS:-0},
    "lines_changed": "${CHANGES}",
    "last_commit": "${LAST_COMMIT}"
  },
  "codebase": {
    "rust_files": ${RUST_FILES:-0},
    "rust_lines": ${RUST_LOC:-0},
    "test_files": ${TEST_FILES:-0},
    "test_lines": ${TEST_LOC:-0},
    "feature_gates": ${FEATURE_GATES:-0}
  },
  "tests": {
    "passed": ${PASSED:-0},
    "failed": ${FAILED:-0},
    "ignored": ${IGNORED:-0}
  },
  "security": {
    "vulnerabilities": "${VULN_COUNT}",
    "clippy_warnings": ${CLIPPY_OUTPUT:-0},
    "format_check": "${FMT_STATUS}"
  },
  "github": {
    "stars": "${STARS}",
    "forks": "${FORKS}",
    "open_issues": "${ISSUES_OPEN}",
    "closed_issues": "${ISSUES_CLOSED}"
  }
}
EOF
}

# ─── Main ────────────────────────────────────────────────────────────────────
main() {
    parse_args "$@"
    find_project_root

    echo "[INFO] Collecting metrics..." >&2

    get_git_metrics
    get_test_metrics
    get_code_metrics
    get_github_metrics
    get_security_metrics

    echo "[INFO] Generating report..." >&2

    if [ "$FORMAT" = "json" ]; then
        REPORT=$(generate_json)
    else
        REPORT=$(generate_markdown)
    fi

    if [ -n "$OUTPUT" ]; then
        echo "$REPORT" > "$OUTPUT"
        echo "[OK] Report written to ${OUTPUT}" >&2
    else
        echo "$REPORT"
    fi
}

main "$@"
