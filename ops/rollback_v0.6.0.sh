#!/bin/sh
# =============================================================================
# Rollback Script: ed2kIA v0.6.0-RC → v0.5.0 STABLE
# =============================================================================
# Usage: ./ops/rollback_v0.6.0.sh [OPTIONS]
#
# Options:
#   --auto              Automatic mode (from canary_deploy.sh)
#   --node IP           Rollback specific node
#   --all-nodes         Rollback all nodes in cluster
#   --preserve-staking  Preserve staking registry data (default: archive)
#   --clean-redb        Clean Phase 6 redb databases
#   --dry-run           Validate without executing
#   --help              Show this help message
#
# Exit Codes:
#   0 - Rollback successful
#   1 - General error
#   2 - Validation failed
#   3 - Partial rollback (some nodes failed)
#
# Requirements:
#   - POSIX-compliant shell
#   - ssh for remote operations
#   - systemctl for service management
#
# Example:
#   ./ops/rollback_v0.6.0.sh --auto
#   ./ops/rollback_v0.6.0.sh --node 192.168.1.100
#   ./ops/rollback_v0.6.0.sh --all-nodes --clean-redb
# =============================================================================

set -euo pipefail

# --- Configuration ---
CURRENT_VERSION="v0.6.0-rc"
TARGET_VERSION="v0.5.0"
SERVICE_NAME="ed2kia"
BINARY_PATH="/usr/local/bin/ed2kia"
REDB_DIR="/var/lib/ed2kia/data"
STAKING_DIR="/var/lib/ed2kia/staking"
BACKUP_DIR="/var/backups/ed2kia"
FEATURE_FLAGS="core-only"

AUTO_MODE=0
TARGET_NODE=""
ALL_NODES=0
PRESERVE_STAKING=0
CLEAN_REDB=0
DRY_RUN=0

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# --- Logging ---
log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"
}

# --- Parse Arguments ---
while [ $# -gt 0 ]; do
    case "$1" in
        --auto)
            AUTO_MODE=1
            shift
            ;;
        --node)
            TARGET_NODE="$2"
            shift 2
            ;;
        --all-nodes)
            ALL_NODES=1
            shift
            ;;
        --preserve-staking)
            PRESERVE_STAKING=1
            shift
            ;;
        --clean-redb)
            CLEAN_REDB=1
            shift
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        --help)
            head -45 "$0" | tail -38
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# --- Pre-Rollback Validation ---
validate_rollback() {
    log_info "Validating rollback prerequisites..."

    # Check if backup exists
    if [ ! -f "${BINARY_PATH}.${TARGET_VERSION}.backup" ] && [ "$DRY_RUN" -eq 0 ]; then
        log_warn "Backup binary not found at ${BINARY_PATH}.${TARGET_VERSION}.backup"
        log_info "Will attempt to restore from Docker image or package manager"
    fi

    # Verify service is running
    if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
        log_info "Service $SERVICE_NAME is currently running"
    else
        log_warn "Service $SERVICE_NAME is not running"
    fi

    log_info "Rollback validation complete"
}

# --- Stop Service ---
stop_service() {
    target="${1:-localhost}"

    log_info "Stopping $SERVICE_NAME on $target..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would stop service on $target"
        return 0
    fi

    if [ "$target" = "localhost" ]; then
        sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || {
            log_warn "Failed to stop service via systemctl"
            return 0
        }
    else
        ssh "ed2kia@$target" "sudo systemctl stop $SERVICE_NAME" 2>/dev/null || {
            log_warn "Failed to stop service on $target via SSH"
            return 0
        }
    fi

    log_info "Service stopped on $target"
}

# --- Restore Binary ---
restore_binary() {
    target="${1:-localhost}"

    log_info "Restoring $TARGET_VERSION binary on $target..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would restore binary on $target"
        return 0
    fi

    # Method 1: Use backup
    if [ "$target" = "localhost" ] && [ -f "${BINARY_PATH}.${TARGET_VERSION}.backup" ]; then
        sudo cp "${BINARY_PATH}.${TARGET_VERSION}.backup" "$BINARY_PATH"
        log_info "Restored from backup"
        return 0
    fi

    # Method 2: Docker extraction
    log_info "Attempting Docker extraction..."
    if docker pull "ed2kia:${TARGET_VERSION}" 2>/dev/null; then
        docker cp "$(docker create --name tmp-ed2kia ed2kia:${TARGET_VERSION}):/usr/local/bin/ed2kia")" "$BINARY_PATH" 2>/dev/null || {
            log_warn "Docker extraction failed"
        }
        docker rm tmp-ed2kia 2>/dev/null || true
        return 0
    fi

    # Method 3: Package manager
    log_warn "Manual binary restoration required"
    log_info "Download $TARGET_VERSION from: https://github.com/ed2kia/ed2kIA/releases/tag/$TARGET_VERSION"
    return 1
}

# --- Disable Phase 6 Features ---
disable_phase6_features() {
    target="${1:-localhost}"

    log_info "Disabling Phase 6 features on $target..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would disable Phase 6 features on $target"
        return 0
    fi

    # Update config to use core-only features
    CONFIG_FILE="/etc/ed2kia/config.toml"
    if [ -f "$CONFIG_FILE" ]; then
        if [ "$target" = "localhost" ]; then
            # Comment out phase6 features
            sed -i 's/^features.*/# features = ["core-only"]  # Phase 6 disabled/' "$CONFIG_FILE" 2>/dev/null || true
        else
            ssh "ed2kia@$target" "sed -i 's/^features.*/# features = [\"core-only\"]  # Phase 6 disabled/' $CONFIG_FILE" 2>/dev/null || true
        fi
        log_info "Config updated: features = [\"core-only\"]"
    else
        log_warn "Config file not found at $CONFIG_FILE"
    fi
}

# --- Handle Staking Data ---
handle_staking_data() {
    log_info "Handling staking registry data..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would handle staking data"
        return 0
    fi

    if [ "$PRESERVE_STAKING" -eq 1 ]; then
        # Archive staking data
        ARCHIVE_PATH="${BACKUP_DIR}/staking_$(date '+%Y%m%d_%H%M%S').tar.gz"
        mkdir -p "$BACKUP_DIR"
        if [ -d "$STAKING_DIR" ]; then
            tar czf "$ARCHIVE_PATH" -C "$(dirname "$STAKING_DIR")" "$(basename "$STAKING_DIR")" 2>/dev/null || {
                log_warn "Failed to archive staking data"
            }
            log_info "Staking data archived to $ARCHIVE_PATH"
        fi
    else
        # Default: Archive for audit
        log_info "Archiving staking data for audit purposes..."
        ARCHIVE_PATH="${BACKUP_DIR}/staking_audit_$(date '+%Y%m%d_%H%M%S').tar.gz"
        mkdir -p "$BACKUP_DIR"
        if [ -d "$STAKING_DIR" ]; then
            tar czf "$ARCHIVE_PATH" -C "$(dirname "$STAKING_DIR")" "$(basename "$STAKING_DIR")" 2>/dev/null || true
            log_info "Staking data archived to $ARCHIVE_PATH"
        fi
    fi
}

# --- Clean Phase 6 Databases ---
clean_redb_databases() {
    log_info "Cleaning Phase 6 redb databases..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would clean redb databases"
        return 0
    fi

    if [ -d "$REDB_DIR" ]; then
        # Remove Phase 6 specific databases
        find "$REDB_DIR" -name "staking_*.redb" -delete 2>/dev/null || true
        find "$REDB_DIR" -name "federation_*.redb" -delete 2>/dev/null || true
        find "$REDB_DIR" -name "feedback_*.redb" -delete 2>/dev/null || true
        log_info "Phase 6 redb databases cleaned"
    else
        log_info "No redb directory found at $REDB_DIR"
    fi
}

# --- Restart Service ---
restart_service() {
    target="${1:-localhost}"

    log_info "Restarting $SERVICE_NAME on $target..."

    if [ "$DRY_RUN" -eq 1 ]; then
        log_info "[DRY RUN] Would restart service on $target"
        return 0
    fi

    if [ "$target" = "localhost" ]; then
        sudo systemctl start "$SERVICE_NAME" 2>/dev/null || {
            log_error "Failed to start service"
            return 1
        }
    else
        ssh "ed2kia@$target" "sudo systemctl start $SERVICE_NAME" 2>/dev/null || {
            log_error "Failed to start service on $target"
            return 1
        }
    fi

    # Wait for startup
    sleep 5

    # Verify health
    if [ "$target" = "localhost" ]; then
        health_url="http://localhost:3030/api/v1/health"
    else
        health_url="http://$target:3030/api/v1/health"
    fi

    http_code=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 10 "$health_url" 2>/dev/null || echo "000")
    if [ "$http_code" = "200" ]; then
        log_info "✅ Service healthy on $target (HTTP $http_code)"
        return 0
    else
        log_error "❌ Service unhealthy on $target (HTTP $http_code)"
        return 1
    fi
}

# --- Rollback Single Node ---
rollback_node() {
    node="${1:-localhost}"

    log_info "=========================================="
    log_info "Rolling back $node: $CURRENT_VERSION → $TARGET_VERSION"
    log_info "=========================================="

    stop_service "$node"
    restore_binary "$node"
    disable_phase6_features "$node"

    if restart_service "$node"; then
        log_info "✅ Rollback successful on $node"
        return 0
    else
        log_error "❌ Rollback failed on $node"
        return 1
    fi
}

# --- Main Rollback Flow ---
main() {
    log_info "=========================================="
    log_info "ed2kIA Rollback: $CURRENT_VERSION → $TARGET_VERSION"
    log_info "=========================================="

    # Validate
    validate_rollback

    # Handle staking data (local only)
    handle_staking_data

    # Clean redb if requested
    if [ "$CLEAN_REDB" -eq 1 ]; then
        clean_redb_databases
    fi

    # Execute rollback
    FAILED=0
    TOTAL=0

    if [ -n "$TARGET_NODE" ]; then
        # Single node rollback
        rollback_node "$TARGET_NODE" || FAILED=1
        TOTAL=1
    elif [ "$ALL_NODES" -eq 1 ]; then
        # All nodes rollback
        SEED_FILE="launch/genesis/seed_nodes.json"
        if [ -f "$SEED_FILE" ]; then
            while IFS= read -r node; do
                case "$node" in \#*|"") continue ;; esac
                TOTAL=$((TOTAL + 1))
                rollback_node "$node" || FAILED=$((FAILED + 1))
            done < "$SEED_FILE"
        else
            log_warn "Seed nodes file not found, rolling back localhost"
            rollback_node "localhost" || FAILED=1
            TOTAL=1
        fi
    else
        # Default: localhost
        rollback_node "localhost" || FAILED=1
        TOTAL=1
    fi

    # Summary
    log_info "=========================================="
    log_info "Rollback Summary:"
    log_info "  Total nodes: $TOTAL"
    log_info "  Successful: $((TOTAL - FAILED))"
    log_info "  Failed: $FAILED"
    log_info "=========================================="

    if [ "$FAILED" -gt 0 ]; then
        log_error "Partial rollback! $FAILED node(s) require manual intervention"
        exit 3
    fi

    log_info "✅ Full rollback to $TARGET_VERSION complete"
    log_info "Next steps:"
    log_info "  1. Verify network consensus via /api/v1/health"
    log_info "  2. Check governance channel for operator reports"
    log_info "  3. File incident report if rollback was triggered by bug"
    exit 0
}

# Execute main
main "$@"
