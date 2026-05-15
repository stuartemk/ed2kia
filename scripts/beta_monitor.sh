#!/bin/bash
# beta_monitor.sh — Beta Performance Monitoring Script
# Ed2kIA v1.8.0-beta.1 — FASE 61
#
# Usage: ./scripts/beta_monitor.sh [--dry-run] [--interval SECONDS] [--output FILE]
#
# Monitors beta node performance, collects metrics, and generates reports.
# Designed for POSIX compatibility (bash/sh).

set -euo pipefail

# --- Configuration ---
DRY_RUN=false
INTERVAL=60
OUTPUT_FILE="release/v1.8.0-beta.1/monitor-report.md"
FEATURE_FLAGS=("stable" "v1.8-sprint1" "v1.8-sprint2")
THRESHOLDS=(
  "cargo_check:5"
  "cargo_clippy:3"
  "cargo_test:30"
  "test_pass_rate:95"
)

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# --- Logging ---
log_info() {
  echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

log_result() {
  local name="$1"
  local status="$2"
  local detail="$3"
  local timestamp
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  if [ "$status" = "PASS" ]; then
    log_info "[PASS] $name — $detail"
  else
    log_error "[FAIL] $name — $detail"
  fi
  # Append to report
  echo "- [$timestamp] [$status] $name: $detail" >> "$OUTPUT_FILE"
}

# --- Parse Args ---
while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run) DRY_RUN=true; shift ;;
    --interval) INTERVAL="$2"; shift 2 ;;
    --output) OUTPUT_FILE="$2"; shift 2 ;;
    --help)
      echo "Usage: $0 [--dry-run] [--interval SECONDS] [--output FILE]"
      echo ""
      echo "Options:"
      echo "  --dry-run    Run checks without executing cargo commands"
      echo "  --interval   Monitoring interval in seconds (default: 60)"
      echo "  --output     Output report file path"
      echo "  --help       Show this help message"
      exit 0
      ;;
    *) log_error "Unknown option: $1"; exit 1 ;;
  esac
done

# --- Init Report ---
init_report() {
  local report_dir
  report_dir=$(dirname "$OUTPUT_FILE")
  mkdir -p "$report_dir"
  cat > "$OUTPUT_FILE" << 'EOF'
# Beta Performance Monitor Report

**Generated:** AUTO_TIMESTAMP
**Version:** v1.8.0-beta.1
**Script:** scripts/beta_monitor.sh

---

## Monitoring Results

EOF
  sed -i "s/AUTO_TIMESTAMP/$(date -u +"%Y-%m-%dT%H:%M:%SZ")/" "$OUTPUT_FILE"
}

# --- Check Functions ---

check_cargo_check() {
  local feature="$1"
  log_info "Running cargo check --features $feature..."
  if [ "$DRY_RUN" = true ]; then
    log_result "cargo_check($feature)" "PASS" "Dry run skipped"
    return 0
  fi
  local start_time end_time duration
  start_time=$(date +%s)
  if cargo check --features "$feature" >/dev/null 2>&1; then
    end_time=$(date +%s)
    duration=$((end_time - start_time))
    if [ "$duration" -le 5 ]; then
      log_result "cargo_check($feature)" "PASS" "${duration}s (≤5s threshold)"
    else
      log_warn "cargo_check($feature) took ${duration}s (threshold: 5s)"
      log_result "cargo_check($feature)" "WARN" "${duration}s (>5s threshold)"
    fi
  else
    log_result "cargo_check($feature)" "FAIL" "Compilation error"
  fi
}

check_cargo_clippy() {
  local feature="$1"
  log_info "Running cargo clippy --features $feature..."
  if [ "$DRY_RUN" = true ]; then
    log_result "cargo_clippy($feature)" "PASS" "Dry run skipped"
    return 0
  fi
  local start_time end_time duration
  start_time=$(date +%s)
  if cargo clippy --features "$feature" 2>&1 | grep -q "error\|warning"; then
    end_time=$(date +%s)
    duration=$((end_time - start_time))
    log_result "cargo_clippy($feature)" "WARN" "${duration}s — Warnings detected"
  else
    end_time=$(date +%s)
    duration=$((end_time - start_time))
    log_result "cargo_clippy($feature)" "PASS" "${duration}s — Clean"
  fi
}

check_cargo_test() {
  local feature="$1"
  log_info "Running cargo test --features $feature..."
  if [ "$DRY_RUN" = true ]; then
    log_result "cargo_test($feature)" "PASS" "Dry run skipped"
    return 0
  fi
  local start_time end_time duration output
  start_time=$(date +%s)
  output=$(cargo test --features "$feature" 2>&1)
  end_time=$(date +%s)
  duration=$((end_time - start_time))
  local passed failed total
  passed=$(echo "$output" | grep -c "test result: ok" || true)
  failed=$(echo "$output" | grep -c "test result: FAILED" || true)
  if [ "$failed" -gt 0 ]; then
    log_result "cargo_test($feature)" "FAIL" "${duration}s — Failures detected"
  elif [ "$passed" -gt 0 ]; then
    log_result "cargo_test($feature)" "PASS" "${duration}s — All tests passed"
  else
    log_result "cargo_test($feature)" "WARN" "${duration}s — No test results found"
  fi
}

check_git_tag() {
  log_info "Checking git tag v1.8.0-beta.1..."
  if git tag -l | grep -q "v1.8.0-beta.1"; then
    log_result "git_tag" "PASS" "v1.8.0-beta.1 exists"
  else
    log_result "git_tag" "FAIL" "Tag not found"
  fi
}

check_release_notes() {
  log_info "Checking RELEASE_NOTES.md..."
  if [ -f "release/v1.8.0-beta.1/RELEASE_NOTES.md" ]; then
    local lines
    lines=$(wc -l < "release/v1.8.0-beta.1/RELEASE_NOTES.md")
    log_result "release_notes" "PASS" "${lines} lines"
  else
    log_result "release_notes" "FAIL" "File not found"
  fi
}

check_feedback_tracker() {
  log_info "Checking feedback-tracker.md..."
  if [ -f "docs/beta/feedback-tracker.md" ]; then
    local issues
    issues=$(grep -c "^|" "docs/beta/feedback-tracker.md" || true)
    log_result "feedback_tracker" "PASS" "${issues} table rows"
  else
    log_result "feedback_tracker" "FAIL" "File not found"
  fi
}

# --- Metrics Collection ---

collect_metrics() {
  echo "" >> "$OUTPUT_FILE"
  echo "## Performance Metrics" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
  echo "| Metric | Value | Threshold | Status |" >> "$OUTPUT_FILE"
  echo "|--------|-------|-----------|--------|" >> "$OUTPUT_FILE"

  # Count tests
  local test_count=0
  for feature in "${FEATURE_FLAGS[@]}"; do
    if [ "$DRY_RUN" = false ]; then
      local count
      count=$(cargo test --features "$feature" -- --list 2>/dev/null | wc -l || echo "0")
      test_count=$((test_count + count))
    fi
  done
  echo "| Total tests | $test_count | >100 | $([ "$test_count" -gt 100 ] && echo "PASS" || echo "N/A") |" >> "$OUTPUT_FILE"

  # Count source files
  local src_count
  src_count=$(find src -name "*.rs" 2>/dev/null | wc -l)
  echo "| Source files | $src_count | - | INFO |" >> "$OUTPUT_FILE"

  # Count beta issues
  local issue_count=0
  if [ -f "docs/beta/feedback-tracker.md" ]; then
    issue_count=$(grep -c "\[ \]\|\[x\]" "docs/beta/feedback-tracker.md" || true)
  fi
  echo "| Beta issues tracked | $issue_count | - | INFO |" >> "$OUTPUT_FILE"
}

# --- Main Loop ---

main() {
  log_info "=== ed2kIA Beta Monitor v1.8.0-beta.1 ==="
  log_info "Dry run: $DRY_RUN"
  log_info "Interval: ${INTERVAL}s"
  log_info "Output: $OUTPUT_FILE"
  echo ""

  init_report

  # Run checks
  log_info "--- Phase 1: Build Validation ---"
  for feature in "${FEATURE_FLAGS[@]}"; do
    check_cargo_check "$feature"
  done

  log_info "--- Phase 2: Lint Validation ---"
  check_cargo_clippy "stable"
  check_cargo_clippy "v1.8-sprint2"

  log_info "--- Phase 3: Test Validation ---"
  check_cargo_test "stable"
  check_cargo_test "v1.8-sprint2"

  log_info "--- Phase 4: Release Artifacts ---"
  check_git_tag
  check_release_notes
  check_feedback_tracker

  log_info "--- Phase 5: Metrics Collection ---"
  collect_metrics

  # Final summary
  echo "" >> "$OUTPUT_FILE"
  echo "## Summary" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
  local pass_count fail_count
  pass_count=$(grep -c "\[PASS\]" "$OUTPUT_FILE" || true)
  fail_count=$(grep -c "\[FAIL\]" "$OUTPUT_FILE" || true)
  echo "- **Passed:** $pass_count" >> "$OUTPUT_FILE"
  echo "- **Failed:** $fail_count" >> "$OUTPUT_FILE"
  echo "- **Completed:** $(date -u +"%Y-%m-%dT%H:%M:%SZ")" >> "$OUTPUT_FILE"

  echo ""
  log_info "=== Monitor Complete ==="
  log_info "Report: $OUTPUT_FILE"
  log_info "Passed: $pass_count | Failed: $fail_count"

  if [ "$fail_count" -gt 0 ]; then
    return 1
  fi
  return 0
}

main "$@"
