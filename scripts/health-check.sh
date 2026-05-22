#!/usr/bin/env bash
# =============================================================================
# ed2kIA Health Check Script — v2.1.0-stable
# =============================================================================
# POSIX-compliant, idempotent health check for ed2kIA production nodes.
# Usage: ./scripts/health-check.sh [--port PORT] [--host HOST] [--timeout SECS]
# =============================================================================
set -euo pipefail

# --- Configuration ---
PORT="${ED2KIA_PORT:-9000}"
HOST="${ED2KIA_HOST:-localhost}"
TIMEOUT="${ED2KIA_TIMEOUT:-5}"
EXIT_CODE=0

# --- Color Output ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# --- Helper Functions ---
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; EXIT_CODE=1; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_info() { echo -e "[INFO] $1"; }

# --- Pre-flight Checks ---
log_info "ed2kIA Health Check — v2.1.0-stable"
log_info "Target: ${HOST}:${PORT} (timeout: ${TIMEOUT}s)"
echo ""

# --- Check 1: Process Running ---
if command -v pgrep >/dev/null 2>&1; then
    if pgrep -f "ed2kia" >/dev/null 2>&1; then
        log_pass "ed2kIA process is running"
    else
        log_fail "ed2kIA process is NOT running"
    fi
else
    log_info "pgrep not available, skipping process check"
fi

# --- Check 2: Port Listening ---
if command -v ss >/dev/null 2>&1; then
    if ss -tlnp | grep -q ":${PORT} " 2>/dev/null; then
        log_pass "Port ${PORT} is listening"
    else
        log_fail "Port ${PORT} is NOT listening"
    fi
elif command -v netstat >/dev/null 2>&1; then
    if netstat -tlnp | grep -q ":${PORT} " 2>/dev/null; then
        log_pass "Port ${PORT} is listening"
    else
        log_fail "Port ${PORT} is NOT listening"
    fi
else
    log_warn "Neither ss nor netstat available, skipping port check"
fi

# --- Check 3: HTTP Health Endpoint ---
if command -v curl >/dev/null 2>&1; then
    HTTP_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" \
        --connect-timeout "${TIMEOUT}" \
        --max-time "${TIMEOUT}" \
        "http://${HOST}:${PORT}/health" 2>/dev/null || echo "000")

    if [ "$HTTP_RESPONSE" = "200" ]; then
        log_pass "HTTP /health endpoint returns 200"
    elif [ "$HTTP_RESPONSE" = "000" ]; then
        log_fail "HTTP /health endpoint unreachable"
    else
        log_warn "HTTP /health endpoint returns ${HTTP_RESPONSE}"
    fi
else
    log_warn "curl not available, skipping HTTP health check"
fi

# --- Check 4: Disk Space ---
if command -v df >/dev/null 2>&1; then
    DATA_DIR="${ED2KIA_DATA_DIR:-/data/ed2kia}"
    if [ -d "$DATA_DIR" ]; then
        USAGE=$(df "$DATA_DIR" | tail -1 | awk '{print $5}' | tr -d '%')
        if [ "$USAGE" -lt 90 ]; then
            log_pass "Disk usage: ${USAGE}% (< 90%)"
        else
            log_fail "Disk usage: ${USAGE}% (>= 90%)"
        fi
    else
        log_info "Data directory ${DATA_DIR} not found, skipping disk check"
    fi
fi

# --- Check 5: Memory Usage ---
if command -v free >/dev/null 2>&1; then
    MEM_TOTAL=$(free | grep Mem | awk '{print $2}')
    MEM_USED=$(free | grep Mem | awk '{print $3}')
    MEM_PCT=$((MEM_USED * 100 / MEM_TOTAL))
    if [ "$MEM_PCT" -lt 90 ]; then
        log_pass "Memory usage: ${MEM_PCT}% (< 90%)"
    else
        log_fail "Memory usage: ${MEM_PCT}% (>= 90%)"
    fi
fi

# --- Check 6: Log File Accessibility ---
LOG_DIR="${ED2KIA_LOG_DIR:-/data/ed2kia/logs}"
if [ -d "$LOG_DIR" ]; then
    if [ -w "$LOG_DIR" ]; then
        log_pass "Log directory ${LOG_DIR} is writable"
    else
        log_fail "Log directory ${LOG_DIR} is NOT writable"
    fi

    # Check for recent log activity (last 5 minutes)
    if find "$LOG_DIR" -name "*.log" -mmin -5 >/dev/null 2>&1; then
        log_pass "Recent log activity detected (last 5 min)"
    else
        log_warn "No recent log activity (last 5 min)"
    fi
else
    log_info "Log directory ${LOG_DIR} not found, skipping log check"
fi

# --- Check 7: Data Directory Permissions ---
DATA_DIR="${ED2KIA_DATA_DIR:-/data/ed2kia}"
if [ -d "$DATA_DIR" ]; then
    OWNER=$(stat -c '%U' "$DATA_DIR" 2>/dev/null || stat -f '%Su' "$DATA_DIR" 2>/dev/null || echo "unknown")
    if [ "$OWNER" = "ed2kia" ] || [ "$OWNER" = "$(whoami)" ]; then
        log_pass "Data directory owned by ${OWNER}"
    else
        log_warn "Data directory owned by ${OWNER} (expected ed2kia or current user)"
    fi
fi

# --- Summary ---
echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  Health Check: ALL CHECKS PASSED${NC}"
    echo -e "${GREEN}=========================================${NC}"
else
    echo -e "${RED}=========================================${NC}"
    echo -e "${RED}  Health Check: SOME CHECKS FAILED${NC}"
    echo -e "${RED}=========================================${NC}"
fi

exit $EXIT_CODE
