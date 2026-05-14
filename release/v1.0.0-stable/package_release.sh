#!/usr/bin/env bash
# ed2kIA v1.0.0 STABLE - Release Packaging Script
# POSIX compliant, no external dependencies beyond standard Unix tools
set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================
VERSION="1.0.0"
STABLE_TAG="v${VERSION}"
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
${PROJECT_NAME} ${STABLE_TAG} Release Packaging Script

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
        ${PROJECT_NAME}-${STABLE_TAG}-{os}-{arch}.tar.gz

EOF
    exit 0
}

# ============================================================================
# Platform Detection
# ============================================================================

detect_platform() {
    progress "Detecting platform"

    if [[ -n "${OVERRIDE_PLATFORM:-}" ]]; then
        OS="$(echo "$OVERRIDE_PLATFORM" | cut -d'-' -f1)"
        ARCH="$(echo "$OVERRIDE_PLATFORM" | cut -d'-' -f2)"
        log_warn "Using overridden platform: ${OS}-${ARCH}"
        return 0
    fi

    local raw_os
    raw_os="$(uname -s)"
    local raw_arch
    raw_arch="$(uname -m)"

    case "$raw_os" in
        Linux*)
            OS="linux"
            ;;
        Darwin*)
            OS="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*|Windows*)
            OS="windows"
            ;;
        *)
            log_error "Unsupported operating system: $raw_os"
            echo "Supported platforms: Linux, macOS, Windows"
            exit 1
            ;;
    esac

    case "$raw_arch" in
        x86_64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        armv7l)
            ARCH="armv7"
            ;;
        *)
            log_warn "Unknown architecture: $raw_arch, defaulting to x86_64"
            ARCH="x86_64"
            ;;
    esac

    log_success "Platform detected: ${OS}-${ARCH}"
}

# ============================================================================
# Build Release
# ============================================================================

build_release() {
    progress "Building release binaries"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would execute: cargo build --release --features ${CARGO_FEATURES}"
        return 0
    fi

    log_info "Running: cargo build --release --features ${CARGO_FEATURES}"

    if ! cargo build --release --features "${CARGO_FEATURES}" 2>&1 | tee "${ROOT_DIR}/cargo_release_build.log"; then
        log_error "Build failed! Check ${ROOT_DIR}/cargo_release_build.log for details"
        exit 1
    fi

    # Verify binary exists
    local binary_name="${PROJECT_NAME}"
    if [[ "$OS" == "windows" ]]; then
        binary_name="${PROJECT_NAME}.exe"
    fi

    if [[ ! -f "${BUILD_DIR}/${binary_name}" ]]; then
        log_error "Binary not found at ${BUILD_DIR}/${binary_name}"
        exit 1
    fi

    log_success "Build completed successfully"
    log_info "Binary: ${BUILD_DIR}/${binary_name}"
}

# ============================================================================
# Generate Checksums
# ============================================================================

generate_checksums() {
    progress "Generating SHA-256 checksums"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would generate SHA-256 checksums for binaries in ${BUILD_DIR}"
        return 0
    fi

    local checksum_file="${ARTIFACT_DIR}/checksums.sha256"
    local binary_name="${PROJECT_NAME}"
    if [[ "$OS" == "windows" ]]; then
        binary_name="${PROJECT_NAME}.exe"
    fi

    # Detect available checksum tool
    local sha_cmd=""
    if command -v sha256sum &>/dev/null; then
        sha_cmd="sha256sum"
        log_info "Using sha256sum"
    elif command -v shasum &>/dev/null; then
        sha_cmd="shasum"
        log_info "Using shasum -a 256"
    else
        log_error "No SHA-256 tool found (sha256sum or shasum required)"
        exit 1
    fi

    # Generate checksums
    echo "# ${PROJECT_NAME} ${STABLE_TAG} Release Checksums" > "$checksum_file"
    echo "# Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> "$checksum_file"
    echo "# Platform: ${OS}-${ARCH}" >> "$checksum_file"
    echo "" >> "$checksum_file"

    if [[ "$sha_cmd" == "sha256sum" ]]; then
        (cd "$BUILD_DIR" && sha256sum "$binary_name") >> "$checksum_file"
    else
        (cd "$BUILD_DIR" && shasum -a 256 "$binary_name") >> "$checksum_file"
    fi

    log_success "Checksums generated: ${checksum_file}"
    cat "$checksum_file"
}

# ============================================================================
# Create Tarball
# ============================================================================

create_tarball() {
    progress "Creating release tarball"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would create tarball: ${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}.tar.gz"
        log_info "[DRY-RUN] Contents would include:"
        log_info "  - Binary from target/release/"
        log_info "  - docs/ directory"
        log_info "  - release/ directory"
        log_info "  - web/ directory"
        log_info "  - LICENSE file"
        log_info "  - README.md"
        return 0
    fi

    # Create artifact directory
    mkdir -p "$ARTIFACT_DIR"

    # Create staging directory
    local staging_dir="${ARTIFACT_DIR}/${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}"
    mkdir -p "$staging_dir"

    local binary_name="${PROJECT_NAME}"
    if [[ "$OS" == "windows" ]]; then
        binary_name="${PROJECT_NAME}.exe"
    fi

    # Copy binary
    log_info "Copying binary to staging..."
    cp "${BUILD_DIR}/${binary_name}" "${staging_dir}/"

    # Copy documentation
    if [[ -d "${ROOT_DIR}/docs" ]]; then
        log_info "Copying docs/ directory..."
        cp -r "${ROOT_DIR}/docs" "${staging_dir}/"
    fi

    # Copy release directory
    if [[ -d "${ROOT_DIR}/release" ]]; then
        log_info "Copying release/ directory..."
        cp -r "${ROOT_DIR}/release" "${staging_dir}/"
    fi

    # Copy web directory
    if [[ -d "${ROOT_DIR}/web" ]]; then
        log_info "Copying web/ directory..."
        cp -r "${ROOT_DIR}/web" "${staging_dir}/"
    fi

    # Copy LICENSE
    if [[ -f "${ROOT_DIR}/LICENSE" ]]; then
        log_info "Copying LICENSE..."
        cp "${ROOT_DIR}/LICENSE" "${staging_dir}/"
    fi

    # Copy README.md
    if [[ -f "${ROOT_DIR}/README.md" ]]; then
        log_info "Copying README.md..."
        cp "${ROOT_DIR}/README.md" "${staging_dir}/"
    fi

    # Copy checksums if generated
    local checksum_file="${ARTIFACT_DIR}/checksums.sha256"
    if [[ -f "$checksum_file" ]]; then
        cp "$checksum_file" "${staging_dir}/"
    fi

    # Create tarball
    local tarball_name="${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}.tar.gz"
    log_info "Creating tarball: ${tarball_name}"

    (cd "$ARTIFACT_DIR" && tar -czf "$tarball_name" "${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}")

    # Clean up staging directory
    rm -rf "$staging_dir"

    log_success "Tarball created: ${ARTIFACT_DIR}/${tarball_name}"

    # Show tarball size
    local size
    size="$(du -h "${ARTIFACT_DIR}/${tarball_name}" | cut -f1)"
    log_info "Tarball size: ${size}"
}

# ============================================================================
# Validate Package
# ============================================================================

validate_package() {
    progress "Validating package integrity"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would validate tarball integrity"
        return 0
    fi

    local tarball_name="${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}.tar.gz"
    local tarball_path="${ARTIFACT_DIR}/${tarball_name}"

    if [[ ! -f "$tarball_path" ]]; then
        log_error "Tarball not found: ${tarball_path}"
        exit 1
    fi

    # Create validation directory
    local validate_dir="${ARTIFACT_DIR}/validate_$$"
    mkdir -p "$validate_dir"

    # Extract tarball
    log_info "Extracting tarball for validation..."
    if ! (cd "$validate_dir" && tar -xzf "$tarball_path"); then
        log_error "Failed to extract tarball - integrity check failed"
        rm -rf "$validate_dir"
        exit 1
    fi

    local errors=0

    # Check for binary
    local binary_name="${PROJECT_NAME}"
    if [[ "$OS" == "windows" ]]; then
        binary_name="${PROJECT_NAME}.exe"
    fi

    local binary_path="${validate_dir}/${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}/${binary_name}"
    if [[ -f "$binary_path" ]]; then
        log_success "Binary found in tarball"
    else
        log_error "Binary missing from tarball: ${binary_path}"
        errors=$((errors + 1))
    fi

    # Check for LICENSE
    if [[ -f "${validate_dir}/${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}/LICENSE" ]]; then
        log_success "LICENSE found in tarball"
    else
        log_error "LICENSE missing from tarball"
        errors=$((errors + 1))
    fi

    # Check for README.md
    if [[ -f "${validate_dir}/${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}/README.md" ]]; then
        log_success "README.md found in tarball"
    else
        log_error "README.md missing from tarball"
        errors=$((errors + 1))
    fi

    # Check for docs directory
    if [[ -d "${validate_dir}/${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}/docs" ]]; then
        log_success "docs/ directory found in tarball"
    else
        log_warn "docs/ directory missing from tarball"
    fi

    # Check for web directory
    if [[ -d "${validate_dir}/${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}/web" ]]; then
        log_success "web/ directory found in tarball"
    else
        log_warn "web/ directory missing from tarball"
    fi

    # Check for release directory
    if [[ -d "${validate_dir}/${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}/release" ]]; then
        log_success "release/ directory found in tarball"
    else
        log_warn "release/ directory missing from tarball"
    fi

    # Clean up validation directory
    rm -rf "$validate_dir"

    if [[ $errors -gt 0 ]]; then
        log_error "Validation failed with ${errors} error(s)"
        exit 1
    fi

    log_success "Package validation passed"
}

# ============================================================================
# Docker Preparation (Placeholder)
# ============================================================================

docker_prep() {
    progress "Docker preparation notes"

    cat <<'DOCKER_NOTES'
=== Docker Image Preparation ===

This section contains notes for building Docker images from the release.

Prerequisites:
  - Docker installed and running
  - Docker Compose (optional, for multi-container setups)

Build Commands:
  # Build Docker image from release tarball
  docker build -t ed2kia:v1.0.0-stable -f deploy/Dockerfile .

  # Tag for registry
  docker tag ed2kia:v1.0.0-stable registry.example.com/ed2kia:v1.0.0-stable

  # Push to registry
  docker push registry.example.com/ed2kia:v1.0.0-stable

Multi-Platform Build (requires Docker Buildx):
  docker buildx build --platform linux/amd64,linux/arm64 \
    -t ed2kia:v1.0.0-stable --push .

Docker Compose:
  # Start services
  docker-compose -f deploy/docker-compose.yml up -d

  # View logs
  docker-compose -f deploy/docker-compose.yml logs -f

  # Stop services
  docker-compose -f deploy/docker-compose.yml down

Notes:
  - The Dockerfile is located at: deploy/Dockerfile
  - Docker Compose configuration: deploy/docker-compose.yml
  - Systemd service files: deploy/systemd/
  - Environment configuration: deploy/systemd/ed2kia.env

For production deployment, review:
  - Resource limits in docker-compose.yml
  - Security settings in Dockerfile
  - Network configuration
  - Volume mounts for persistent data
DOCKER_NOTES

    log_success "Docker preparation notes displayed"
}

# ============================================================================
# Cross-Compilation Placeholders
# ============================================================================

cross_compile_linux() {
    log_info "Cross-compilation for Linux (placeholder)"
    cat <<'CROSS_LINUX'

=== Linux Cross-Compilation ===

To cross-compile for Linux from macOS/Windows:

1. Install cross-compilation toolchain:
   # For x86_64-unknown-linux-gnu
   rustup target add x86_64-unknown-linux-gnu

   # For aarch64-unknown-linux-gnu
   rustup target add aarch64-unknown-linux-gnu

2. Build with target:
   cargo build --release --target x86_64-unknown-linux-gnu --features stable

3. Static linking (optional):
   cargo build --release --target x86_64-unknown-linux-musl --features stable

Note: musl target requires musl-tools or equivalent.
CROSS_LINUX
}

cross_compile_macos() {
    log_info "Cross-compilation for macOS (placeholder)"
    cat <<'CROSS_MACOS'

=== macOS Cross-Compilation ===

To cross-compile for macOS from Linux:

1. Install osxcross toolchain (Linux host):
   # Follow instructions at: https://github.com/tpoechtrager/osxcross

2. Add macOS target:
   rustup target add x86_64-apple-darwin
   rustup target add aarch64-apple-darwin

3. Configure .cargo/config.toml:
   [target.x86_64-apple-darwin]
   linker = "osx-clang"
   arch = "x86_64"

   [target.aarch64-apple-darwin]
   linker = "osx-clang"
   arch = "arm64"

4. Build:
   cargo build --release --target aarch64-apple-darwin --features stable

Note: Native macOS build is recommended when possible.
CROSS_MACOS
}

cross_compile_windows() {
    log_info "Cross-compilation for Windows (placeholder)"
    cat <<'CROSS_WINDOWS'

=== Windows Cross-Compilation ===

To cross-compile for Windows from Linux/macOS:

1. Install mingw toolchain:
   # Ubuntu/Debian
   sudo apt install gcc-mingw-w64

   # macOS
   brew install mingw-w64

2. Add Windows target:
   rustup target add x86_64-pc-windows-gnu

3. Configure .cargo/config.toml:
   [target.x86_64-pc-windows-gnu]
   linker = "x86_64-w64-mingw32-gcc"

4. Build:
   cargo build --release --target x86_64-pc-windows-gnu --features stable

Note: For native Windows builds, use MSVC toolchain with Visual Studio.
CROSS_WINDOWS
}

# ============================================================================
# CI/CD Integration Notes
# ============================================================================

show_ci_notes() {
    log_info "CI/CD Integration Notes"
    cat <<'CI_NOTES'

=== GitHub Actions Integration ===

This script is designed to integrate with .github/workflows/ci_cd_stable.yml

Example workflow steps:

  name: Release Pipeline
  on:
    push:
      tags:
        - 'v1.0.0*'

  jobs:
    build-and-release:
      runs-on: ${{ matrix.os }}
      strategy:
        matrix:
          os: [ubuntu-latest, macos-latest, windows-latest]

      steps:
        - uses: actions/checkout@v4

        - name: Setup Rust
          uses: dtolnay/rust-toolchain@stable
          with:
            targets: ${{ matrix.target }}

        - name: Build Release
          run: |
            bash release/v1.0.0-stable/package_release.sh

        - name: Upload Artifacts
          uses: actions/upload-artifact@v4
          with:
            name: ed2kIA-${{ matrix.os }}
            path: target/artifacts/

        - name: Create Release
          uses: softprops/action-gh-release@v1
          with:
            files: |
              target/artifacts/*.tar.gz
              target/artifacts/checksums.sha256
          env:
            GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

For full workflow, see: .github/workflows/ci_cd_stable.yml
CI_NOTES
}

# ============================================================================
# Main Orchestration
# ============================================================================

main() {
    # Parse arguments
    DRY_RUN="false"
    CLEAN="false"
    OVERRIDE_PLATFORM=""

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                ;;
            --dry-run)
                DRY_RUN="true"
                log_warn "Running in dry-run mode"
                shift
                ;;
            --clean)
                CLEAN="true"
                shift
                ;;
            --platform)
                OVERRIDE_PLATFORM="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Use -h or --help for usage information"
                exit 1
                ;;
        esac
    done

    echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}  ${YELLOW}${PROJECT_NAME} ${STABLE_TAG} Release Packaging${NC}              ${GREEN}║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Clean previous artifacts if requested
    if [[ "$CLEAN" == "true" ]]; then
        log_info "Cleaning previous artifacts..."
        if [[ "$DRY_RUN" != "true" ]]; then
            rm -rf "${ROOT_DIR}/target/artifacts"
            log_success "Artifacts cleaned"
        else
            log_info "[DRY-RUN] Would clean: ${ROOT_DIR}/target/artifacts"
        fi
    fi

    # Step 1: Detect platform
    detect_platform

    # Step 2: Build release
    build_release

    # Step 3: Generate checksums
    generate_checksums

    # Step 4: Create tarball
    create_tarball

    # Step 5: Validate package
    validate_package

    # Step 6: Docker prep notes
    docker_prep

    # Summary
    echo -e "\n${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}  ${GREEN}Release Packaging Complete${NC}                         ${GREEN}║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    log_info "Platform: ${OS}-${ARCH}"
    log_info "Version: ${STABLE_TAG}"

    if [[ "$DRY_RUN" != "true" ]]; then
        local tarball_name="${PROJECT_NAME}-${STABLE_TAG}-${OS}-${ARCH}.tar.gz"
        log_info "Tarball: ${ARTIFACT_DIR}/${tarball_name}"
        log_info "Checksums: ${ARTIFACT_DIR}/checksums.sha256"

        echo ""
        log_info "Next steps:"
        log_info "  1. Review tarball contents"
        log_info "  2. Test installation from tarball"
        log_info "  3. Sign release (if applicable)"
        log_info "  4. Upload to release repository"
        log_info "  5. Update documentation"
    else
        log_warn "This was a dry-run. No files were created."
    fi

    echo ""
    log_success "Done!"
}

# Run main with all arguments
main "$@"
