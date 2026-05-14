# Operator Onboarding Guide - ed2kIA v0.5.0

## Overview

This guide walks new node operators through the registration process, initial reputation validation, and performance tuning for production operation of ed2kIA nodes.

---

## Registration Flow

### Step 1: Pre-Registration Checklist

- [ ] Hardware meets minimum requirements (4 vCPU, 8GB RAM, 50GB SSD, 100Mbps)
- [ ] OS is supported (Ubuntu 22.04+, Debian 12+, or equivalent)
- [ ] Ports 3030 (P2P) and 9090 (metrics) are accessible
- [ ] Stable internet connection (symmetric bandwidth preferred)
- [ ] Docker installed (if using container deployment)
- [ ] Rust toolchain installed (if building from source)

### Step 2: Node Initialization

```bash
# 1. Download and verify ed2kIA binary
wget https://github.com/ed2kIA/ed2kIA/releases/download/v0.5.0/ed2kia-x86_64-unknown-linux-gnu.tar.gz
sha256sum ed2kia-x86_64-unknown-linux-gnu.tar.gz
# Compare with release/checksums.txt

# 2. Extract and install
tar xzf ed2kia-x86_64-unknown-linux-gnu.tar.gz
sudo mv ed2kia /usr/local/bin/

# 3. Initialize node
ed2kia init \
  --data-dir /var/lib/ed2kia \
  --listen-port 3030 \
  --seed-nodes "/dns/seed-alpha.ed2kIA/tcp/3030,/dns/seed-bravo.ed2kIA/tcp/3030"

# 4. Verify initialization
ls -la /var/lib/ed2kia/
# Expected: config.toml, keys/, data/
```

### Step 3: Configuration Review

Edit `/var/lib/ed2kia/config.toml`:

```toml
[network]
listen_port = 3030
max_peers = 50

[sae]
model_path = "/var/lib/ed2kia/models/sae"
max_layers = 4

[wasm]
memory_limit_mb = 256

[monitoring]
metrics_enabled = true
metrics_port = 9090
```

### Step 4: Start Node

```bash
# Start node
ed2kia run --data-dir /var/lib/ed2kia

# Or via systemd
sudo systemctl enable ed2kia
sudo systemctl start ed2kia

# Verify
curl http://localhost:3030/api/health
# Expected: {"status":"healthy","checks":[...]}
```

---

## Reputation Validation

### Initial Reputation

New nodes start with **reputation score = 0.5** (neutral). Reputation increases through:

| Action | Score Impact | Timeframe |
|--------|-------------|-----------|
| Successful SAE forward pass | +0.01 | Per batch |
| ZKP proof generation | +0.02 | Per proof |
| Consensus vote (agree) | +0.005 | Per vote |
| Consensus vote (disagree, correct) | +0.01 | Per vote |
| Failed SAE forward | -0.02 | Per failure |
| Consensus vote (disagree, wrong) | -0.01 | Per vote |
| Inactivity (decay) | -50%/30d | Continuous |

### Reaching Operational Status

| Reputation | Status | Capabilities |
|-----------|--------|-------------|
| 0.0 - 0.3 | Untrusted | Read-only, no consensus participation |
| 0.3 - 0.4 | Probation | Limited consensus, no layer leases |
| 0.4 - 0.7 | Active | Full consensus, layer leases |
| 0.7 - 1.0 | Trusted | Governance voting, seed candidate |

### Checking Reputation

```bash
# Query your node's reputation
curl http://localhost:3030/api/network | jq '.reputation'

# Query network-wide reputation
curl http://localhost:3030/api/network | jq '.peers[].reputation'

# Check reputation decay
ed2kia reputation status --data-dir /var/lib/ed2kia
```

### Accelerating Reputation Growth

1. **Run consistently**: 24/7 uptime prevents decay
2. **Generate ZKP proofs**: Higher bonus multiplier (1.2x)
3. **Vote accurately**: Consensus participation builds trust
4. **Maintain low latency**: Fast responses improve scores
5. **Avoid errors**: Failed operations reduce reputation

---

## Performance Tuning

### CPU Optimization

```toml
# For high-CPU nodes (8+ cores)
[sae]
max_layers = 8          # Process more layers
topk_features = 1024    # Higher feature resolution

# For low-CPU nodes (2-4 cores)
[sae]
max_layers = 2          # Fewer layers
topk_features = 256     # Lower resolution
```

### Memory Optimization

```toml
# For high-RAM nodes (16GB+)
[wasm]
memory_limit_mb = 512

[sae]
batch_size = 128

# For low-RAM nodes (4-8GB)
[wasm]
memory_limit_mb = 128

[sae]
batch_size = 32
```

### Network Optimization

```toml
# For high-bandwidth nodes (1Gbps+)
[network]
max_peers = 100
dial_timeout_secs = 10

# For constrained networks (100Mbps)
[network]
max_peers = 30
dial_timeout_secs = 30
```

### Monitoring Your Performance

```bash
# Check SAE latency
curl http://localhost:9090/metrics | grep sae_latency

# Check consensus participation
curl http://localhost:9090/metrics | grep consensus_votes

# Check WASM memory
curl http://localhost:9090/metrics | grep wasm_memory

# Check peer count
curl http://localhost:3030/api/network | jq '.peer_count'
```

---

## Security Hardening

### System-Level

```bash
# Create dedicated user
sudo useradd -r -s /bin/false ed2kia
sudo chown -R ed2kia:ed2kia /var/lib/ed2kia

# Firewall rules
sudo ufw allow 3030/tcp   # P2P
sudo ufw allow 9090/tcp   # Metrics (internal only)
sudo ufw enable

# File permissions
chmod 700 /var/lib/ed2kia/keys
chmod 600 /var/lib/ed2kia/keys/*
```

### Node-Level

```toml
# Disable unnecessary features
[wasm]
allow_network = false
allow_host_fs = false

# Enable health checks
[monitoring]
health_check_interval_secs = 30
```

### Key Management

```bash
# Backup your keys
tar czf ed2kia-keys-backup-$(date +%Y%m%d).tar.gz /var/lib/ed2kia/keys/
gpg --symmetric --cipher-algo AES256 ed2kia-keys-backup-*.tar.gz
# Store encrypted backup securely

# Never share your private key
# If compromised, reinitialize: ed2kia init --force
```

---

## Troubleshooting

### Common Issues

| Issue | Diagnosis | Fix |
|-------|-----------|-----|
| No peers | `peer_count = 0` | Check firewall, seed nodes |
| High latency | `sae_latency > 1s` | Reduce max_layers, check CPU |
| Low reputation | `score < 0.4` | Run consistently, vote accurately |
| OOM kills | `dmesg \| grep OOM` | Increase RAM, reduce wasm memory |
| Disk full | `df -h` | Clean logs, compact feedback store |

### Getting Help

1. Check logs: `journalctl -u ed2kia -n 100`
2. Review metrics: `curl http://localhost:9090/metrics`
3. Search issues: https://github.com/ed2kIA/ed2kIA/issues
4. Ask community: #ed2k-operators (Discord/Slack)
5. File bug report: Use `.github/ISSUE_TEMPLATE/node_operator_issue.md`

---

## Next Steps

After successful onboarding:

- [ ] Monitor node for 48 hours
- [ ] Reach reputation ≥ 0.4 (active status)
- [ ] Join community channels
- [ ] Review governance proposals
- [ ] Consider becoming a seed node (reputation ≥ 0.8)
