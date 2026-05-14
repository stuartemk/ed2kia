#!/bin/bash
# ed2kIA Release Packager
# Genera binarios estáticos, incluye web/, deploy/, docs/ y firma con Ed25519.
# Uso: ./packager.sh [--package|--sign|--checksums]

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuración
VERSION="${ED2KIA_VERSION:-0.5.0}"
PROJECT_NAME="ed2kIA"
RELEASE_DIR="./release"
BUILD_DIR="./release/build"
OUTPUT_DIR="./release/dist"
KEY_FILE="${ED2KIA_SIGNING_KEY:-./release/signing.key}"

# Targets soportados
TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    "x86_64-pc-windows-msvc"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

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

# Verificar dependencias
check_dependencies() {
    log_info "Verificando dependencias..."

    if ! command -v cargo &> /dev/null; then
        log_error "Cargo no encontrado. Instala Rust: https://rustup.rs/"
        exit 1
    fi

    if ! command -v tar &> /dev/null; then
        log_error "tar no encontrado"
        exit 1
    fi

    if ! command -v sha256sum &> /dev/null; then
        log_error "sha256sum no encontrado"
        exit 1
    fi

    log_success "Dependencias verificadas"
}

# Build para un target específico
build_target() {
    local target=$1
    log_info "Compilando para ${target}..."

    # Instalar target si no existe
    rustup target add "${target}" --allow-no-profile 2>/dev/null || true

    # Build release
    if cargo build --release --target "${target}" 2>&1 | tail -5; then
        log_success "Build exitoso para ${target}"
    else
        log_error "Build fallido para ${target}"
        return 1
    fi
}

# Empaquetar binario para un target
package_target() {
    local target=$1
    local binary_name="ed2kia"
    local src_binary="target/${target}/release/${binary_name}"

    # Windows usa .exe
    if [[ "${target}" == *"windows"* ]]; then
        binary_name="ed2kia.exe"
        src_binary="target/${target}/release/${binary_name}"
    fi

    if [ ! -f "${src_binary}" ]; then
        log_error "Binario no encontrado: ${src_binary}"
        return 1
    fi

    # Determinar nombre del paquete
    local os_arch
    if [[ "${target}" == *"linux-gnu"* ]]; then
        os_arch="linux"
    elif [[ "${target}" == *"linux-musl"* ]]; then
        os_arch="linux-musl"
    elif [[ "${target}" == *"windows"* ]]; then
        os_arch="windows"
    elif [[ "${target}" == *"apple-darwin"* ]]; then
        os_arch="macos"
    fi

    if [[ "${target}" == *aarch64* ]] || [[ "${target}" == *arm64* ]]; then
        os_arch="${os_arch}-arm64"
    else
        os_arch="${os_arch}-amd64"
    fi

    local package_name="${PROJECT_NAME}-${VERSION}-${os_arch}"
    local package_dir="${OUTPUT_DIR}/${package_name}"

    log_info "Empaquetando ${package_name}..."

    # Crear estructura del paquete
    mkdir -p "${package_dir}/bin"
    mkdir -p "${package_dir}/web"
    mkdir -p "${package_dir}/deploy"
    mkdir -p "${package_dir}/docs"

    # Copiar binario
    cp "${src_binary}" "${package_dir}/bin/"

    # Copiar web assets
    if [ -d "./web" ]; then
        cp -r ./web/* "${package_dir}/web/"
    fi

    # Copiar deploy
    if [ -d "./deploy" ]; then
        cp -r ./deploy/* "${package_dir}/deploy/"
    fi

    # Copiar docs
    if [ -d "./docs" ]; then
        cp -r ./docs/* "${package_dir}/docs/"
    fi

    # Copiar LICENSE y README
    if [ -f "./LICENSE" ]; then
        cp ./LICENSE "${package_dir}/"
    fi
    if [ -f "./README.md" ]; then
        cp ./README.md "${package_dir}/"
    fi

    # Generar checksum del binario
    (cd "${package_dir}" && sha256sum "bin/${binary_name}" > "bin/${binary_name}.sha256")

    # Crear archive
    if [[ "${target}" == *"windows"* ]]; then
        (cd "${OUTPUT_DIR}" && zip -r "${package_name}.zip" "${package_name}/")
        log_success "Creado: ${package_name}.zip"
    else
        (cd "${OUTPUT_DIR}" && tar -czf "${package_name}.tar.gz" "${package_name}/")
        log_success "Creado: ${package_name}.tar.gz"
    fi

    # Limpiar directorio temporal
    rm -rf "${package_dir}"
}

# Generar checksums de todos los paquetes
generate_checksums() {
    log_info "Generando checksums..."

    local checksums_file="${OUTPUT_DIR}/checksums.txt"
    > "${checksums_file}"

    for file in "${OUTPUT_DIR}"/*.{tar.gz,zip}; do
        [ -f "${file}" ] || continue
        sha256sum "$(basename "${file}")" >> "${checksums_file}"
    done

    log_success "Checksums guardados en: ${checksums_file}"
}

# Firmar paquetes (placeholder para Ed25519)
sign_packages() {
    if [ ! -f "${KEY_FILE}" ]; then
        log_warn "Key de firma no encontrada: ${KEY_FILE}"
        log_warn "Saltando firma. Genera una key con: ed25519-keygen > ${KEY_FILE}"
        return 0
    fi

    log_info "Firmando paquetes..."

    # TODO: Phase 6 - Implementar firma real con ed25519-dalek
    # Por ahora, generar placeholder
    for file in "${OUTPUT_DIR}"/*.{tar.gz,zip}; do
        [ -f "${file}" ] || continue
        echo "SIGNATURE_PLACEHOLDER:$(basename "${file}")" > "${file}.sig"
    done

    log_success "Paquetes firmados (placeholder)"
}

# Comando: package
do_package() {
    log_info "=== ed2kIA Release Packager v${VERSION} ==="
    log_info "Target: Todos los targets soportados"

    check_dependencies

    # Limpiar directorios
    rm -rf "${RELEASE_DIR}"
    mkdir -p "${OUTPUT_DIR}"

    # Build y package para cada target
    local failed=0
    for target in "${TARGETS[@]}"; do
        if build_target "${target}"; then
            package_target "${target}" || ((failed++)) || true
        else
            ((failed++)) || true
        fi
    done

    # Generar checksums
    generate_checksums

    # Firmar
    sign_packages

    # Resumen
    echo ""
    log_info "=== Resumen ==="
    log_info "Version: ${VERSION}"
    log_info "Output: ${OUTPUT_DIR}"
    log_info "Failed targets: ${failed}"
    echo ""
    ls -la "${OUTPUT_DIR}/"

    if [ ${failed} -eq 0 ]; then
        log_success "¡Release completado exitosamente!"
    else
        log_warn "${failed} target(s) fallaron. Revisa los logs."
    fi
}

# Comando: checksums
do_checksums() {
    if [ ! -d "${OUTPUT_DIR}" ]; then
        log_error "Directorio de release no encontrado. Ejecuta --package primero."
        exit 1
    fi
    generate_checksums
}

# Comando: sign
do_sign() {
    if [ ! -d "${OUTPUT_DIR}" ]; then
        log_error "Directorio de release no encontrado. Ejecuta --package primero."
        exit 1
    fi
    sign_packages
}

# Main
main() {
    local command="${1:---package}"

    case "${command}" in
        --package)
            do_package
            ;;
        --sign)
            do_sign
            ;;
        --checksums)
            do_checksums
            ;;
        --help|-h)
            echo "ed2kIA Release Packager v${VERSION}"
            echo ""
            echo "Uso: ./packager.sh [COMANDO]"
            echo ""
            echo "Comandos:"
            echo "  --package    Build y empaquetar todos los targets (default)"
            echo "  --sign       Firmar paquetes existentes"
            echo "  --checksums  Generar checksums"
            echo "  --help       Mostrar esta ayuda"
            echo ""
            echo "Variables de entorno:"
            echo "  ED2KIA_VERSION   Version del release (default: 0.5.0)"
            echo "  ED2KIA_SIGNING_KEY  Path a la key de firma Ed25519"
            ;;
        *)
            log_error "Comando desconocido: ${command}"
            echo "Usa --help para ver opciones"
            exit 1
            ;;
    esac
}

main "$@"
