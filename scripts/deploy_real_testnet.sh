#!/usr/bin/env bash
# =============================================================================
# ed2kIA v9.23.0-sprint87 — Real Testnet Deployment Script
# Sprint 87: The Reality Engine & Zero-Warning Production Core
# =============================================================================
# Deploys 3 nodes on separate VPS with tc/netem latency simulation.
# Usage: ./scripts/deploy_real_testnet.sh [--dry-run]
#
# Prerequisites:
#   - SSH key-based access to 3 VPS (edit NODES below)
#   - Rust toolchain installed on each node
#   - tc (traffic control) available on each node
# =============================================================================

set -euo pipefail

# ─── Configuration ───────────────────────────────────────────────────────────
DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
fi

ED2KIA_VERSION="9.23.0-sprint87"
REPO_URL="https://github.com/ed2kia/ed2kia.git"
BRANCH="main"

# Node configuration: "user@host:ssh_port:node_id:role"
# Roles: bootstrapper, validator, observer
NODES=(
    "root@node1.example.com:22:0:bootstrapper"
    "root@node2.example.com:22:1:validator"
    "root@node3.example.com:22:2:observer"
)

# Latency simulation (tc/netem) between nodes in milliseconds
# Node 0 → Node 1: 50ms + 10ms jitter
# Node 0 → Node 2: 120ms + 20ms jitter
# Node 1 → Node 2: 80ms + 15ms jitter
LATENCY_MATRIX=(
    "0:1:50:10"
    "0:2:120:20"
    "1:2:80:15"
)

# ─── Functions ───────────────────────────────────────────────────────────────

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"
}

run_remote() {
    local user_host="$1"
    local ssh_port="$2"
    local cmd="$3"
    if [[ "$DRY_RUN" == "true" ]]; then
        log "[DRY-RUN] ssh -p $ssh_port $user_host '$cmd'"
    else
        ssh -p "$ssh_port" -o StrictHostKeyChecking=no -o ConnectTimeout=10 "$user_host" "$cmd"
    fi
}

scp_to() {
    local user_host="$1"
    local ssh_port="$2"
    local local_file="$3"
    local remote_path="$4"
    if [[ "$DRY_RUN" == "true" ]]; then
        log "[DRY-RUN] scp -P $ssh_port $local_file $user_host:$remote_path"
    else
        scp -P "$ssh_port" -o StrictHostKeyChecking=no -o ConnectTimeout=10 "$local_file" "$user_host:$remote_path"
    fi
}

# ─── Deployment Steps ────────────────────────────────────────────────────────

deploy_node() {
    local node_spec="$1"
    IFS=':' read -r user_host ssh_port node_id role <<< "$node_spec"
    local user_host_part="${user_host%%:*}"
    local host_part="${user_host#*@}"

    log "Deploying node $node_id ($role) at $user_host (port $ssh_port)..."

    # Step 1: Clone or update repository
    run_remote "$user_host_part" "$ssh_port" "
        set -e
        if [ -d /opt/ed2kia ]; then
            cd /opt/ed2kia && git fetch origin && git checkout $BRANCH && git pull
        else
            mkdir -p /opt && git clone --branch $BRANCH $REPO_URL /opt/ed2kia
        fi
    "

    # Step 2: Build release binary
    run_remote "$user_host_part" "$ssh_port" "
        set -e
        cd /opt/ed2kia
        cargo build --release --bin ed2kia-node 2>&1 | tail -5
    "

    # Step 3: Generate node configuration
    run_remote "$user_host_part" "$ssh_port" "
        set -e
        mkdir -p /opt/ed2kia/config
        cat > /opt/ed2kia/config/node-${node_id}.toml << EOF
[node]
id = ${node_id}
role = \"${role}\"
listen_addr = \"/ip4/0.0.0.0/tcp/900${node_id}\"
announce_addr = \"/ip4/$(hostname -I | awk '{print \$1}')/tcp/900${node_id}\"

[p2p]
bootstrapper = ${node_id}
max_peers = 16
sync_interval_ms = 5000

[sae_audit]
enabled = true
total_neurons = 16384
sparsity_threshold = 0.1

[logging]
level = \"info\"
file = \"/var/log/ed2kia/node-${node_id}.log\"
EOF
    "

    # Step 4: Setup systemd service
    run_remote "$user_host_part" "$ssh_port" "
        set -e
        cat > /etc/systemd/system/ed2kia-node@${node_id}.service << 'SERVICE'
[Unit]
Description=ed2kIA Node %i
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/ed2kia
ExecStart=/opt/ed2kia/target/release/ed2kia-node --config /opt/ed2kia/config/node-%i.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
SERVICE
        systemctl daemon-reload
        systemctl enable ed2kia-node@${node_id}.service
    "

    log "Node $node_id ($role) deployment complete."
}

setup_latency_simulation() {
    log "Setting up tc/netem latency simulation..."

    for spec in "${LATENCY_MATRIX[@]}"; do
        IFS=':' read -r from_id to_id latency jitter <<< "$spec"
        local from_node="${NODES[$from_id]}"
        local to_node="${NODES[$to_id]}"
        IFS=':' read -r from_user_host from_port from_nid from_role <<< "$from_node"
        IFS=':' read -r to_user_host to_port to_nid to_role <<< "$to_node"
        local to_ip="${to_user_host#*@}"

        log "  Node $from_id → Node $to_id: ${latency}ms + ${jitter}ms jitter"

        run_remote "${from_user_host%%:*}" "$from_port" "
            set -e
            # Create chain for traffic to node $to_id
            tc qdisc del dev eth0 root 2>/dev/null || true
            tc qdisc add dev eth0 root handle 1: htb
            tc class add dev eth0 parent 1: classid 1:1 htb rate 100mbit
            tc filter add dev eth0 parent 1: protocol ip u32 match ip daddr $to_ip flowid 1:1
            tc qdisc add dev eth0 parent 1:1 handle 10: netem delay ${latency}ms ${jitter}ms distribution normal
        "
    done

    log "Latency simulation configured."
}

start_all_nodes() {
    log "Starting all nodes..."
    for node_spec in "${NODES[@]}"; do
        IFS=':' read -r user_host ssh_port node_id role <<< "$node_spec"
        run_remote "${user_host%%:*}" "$ssh_port" "
            systemctl start ed2kia-node@${node_id}.service
            sleep 2
            systemctl status ed2kia-node@${node_id}.service --no-pager
        "
    done
    log "All nodes started."
}

verify_network() {
    log "Verifying network health..."

    # Check bootstrapper is accepting connections
    local bootstrapper="${NODES[0]}"
    IFS=':' read -r user_host ssh_port node_id role <<< "$bootstrapper"
    run_remote "${user_host%%:*}" "$ssh_port" "
        journalctl -u ed2kia-node@0 --since '1 min ago' --no-pager | tail -20
    "

    # Check peer count on each node
    for node_spec in "${NODES[@]}"; do
        IFS=':' read -r user_host ssh_port node_id role <<< "$node_spec"
        log "  Node $node_id ($role) logs:"
        run_remote "${user_host%%:*}" "$ssh_port" "
            journalctl -u ed2kia-node@${node_id} --since '30 sec ago' --no-pager | grep -i 'peer\|connect\|sync' | tail -5
        "
    done

    log "Network verification complete."
}

cleanup_latency() {
    log "Cleaning up tc/netem rules..."
    for node_spec in "${NODES[@]}"; do
        IFS=':' read -r user_host ssh_port node_id role <<< "$node_spec"
        run_remote "${user_host%%:*}" "$ssh_port" "
            tc qdisc del dev eth0 root 2>/dev/null || true
        "
    done
    log "Cleanup complete."
}

# ─── Main ────────────────────────────────────────────────────────────────────

main() {
    log "=========================================="
    log "ed2kIA v${ED2KIA_VERSION} Testnet Deployment"
    log "Sprint 87: The Reality Engine"
    log "=========================================="
    log "Nodes: ${#NODES[@]}"
    log "Dry Run: $DRY_RUN"
    log ""

    # Deploy each node
    for node_spec in "${NODES[@]}"; do
        deploy_node "$node_spec"
    done

    # Setup latency simulation
    setup_latency_simulation

    # Start all nodes
    start_all_nodes

    # Verify network
    sleep 5
    verify_network

    log ""
    log "=========================================="
    log "Deployment Summary"
    log "=========================================="
    log "Version: $ED2KIA_VERSION"
    log "Nodes deployed: ${#NODES[@]}"
    log "Latency simulation: active"
    log ""
    log "Monitoring commands:"
    for node_spec in "${NODES[@]}"; do
        IFS=':' read -r user_host ssh_port node_id role <<< "$node_spec"
        log "  Node $node_id ($role): ssh -p $ssh_port ${user_host%%:*} 'journalctl -u ed2kia-node@${node_id} -f'"
    done
    log ""
    log "To stop: systemctl stop ed2kia-node@{0..2}"
    log "To cleanup latency: ./scripts/deploy_real_testnet.sh --cleanup"
    log "=========================================="
}

case "${1:-}" in
    --dry-run)
        DRY_RUN=true
        main
        ;;
    --cleanup)
        cleanup_latency
        ;;
    *)
        main
        ;;
esac
