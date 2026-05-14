#!/usr/bin/env bash
# ed2kIA v1.1.0 Sprint 5 - Release Packaging Script
# POSIX compliant, no external dependencies beyond standard Unix tools
set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================
VERSION="1.1.0-sprint5"
PROJECT_NAME="ed2kIA"
CARGO_FEATURES="v1.1-sprint5"
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
            ;;
        --clean)
            CLEAN=true
            ;;
        --platform)
            PLATFORM="$2"
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
    shift
done

if [[ -z "$PLATFORM" ]]; then
    PLATFORM="$(detect_platform)"
fi

log_info "Platform: ${PLATFORM}"
log_info "Version: ${VERSION}"
log_info "Features: ${CARGO_FEATURES}"

# ============================================================================
# Step 1: Clean (optional)
# ============================================================================

progress "Clean previous artifacts"

if [[ "$CLEAN" == true ]]; then
    if [[ "$DRY_RUN" == false ]]; then
        rm -rf "${ARTIFACT_DIR}"
        rm -rf "${BUILD_DIR}"
        log_success "Cleaned artifacts and build directories"
    else
        log_info "[DRY-RUN] Would clean ${ARTIFACT_DIR} and ${BUILD_DIR}"
    fi
else
    log_info "Skipping clean (use --clean to enable)"
fi

# ============================================================================
# Step 2: Validation
# ============================================================================

progress "Run validation tests"

if [[ "$DRY_RUN" == false ]]; then
    # E2E Tests
    log_info "Running E2E tests..."
    cargo test --test v1_1_sprint5_e2e --features "${CARGO_FEATURES}" || {
        log_error "E2E tests failed"
        exit 1
    }
    log_success "E2E tests passed"

    # Stress Tests
    log_info "Running stress tests..."
    cargo test --test full_network_stress --features "${CARGO_FEATURES}" || {
        log_error "Stress tests failed"
        exit 1
    }
    log_success "Stress tests passed"

    # Library Tests
    log_info "Running library tests..."
    cargo test --features "${CARGO_FEATURES}" --lib || {
        log_error "Library tests failed"
        exit 1
    }
    log_success "Library tests passed"
else
    log_info "[DRY-RUN] Would run E2E, stress, and library tests"
fi

# ============================================================================
# Step 3: Clippy
# ============================================================================

progress "Run clippy linter"

if [[ "$DRY_RUN" == false ]]; then
    cargo clippy --features "${CARGO_FEATURES}" -- -D warnings || {
        log_error "Clippy found warnings"
        exit 1
    }
    log_success "Clippy clean (0 warnings)"
else
    log_info "[DRY-RUN] Would run clippy"
fi

# ============================================================================
# Step 4: Build Release
# ============================================================================

progress "Build release binary"

if [[ "$DRY_RUN" == false ]]; then
    cargo build --release --features "${CARGO_FEATURES}" || {
        log_error "Release build failed"
        exit 1
    }
    log_success "Release build complete"
else
    log_info "[DRY-RUN] Would build release binary"
fi

# ============================================================================
# Step 5: Package Artifacts
# ============================================================================

progress "Package release artifacts"

if [[ "$DRY_RUN" == false ]]; then
    mkdir -p "${ARTIFACT_DIR}"

    ARTIFACT_NAME="${PROJECT_NAME}-${VERSION}-${PLATFORM}"
    ARTIFACT_PATH="${ARTIFACT_DIR}/${ARTIFACT_NAME}"

    mkdir -p "${ARTIFACT_PATH}/bin"
    mkdir -p "${ARTIFACT_PATH}/docs"
    mkdir -p "${ARTIFACT_PATH}/release"

    # Copy binary
    if [[ -f "${BUILD_DIR}/ed2kia" ]]; then
        cp "${BUILD_DIR}/ed2kia" "${ARTIFACT_PATH}/bin/"
        log_success "Binary copied"
    else
        log_warn "Binary not found at ${BUILD_DIR}/ed2kia"
    fi

    # Copy docs
    cp "${ROOT_DIR}/docs/v1.1.0_sprint5_release_notes.md" "${ARTIFACT_PATH}/docs/" 2>/dev/null || true
    cp "${ROOT_DIR}/LICENSE" "${ARTIFACT_PATH}/" 2>/dev/null || true

    # Copy release files
    cp "${RELEASE_DIR}/validation_report.json" "${ARTIFACT_PATH}/release/" 2>/dev/null || true

    # Create tarball
    cd "${ARTIFACT_DIR}"
    tar -czf "${ARTIFACT_NAME}.tar.gz" "${ARTIFACT_NAME}"
    rm -rf "${ARTIFACT_PATH}"

    # Generate checksum
    if command -v sha256sum &> /dev/null; then
        sha256sum "${ARTIFACT_NAME}.tar.gz" > "${ARTIFACT_NAME}.sha256"
    elif command -v shasum &> /dev/null; then
        shasum -a 256 "${ARTIFACT_NAME}.tar.gz" > "${ARTIFACT_NAME}.sha256"
    fi

    log_success "Artifact: ${ARTIFACT_DIR}/${ARTIFACT_NAME}.tar.gz"
    log_success "Checksum: ${ARTIFACT_DIR}/${ARTIFACT_NAME}.sha256"
else
    log_info "[DRY-RUN] Would package artifacts"
fi

# ============================================================================
# Step 6: Summary
# ============================================================================

progress "Release Summary"

log_info "Version: ${VERSION}"
log_info "Platform: ${PLATFORM}"
log_info "Features: ${CARGO_FEATURES}"

if [[ "$DRY_RUN" == false ]]; then
    if [[ -f "${ARTIFACT_DIR}/${ARTIFACT_NAME}.tar.gz" ]]; then
        local_size=$(du -h "${ARTIFACT_DIR}/${ARTIFACT_NAME}.tar.gz" | cut -f1)
        log_success "Artifact size: ${local_size}"
        log_success "Release package ready!"
    else
        log_error "Artifact not found"
        exit 1
    fi
else
    log_info "[DRY-RUN] Summary preview complete"
fi

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Release packaging complete!${NC}"
echo -e "${GREEN}========================================${NC}"
