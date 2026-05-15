#!/bin/sh
# update_weekly_metrics.sh — POSIX shell script for automated weekly metrics update
#
# Extracts data from CI, GitHub API, and local logs to update the weekly standup.
# Usage: ./scripts/update_weekly_metrics.sh [week_number]
#
# Requires: curl, jq (optional), git
# Compatible: GitHub Actions CI, Linux/macOS shells

set -e

WEEK=${1:-1}
STANDUP_FILE="docs/operations/weekly-standup-week${WEEK}.md"
DASHBOARD_FILE="docs/operations/daily-metrics-dashboard.md"
REPO_OWNER="Stuartemk"
REPO_NAME="ed2kIA"
TODAY=$(date -u +%Y-%m-%d 2>/dev/null || echo "2026-05-14")
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo "2026-05-15T00:00:00Z")

echo "============================================"
echo "ed2kIA Weekly Metrics Update — Week ${WEEK}"
echo "Fecha: ${TODAY}"
echo "============================================"

# ─── Helper Functions ───

check_cmd() {
    if command -v "$1" >/dev/null 2>&1; then
        return 0
    else
        echo "WARN: $1 not found (optional)"
        return 1
    fi
}

# ─── Check 1: Standup File Exists ───

echo ""
echo "[1/8] Checking standup file..."
if [ -f "${STANDUP_FILE}" ]; then
    echo "  OK: ${STANDUP_FILE} exists"
else
    echo "  FAIL: ${STANDUP_FILE} not found"
    exit 1
fi

# ─── Check 2: Git Status ───

echo ""
echo "[2/8] Git status..."
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
    LAST_COMMIT=$(git log -1 --format="%h %s" 2>/dev/null || echo "no commits")
    COMMIT_COUNT=$(git log --oneline --since="7 days ago" 2>/dev/null | wc -l | tr -d ' ')
    echo "  Branch: ${CURRENT_BRANCH}"
    echo "  Last commit: ${LAST_COMMIT}"
    echo "  Commits (7d): ${COMMIT_COUNT}"
else
    echo "  WARN: Not a git repository"
    CURRENT_BRANCH="unknown"
    LAST_COMMIT="N/A"
    COMMIT_COUNT="0"
fi

# ─── Check 3: GitHub API — Stars & Forks ───

echo ""
echo "[3/8] GitHub metrics..."
STARS="0"
FORKS="0"
OPEN_ISSUES="0"

if check_cmd curl; then
    GITHUB_DATA=$(curl -s "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}" 2>/dev/null || echo "{}")
    STARS=$(echo "${GITHUB_DATA}" | grep -o '"stargazers_count":[0-9]*' | cut -d: -f2 || echo "0")
    FORKS=$(echo "${GITHUB_DATA}" | grep -o '"forks_count":[0-9]*' | cut -d: -f2 || echo "0")
    if [ -z "${STARS}" ]; then STARS=0; fi
    if [ -z "${FORKS}" ]; then FORKS=0; fi
    echo "  Stars: ${STARS}"
    echo "  Forks: ${FORKS}"
else
    echo "  SKIP: curl not available"
fi

# ─── Check 4: Open Issues ───

echo ""
echo "[4/8] Open issues..."
if check_cmd curl; then
    OPEN_ISSUES=$(curl -s "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/issues?state=open&per_page=1" 2>/dev/null | grep -o '"total_count":[0-9]*' | cut -d: -f2 || echo "0")
    if [ -z "${OPEN_ISSUES}" ]; then OPEN_ISSUES=0; fi
    echo "  Open issues: ${OPEN_ISSUES}"
else
    echo "  SKIP: curl not available"
fi

# ─── Check 5: Cargo Check ───

echo ""
echo "[5/8] Cargo check..."
if command -v cargo >/dev/null 2>&1; then
    CARGO_RESULT=$(cargo check --features stable 2>&1 | tail -1 || echo "FAILED")
    if echo "${CARGO_RESULT}" | grep -q "Finished"; then
        echo "  OK: cargo check passed"
        CARGO_STATUS="PASS"
    else
        echo "  WARN: ${CARGO_RESULT}"
        CARGO_STATUS="WARN"
    fi
else
    echo "  SKIP: cargo not available"
    CARGO_STATUS="SKIP"
fi

# ─── Check 6: Test Count ───

echo ""
echo "[6/8] Test summary..."
if command -v cargo >/dev/null 2>&1; then
    TEST_OUTPUT=$(cargo test --lib --features stable 2>&1 | grep "test result" | tail -1 || echo "no results")
    PASSED=$(echo "${TEST_OUTPUT}" | grep -o '[0-9]* passed' | grep -o '[0-9]*' || echo "0")
    FAILED=$(echo "${TEST_OUTPUT}" | grep -o '[0-9]* failed' | grep -o '[0-9]*' || echo "0")
    echo "  Passed: ${PASSED}"
    echo "  Failed: ${FAILED}"
else
    echo "  SKIP: cargo not available"
    PASSED="0"
    FAILED="0"
fi

# ─── Check 7: File Counts ───

echo ""
echo "[7/8] Codebase stats..."
SRC_FILES=$(find src -name "*.rs" 2>/dev/null | wc -l | tr -d ' ')
TEST_FILES=$(find tests -name "*.rs" 2>/dev/null | wc -l | tr -d ' ')
DOC_FILES=$(find docs -name "*.md" 2>/dev/null | wc -l | tr -d ' ')
echo "  Source files: ${SRC_FILES}"
echo "  Test files: ${TEST_FILES}"
echo "  Doc files: ${DOC_FILES}"

# ─── Check 8: Update Standup Timestamp ───

echo ""
echo "[8/8] Updating standup timestamp..."
if [ -f "${STANDUP_FILE}" ]; then
    # Update the timestamp line if it exists
    if grep -q "Última actualización" "${STANDUP_FILE}"; then
        sed -i "s/Última actualización:.*/Última actualización: ${TIMESTAMP}/" "${STANDUP_FILE}" 2>/dev/null || \
        sed -i '' "s/Última actualización:.*/Última actualización: ${TIMESTAMP}/" "${STANDUP_FILE}" 2>/dev/null || \
        echo "  WARN: Could not update timestamp (sed not compatible)"
    fi
    echo "  OK: Standup updated"
fi

# ─── Summary ───

echo ""
echo "============================================"
echo "SUMMARY — Week ${WEEK}"
echo "============================================"
echo "Stars: ${STARS} | Forks: ${FORKS} | Issues: ${OPEN_ISSUES}"
echo "Commits (7d): ${COMMIT_COUNT}"
echo "Tests: ${PASSED} passed, ${FAILED} failed"
echo "Cargo: ${CARGO_STATUS}"
echo "Source: ${SRC_FILES} | Tests: ${TEST_FILES} | Docs: ${DOC_FILES}"
echo "Updated: ${TIMESTAMP}"
echo "============================================"

# ─── Auto-Push if in git repo ───

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo ""
    echo "Auto-push: Committing metrics update..."
    git add -A
    git diff --cached --quiet 2>/dev/null || {
        git commit -m "docs(ops): update week ${WEEK} metrics (${TODAY})"
        echo "  Committed. Push to origin?"
        # Note: Push is manual to avoid rate limiting
        echo "  Run: git push origin main"
    }
fi

echo ""
echo "Done."
exit 0
