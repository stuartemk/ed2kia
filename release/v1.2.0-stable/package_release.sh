#!/usr/bin/env bash
# ed2kIA v1.2.0 STABLE - Release Packaging Script
# POSIX compliant, no external dependencies beyond standard Unix tools
set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================
VERSION="1.2.0-stable"
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
NC='\033[0m'

# ============================================================================
# Utility Functions
# ============================================================================
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

progress() {
    STEP_COUNT=$((STEP_COUNT + 1))
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}[Step ${STEP_COUNT}/${STEPS_TOTAL}]${NC} $1"
    echo -e "${BLUE}========================================${NC}"
}

usage() {
    cat <<EOF
${PROJECT_NAME} ${VERSION} Release Packaging Script

USAGE:
    $(basename "$0") [OPTIONS]

OPTIONS:
    -h, --help          Show this help message
    --dry-run           Preview actions without executing
    --clean             Clean previous artifacts before building

OUTPUT:
    target/artifacts/${PROJECT_NAME}-${VERSION}-{os}-{arch}.tar.gz
EOF
}

DRY_RUN=false
CLEAN=false
PLATFORM=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        --dry-run) DRY_RUN=true ;;
        --clean) CLEAN=true ;;
        *) log_error "Unknown option: $1"; usage; exit 1 ;;
    esac
    shift
done

# ============================================================================
# Step 1: Platform Detection
# ============================================================================
progress "Platform Detection"

if [[ -n "$PLATFORM" ]]; then
    OS_NAME="${PLATFORM%-*}"
    ARCH_NAME="${PLATFORM#*-}"
else
    case "$(uname -s)" in
        Linux*) OS_NAME="linux" ;;
        Darwin*) OS_NAME="macos" ;;
        MINGW*|MSYS*|CYGWIN*) OS_NAME="windows" ;;
        *) log_error "Unsupported OS: $(uname -s)"; exit 1 ;;
    esac
    case "$(uname -m)" in
        x86_64) ARCH_NAME="x86_64" ;;
        aarch64|arm64) ARCH_NAME="aarch64" ;;
        *) log_error "Unsupported arch: $(uname -m)"; exit 1 ;;
    esac
fi

ARTIFACT_NAME="${PROJECT_NAME}-${VERSION}-${OS_NAME}-${ARCH_NAME}.tar.gz"
ARTIFACT_PATH="${ARTIFACT_DIR}/${ARTIFACT_NAME}"

log_info "Platform: ${OS_NAME}-${ARCH_NAME}"
log_info "Artifact: ${ARTIFACT_NAME}"

# ============================================================================
# Step 2: Pre-flight Checks
# ============================================================================
progress "Pre-flight Checks"

run_cmd() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] $*"
    else
        "$@"
    fi
}

if command -v cargo &>/dev/null; then
    log_success "cargo found: $(cargo --version)"
else
    log_error "cargo not found in PATH"; exit 1
fi

# ============================================================================
# Step 3: Clean (optional)
# ============================================================================
if [[ "$CLEAN" == "true" ]]; then
    progress "Cleaning Previous Artifacts"
    run_cmd rm -rf "${ARTIFACT_DIR}"
    run_cmd rm -rf "${BUILD_DIR}"
fi

# ============================================================================
# Step 4: Build Release
# ============================================================================
progress "Building Release Binary"

run_cmd cargo build --release --features "${CARGO_FEATURES}"

if [[ -f "${BUILD_DIR}/ed2kia" ]] || [[ -f "${BUILD_DIR}/ed2kia.exe" ]]; then
    log_success "Release binary built successfully"
else
    log_error "Release binary not found"; exit 1
fi

# ============================================================================
# Step 5: Package Artifacts
# ============================================================================
progress "Packaging Artifacts"

mkdir -p "${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}"

# Copy binary
if [[ -f "${BUILD_DIR}/ed2kia" ]]; then
    cp "${BUILD_DIR}/ed2kia" "${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}/"
elif [[ -f "${BUILD_DIR}/ed2kia.exe" ]]; then
    cp "${BUILD_DIR}/ed2kia.exe" "${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}/"
fi

# Copy docs and configs
cp -r "${ROOT_DIR}/docs" "${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}/" 2>/dev/null || true
cp "${ROOT_DIR}/LICENSE" "${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}/" 2>/dev/null || true
cp "${ROOT_DIR}/README.md" "${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}/" 2>/dev/null || true
cp -r "${ROOT_DIR}/deploy" "${ARTIFACT_DIR}/${PROJECT_NAME}-${VERSION}/" 2>/dev/null || true

# Create tarball
cd "${ARTIFACT_DIR}"
if [[ "$DRY_RUN" != "true" ]]; then
    tar -czf "${ARTIFACT_NAME}" "${PROJECT_NAME}-${VERSION}/"
    rm -rf "${PROJECT_NAME}-${VERSION}"
fi
cd "${ROOT_DIR}"

log_success "Artifact packaged: ${ARTIFACT_PATH}"

# ============================================================================
# Step 6: Generate Checksums
# ============================================================================
progress "Generating Checksums"

if [[ "$DRY_RUN" != "true" ]] && [[ -f "${ARTIFACT_PATH}" ]]; then
    if command -v sha256sum &>/dev/null; then
        sha256sum "${ARTIFACT_PATH}" > "${ARTIFACT_DIR}/checksums.sha256"
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "${ARTIFACT_PATH}" > "${ARTIFACT_DIR}/checksums.sha256"
    fi
    log_success "Checksums saved to checksums.sha256"
fi

# ============================================================================
# Summary
# ============================================================================
echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Release Packaging Complete${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "Version:    ${VERSION}"
echo -e "Platform:   ${OS_NAME}-${ARCH_NAME}"
echo -e "Artifact:   ${ARTIFACT_PATH}"
echo -e "Features:   ${CARGO_FEATURES}"
