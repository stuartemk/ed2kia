# Operations Runbook - ed2kIA v0.5.0

## Table of Contents

1. [Monitoring & Alerting](#monitoring--alerting)
2. [Health Checks](#health-checks)
3. [Incident Response](#incident-response)
4. [Scaling Operations](#scaling-operations)
5. [Backup & Recovery](#backup--recovery)
6. [Maintenance Procedures](#maintenance-procedures)

---

## Monitoring & Alerting

### Key Metrics

| Metric | Source | Warning Threshold | Critical Threshold |
|--------|--------|-------------------|-------------------|
| SAE Latency | `/api/metrics` | >500ms avg | >1000ms avg |
| Consensus Rate | `/api/network` | <70% agreement | <50% agreement |
| WASM Memory | `/api/metrics` | >200MB | >500MB |
| Node Reputation | `/api/network` | <0.4 | <0.2 |
| Peer Count | `/api/network` | <5 peers | <2 peers |
| CPU Usage | System | >80% sustained | >95% sustained |
| RAM Usage | System | >80% | >95% |
| Disk Usage | System | >80% | >95% |

### Alert Channels

- **PagerDuty/OpsGenie**: Critical incidents requiring immediate response
- **Slack #ed2k-ops**: Warnings and informational alerts
- **Email**: Daily digests and non-urgent notifications

### Prometheus Queries

```promql
# SAE latency p95
histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m]))

# Consensus agreement rate
sum(rate(consensus_votes_total{result="agree"}[5m])) / sum(rate(consensus_votes_total[5m]))

# WASM memory usage
wasm_memory_bytes{node=~"$node"}

# Reputation score
node_reputation_score{node=~"$node"}
```

### Grafana Dashboards

- **Node Overview**: CPU, RAM, Disk, Network I/O
- **SAE Performance**: Latency, throughput, error rates
- **Consensus Health**: Agreement rate, vote distribution, batch processing
- **P2P Network**: Peer count, gossipsub latency, message rates
- **WASM Sandbox**: Memory usage, execution times, security events

---

## Health Checks

### Automated Checks (via `/api/health`)

| Check | Description | Expected |
|-------|-------------|----------|
| `p2p_connected` | Node has active peer connections | peer_count >= 3 |
| `sae_loaded` | SAE model loaded and ready | model != null |
| `wasm_runtime` | WASM runtime operational | can_execute = true |
| `feedback_store` | Feedback store accessible | can_write = true |
| `disk_space` | Sufficient disk space available | usage < 90% |
| `memory_pressure` | Memory within limits | usage < 85% |

### Manual Verification

```bash
# Check node status
curl http://localhost:3030/api/health

# Check network status
curl http://localhost:3030/api/network

# Check metrics
curl http://localhost:3030/api/metrics

# Verify SAE forward pass
curl -X POST http://localhost:3030/api/feedback \
  -H "Content-Type: application/json" \
  -d '{"batch_id":"test","features":[],"decision":"approve"}'
```

---

## Incident Response

### Severity Levels

| Level | Description | Response Time | Examples |
|-------|-------------|---------------|----------|
| **P1 - Critical** | Network-wide outage or data loss | 15 min | Consensus failure, ZKP verification broken |
| **P2 - High** | Major degradation affecting many nodes | 1 hour | SAE latency >2s, peer discovery broken |
| **P3 - Medium** | Partial degradation, workaround exists | 4 hours | Single node reputation drop, lease renewal failures |
| **P4 - Low** | Minor issue, no user impact | Next sprint | Log spam, cosmetic UI issues |

### P1 Incident Response Procedure

1. **Detect**: Alert triggers or user reports
2. **Acknowledge**: On-call engineer acknowledges within 15 min
3. **Assess**: Determine scope and impact
   ```bash
   # Check affected nodes
   curl http://monitoring:3000/api/search | jq '.[] | select(.title=~"ed2k")'
   
   # Check consensus status across network
   for node in $(cat node_list.txt); do
     curl -s http://$node:3030/api/network | jq '.consensus_rate'
   done
   ```
4. **Contain**: Isolate the issue
   - Restart affected services if safe
   - Disable problematic features via feature flags
   - Notify network operators via Slack #ed2k-incidents
5. **Resolve**: Apply fix
   - Hotfix deployment if needed
   - Coordinate network-wide updates
6. **Recover**: Verify restoration
   - Monitor metrics return to normal
   - Confirm consensus rate >90%
   - Verify peer connections stable
7. **Post-Mortem**: Within 48 hours
   - Document timeline, root cause, impact
   - Define preventive actions
   - Update runbook with learnings

### Common Incidents

#### SAE Latency Spike

**Symptoms**: `/api/metrics` shows SAE latency >500ms

**Diagnosis**:
```bash
# Check GPU/CPU usage
nvidia-smi  # or top/htop for CPU
# Check memory pressure
free -h
# Check SAE model status
curl http://localhost:3030/api/health | jq '.checks[] | select(.name=="sae_loaded")'
```

**Resolution**:
1. Restart SAE loader: `systemctl restart ed2kia`
2. If persistent, check for memory leaks in WASM sandbox
3. Scale horizontally by adding more nodes

#### Consensus Rate Drop

**Symptoms**: Agreement rate <70%

**Diagnosis**:
```bash
# Check peer reputation scores
curl http://localhost:3030/api/network | jq '.peers[].reputation'
# Check for Byzantine nodes
curl http://localhost:3030/api/metrics | grep 'consensus_byzantine'
```

**Resolution**:
1. Identify and isolate low-reputation nodes
2. Verify ZKP verification is functioning
3. Check network partition (peer count drop)

#### WASM Memory Leak

**Symptoms**: WASM memory growing beyond 500MB

**Diagnosis**:
```bash
# Check sandbox stats
curl http://localhost:3030/api/metrics | grep 'wasm_memory'
```

**Resolution**:
1. Clear WASM cache: send SIGUSR1 to ed2kia process
2. Restart node if memory doesn't stabilize
3. Report bug with memory profiling data

---

## Scaling Operations

### Adding Nodes

1. Provision infrastructure (see Node Operator Guide)
2. Configure with existing seed nodes
3. Verify peer discovery and consensus participation
4. Monitor for 24 hours before considering stable

### Removing Nodes

1. Graceful shutdown: `ed2kia exit`
2. Verify leases transferred to other nodes
3. Remove from monitoring dashboards
4. Update seed node list if applicable

### Load Balancing

- LayerRouter automatically distributes SAE layers based on node capacity
- Monitor `layer_distribution` metric for balance
- Manual rebalancing: `ed2kia rebalance --force`

---

## Backup & Recovery

### Backup Schedule

| Data | Frequency | Retention | Location |
|------|-----------|-----------|----------|
| Feedback Store (redb) | Daily | 30 days | `/var/lib/ed2kia/feedback/` |
| Node Configuration | On change | 10 versions | `/etc/ed2kia/` |
| Reputation Ledger | Hourly | 90 days | `/var/lib/ed2kia/reputation/` |

### Backup Commands

```bash
# Backup feedback store
cp /var/lib/ed2kia/feedback/feedback.redb /backups/feedback_$(date +%Y%m%d).redb

# Backup configuration
tar czf /backups/ed2kia_config_$(date +%Y%m%d).tar.gz /etc/ed2kia/

# Verify backup integrity
sha256sum /backups/feedback_*.redb >> /backups/checksums.txt
```

### Recovery Procedure

1. Stop service: `systemctl stop ed2kia`
2. Restore data from backup
3. Verify file permissions: `chown -R ed2kia:ed2kia /var/lib/ed2kia/`
4. Start service: `systemctl start ed2kia`
5. Verify health: `curl http://localhost:3030/api/health`

---

## Maintenance Procedures

### Rolling Update

```bash
# Update one node at a time to maintain network availability
for node in $(cat node_list.txt); do
  ssh $node "systemctl stop ed2kia"
  ssh $node "ed2kia-update --to v0.5.1"
  ssh $node "systemctl start ed2kia"
  # Wait for health check
  until curl -sf http://$node:3030/api/health | jq -e '.status=="healthy"'; do
    sleep 10
  done
  echo "✓ $node updated and healthy"
done
```

### Certificate Rotation

- Ed25519 keys are node-local and don't expire
- Rotate only if compromised: regenerate via `ed2kia init --force`
- Update peer reputation ledger with new key

### Log Rotation

```bash
# Configured in systemd service
# /etc/systemd/system/ed2kia.service
[Service]
StandardOutput=journal
StandardError=journal

# View logs
journalctl -u ed2kia -f
# Rotate manually if needed
journalctl --vacuum-time=7d
```
