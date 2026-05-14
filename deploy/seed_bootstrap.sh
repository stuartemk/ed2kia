#!/usr/bin/env bash
set -euo pipefail
# ed2kIA Seed Node Bootstrap Script
# ==================================
# Generates deterministic PeerIds from seed phrases, creates config, starts nodes.
#
# Usage:
#   ./seed_bootstrap.sh [--config seed_config.toml] [--dry-run]
#
# Requirements:
#   - POSIX-compliant environment (Linux/macOS)
#   - ed25519-dalek CLI or openssl for key generation (placeholder)
#   - ed2kIA binary in PATH or specified via ED2KIA_BIN
#
# Environment Variables:
#   ED2KIA_BIN    - Path to ed2kIA binary (default: ed2kIA)
#   ED2KIA_DATA   - Data directory for keys/config (default: ./data)

# ─── Defaults ────────────────────────────────────────────────────────────────
CONFIG_FILE="seed_config.toml"
DRY_RUN=false
ED2KIA_BIN="${ED2KIA_BIN:-ed2kIA}"
ED2KIA_DATA="${ED2KIA_DATA:-./data}"
HEALTH_CHECK_RETRIES=10
HEALTH_CHECK_INTERVAL=3

# ─── Colors (disabled if not a terminal) ─────────────────────────────────────
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  BLUE=''
  NC=''
fi

# ─── Logging ─────────────────────────────────────────────────────────────────
log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ─── Usage ───────────────────────────────────────────────────────────────────
usage() {
  cat <<EOF
ed2kIA Seed Node Bootstrap Script

Usage:
  \$0 [--config seed_config.toml] [--dry-run]

Options:
  --config FILE   Path to seed configuration TOML file (default: seed_config.toml)
  --dry-run       Validate configuration without starting nodes
  -h, --help      Show this help message

Environment Variables:
  ED2KIA_BIN      Path to ed2kIA binary (default: ed2kIA)
  ED2KIA_DATA     Data directory for keys/config (default: ./data)

Example:
  \$0 --config /etc/ed2kIA/seed_config.toml
  \$0 --dry-run --config seed_config.toml
EOF
}

# ─── Parse Arguments ─────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --config)
      CONFIG_FILE="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      log_error "Unknown option: $1"
      usage
      exit 1
      ;;
  esac
done

# ─── Validate Config File Exists ─────────────────────────────────────────────
if [ ! -f "$CONFIG_FILE" ]; then
  log_error "Configuration file not found: $CONFIG_FILE"
  log_info "Copy deploy/seed_config.example.toml to $CONFIG_FILE and fill in the values."
  exit 1
fi

log_info "Using configuration: $CONFIG_FILE"

# ─── TOML Parser (Minimal, POSIX-Compatible) ─────────────────────────────────
# Extracts values from simple TOML files (no arrays, nested tables only)
toml_get() {
  local file="$1"
  local section="$2"
  local key="$3"
  local in_section=false

  while IFS= read -r line; do
    # Skip comments and empty lines
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// }" ]] && continue

    # Check for section header
    if [[ "$line" =~ ^\[([a-zA-Z0-9_]+)\] ]]; then
      if [[ "${BASH_REMATCH[1]}" == "$section" ]]; then
        in_section=true
      else
        if $in_section; then
          break
        fi
      fi
      continue
    fi

    # Extract key = value in target section
    if $in_section; then
      if [[ "$line" =~ ^[[:space:]]*${key}[[:space:]]*=[[:space:]]*\"(.*)\"[[:space:]]*$ ]]; then
        echo "${BASH_REMATCH[1]}"
        return 0
      elif [[ "$line" =~ ^[[:space:]]*${key}[[:space:]]*=[[:space:]]*([a-zA-Z0-9_]+)[[:space:]]*$ ]]; then
        echo "${BASH_REMATCH[1]}"
        return 0
      fi
    fi
  done < "$file"
  return 1
}

# ─── Generate Deterministic ed25519 Keys ─────────────────────────────────────
# Uses seed phrase to generate deterministic keys via ed25519-dalek or openssl
# Returns: private_key (hex), public_key (hex), peer_id (base58)
generate_keys() {
  local seed_phrase="$1"
  local node_id="$2"
  local key_dir="${ED2KIA_DATA}/keys/${node_id}"

  mkdir -p "$key_dir"

  local priv_key_file="${key_dir}/private.key"
  local pub_key_file="${key_dir}/public.key"
  local peer_id_file="${key_dir}/peer_id.txt"

  # Check if keys already exist (deterministic = same seed = same keys)
  if [ -f "$priv_key_file" ] && [ -f "$pub_key_file" ] && [ -f "$peer_id_file" ]; then
    log_info "Keys already exist for node $node_id (reusing)"
    cat "$priv_key_file"
    echo "---"
    cat "$pub_key_file"
    echo "---"
    cat "$peer_id_file"
    return 0
  fi

  log_info "Generating deterministic ed25519 keys for node $node_id"

  # Try ed25519-dalek CLI first, then openssl, then placeholder
  if command -v ed25519-dalek &>/dev/null; then
    # ed25519-dalek key generation from seed
    local seed_hex
    seed_hex=$(echo -n "$seed_phrase" | sha512sum | awk '{print $1}')
    local priv_key pub_key peer_id

    priv_key=$(ed25519-dalek keygen --seed "$seed_hex" 2>/dev/null | head -1)
    pub_key=$(ed25519-dalek pubkey --seed "$seed_hex" 2>/dev/null | head -1)
    peer_id="12D3Koo${priv_key:0:46}"  # Libp2p PeerId format (simplified)

    echo "$priv_key" > "$priv_key_file"
    echo "$pub_key" > "$pub_key_file"
    echo "$peer_id" > "$peer_id_file"

    chmod 600 "$priv_key_file"

    cat "$priv_key_file"
    echo "---"
    cat "$pub_key_file"
    echo "---"
    cat "$peer_id_file"

  elif command -v openssl &>/dev/null; then
    # Fallback: Generate ed25519 keys using openssl (non-deterministic)
    log_warn "ed25519-dalek not found, using openssl (non-deterministic keys)"

    local priv_key pub_key peer_id

    priv_key=$(openssl genpkey -algorithm ED25519 2>/dev/null | openssl pkey -outform DER 2>/dev/null | xxd -p | tr -d '\n')
    pub_key=$(openssl genpkey -algorithm ED25519 2>/dev/null | openssl pkey -pubout -outform DER 2>/dev/null | xxd -p | tr -d '\n')
    peer_id="12D3Koo$(echo -n "$pub_key" | sha256sum | awk '{print $1}' | head -c 46)"

    echo "$priv_key" > "$priv_key_file"
    echo "$pub_key" > "$pub_key_file"
    echo "$peer_id" > "$peer_id_file"

    chmod 600 "$priv_key_file"

    cat "$priv_key_file"
    echo "---"
    cat "$pub_key_file"
    echo "---"
    cat "$peer_id_file"

  else
    # Placeholder: Generate mock keys for testing
    log_warn "No key generation tool found, using placeholder keys (NOT FOR PRODUCTION)"

    local seed_hash priv_key pub_key peer_id
    seed_hash=$(echo -n "$seed_phrase" | sha256sum | awk '{print $1}')
    priv_key="${seed_hash}${seed_hash}"
    pub_key=$(echo -n "$priv_key" | sha256sum | awk '{print $1}')
    peer_id="12D3Koo$(echo -n "$pub_key" | head -c 46)"

    echo "$priv_key" > "$priv_key_file"
    echo "$pub_key" > "$pub_key_file"
    echo "$peer_id" > "$peer_id_file"

    chmod 600 "$priv_key_file"

    cat "$priv_key_file"
    echo "---"
    cat "$pub_key_file"
    echo "---"
    cat "$peer_id_file"
  fi
}

# ─── Create Node Config from Template ────────────────────────────────────────
create_node_config() {
  local node_id="$1"
  local listen_addr="$2"
  local announce_addr="$3"
  local peer_id="$4"
  local max_peers="$5"
  local block_time_ms="$6"
  local metrics_port="$7"
  local log_level="$8"
  local log_file="$9"

  local config_dir="${ED2KIA_DATA}/config/${node_id}"
  local config_path="${config_dir}/node.toml"

  mkdir -p "$config_dir"

  cat > "$config_path" <<NODE_CONFIG
# ed2kIA Node Configuration - Auto-generated
# Node: ${node_id}
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

[identity]
peer_id = "${peer_id}"
key_path = "${ED2KIA_DATA}/keys/${node_id}/private.key"

[network]
listen_addr = "${listen_addr}"
announce_addr = "${announce_addr}"
protocol = "gossipsub"
max_peers = ${max_peers}

[consensus]
enabled = true
block_time_ms = ${block_time_ms}

[monitoring]
prometheus_enabled = true
metrics_port = ${metrics_port}

[logging]
level = "${log_level}"
file = "${log_file}"
NODE_CONFIG

  log_info "Created node config: $config_path"
  echo "$config_path"
}

# ─── Validate Configuration ──────────────────────────────────────────────────
validate_config() {
  local config_path="$1"
  local errors=0

  log_info "Validating configuration: $config_path"

  # Check required fields exist
  local peer_id listen_addr announce_addr
  peer_id=$(toml_get "$config_path" "identity" "peer_id" || echo "")
  listen_addr=$(toml_get "$config_path" "network" "listen_addr" || echo "")
  announce_addr=$(toml_get "$config_path" "network" "announce_addr" || echo "")

  if [ -z "$peer_id" ]; then
    log_error "Missing required field: identity.peer_id"
    errors=$((errors + 1))
  fi

  if [ -z "$listen_addr" ]; then
    log_error "Missing required field: network.listen_addr"
    errors=$((errors + 1))
  fi

  if [ -z "$announce_addr" ]; then
    log_error "Missing required field: network.announce_addr"
    errors=$((errors + 1))
  fi

  # Check for placeholder values
  if [[ "$announce_addr" == *"<YOUR_IP>"* ]]; then
    log_error "announce_addr contains placeholder <YOUR_IP> - must be replaced with actual IP"
    errors=$((errors + 1))
  fi

  # Verify key file exists
  local key_path
  key_path=$(toml_get "$config_path" "identity" "key_path" || echo "")
  if [ -n "$key_path" ] && [ ! -f "$key_path" ]; then
    log_error "Key file not found: $key_path"
    errors=$((errors + 1))
  fi

  if [ $errors -eq 0 ]; then
    log_ok "Configuration valid"
    return 0
  else
    log_error "Configuration has $errors error(s)"
    return 1
  fi
}

# ─── Health Check ────────────────────────────────────────────────────────────
health_check() {
  local metrics_port="$1"
  local node_id="$2"
  local retry=0

  log_info "Performing health check for node $node_id on port $metrics_port"

  while [ $retry -lt $HEALTH_CHECK_RETRIES ]; do
    if command -v curl &>/dev/null; then
      if curl -s -f "http://localhost:${metrics_port}/metrics" >/dev/null 2>&1; then
        log_ok "Node $node_id is healthy (metrics endpoint responding)"
        return 0
      fi
    elif command -v wget &>/dev/null; then
      if wget -q --spider "http://localhost:${metrics_port}/metrics" 2>/dev/null; then
        log_ok "Node $node_id is healthy (metrics endpoint responding)"
        return 0
      fi
    else
      log_warn "No curl/wget available, skipping HTTP health check"
      # Fallback: Check if process is running
      if pgrep -f "ed2kIA.*${node_id}" >/dev/null 2>&1; then
        log_ok "Node $node_id process is running"
        return 0
      fi
    fi

    retry=$((retry + 1))
    log_info "Health check attempt $retry/$HEALTH_CHECK_RETRIES (waiting ${HEALTH_CHECK_INTERVAL}s...)"
    sleep "$HEALTH_CHECK_INTERVAL"
  done

  log_error "Node $node_id failed health check after $HEALTH_CHECK_RETRIES attempts"
  return 1
}

# ─── Start Node ──────────────────────────────────────────────────────────────
start_node() {
  local config_path="$1"
  local node_id="$2"

  log_info "Starting node $node_id with config: $config_path"

  if [ ! -x "$(command -v "$ED2KIA_BIN" 2>/dev/null)" ]; then
    log_warn "ed2kIA binary not found in PATH ($ED2KIA_BIN)"
    log_info "In production, run: $ED2KIA_BIN --config $config_path"
    return 0
  fi

  # Start node in background
  nohup "$ED2KIA_BIN" --config "$config_path" >> "${ED2KIA_DATA}/logs/${node_id}.log" 2>&1 &
  local pid=$!
  echo "$pid" > "${ED2KIA_DATA}/pids/${node_id}.pid"

  log_ok "Node $node_id started with PID $pid"
}

# ─── Main Bootstrap Logic ────────────────────────────────────────────────────
main() {
  log_info "ed2kIA Seed Node Bootstrap"
  log_info "=========================="

  # Create required directories
  mkdir -p "${ED2KIA_DATA}/keys"
  mkdir -p "${ED2KIA_DATA}/config"
  mkdir -p "${ED2KIA_DATA}/logs"
  mkdir -p "${ED2KIA_DATA}/pids"

  # Read configuration
  local node_id listen_addr announce_addr seed_phrase
  local max_peers block_time_ms metrics_port log_level log_file

  node_id=$(toml_get "$CONFIG_FILE" "node" "id" || echo "")
  listen_addr=$(toml_get "$CONFIG_FILE" "node" "listen_addr" || echo "")
  announce_addr=$(toml_get "$CONFIG_FILE" "node" "announce_addr" || echo "")
  seed_phrase=$(toml_get "$CONFIG_FILE" "node" "seed_phrase" || echo "")
  max_peers=$(toml_get "$CONFIG_FILE" "p2p" "max_peers" || echo "100")
  block_time_ms=$(toml_get "$CONFIG_FILE" "consensus" "block_time_ms" || echo "1000")
  metrics_port=$(toml_get "$CONFIG_FILE" "monitoring" "metrics_port" || echo "9090")
  log_level=$(toml_get "$CONFIG_FILE" "logging" "level" || echo "info")
  log_file=$(toml_get "$CONFIG_FILE" "logging" "file" || echo "${ED2KIA_DATA}/logs/${node_id}.log")

  # Validate required fields
  if [ -z "$node_id" ]; then
    log_error "Missing required field: node.id"
    exit 1
  fi

  if [ -z "$seed_phrase" ] || [ "$seed_phrase" = "<YOUR_SEED_PHRASE>" ]; then
    log_error "Missing or placeholder seed_phrase - must be set for key generation"
    exit 1
  fi

  if [ -z "$listen_addr" ]; then
    log_error "Missing required field: node.listen_addr"
    exit 1
  fi

  if [ -z "$announce_addr" ]; then
    log_error "Missing required field: node.announce_addr"
    exit 1
  fi

  log_info "Node ID: $node_id"
  log_info "Listen: $listen_addr"
  log_info "Announce: $announce_addr"

  # Dry run mode
  if $DRY_RUN; then
    log_info "=== DRY RUN MODE ==="
    log_info "Would generate keys for node: $node_id"
    log_info "Would create config in: ${ED2KIA_DATA}/config/${node_id}/"
    log_info "Would start node on: $listen_addr"
    echo ""
    log_info "Configuration preview:"
    echo "---"
    cat "$CONFIG_FILE"
    echo "---"
    log_ok "Dry run complete - no changes made"
    exit 0
  fi

  # Generate keys
  log_info "Step 1: Generating keys..."
  local key_output priv_key pub_key peer_id
  key_output=$(generate_keys "$seed_phrase" "$node_id")
  priv_key=$(echo "$key_output" | sed -n '1p')
  pub_key=$(echo "$key_output" | sed -n '3p')
  peer_id=$(echo "$key_output" | sed -n '5p' | tr -d '\n')

  log_ok "Keys generated (PeerId: $peer_id)"

  # Create node config
  log_info "Step 2: Creating node configuration..."
  local config_path
  config_path=$(create_node_config \
    "$node_id" \
    "$listen_addr" \
    "$announce_addr" \
    "$peer_id" \
    "$max_peers" \
    "$block_time_ms" \
    "$metrics_port" \
    "$log_level" \
    "$log_file")

  # Validate config
  log_info "Step 3: Validating configuration..."
  if ! validate_config "$config_path"; then
    log_error "Configuration validation failed - aborting"
    exit 1
  fi

  # Start node
  log_info "Step 4: Starting node..."
  start_node "$config_path" "$node_id"

  # Health check
  log_info "Step 5: Health check..."
  if health_check "$metrics_port" "$node_id"; then
    log_ok "Bootstrap complete for node $node_id"
  else
    log_warn "Node started but health check failed - check logs at ${ED2KIA_DATA}/logs/${node_id}.log"
  fi

  log_info "=========================="
  log_info "Node $node_id bootstrap finished"
  log_info "Config: $config_path"
  log_info "Logs: ${ED2KIA_DATA}/logs/${node_id}.log"
  log_info "Metrics: http://localhost:${metrics_port}/metrics"
}

# ─── Entry Point ─────────────────────────────────────────────────────────────
main "$@"
