# Node Operator Guide - ed2kIA v0.5.0

## Table of Contents

1. [Requirements](#requirements)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Running the Node](#running-the-node)
5. [Tuning & Optimization](#tuning--optimization)
6. [Debugging](#debugging)
7. [Upgrades](#upgrades)
8. [Troubleshooting](#troubleshooting)

---

## Requirements

### Minimum Hardware

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 vCPU | 8 vCPU |
| RAM | 8GB | 16GB |
| Storage | 50GB SSD | 100GB NVMe |
| Network | 100Mbps symmetric | 1Gbps symmetric |
| OS | Ubuntu 22.04 / Debian 12 | Ubuntu 24.04 |

### Software Dependencies

- **Rust 1.75+** (for building from source)
- **libssl-dev** / **openssl** (system dependency)
- **build-essential** (compilation tools)
- **Git** (repository access)

### Network Requirements

- **Outbound**: Ports 443, 80 (for updates and peer discovery)
- **Inbound**: Port 3030 (P2P communication, configurable)
- **DNS**: Functional DNS resolution for seed nodes

---

## Installation

### Option A: Pre-built Binary (Recommended)

```bash
# Download release binary
wget https://github.com/ed2kIA/ed2kIA/releases/download/v0.5.0/ed2kia-x86_64-unknown-linux-gnu.tar.gz

# Verify checksum
sha256sum ed2kia-x86_64-unknown-linux-gnu.tar.gz
# Compare with release/checksums.txt

# Extract
tar xzf ed2kia-x86_64-unknown-linux-gnu.tar.gz

# Install
sudo mv ed2kia /usr/local/bin/
sudo chmod +x /usr/local/bin/ed2kia
```

### Option B: Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone repository
git clone https://github.com/ed2kIA/ed2kIA.git
cd ed2kIA

# Build with core-only features (Fases 1-3)
cargo build --release --features "core-only"

# Install
sudo cp target/release/ed2kia /usr/local/bin/
```

### Option C: Docker

```bash
# Pull image
docker pull ghcr.io/ed2kIA/ed2kia:v0.5.0

# Run container
docker run -d \
  --name ed2kia \
  -p 3030:3030 \
  -v /var/lib/ed2kia:/data \
  -e ED2K_SEED_NODES="/dns/seed1.ed2kIA/tcp/3030" \
  ghcr.io/ed2kIA/ed2kia:v0.5.0
```

---

## Configuration

### Initial Setup

```bash
# Initialize node (creates config and keys)
ed2kia init \
  --data-dir /var/lib/ed2kia \
  --listen-port 3030 \
  --seed-nodes "/dns/seed1.ed2kIA/tcp/3030,/dns/seed2.ed2kIA/tcp/3030"
```

### Configuration File (`/etc/ed2kia/config.toml`)

```toml
# Network settings
[network]
listen_port = 3030
max_peers = 50
seed_nodes = [
  "/dns/seed1.ed2kIA/tcp/3030",
  "/dns/seed2.ed2kIA/tcp/3030",
  "/dns/seed3.ed2kIA/tcp/3030"
]

# SAE settings
[sae]
model_path = "/var/lib/ed2kia/models/sae"
max_layers = 4
topk_features = 512

# WASM sandbox
[wasm]
memory_limit_mb = 256
execution_timeout_ms = 5000
allow_network = false

# Logging
[logging]
level = "info"  # debug, info, warn, error
file = "/var/log/ed2kia/node.log"
max_size_mb = 100
max_files = 5

# Monitoring
[monitoring]
metrics_enabled = true
metrics_port = 9090
health_check_interval_secs = 30
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ED2K_DATA_DIR` | Data directory | `~/.ed2kia` |
| `ED2K_LISTEN_PORT` | P2P listen port | `3030` |
| `ED2K_SEED_NODES` | Comma-separated seed nodes | Built-in defaults |
| `ED2K_LOG_LEVEL` | Log level | `info` |
| `ED2K_FEATURE_FLAGS` | Feature flags | `core-only` |

---

## Running the Node

### systemd (Production)

```bash
# Create service user
sudo useradd -r -s /bin/false ed2kia

# Create directories
sudo mkdir -p /var/lib/ed2kia /var/log/ed2kia /etc/ed2kia
sudo chown -R ed2kia:ed2kia /var/lib/ed2kia /var/log/ed2kia

# Install service file
sudo cp deploy/systemd/ed2kia.service /etc/systemd/system/
sudo systemctl daemon-reload

# Enable and start
sudo systemctl enable ed2kia
sudo systemctl start ed2kia

# Check status
sudo systemctl status ed2kia
journalctl -u ed2kia -f
```

### Manual (Development)

```bash
# Start node
ed2kia run --data-dir ./data --listen-port 3030

# Run with debug logging
RUST_LOG=debug ed2kia run

# Test SAE forward pass
ed2kia test-forward --input tests/sample_input.json
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'
services:
  ed2kia:
    image: ghcr.io/ed2kIA/ed2kia:v0.5.0
    ports:
      - "3030:3030"
      - "9090:9090"  # metrics
    volumes:
      - ed2kia-data:/data
      - ./config.toml:/etc/ed2kia/config.toml
    environment:
      - ED2K_SEED_NODES=/dns/seed1.ed2kIA/tcp/3030
    restart: unless-stopped

volumes:
  ed2kia-data:
```

---

## Tuning & Optimization

### CPU/RAM Optimization

```toml
# For low-resource nodes
[sae]
max_layers = 2          # Reduce from default 4
topk_features = 256     # Reduce from default 512

[wasm]
memory_limit_mb = 128   # Reduce from default 256
```

### Network Optimization

```toml
# For high-latency connections
[network]
max_peers = 20          # Reduce from default 50
dial_timeout_secs = 30  # Increase for slow connections
```

### Storage Optimization

```bash
# Clean old logs
find /var/log/ed2kia -name "*.log.*" -mtime +7 -delete

# Compact feedback store
ed2kia compact --data-dir /var/lib/ed2kia

# Check disk usage
du -sh /var/lib/ed2kia/*
```

### Performance Monitoring

```bash
# Real-time metrics
curl http://localhost:9090/metrics | grep ed2kia

# Check SAE latency
curl http://localhost:3030/api/metrics | jq '.sae_latency_ms'

# Monitor consensus participation
curl http://localhost:3030/api/network | jq '.consensus_participation'
```

---

## Debugging

### Enable Debug Logging

```bash
# Temporary (next run)
RUST_LOG=debug ed2kia run

# Permanent (config.toml)
[logging]
level = "debug"

# Specific modules
RUST_LOG=ed2kia::p2p=debug,ed2kia::sae=trace ed2kia run
```

### Common Debug Commands

```bash
# Check node identity
ed2kia id --data-dir /var/lib/ed2kia

# List connected peers
curl http://localhost:3030/api/network | jq '.peers'

# Verify SAE model
ed2kia verify-model --data-dir /var/lib/ed2kia

# Test WASM sandbox
ed2kia test-wasm --module tests/test_sae.wasm

# Check reputation
curl http://localhost:3030/api/network | jq '.reputation'
```

### Core Dump Analysis

```bash
# Enable core dumps
echo "core" | sudo tee /proc/sys/kernel/core_pattern

# After crash, analyze
gdb /usr/local/bin/ed2kia core.12345
(gdb) bt
(gdb) info threads
```

### Memory Leak Detection

```bash
# Run with valgrind (development only)
valgrind --leak-check=full --show-leak-kinds=all ed2kia run

# Check WASM memory growth
watch -n 5 'curl -s http://localhost:3030/api/metrics | jq ".wasm_memory_mb"'
```

---

## Upgrades

### Minor Version (v0.5.x → v0.5.y)

```bash
# Download new binary
wget https://github.com/ed2kIA/ed2kIA/releases/download/v0.5.1/ed2kia

# Stop service
sudo systemctl stop ed2kia

# Backup data
sudo cp -r /var/lib/ed2kia /var/lib/ed2kia.backup.$(date +%Y%m%d)

# Replace binary
sudo mv ed2kia /usr/local/bin/

# Start service
sudo systemctl start ed2kia

# Verify
curl http://localhost:3030/api/health
```

### Major Version (v0.x → v0.y)

1. Read migration notes in `docs/MIGRATION_LOG_v0.y.0.md`
2. Backup all data
3. Update configuration file (check for deprecated options)
4. Run migration tool if needed: `ed2kia migrate --from v0.4.0 --to v0.5.0`
5. Start and verify

### Rollback Procedure

```bash
# Stop current version
sudo systemctl stop ed2kia

# Restore previous binary
sudo mv /usr/local/bin/ed2kia.old /usr/local/bin/ed2kia

# Restore data if needed
sudo rm -rf /var/lib/ed2kia
sudo mv /var/lib/ed2kia.backup.* /var/lib/ed2kia

# Start
sudo systemctl start ed2kia
```

---

## Troubleshooting

### Node Won't Start

**Symptom**: Service fails immediately

**Check**:
```bash
journalctl -u ed2kia -n 50
ls -la /var/lib/ed2kia/  # Check permissions
ed2kia --version  # Verify binary
```

**Fix**:
- Ensure data directory exists and is writable
- Check port 3030 is not in use: `ss -tlnp | grep 3030`
- Verify config syntax: `ed2kia validate-config /etc/ed2kia/config.toml`

### No Peer Connections

**Symptom**: Peer count = 0

**Check**:
```bash
# Test connectivity to seed nodes
nc -zv seed1.ed2kIA 3030
# Check firewall
sudo ufw status
# Verify DNS resolution
dig seed1.ed2kIA
```

**Fix**:
- Open inbound port 3030
- Update seed node list in config
- Check NAT/port forwarding if behind router

### High SAE Latency

**Symptom**: Latency >1000ms

**Check**:
```bash
# System resources
top -bn1 | head -20
free -h
# SAE-specific
curl http://localhost:3030/api/metrics | jq '.sae'
```

**Fix**:
- Reduce `max_layers` in config
- Upgrade CPU/RAM
- Check for CPU throttling (laptop power mode)

### Reputation Drop

**Symptom**: Reputation score <0.4

**Check**:
```bash
# Detailed reputation
curl http://localhost:3030/api/network | jq '.reputation_details'
# Recent votes
curl http://localhost:3030/api/metrics | jq '.consensus_votes'
```

**Fix**:
- Verify SAE outputs are correct (not corrupted)
- Check for clock skew: `timedatectl`
- Ensure stable network connection
- Wait for reputation to recover (automatic over time)
