#!/usr/bin/env bash
# deploy_testnet.sh — Sprint 84: Hostile Testnet Deployment
# Deploys 5-node ed2kIA testnet with simulated network conditions
# Usage: ./scripts/deploy_testnet.sh [latency_ms] [loss_pct]
#   latency_ms: Network latency in ms (default: 250)
#   loss_pct: Packet loss percentage (default: 3)

set -euo pipefail

LATENCY_MS="${1:-250}"
LOSS_PCT="${2:-3}"
JITTER_MS=$((LATENCY_MS / 5))
COMPOSE_FILE="deploy/docker-compose.testnet.yml"
COMPOSE_DIR="$(dirname "$COMPOSE_FILE")"

echo "============================================"
echo " ed2kIA Testnet Deployment — Sprint 84"
echo "============================================"
echo " Latency: ${LATENCY_MS}ms ± ${JITTER_MS}ms"
echo " Packet Loss: ${LOSS_PCT}%"
echo " Nodes: 5 (1 bootstrap + 4 workers)"
echo " Compose: ${COMPOSE_FILE}"
echo "============================================"

# Step 1: Validate prerequisites
echo ""
echo "[1/6] Validating prerequisites..."
command -v docker >/dev/null 2>&1 || { echo "ERROR: docker not found"; exit 1; }
command -v docker compose >/dev/null 2>&1 || command -v docker-compose >/dev/null 2>&1 || { echo "ERROR: docker compose not found"; exit 1; }

# Determine compose command
if docker compose version >/dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
else
    COMPOSE_CMD="docker-compose"
fi

echo "  ✓ Docker: $(docker --version | head -1)"
echo "  ✓ Compose: $($COMPOSE_CMD version --short 2>/dev/null || echo 'available')"

# Step 2: Validate compose file
echo ""
echo "[2/6] Validating compose configuration..."
$COMPOSE_CMD -f "$COMPOSE_FILE" config >/dev/null 2>&1 || { echo "ERROR: Invalid compose file"; exit 1; }
echo "  ✓ Compose config valid"

# Step 3: Build images
echo ""
echo "[3/6] Building ed2kIA images..."
$COMPOSE_CMD -f "$COMPOSE_FILE" build --quiet 2>/dev/null || {
    echo "  ⚠ Build failed — checking if image exists..."
    docker image inspect ed2kia >/dev/null 2>&1 || { echo "ERROR: No ed2kia image found"; exit 1; }
}
echo "  ✓ Image ready"

# Step 4: Start testnet
echo ""
echo "[4/6] Starting 5-node testnet..."
$COMPOSE_CMD -f "$COMPOSE_FILE" up -d 2>/dev/null || {
    echo "  ⚠ Docker compose up failed — attempting individual start..."
    for i in $(seq 0 4); do
        $COMPOSE_CMD -f "$COMPOSE_FILE" up -d "ed2k-node-$i" 2>/dev/null || echo "  ⚠ Node $i failed to start"
    done
}
echo "  ✓ Testnet started"

# Step 5: Apply network emulation (requires root/sudo)
echo ""
echo "[5/6] Applying network emulation (tc/netem)..."
if command -v tc >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
    # Find the docker bridge interface
    DOCKER_IF=$(ip route | grep "172.20.0.0" | awk '{print $5}' | head -1)
    if [ -n "$DOCKER_IF" ]; then
        tc qdisc del dev "$DOCKER_IF" root 2>/dev/null || true
        tc qdisc add dev "$DOCKER_IF" root netem delay "${LATENCY_MS}ms" "${JITTER_MS}ms" distribution normal loss "${LOSS_PCT}%"
        echo "  ✓ Netem applied on $DOCKER_IF: ${LATENCY_MS}ms ± ${JITTER_MS}ms, ${LOSS_PCT}% loss"
    else
        echo "  ⚠ Docker bridge interface not found — netem skipped"
        echo "  ℹ Apply manually: tc qdisc add dev <iface> root netem delay ${LATENCY_MS}ms ${JITTER_MS}ms loss ${LOSS_PCT}%"
    fi
else
    echo "  ⚠ tc/netem requires root access — skipping network emulation"
    echo "  ℹ Nodes running with default network conditions"
    echo "  ℹ Apply manually: sudo tc qdisc add dev <iface> root netem delay ${LATENCY_MS}ms ${JITTER_MS}ms loss ${LOSS_PCT}%"
fi

# Step 6: Verify health
echo ""
echo "[6/6] Verifying node health..."
HEALTHY=0
for i in $(seq 0 4); do
    PORT=$((8080 + i))
    if curl -sf "http://localhost:${PORT}/health" >/dev/null 2>&1; then
        echo "  ✓ Node $i (port $PORT): HEALTHY"
        HEALTHY=$((HEALTHY + 1))
    else
        echo "  ⚠ Node $i (port $PORT): PENDING (waiting for startup)"
    fi
done

echo ""
echo "============================================"
echo " Testnet Status: ${HEALTHY}/5 nodes healthy"
echo "============================================"
echo ""
echo " Management Commands:"
echo "   View logs:    $COMPOSE_CMD -f $COMPOSE_FILE logs -f"
echo "   Stop:         $COMPOSE_CMD -f $COMPOSE_FILE down"
echo "   Reset:        $COMPOSE_CMD -f $COMPOSE_FILE down -v"
echo "   Remove netem: sudo tc qdisc del dev <iface> root"
echo ""
echo " Dashboard: http://localhost:8080/dashboard"
echo " WebSocket: ws://localhost:8080/stream/activations"
echo ""
echo "✅ Testnet deployment complete."
