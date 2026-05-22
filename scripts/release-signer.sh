#!/usr/bin/env bash
# =============================================================================
# ed2kIA Release Signer — Ed25519 Cryptographic Signatures (Sprint27)
# =============================================================================
# Usage:
#   ./scripts/release-signer.sh --sign <file>          → generates <file>.sig
#   ./scripts/release-signer.sh --verify <file> <sig>  → verifies signature
#
# Compatible with OpenSSL (POSIX standard). No external dependencies.
# Feature gate: v2.1-security-audit
# =============================================================================

set -euo pipefail

# ─── Configuration ───────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
KEY_FILE="${PROJECT_ROOT}/release.key.pem"
SIG_DIR="${PROJECT_ROOT}/docs/release-signatures"
LOG_FILE="${SIG_DIR}/signing-log.md"

# ─── Cleanup Trap ────────────────────────────────────────────────────────────
cleanup() {
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo "❌ release-signer.sh exited with code ${exit_code}" >&2
    fi
    exit $exit_code
}
trap cleanup EXIT INT TERM

# ─── Helpers ─────────────────────────────────────────────────────────────────
log() {
    local timestamp
    timestamp="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    echo "[${timestamp}] $*"
}

ensure_sig_dir() {
    mkdir -p "$SIG_DIR"
    if [ ! -f "$LOG_FILE" ]; then
        cat > "$LOG_FILE" << 'EOF'
# Release Signing Log — ed2kIA

All cryptographic signatures generated with Ed25519 via OpenSSL.
Each entry includes timestamp, file, SHA-256 hash and signature status.

| Date (UTC) | File | SHA-256 | Action | Status |
|------------|------|---------|--------|--------|
EOF
    fi
}

append_log() {
    local file="$1"
    local action="$2"
    local status="$3"
    local sha256
    sha256="$(sha256sum "$file" 2>/dev/null | awk '{print $1}' || echo 'N/A')"
    local timestamp
    timestamp="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    echo "| ${timestamp} | $(basename "$file") | ${sha256} | ${action} | ${status} |" >> "$LOG_FILE"
}

usage() {
    cat << 'EOF'
ed2kIA Release Signer — Ed25519 Cryptographic Signatures

Usage:
  ./scripts/release-signer.sh --sign <file>          Generate Ed25519 signature
  ./scripts/release-signer.sh --verify <file> <sig>  Verify Ed25519 signature
  ./scripts/release-signer.sh --init                  Generate new signing key

Options:
  --sign <file>       Sign the specified file, producing <file>.sig
  --verify <file> <sig>  Verify <file> against signature <sig>
  --init              Generate a new Ed25519 signing key (release.key.pem)
  -h, --help          Show this help message

Examples:
  ./scripts/release-signer.sh --init
  ./scripts/release-signer.sh --sign target/release/ed2kia
  ./scripts/release-signer.sh --verify target/release/ed2kia target/release/ed2kia.sig
EOF
}

# ─── Key Initialization ──────────────────────────────────────────────────────
cmd_init() {
    log "Initializing Ed25519 signing key..."
    if [ -f "$KEY_FILE" ]; then
        log "⚠️  Key already exists at ${KEY_FILE}. Use --force to regenerate."
        echo "Key file: ${KEY_FILE}"
        echo "Fingerprint: $(openssl pkey -in "$KEY_FILE" -pubout -outform DER 2>/dev/null | openssl dgst -sha256 | awk '{print $NF}')"
        return 0
    fi

    openssl genpkey -algorithm ed25519 -out "$KEY_FILE"
    chmod 600 "$KEY_FILE"
    log "✅ Ed25519 key generated: ${KEY_FILE}"

    local fingerprint
    fingerprint="$(openssl pkey -in "$KEY_FILE" -pubout -outform DER 2>/dev/null | openssl dgst -sha256 | awk '{print $NF}')"
    log "🔑 Public key fingerprint (SHA-256): ${fingerprint}"
    log "📋 Store this key securely. It is required for signature verification."
}

# ─── Sign Command ────────────────────────────────────────────────────────────
cmd_sign() {
    local target_file="$1"

    if [ ! -f "$KEY_FILE" ]; then
        log "❌ Signing key not found at ${KEY_FILE}. Run --init first."
        return 1
    fi

    if [ ! -f "$target_file" ]; then
        log "❌ File not found: ${target_file}"
        return 1
    fi

    ensure_sig_dir

    local sig_file="${target_file}.sig"
    log "Signing ${target_file} → ${sig_file}"

    # Generate Ed25519 signature using OpenSSL
    openssl pkeyutl -sign \
        -inkey "$KEY_FILE" \
        -rawin \
        -in "$target_file" \
        -out "$sig_file"

    log "✅ Signature generated: ${sig_file}"
    log "   Signature size: $(wc -c < "$sig_file") bytes"

    append_log "$target_file" "sign" "✅ SIGNED"
}

# ─── Verify Command ──────────────────────────────────────────────────────────
cmd_verify() {
    local target_file="$1"
    local sig_file="$2"

    if [ ! -f "$KEY_FILE" ]; then
        log "❌ Signing key not found at ${KEY_FILE}. Run --init first."
        return 1
    fi

    if [ ! -f "$target_file" ]; then
        log "❌ File not found: ${target_file}"
        return 1
    fi

    if [ ! -f "$sig_file" ]; then
        log "❌ Signature file not found: ${sig_file}"
        return 1
    fi

    ensure_sig_dir

    log "Verifying ${target_file} against ${sig_file}..."

    # Extract public key from private key for verification
    local pub_key
    pub_key="$(mktemp)"
    trap "rm -f '$pub_key'" EXIT

    openssl pkey -in "$KEY_FILE" -pubout -out "$pub_key" 2>/dev/null

    if openssl pkeyutl -verify \
        -pubin -inkey "$pub_key" \
        -rawin \
        -in "$target_file" \
        -sigfile "$sig_file" 2>/dev/null; then
        log "✅ VERIFICATION PASSED — Signature is valid"
        append_log "$target_file" "verify" "✅ VALID"
        rm -f "$pub_key"
        return 0
    else
        log "❌ VERIFICATION FAILED — Signature does not match"
        append_log "$target_file" "verify" "❌ INVALID"
        rm -f "$pub_key"
        return 1
    fi
}

# ─── Main ────────────────────────────────────────────────────────────────────
main() {
    if [ $# -eq 0 ]; then
        usage
        exit 1
    fi

    case "$1" in
        --sign)
            if [ $# -lt 2 ]; then
                log "❌ --sign requires a file argument"
                usage
                exit 1
            fi
            cmd_sign "$2"
            ;;
        --verify)
            if [ $# -lt 3 ]; then
                log "❌ --verify requires <file> and <signature> arguments"
                usage
                exit 1
            fi
            cmd_verify "$2" "$3"
            ;;
        --init)
            cmd_init
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log "❌ Unknown command: $1"
            usage
            exit 1
            ;;
    esac
}

main "$@"
