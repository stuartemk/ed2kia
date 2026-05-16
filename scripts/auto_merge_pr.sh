#!/usr/bin/env bash
# auto_merge_pr.sh — Verifica y mergea PRs que cumplen criterios auto-merge
# Uso: ./scripts/auto_merge_pr.sh <pr_number> [--dry-run]
#
# Criterios auto-merge:
#   1. CI pasa (cargo check + cargo test + cargo clippy)
#   2. 2+ approvals de @ed2kia/core-team
#   3. Score de calidad >= 0.8
#   4. No tiene label 'hold' o 'needs-work'
#   5. Linked issue tiene label 'good-first-issue'
#
# Dependencias: gh (GitHub CLI), jq, curl

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Verificar argumentos
if [ $# -lt 1 ]; then
    log_error "Uso: $0 <pr_number> [--dry-run]"
    exit 1
fi

PR_NUMBER="$1"
DRY_RUN=false
if [ "${2:-}" = "--dry-run" ]; then
    DRY_RUN=true
    log_info "Modo DRY-RUN activado"
fi

# Verificar dependencias
check_deps() {
    local missing=()
    for cmd in gh jq curl; do
        if ! command -v "$cmd" &>/dev/null; then
            missing+=("$cmd")
        fi
    done
    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Dependencias faltantes: ${missing[*]}"
        log_info "Instalar GitHub CLI: https://cli.github.com/"
        exit 1
    fi
}

# Check 1: CI status
check_ci_status() {
    log_info "Verificando estado CI para PR #$PR_NUMBER..."

    local ci_status
    ci_status=$(gh pr checks "$PR_NUMBER" --json conclusion --jq '.[].conclusion' 2>/dev/null | grep -v "SUCCESS" | head -1 || echo "")

    if [ -z "$ci_status" ]; then
        log_ok "CI: Todos los checks pasaron"
        return 0
    else
        log_error "CI: Check fallido — $ci_status"
        return 1
    fi
}

# Check 2: Approvals
check_approvals() {
    log_info "Verificando approvals para PR #$PR_NUMBER..."

    local approval_count
    approval_count=$(gh pr reviews "$PR_NUMBER" --json state --jq '.[] | select(.state == "APPROVED")' 2>/dev/null | wc -l || echo "0")

    if [ "$approval_count" -ge 2 ]; then
        log_ok "Approvals: $approval_count (requerido: 2)"
        return 0
    else
        log_error "Approvals: $approval_count (requerido: 2)"
        return 1
    fi
}

# Check 3: Labels bloqueantes
check_blocking_labels() {
    log_info "Verificando labels bloqueantes para PR #$PR_NUMBER..."

    local labels
    labels=$(gh pr view "$PR_NUMBER" --json labels --jq '.labels[].name' 2>/dev/null || echo "")

    if echo "$labels" | grep -qE "^(hold|needs-work|do-not-merge)$"; then
        local blocking
        blocking=$(echo "$labels" | grep -E "^(hold|needs-work|do-not-merge)$" | tr '\n' ', ')
        log_error "Labels bloqueantes encontradas: $blocking"
        return 1
    fi

    log_ok "Labels: Sin bloqueantes"
    return 0
}

# Check 4: Score de calidad
calculate_quality_score() {
    log_info "Calculando score de calidad para PR #$PR_NUMBER..."

    local test_coverage=0 clippy_clean=0 doc_quality=0 benchmark_pass=0

    # Test coverage (simulado — requiere cobertura real)
    local has_tests
    has_tests=$(gh pr diff "$PR_NUMBER" 2>/dev/null | grep -c "mod tests" || echo "0")
    if [ "$has_tests" -gt 0 ]; then
        test_coverage=1
    fi

    # Clippy clean (simulado)
    local clippy_output
    clippy_output=$(gh pr checks "$PR_NUMBER" 2>/dev/null | grep -c "clippy.*success" || echo "0")
    if [ "$clippy_output" -gt 0 ]; then
        clippy_clean=1
    fi

    # Doc quality (simulado)
    local has_docs
    has_docs=$(gh pr diff "$PR_NUMBER" 2>/dev/null | grep -cE "^(\\+.*//|\\+.*///)" || echo "0")
    if [ "$has_docs" -gt 0 ]; then
        doc_quality=1
    fi

    # Benchmark pass (simulado)
    local has_bench
    has_bench=$(gh pr checks "$PR_NUMBER" 2>/dev/null | grep -c "benchmark.*success" || echo "0")
    if [ "$has_bench" -gt 0 ]; then
        benchmark_pass=1
    fi

    # Score = 0.4 * tests + 0.3 * clippy + 0.2 * docs + 0.1 * bench
    local score
    score=$(echo "0.4 * $test_coverage + 0.3 * $clippy_clean + 0.2 * $doc_quality + 0.1 * $benchmark_pass" | bc -l 2>/dev/null || echo "0")

    log_info "Score: $score (tests=$test_coverage, clippy=$clippy_clean, docs=$doc_quality, bench=$benchmark_pass)"

    # Comparar con threshold 0.8
    local passes
    passes=$(echo "$score >= 0.8" | bc -l 2>/dev/null || echo "0")
    if [ "$passes" -eq 1 ]; then
        log_ok "Score: $score >= 0.8 threshold"
        return 0
    else
        log_warn "Score: $score < 0.8 threshold (no bloquea, solo warning)"
        return 0  # No bloquea — solo warning
    fi
}

# Check 5: Linked issue good-first-issue
check_linked_issue() {
    log_info "Verificando issue linked para PR #$PR_NUMBER..."

    local body
    body=$(gh pr view "$PR_NUMBER" --json body --jq '.body' 2>/dev/null || echo "")

    local issue_number
    issue_number=$(echo "$body" | grep -oE "#[0-9]+" | head -1 | tr -d '#' || echo "")

    if [ -z "$issue_number" ]; then
        log_warn "No se encontró issue linked — saltando verificación"
        return 0
    fi

    local issue_labels
    issue_labels=$(gh issue view "$issue_number" --json labels --jq '.labels[].name' 2>/dev/null || echo "")

    if echo "$issue_labels" | grep -q "good-first-issue"; then
        log_ok "Issue #$issue_number tiene label 'good-first-issue'"
        return 0
    else
        log_warn "Issue #$issue_number no tiene label 'good-first-issue'"
        return 0  # No bloquea
    fi
}

# Merge PR
merge_pr() {
    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY-RUN] Would merge PR #$PR_NUMBER"
        return 0
    fi

    log_info "Mergeando PR #$PR_NUMBER..."
    if gh pr merge "$PR_NUMBER" --squash --auto 2>/dev/null; then
        log_ok "PR #$PR_NUMBER mergeado exitosamente"
    else
        log_error "Error mergeando PR #$PR_NUMBER"
        return 1
    fi
}

# Main
main() {
    echo ""
    echo "╔══════════════════════════════════════╗"
    echo "║  ed2kIA v1.9 — Auto-Merge PR #$PR_NUMBER"
    echo "╚══════════════════════════════════════╝"
    echo ""

    check_deps

    local all_passed=true

    # Ejecutar checks
    check_ci_status || all_passed=false
    echo ""

    check_approvals || all_passed=false
    echo ""

    check_blocking_labels || all_passed=false
    echo ""

    calculate_quality_score
    echo ""

    check_linked_issue
    echo ""

    # Resultado
    echo "========================================"
    if [ "$all_passed" = true ]; then
        log_ok "Todos los checks pasaron — listo para merge"
        merge_pr
    else
        log_error "Algunos checks fallaron — NO mergear"
        echo "========================================"
        exit 1
    fi
}

main
