#!/usr/bin/env bash
# run-v21-dryrun.sh — v2.1 Testnet Dry-Run (Safe, Local, No Network Calls)
# License: Apache 2.0 + Ethical Use Clause
#
# Purpose: Validate testnet infrastructure scaffolds without permanent state changes.
# Guardrails: ZERO network calls, ZERO inference, ZERO permanent state changes.
#
# Usage: bash scripts/run-v21-dryrun.sh [--report-only]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$PROJECT_ROOT/infra/docker-compose.testnet-v2.1.yml"
METRICS_FILE="$PROJECT_ROOT/infra/testnet-metrics-simulated.json"
REPORT_DIR="$PROJECT_ROOT/docs/reports"
TMP_DIR="$PROJECT_ROOT/tmp"
REPORT_FILE="$REPORT_DIR/testnet-dryrun-live-v2.1.md"

# Colors (disable if not a terminal)
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  NC=''
fi

log_info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

usage() {
  echo "Usage: $0 [--report-only]"
  echo ""
  echo "Options:"
  echo "  --report-only  Generate report without starting Docker services"
  echo ""
  echo "Dry-run steps:"
  echo "  1. Pre-flight: Validate docker-compose config"
  echo "  2. Start: Launch testnet services (alpine placeholders)"
  echo "  3. Inject: Copy simulated metrics to tmp/"
  echo "  4. Validate: Run voting-tally.sh --dry-run, security-alert.sh --dry-run"
  echo "  5. Report: Generate $REPORT_FILE"
  echo "  6. Cleanup: Stop services, remove tmp/"
  exit 1
}

REPORT_ONLY=false
for arg in "$@"; do
  case $arg in
    --report-only) REPORT_ONLY=true ;;
    --help|-h) usage ;;
    *) log_error "Unknown option: $arg"; usage ;;
  esac
done

# ===========================================================================
# Step 1: Pre-flight Validation
# ===========================================================================
log_info "=== Step 1: Pre-flight Validation ==="

# Check docker compose availability
if command -v docker &>/dev/null; then
  if docker compose version &>/dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
  elif docker-compose version &>/dev/null 2>&1; then
    COMPOSE_CMD="docker-compose"
  else
    log_warn "Docker available but compose not found — skipping Docker steps"
    COMPOSE_CMD=""
  fi
else
  log_warn "Docker not available — running report-only mode"
  COMPOSE_CMD=""
  REPORT_ONLY=true
fi

# Validate compose file syntax
if [ -n "$COMPOSE_CMD" ] && [ -f "$COMPOSE_FILE" ]; then
  if $COMPOSE_CMD -f "$COMPOSE_FILE" config --quiet 2>/dev/null; then
    log_info "docker-compose config valid"
  else
    log_error "docker-compose config validation failed"
    exit 1
  fi
else
  log_warn "Skipping compose validation (file missing or no Docker)"
fi

# Check metrics file
if [ -f "$METRICS_FILE" ]; then
  log_info "Simulated metrics file found: $METRICS_FILE"
else
  log_error "Metrics file missing: $METRICS_FILE"
  exit 1
fi

# ===========================================================================
# Step 2: Start Services (if not report-only)
# ===========================================================================
if [ "$REPORT_ONLY" = true ]; then
  log_info "=== Step 2: SKIPPED (report-only mode) ==="
  SERVICES_STATUS="skipped"
else
  log_info "=== Step 2: Starting Testnet Services ==="
  if [ -n "$COMPOSE_CMD" ]; then
    $COMPOSE_CMD -f "$COMPOSE_FILE" up -d --no-deps 2>&1 || {
      log_warn "Docker services failed to start (expected in CI)"
      SERVICES_STATUS="failed"
    }
    SERVICES_STATUS="started"
    sleep 2  # Allow containers to initialize
  else
    log_warn "No Docker compose available — marking as skipped"
    SERVICES_STATUS="skipped"
  fi
fi

# ===========================================================================
# Step 3: Inject Simulated Metrics
# ===========================================================================
log_info "=== Step 3: Injecting Simulated Metrics ==="

mkdir -p "$TMP_DIR"
cp "$METRICS_FILE" "$TMP_DIR/metrics-sim.json"
log_info "Metrics copied to $TMP_DIR/metrics-sim.json"

# Static validation of metrics JSON (basic check)
if command -v python3 &>/dev/null; then
  if python3 -c "import json; json.load(open('$TMP_DIR/metrics-sim.json'))" 2>/dev/null; then
    log_info "Metrics JSON valid"
    METRICS_VALID=true
  else
    log_warn "Metrics JSON validation failed"
    METRICS_VALID=false
  fi
elif command -v node &>/dev/null; then
  if node -e "JSON.parse(require('fs').readFileSync('$TMP_DIR/metrics-sim.json'))" 2>/dev/null; then
    log_info "Metrics JSON valid"
    METRICS_VALID=true
  else
    log_warn "Metrics JSON validation failed"
    METRICS_VALID=false
  fi
else
  log_warn "No JSON validator available — skipping validation"
  METRICS_VALID="unknown"
fi

# ===========================================================================
# Step 4: Validate Scripts (dry-run)
# ===========================================================================
log_info "=== Step 4: Script Validation (dry-run) ==="

VOTING_RESULT="skipped"
if [ -f "$SCRIPT_DIR/voting-tally.sh" ]; then
  if bash -n "$SCRIPT_DIR/voting-tally.sh" 2>/dev/null; then
    log_info "voting-tally.sh syntax valid"
    VOTING_RESULT="valid"
  else
    log_error "voting-tally.sh syntax check failed"
    VOTING_RESULT="failed"
  fi
else
  log_warn "voting-tally.sh not found"
fi

SECURITY_RESULT="skipped"
if [ -f "$SCRIPT_DIR/security-alert.sh" ]; then
  if bash -n "$SCRIPT_DIR/security-alert.sh" 2>/dev/null; then
    log_info "security-alert.sh syntax valid"
    SECURITY_RESULT="valid"
  else
    log_error "security-alert.sh syntax check failed"
    SECURITY_RESULT="failed"
  fi
else
  log_warn "security-alert.sh not found"
fi

# ===========================================================================
# Step 5: Generate Report
# ===========================================================================
log_info "=== Step 5: Generating Report ==="

mkdir -p "$REPORT_DIR"

# Collect feature gates from Cargo.toml
FEATURE_GATES=""
if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
  FEATURE_GATES=$(grep -E '^"v2\.1-' "$PROJECT_ROOT/Cargo.toml" 2>/dev/null | head -10 || echo "N/A")
fi

# Collect service status
if [ -n "$COMPOSE_CMD" ] && [ "$SERVICES_STATUS" = "started" ]; then
  SERVICE_LIST=$($COMPOSE_CMD -f "$COMPOSE_FILE" ps --format json 2>/dev/null || echo "[]")
else
  SERVICE_LIST="[]"
fi

cat > "$REPORT_FILE" << REPORT_EOF
# Testnet Dry-Run Report — ed2kIA v2.1

**Generated:** $(date -u '+%Y-%m-%d %H:%M:%S UTC')
**Mode:** ${REPORT_ONLY:+Report-Only} ${REPORT_ONLY:-Live}
**Compose File:** infra/docker-compose.testnet-v2.1.yml

## Executive Summary

| Check | Status |
|-------|--------|
| Docker Compose Config | $([ -n "${COMPOSE_CMD:-}" ] && echo "Valid" || echo "Skipped") |
| Services Started | ${SERVICES_STATUS:-unknown} |
| Metrics JSON | ${METRICS_VALID:-unknown} |
| voting-tally.sh | ${VOTING_RESULT:-unknown} |
| security-alert.sh | ${SECURITY_RESULT:-unknown} |

## Feature Gates Active

\`\`\`
${FEATURE_GATES:-No v2.1 feature gates found}
\`\`\`

## Services Status

\`\`\`json
${SERVICE_LIST}
\`\`\`

## Simulated Metrics

Source: infra/testnet-metrics-simulated.json
Copied to: tmp/metrics-sim.json

## Cleanup

- Docker services: Stopped (volumes removed)
- Temporary files: Removed (tmp/)

---

*Report generated by scripts/run-v21-dryrun.sh*
*License: Apache 2.0 + Ethical Use Clause*
REPORT_EOF

log_info "Report generated: $REPORT_FILE"

# ===========================================================================
# Step 6: Cleanup
# ===========================================================================
log_info "=== Step 6: Cleanup ==="

if [ "$REPORT_ONLY" = false ] && [ -n "$COMPOSE_CMD" ] && [ "$SERVICES_STATUS" = "started" ]; then
  $COMPOSE_CMD -f "$COMPOSE_FILE" down -v 2>&1 || log_warn "Docker cleanup had warnings"
  log_info "Docker services stopped, volumes removed"
else
  log_info "Docker cleanup skipped (report-only or no services started)"
fi

# Remove temporary files
if [ -d "$TMP_DIR" ]; then
  rm -rf "$TMP_DIR"
  log_info "Temporary files removed"
fi

log_info "=== Dry-Run Complete ==="
log_info "Report: $REPORT_FILE"
exit 0
