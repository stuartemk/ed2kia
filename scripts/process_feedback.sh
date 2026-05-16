#!/usr/bin/env bash
# process_feedback.sh — Procesamiento automatizado de feedback beta v1.9
# Uso: ./scripts/process_feedback.sh [modo]
# Modos: triage | stats | report | all
#
# Dependencias: jq, date, grep, awk
# Salida: docs/feedback_integration_log.md (actualizado)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
FEEDBACK_DIR="$PROJECT_ROOT/ops/feedback"
SCHEMA_FILE="$FEEDBACK_DIR/community_feedback_schema.json"
LOG_FILE="$PROJECT_ROOT/docs/feedback_integration_log.md"
ISSUES_BATCH="$PROJECT_ROOT/ISSUES_BATCH_V1.9.md"

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info()  { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Verificar dependencias
check_deps() {
    local missing=()
    for cmd in jq date grep awk; do
        if ! command -v "$cmd" &>/dev/null; then
            missing+=("$cmd")
        fi
    done
    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Dependencias faltantes: ${missing[*]}"
        exit 1
    fi
    log_ok "Dependencias verificadas"
}

# Validar entrada contra schema
validate_entry() {
    local entry_file="$1"
    if [ ! -f "$SCHEMA_FILE" ]; then
        log_warn "Schema no encontrado: $SCHEMA_FILE — saltando validación"
        return 0
    fi
    # Validación básica: campos requeridos
    local has_type has_severity has_source
    has_type=$(jq -r 'has("type")' "$entry_file" 2>/dev/null || echo "false")
    has_severity=$(jq -r 'has("severity")' "$entry_file" 2>/dev/null || echo "false")
    has_source=$(jq -r 'has("source")' "$entry_file" 2>/dev/null || echo "false")
    if [ "$has_type" = "true" ] && [ "$has_severity" = "true" ] && [ "$has_source" = "true" ]; then
        log_ok "Validación OK: $entry_file"
        return 0
    else
        log_error "Validación FALLIDA: $entry_file (faltan campos requeridos)"
        return 1
    fi
}

# Triage automático: clasificar por prioridad
triage_entry() {
    local entry_file="$1"
    local severity source
    severity=$(jq -r '.severity // "unknown"' "$entry_file" 2>/dev/null || echo "unknown")
    source=$(jq -r '.source // "unknown"' "$entry_file" 2>/dev/null || echo "unknown")

    case "$severity" in
        critical)
            echo "P0"
            ;;
        high)
            echo "P1"
            ;;
        medium)
            echo "P2"
            ;;
        low)
            echo "P3"
            ;;
        *)
            echo "P2"  # Default a medio
            ;;
    esac
}

# Generar estadísticas
generate_stats() {
    log_info "Generando estadísticas de feedback..."

    local total=0 accepted=0 rejected=0 pending=0
    local p0=0 p1=0 p2=0 p3=0

    # Contar entradas de feedback (simulado desde log)
    if [ -f "$LOG_FILE" ]; then
        total=$(grep -c "FB-" "$LOG_FILE" 2>/dev/null || echo "0")
        p0=$(grep -c "| FB-.*P0" "$LOG_FILE" 2>/dev/null || echo "0")
        p1=$(grep -c "| FB-.*P1" "$LOG_FILE" 2>/dev/null || echo "0")
        p2=$(grep -c "| FB-.*P2" "$LOG_FILE" 2>/dev/null || echo "0")
        p3=$(grep -c "| FB-.*P3" "$LOG_FILE" 2>/dev/null || echo "0")
    fi

    echo ""
    echo "========================================="
    echo "  FEEDBACK BETA v1.9 — ESTADÍSTICAS"
    echo "========================================="
    echo "  Total entradas:    $total"
    echo "  P0 (Crítico):      $p0"
    echo "  P1 (Alto):         $p1"
    echo "  P2 (Medio):        $p2"
    echo "  P3 (Bajo):         $p3"
    echo "========================================="
    echo ""
}

# Generar reporte completo
generate_report() {
    log_info "Generando reporte de integración..."

    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    cat <<EOF

========================================
  REPORTE INTEGRACIÓN FEEDBACK v1.9
  Generado: $timestamp
========================================

1. RESUMEN
   - Feedback procesado: Verificar docs/feedback_integration_log.md
   - SLA 48h: Consultar métricas en log
   - Prioridades: P0→P3 según severity

2. ACCIONES REQUERIDAS
   - [ ] Review P0 items con security team
   - [ ] Asignar P1 items a sprint v1.9-s3
   - [ ] Documentar decisiones P2/P3
   - [ ] Notificar contributors sobre aceptados

3. PRÓXIMOS PASOS
   - Integrar con backlog sprint
   - Actualizar ISSUES_BATCH_V1.9.md
   - Publicar transparencia report

========================================
EOF
}

# Modo: triage — procesar nuevas entradas
mode_triage() {
    log_info "Modo TRIAGE — procesando nuevas entradas..."

    # Buscar archivos JSON en feedback directory
    local count=0
    for entry in "$FEEDBACK_DIR"/*.json; do
        [ -f "$entry" ] || continue
        # Saltar schema
        [ "$entry" = "$SCHEMA_FILE" ] && continue

        if validate_entry "$entry"; then
            local priority
            priority=$(triage_entry "$entry")
            log_info "  $entry → $priority"
            count=$((count + 1))
        fi
    done

    if [ $count -eq 0 ]; then
        log_warn "No se encontraron entradas nuevas para triage"
    else
        log_ok "Triage completado: $count entradas procesadas"
    fi
}

# Modo: stats — solo estadísticas
mode_stats() {
    generate_stats
}

# Modo: report — reporte completo
mode_report() {
    generate_report
}

# Modo: all — todo el pipeline
mode_all() {
    log_info "Ejecutando pipeline completo..."
    mode_triage
    echo ""
    mode_stats
    echo ""
    mode_report
}

# Main
main() {
    local mode="${1:-all}"

    echo ""
    echo "╔══════════════════════════════════════╗"
    echo "║  ed2kIA v1.9 — Feedback Processor   ║"
    echo "╚══════════════════════════════════════╝"
    echo ""

    check_deps

    case "$mode" in
        triage)
            mode_triage
            ;;
        stats)
            mode_stats
            ;;
        report)
            mode_report
            ;;
        all)
            mode_all
            ;;
        *)
            log_error "Modo desconocido: $mode"
            echo "Uso: $0 [triage|stats|report|all]"
            exit 1
            ;;
    esac

    echo ""
    log_ok "Procesamiento completado"
}

main "$@"
