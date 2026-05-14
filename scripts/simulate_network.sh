#!/usr/bin/env bash
# =============================================================================
# ed2kIA Network Simulation
# Launches 3 local nodes (via Docker), runs full flow, validates metrics
# =============================================================================
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

NODES=3
BASE_P2P_PORT=9000
BASE_HTTP_PORT=3000
NETWORK_NAME="ed2kia_test_$$"

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

cleanup() {
    log_info "Cleaning up network $NETWORK_NAME..."
    docker compose -f deploy/docker-compose.yml -p "$NETWORK_NAME" down --remove-orphases 2>/dev/null || true
    log_info "Cleanup complete."
}

trap cleanup EXIT

echo "=============================================="
echo "  ed2kIA Network Simulation"
echo "  Nodes: $NODES | Network: $NETWORK_NAME"
echo "=============================================="
echo ""

# ------------------------------------------------------------------------------
# Pre-flight checks
# ------------------------------------------------------------------------------
log_info "Pre-flight checks..."

if ! command -v docker &> /dev/null; then
    log_error "Docker not found. Install Docker Desktop or Docker Engine."
    exit 1
fi

if ! docker info &> /dev/null; then
    log_error "Docker daemon not running. Start Docker and try again."
    exit 1
fi

if [ ! -f "deploy/docker-compose.yml" ]; then
    log_error "deploy/docker-compose.yml not found."
    exit 1
fi

log_success "Pre-flight checks passed."
echo ""

# ------------------------------------------------------------------------------
# Step 1: Build Docker image
# ------------------------------------------------------------------------------
log_info "Step 1: Building Docker image..."

if docker build -t ed2kia:sim -f deploy/Dockerfile . 2>&1 | tail -1 | grep -q "Successfully built"; then
    log_success "Docker image built: ed2kia:sim"
else
    log_error "Docker build failed."
    exit 1
fi

echo ""

# ------------------------------------------------------------------------------
# Step 2: Configure multi-node environment
# ------------------------------------------------------------------------------
log_info "Step 2: Configuring $NODES-node environment..."

# Generate unique bootstrap peers list
BOOTSTRAP_PEERS=""
for i in $(seq 1 $NODES); do
    P2P_PORT=$((BASE_P2P_PORT + i - 1))
    HTTP_PORT=$((BASE_HTTP_PORT + i - 1))
    if [ -n "$BOOTSTRAP_PEERS" ]; then
        BOOTSTRAP_PEERS="${BOOTSTRAP_PEERS},"
    fi
    BOOTSTRAP_PEERS="${BOOTSTRAP_PEERS}ed2kia-node${i}:${P2P_PORT}"
done

log_info "Bootstrap peers: $BOOTSTRAP_PEERS"

# Create override file for simulation
cat > "/tmp/ed2kia_sim_override_${NETWORK_NAME}.yml" << EOF
version: "3.8"
services:
  ed2kia-node1:
    image: ed2kia:sim
    environment:
      - ED2KIA_NODE_ID=node1
      - ED2KIA_P2P_PORT=$((BASE_P2P_PORT))
      - ED2KIA_HTTP_PORT=$((BASE_HTTP_PORT))
      - ED2KIA_BOOTSTRAP_PEERS=$BOOTSTRAP_PEERS
      - ED2KIA_IS_BOOTSTRAP=true
      - ED2KIA_LOG_LEVEL=debug
    ports:
      - "$((BASE_P2P_PORT)):$(BASE_P2P_PORT)"
      - "$((BASE_HTTP_PORT)):$(BASE_HTTP_PORT)"
    networks:
      - ed2kia-sim
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:$((BASE_HTTP_PORT))/api/health"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s

  ed2kia-node2:
    image: ed2kia:sim
    environment:
      - ED2KIA_NODE_ID=node2
      - ED2KIA_P2P_PORT=$((BASE_P2P_PORT + 1))
      - ED2KIA_HTTP_PORT=$((BASE_HTTP_PORT + 1))
      - ED2KIA_BOOTSTRAP_PEERS=$BOOTSTRAP_PEERS
      - ED2KIA_IS_BOOTSTRAP=false
      - ED2KIA_LOG_LEVEL=debug
    ports:
      - "$((BASE_P2P_PORT + 1)):$((BASE_P2P_PORT + 1))"
      - "$((BASE_HTTP_PORT + 1)):$((BASE_HTTP_PORT + 1))"
    networks:
      - ed2kia-sim
    depends_on:
      ed2kia-node1:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:$((BASE_HTTP_PORT + 1))/api/health"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s

  ed2kia-node3:
    image: ed2kia:sim
    environment:
      - ED2KIA_NODE_ID=node3
      - ED2KIA_P2P_PORT=$((BASE_P2P_PORT + 2))
      - ED2KIA_HTTP_PORT=$((BASE_HTTP_PORT + 2))
      - ED2KIA_BOOTSTRAP_PEERS=$BOOTSTRAP_PEERS
      - ED2KIA_IS_BOOTSTRAP=false
      - ED2KIA_LOG_LEVEL=debug
    ports:
      - "$((BASE_P2P_PORT + 2)):$((BASE_P2P_PORT + 2))"
      - "$((BASE_HTTP_PORT + 2)):$((BASE_HTTP_PORT + 2))"
    networks:
      - ed2kia-sim
    depends_on:
      ed2kia-node1:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:$((BASE_HTTP_PORT + 2))/api/health"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s

networks:
  ed2kia-sim:
    driver: bridge
    name: "$NETWORK_NAME"
EOF

log_success "Multi-node configuration created."
echo ""

# ------------------------------------------------------------------------------
# Step 3: Launch network
# ------------------------------------------------------------------------------
log_info "Step 3: Launching $NODES-node network..."

docker compose -f "/tmp/ed2kia_sim_override_${NETWORK_NAME}.yml" -p "$NETWORK_NAME" up -d

log_info "Waiting for nodes to initialize (60s)..."
sleep 60

echo ""

# ------------------------------------------------------------------------------
# Step 4: Validate node health
# ------------------------------------------------------------------------------
log_info "Step 4: Validating node health..."

HEALTHY_NODES=0
for i in $(seq 1 $NODES); do
    HTTP_PORT=$((BASE_HTTP_PORT + i - 1))
    NODE_NAME="ed2kia-node${i}"

    # Check health endpoint
    RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:${HTTP_PORT}/api/health" 2>/dev/null || echo "000")

    if [ "$RESPONSE" = "200" ]; then
        log_success "$NODE_NAME is healthy (HTTP $RESPONSE)"
        ((HEALTHY_NODES++))
    else
        log_error "$NODE_NAME health check failed (HTTP $RESPONSE)"
    fi
done

echo ""

if [ $HEALTHY_NODES -lt $NODES ]; then
    log_warn "Only $HEALTHY_NODES/$NODES nodes healthy. Continuing with validation..."
fi

# ------------------------------------------------------------------------------
# Step 5: Validate P2P connectivity
# ------------------------------------------------------------------------------
log_info "Step 5: Validating P2P connectivity..."

for i in $(seq 1 $NODES); do
    HTTP_PORT=$((BASE_HTTP_PORT + i - 1))
    NODE_NAME="ed2kia-node${i}"

    # Check network endpoint for peer count
    PEER_COUNT=$(curl -s "http://localhost:${HTTP_PORT}/api/network" 2>/dev/null | jq '.data.total_peers // 0' 2>/dev/null || echo "0")

    if [ "$PEER_COUNT" -ge 1 ]; then
        log_success "$NODE_NAME has $PEER_COUNT connected peer(s)"
    else
        log_warn "$NODE_NAME has no connected peers"
    fi
done

echo ""

# ------------------------------------------------------------------------------
# Step 6: Validate metrics endpoint
# ------------------------------------------------------------------------------
log_info "Step 6: Validating metrics endpoint..."

for i in $(seq 1 $NODES); do
    HTTP_PORT=$((BASE_HTTP_PORT + i - 1))
    NODE_NAME="ed2kia-node${i}"

    METRICS=$(curl -s "http://localhost:${HTTP_PORT}/api/metrics" 2>/dev/null)
    if [ -n "$METRICS" ] && [ "$METRICS" != "null" ]; then
        log_success "$NODE_NAME metrics endpoint responding"
    else
        log_warn "$NODE_NAME metrics endpoint not responding"
    fi
done

echo ""

# ------------------------------------------------------------------------------
# Step 7: Simulate feedback flow
# ------------------------------------------------------------------------------
log_info "Step 7: Simulating feedback flow..."

FEEDBACK_PAYLOAD='{
    "layer_id": "layer_0",
    "feature_idx": 42,
    "feature_value": 0.85,
    "decision": "approved",
    "annotator_id": "simulator"
}'

HTTP_PORT=$BASE_HTTP_PORT
RESPONSE=$(curl -s -X POST "http://localhost:${HTTP_PORT}/api/feedback" \
    -H "Content-Type: application/json" \
    -d "$FEEDBACK_PAYLOAD" 2>/dev/null || echo "{}")

if echo "$RESPONSE" | jq -e '.success == true' &> /dev/null; then
    log_success "Feedback submission successful"
else
    log_warn "Feedback submission failed or returned error"
fi

echo ""

# ------------------------------------------------------------------------------
# Step 8: Collect final metrics
# ------------------------------------------------------------------------------
log_info "Step 8: Collecting final metrics..."

echo -e "${CYAN}--- Final Network Metrics ---${NC}"
for i in $(seq 1 $NODES); do
    HTTP_PORT=$((BASE_HTTP_PORT + i - 1))
    NODE_NAME="ed2kia-node${i}"

    echo -e "\n${CYAN}[$NODE_NAME]${NC}"
    curl -s "http://localhost:${HTTP_PORT}/api/status" 2>/dev/null | jq '.' 2>/dev/null || echo "  (unable to retrieve)"
done

echo ""

# ------------------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------------------
echo "=============================================="
echo "  Simulation Summary"
echo "=============================================="
echo -e "  Nodes launched: $NODES"
echo -e "  Healthy nodes: $HEALTHY_NODES"
echo -e "  Network: $NETWORK_NAME"
echo ""

if [ $HEALTHY_NODES -eq $NODES ]; then
    log_success "All $NODES nodes healthy. Simulation PASSED."
    echo ""
    log_info "To inspect logs: docker compose -f /tmp/ed2kia_sim_override_${NETWORK_NAME}.yml -p $NETWORK_NAME logs"
    log_info "To keep running: Skip cleanup (remove trap)"
else
    log_warn "Simulation completed with $((NODES - HEALTHY_NODES)) unhealthy node(s)."
    log_info "Check logs: docker compose -f /tmp/ed2kia_sim_override_${NETWORK_NAME}.yml -p $NETWORK_NAME logs"
fi

# Cleanup is handled by trap
exit 0
