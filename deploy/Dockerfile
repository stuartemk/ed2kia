# =============================================================================
# ed2kIA - Multi-stage Dockerfile
# =============================================================================
# Stage 1: Build
# Stage 2: Runtime (minimal image)
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM rust:1.75-slim-bookworm AS builder

# Instalar dependencias del sistema
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Crear directorio de trabajo
WORKDIR /usr/src/ed2kia

# Copiar archivos de manifiesto primero (cache de dependencias)
COPY Cargo.toml Cargo.lock* ./

# Crear archivo dummy para permitir pre-build de dependencias
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copiar código fuente real
COPY src ./src

# Construir binario optimizado con features de producción
ARG FEATURES="stable,v2.1-neuroplasticity,v2.1-steering-bridge,v2.1-quantum-feedback,v2.1-prod-readiness,v2.1-observability"
RUN cargo build --release --bin ed2kia --features "${FEATURES}" && \
    cargo clean --release && \
    echo "Build complete: $(/usr/src/ed2kia/target/release/ed2kia --version 2>/dev/null || echo 'ok')"

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Metadatos
LABEL org.opencontainers.image.title="ed2kIA"
LABEL org.opencontainers.image.description="Decentralized Distributed Interpretability Network"
LABEL org.opencontainers.image.license="Apache-2.0"
LABEL org.opencontainers.image.source="https://github.com/ed2kia/ed2kia"

# Instalar dependencias mínimas
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* && \
    useradd --create-home --shell /bin/bash ed2kia

# Copiar binario del builder
COPY --from=builder /usr/src/ed2kia/target/release/ed2kia /usr/local/bin/ed2kia

# Crear directorios de datos
RUN mkdir -p /data/ed2kia/models \
             /data/ed2kia/config \
             /data/ed2kia/logs \
             /data/ed2kia/wasm && \
    chown -R ed2kia:ed2kia /data/ed2kia

# Cambiar a usuario no-root
USER ed2kia

WORKDIR /data/ed2kia

# Puertos
EXPOSE 9000/tcp
EXPOSE 9000/udp

# Variables de entorno
ENV RUST_LOG=ed2kia=info
ENV DATA_DIR=/data/ed2kia
ENV MODEL_DIR=/data/ed2kia/models
ENV WASM_DIR=/data/ed2kia/wasm

# Healthcheck
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ed2kia --help || exit 1

# Comando por defecto
ENTRYPOINT ["ed2kia"]
CMD ["--port", "9000", "join"]

# -----------------------------------------------------------------------------
# Stage 3: Debug (opcional, para desarrollo)
# -----------------------------------------------------------------------------
FROM builder AS debug

# Instalar herramientas de debugging
RUN apt-get update && apt-get install -y \
    gdb \
    strace \
    tcpdump \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/ed2kia
CMD ["cargo", "run", "--release"]
