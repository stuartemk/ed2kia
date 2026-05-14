#!/usr/bin/env bash
# ed2kIA v1.1.0 STABLE - Release Packaging Script
# POSIX compliant, no external dependencies beyond standard Unix tools
set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================
VERSION="1.1.0-stable"
PROJECT_NAME="ed2kIA"
CARGO_FEATURES="stable"
RELEASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${RELEASE_DIR}/../.." && pwd)"
BUILD_DIR="${ROOT_DIR}/target/release"
ARTIFACT_DIR="${ROOT_DIR}/target/artifacts"
STEPS_TOTAL=6
STEP_COUNT=0

# ============================================================================
# Color-coded output
# ============================================================================
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ============================================================================
# Utility Functions
# ============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

progress() {
    STEP_COUNT=$((STEP_COUNT + 1))
    local current=$STEP_COUNT
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}[Step ${current}/${STEPS_TOTAL}]${NC} $1"
    echo -e "${BLUE}========================================${NC}"
}

usage() {
    cat <<EOF
${PROJECT_NAME} ${VERSION} Release Packaging Script

USAGE:
    $(basename "$0") [OPTIONS]

OPTIONS:
    -h, --help          Show this help message and exit
    --dry-run           Run without executing commands (show what would be done)
    --clean             Clean previous artifacts before building
    --platform OS-ARCH  Override platform detection (e.g., linux-x86_64, macos-aarch64)

EXAMPLES:
    $(basename "$0")                  # Build and package for current platform
    $(basename "$0") --dry-run        # Preview actions without executing
    $(basename "$0") --clean          # Clean and rebuild
    $(basename "$0") --platform linux-x86_64  # Force platform

OUTPUT:
    Creates tarball in target/artifacts/:
        ${PROJECT_NAME}-${VERSION}-{os}-{arch}.tar.gz

EOF
}

# ============================================================================
# Platform Detection
# ============================================================================

detect_platform() {
    local os arch
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"

    case "$os" in
        linux|darwin|freebsd) ;;
        msys*|cygwin*) os="windows" ;;
        *) os="unknown" ;;
    esac

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) arch="unknown" ;;
    esac

    echo "${os}-${arch}"
}

# ============================================================================
# Parse Arguments
# ============================================================================

DRY_RUN=false
CLEAN=false
PLATFORM=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            usage
            exit 0
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# ============================================================================
# Pre-flight Checks
# ============================================================================

preflight() {
    progress "Pre-flight Checks"

    if ! command -v cargo &>/dev/null; then
        log_error "cargo not found in PATH"
        exit 1
    fi
    log_success "cargo found: $(cargo --version)"

    if ! command -v rustc &>/dev/null; then
        log_error "rustc not found in PATH"
        exit 1
    fi
    log_success "rustc found: $(rustc --version)"

    if [ -z "$PLATFORM" ]; then
        PLATFORM="$(detect_platform)"
    fi
    log_info "Target platform: ${PLATFORM}"

    if [ "$PLATFORM" = "unknown-unknown" ]; then
        log_error "Unable to detect platform. Use --platform to specify."
        exit 1
    fi
}

# ============================================================================
# Clean Previous Artifacts
# ============================================================================

clean_artifacts() {
    if [ "$CLEAN" = true ]; then
        progress "Cleaning Previous Artifacts"
        if [ "$DRY_RUN" = true ]; then
            log_info "[DRY-RUN] Would clean ${ARTIFACT_DIR}"
        else
            rm -rf "${ARTIFACT_DIR}"
            log_success "Cleaned ${ARTIFACT_DIR}"
        fi
    fi
}

# ============================================================================
# Build Release Binary
# ============================================================================

build_release() {
    progress "Building Release Binary"
    local cmd="cargo build --release --features ${CARGO_FEATURES} --manifest-path ${ROOT_DIR}/Cargo.toml"

    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY-RUN] Would execute: ${cmd}"
    else
        (cd "${ROOT_DIR}" && eval "$cmd")
        log_success "Release binary built"
    fi
}

# ============================================================================
# Run Validation Tests
# ============================================================================

run_validation() {
    progress "Running Validation Tests"

    local tests=(
        "cargo check --features ${CARGO_FEATURES} --manifest-path ${ROOT_DIR}/Cargo.toml"
        "cargo clippy --features ${CARGO_FEATURES} --manifest-path ${ROOT_DIR}/Cargo.toml"
        "cargo test --features ${CARGO_FEATURES} --lib --manifest-path ${ROOT_DIR}/Cargo.toml"
    )

    for test_cmd in "${tests[@]}"; do
        log_info "Running: ${test_cmd}"
        if [ "$DRY_RUN" = true ]; then
            log_info "[DRY-RUN] Would execute: ${test_cmd}"
        else
            if (cd "${ROOT_DIR}" && eval "$test_cmd" >/dev/null 2>&1); then
                log_success "Passed: ${test_cmd}"
            else
                log_error "Failed: ${test_cmd}"
                exit 1
            fi
        fi
    done
}

# ============================================================================
# Package Artifacts
# ============================================================================

package_artifacts() {
    progress "Packaging Artifacts"

    local os="${PLATFORM%-*}"
    local arch="${PLATFORM#*-}"
    local tarball="${PROJECT_NAME}-${VERSION}-${os}-${arch}.tar.gz"
    local staging="${ARTIFACT_DIR}/staging"

    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY-RUN] Would create: ${ARTIFACT_DIR}/${tarball}"
        return
    fi

    mkdir -p "${ARTIFACT_DIR}"
    mkdir -p "${staging}/${PROJECT_NAME}"

    # Copy binary
    if [ -f "${BUILD_DIR}/ed2kia" ]; then
        cp "${BUILD_DIR}/ed2kia" "${staging}/${PROJECT_NAME}/"
    elif [ -f "${BUILD_DIR}/ed2kia.exe" ]; then
        cp "${BUILD_DIR}/ed2kia.exe" "${staging}/${PROJECT_NAME}/"
    fi

    # Copy documentation
    cp -r "${ROOT_DIR}/docs" "${staging}/${PROJECT_NAME}/" 2>/dev/null || true
    cp "${ROOT_DIR}/README.md" "${staging}/${PROJECT_NAME}/" 2>/dev/null || true
    cp "${ROOT_DIR}/LICENSE" "${staging}/${PROJECT_NAME}/" 2>/dev/null || true

    # Copy release docs
    cp -r "${RELEASE_DIR}" "${staging}/${PROJECT_NAME}/release/" 2>/dev/null || true

    # Create tarball
    (cd "${ARTIFACT_DIR}" && tar -czf "${tarball}" "${PROJECT_NAME}")
    rm -rf "${staging}"

    log_success "Created: ${ARTIFACT_DIR}/${tarball}"

    # Generate checksum
    if command -v sha256sum &>/dev/null; then
        (cd "${ARTIFACT_DIR}" && sha256sum "${tarball}" > "${tarball}.sha256")
        log_success "Checksum: ${tarball}.sha256"
    elif command -v shasum &>/dev/null; then
        (cd "${ARTIFACT_DIR}" && shasum -a 256 "${tarball}" > "${tarball}.sha256")
        log_success "Checksum: ${tarball}.sha256"
    fi
}

# ============================================================================
# Generate Release Notes
# ============================================================================

generate_release_notes() {
    progress "Generating Release Summary"

    local artifact_file="${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}-${PLATFORM%-*}-${PLATFORM#*-}.tar.gz"

    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY-RUN] Would generate release summary"
        return
    fi

    log_info "============================================"
    log_info "  ${PROJECT_NAME} ${VERSION} Release Summary"
    log_info "============================================"
    log_info "Version:       ${VERSION}"
    log_info "Platform:      ${PLATFORM}"
    log_info "Feature Flags: ${CARGO_FEATURES}"
    log_info "Artifact:      ${artifact_file}"

    if [ -f "${artifact_file}" ]; then
        local size
        size="$(du -h "${artifact_file}" | cut -f1)"
        log_info "Size:          ${size}"
    fi

    log_info "============================================"
    log_success "Release packaging complete!"
}

# ============================================================================
# Main
# ============================================================================

main() {
    log_info "Starting ${PROJECT_NAME} ${VERSION} release packaging..."
    log_info "Feature flags: ${CARGO_FEATURES}"

    preflight
    clean_artifacts
    build_release
    run_validation
    package_artifacts
    generate_release_notes

    log_success "All steps completed successfully!"
}

main "$@"
