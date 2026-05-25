#!/usr/bin/env bash
# sign-release.sh — ed2kIA v3.0.0 Release Signing Script
#
# Signs release binaries & documentation with Ed25519,
# generates SHA256SUMS, and produces a signing log.
#
# Usage: bash release/v3.0.0-stable/sign-release.sh [output_dir]
#
# Requirements:
#   - ed25519-dalek CLI or openssl (for signing)
#   - sha256sum or shasum -a 256
#   - POSIX-compatible shell
#
# Exit codes:
#   0 — Success
#   1 — Missing prerequisites
#   2 — Signing failure
#   3 — Checksum generation failure

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

VERSION="v3.0.0-stable"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="${1:-$SCRIPT_DIR/artifacts}"
SIGNING_KEY="${ED2KIA_SIGNING_KEY:-}"
LOG_FILE="$OUTPUT_DIR/signing-log.txt"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

log() {
    local msg="[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] $*"
    echo "$msg" | tee -a "$LOG_FILE"
}

error() {
    log "ERROR: $*" >&2
}

check_prereq() {
    local cmd="$1"
    local name="$2"
    if ! command -v "$cmd" &>/dev/null; then
        error "$name not found ($cmd). Install and retry."
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Pre-flight
# ---------------------------------------------------------------------------

main() {
    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Initialize log
    : > "$LOG_FILE"
    log "=========================================="
    log "ed2kIA $VERSION — Release Signing"
    log "=========================================="
    log "Project root: $PROJECT_ROOT"
    log "Output dir:   $OUTPUT_DIR"

    # Check prerequisites
    check_prereq "sha256sum" || check_prereq "shasum" "SHA256 checksum tool"
    check_prereq "tar" "tar"
    check_prereq "gzip" "gzip"

    # Determine sha256 command
    if command -v sha256sum &>/dev/null; then
        SHA256_CMD="sha256sum"
    elif command -v shasum &>/dev/null; then
        SHA256_CMD="shasum -a 256"
    else
        error "No SHA256 tool found."
        exit 1
    fi

    # -----------------------------------------------------------------------
    # Collect artifacts
    # -----------------------------------------------------------------------
    log "Collecting release artifacts..."

    local release_dir="$PROJECT_ROOT/target/release"
    local artifact_count=0

    if [ -d "$release_dir" ]; then
        # Copy binaries
        for bin in "$release_dir"/ed2kia*; do
            if [ -f "$bin" ]; then
                cp "$bin" "$OUTPUT_DIR/"
                artifact_count=$((artifact_count + 1))
                log "  Copied: $(basename "$bin")"
            fi
        done
    else
        log "WARNING: Release build directory not found ($release_dir)"
        log "  Run: cargo build --release --all-features"
    fi

    # Copy documentation
    for doc in release-notes.md migration-guide-v2.1-to-v3.0.md launch-checklist.md; do
        if [ -f "$SCRIPT_DIR/$doc" ]; then
            cp "$SCRIPT_DIR/$doc" "$OUTPUT_DIR/"
            artifact_count=$((artifact_count + 1))
            log "  Copied: $doc"
        fi
    done

    if [ "$artifact_count" -eq 0 ]; then
        error "No artifacts found to sign."
        exit 2
    fi

    log "Total artifacts: $artifact_count"

    # -----------------------------------------------------------------------
    # Generate SHA256 checksums
    # -----------------------------------------------------------------------
    log "Generating SHA256 checksums..."

    cd "$OUTPUT_DIR"
    if $SHA256_CMD * > SHA256SUMS 2>/dev/null; then
        log "SHA256SUMS generated successfully."
    else
        error "Failed to generate SHA256SUMS."
        exit 3
    fi

    # -----------------------------------------------------------------------
    # Sign artifacts (if signing key available)
    # -----------------------------------------------------------------------
    if [ -n "$SIGNING_KEY" ] && [ -f "$SIGNING_KEY" ]; then
        log "Signing artifacts with Ed25519 key..."

        local sig_count=0
        for artifact in "$OUTPUT_DIR"/*; do
            if [ -f "$artifact" ] && [ "$(basename "$artifact")" != "SHA256SUMS" ]; then
                local basename="$(basename "$artifact")"
                if openssl dgst -sha256 -sign "$SIGNING_KEY" -out "$basename.sig" "$artifact" 2>/dev/null; then
                    sig_count=$((sig_count + 1))
                    log "  Signed: $basename → $basename.sig"
                else
                    error "  Failed to sign: $basename"
                fi
            fi
        done

        log "Signed $sig_count artifacts."

        # Sign SHA256SUMS
        if openssl dgst -sha256 -sign "$SIGNING_KEY" -out "SHA256SUMS.sig" "SHA256SUMS" 2>/dev/null; then
            log "Signed SHA256SUMS."
        fi
    else
        log "WARNING: ED2KIA_SIGNING_KEY not set or key file not found."
        log "  Artifacts are checksummed but NOT signed."
        log "  Set ED2KIA_SIGNING_KEY=/path/to/private.key to enable signing."
    fi

    # -----------------------------------------------------------------------
    # Create release tarball
    # -----------------------------------------------------------------------
    log "Creating release tarball..."

    local tarball="ed2kIA-${VERSION}-artifacts.tar.gz"
    cd "$OUTPUT_DIR"
    if tar -czf "$tarball" --exclude="$tarball" . 2>/dev/null; then
        $SHA256_CMD "$tarball" > "${tarball}.sha256"
        log "Tarball created: $tarball"
        log "Tarball checksum: $(cat "${tarball}.sha256")"
    else
        error "Failed to create tarball."
        exit 2
    fi

    # -----------------------------------------------------------------------
    # Summary
    # -----------------------------------------------------------------------
    log "=========================================="
    log "Release signing complete."
    log "  Artifacts:    $artifact_count"
    log "  Output dir:   $OUTPUT_DIR"
    log "  Tarball:      $OUTPUT_DIR/$tarball"
    log "  Checksums:    $OUTPUT_DIR/SHA256SUMS"
    log "  Log:          $LOG_FILE"
    log "=========================================="

    echo ""
    echo "Release artifacts ready in: $OUTPUT_DIR"
    echo "Verify with: $SHA256_CMD -c SHA256SUMS"
}

main "$@"
