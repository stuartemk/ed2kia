#!/bin/sh
# ed2kIA v9.18.0 — Distributed SAE Audit Network Installer (POSIX compliant)
# Usage: curl -sSf https://ed2kia.network/install.sh | sh
set -e

ARCH=$(uname -m)
OS=$(uname -s)

echo "🌍 ed2kIA v9.18.0 — Distributed SAE Audit Network for Local LLMs"
echo "⚙️ Detecting environment: ${OS} / ${ARCH}"

if command -v cargo >/dev/null 2>&1; then
  echo "📦 Building optimized binary from source..."
  cargo build --release --features v9.18-mvp-deployment
  if command -v sudo >/dev/null 2>&1; then
    sudo cp target/release/ed2k /usr/local/bin/ed2k 2>/dev/null && echo "✅ Installed to /usr/local/bin/ed2k"
  else
    mkdir -p ~/.local/bin
    cp target/release/ed2k ~/.local/bin/ed2k
    echo "✅ Installed to ~/.local/bin/ed2k"
  fi
  echo "🚀 Run: ed2k start --model qwen3.5:2b"
else
  echo "⚠️ Cargo not found. Downloading precompiled binary..."
  BIN_URL="https://github.com/Stuartemk/ed2kIA/releases/download/v9.18.0-sprint82/ed2k-${ARCH}.tar.gz"
  if command -v curl >/dev/null 2>&1; then
    curl -sL "${BIN_URL}" | tar xz -C /tmp 2>/dev/null || {
      echo "❌ Binary download failed. Please install Rust/cargo and try again."
      exit 1
    }
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "${BIN_URL}" | tar xz -C /tmp 2>/dev/null || {
      echo "❌ Binary download failed. Please install Rust/cargo and try again."
      exit 1
    }
  else
    echo "❌ Neither curl nor wget found. Please install Rust/cargo."
    exit 1
  fi
  if command -v sudo >/dev/null 2>&1; then
    sudo cp /tmp/ed2k /usr/local/bin/ed2k 2>/dev/null && echo "✅ Installed to /usr/local/bin/ed2k"
  else
    mkdir -p ~/.local/bin
    cp /tmp/ed2k ~/.local/bin/ed2k
    echo "✅ Installed to ~/.local/bin/ed2k"
  fi
  echo "🚀 Run: ed2k start --model qwen3.5:2b"
fi
