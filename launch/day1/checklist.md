# Day 1 Launch Checklist - ed2kIA v0.5.0

## Overview

This checklist provides minute-by-minute procedures for the first 24 hours after network activation. Assign roles before T=0 and ensure all team members have access to monitoring dashboards and communication channels.

## Roles

| Role | Responsibility | Contact |
|------|---------------|---------|
| **Launch Commander** | Final go/no-go, escalation decisions | TBD |
| **Network Ops** | Node health, peer connectivity, leases | TBD |
| **Monitoring Lead** | Prometheus/Grafana alerts, metric validation | TBD |
| **Consensus Watch** | Agreement rates, ZKP verification, reputation | TBD |
| **Community Liaison** | Operator communications, issue triage | TBD |

---

## T-1h: Pre-Launch Final Checks

- [ ] All 5 seed nodes passing health checks
- [ ] Monitoring stack running (Prometheus, Grafana, Loki, Alertmanager)
- [ ] Grafana dashboards loaded and scraping metrics
- [ ] Alert channels verified (Slack #ed2k-incidents, PagerDuty)
- [ ] Rollback binaries staged and tested
- [ ] Communication channels active (Slack, Discord, email)
- [ ] Launch Commander gives GO confirmation

---

## T+0h: Network Activation

### Immediate (0-5 minutes)

- [ ] Execute `launch/genesis/activation.sh`
- [ ] Verify all 5 seed nodes started (check PIDs)
- [ ] Confirm activation log shows no errors
- [ ] Announce "Network Active" in #ed2k-ops

### First 15 Minutes

- [ ] **Peer Discovery**: Each node has ≥2 peer connections
  ```bash
  curl -s http://localhost:3030/api/network | jq '.peer_count'
  ```
- [ ] **GossipSub**: Messages flowing on `/ed2kIA/sae/v0.5.0` topic
  ```bash
  # Check node logs for gossip activity
  tail -f /var/log/ed2kia/seed-alpha-001.log | grep gossip
  ```
- [ ] **Lease Distribution**: All 16 layers assigned
  ```bash
  curl -s http://localhost:3030/api/network | jq '.layers_assigned'
  ```

### First 30 Minutes

- [ ] **Health Endpoints**: All nodes return `{"status":"healthy"}`
  ```bash
  for node in alpha bravo charlie delta echo; do
    curl -sf http://${node}.ed2kIA:3030/api/health && echo " ✓ ${node}" || echo " ✗ ${node}"
  done
  ```
- [ ] **Metrics Scraping**: Prometheus shows all 5 targets UP
  - Open: `http://localhost:9090/targets`
  - Verify: 5/5 ed2kia targets UP
- [ ] **Grafana Dashboards**: Data appearing in all panels
  - Node Overview: Peer counts visible
  - SAE Performance: Latency data flowing
  - Consensus Health: Vote counts incrementing

---

## T+1h: First Hour Validation

### Network Health

- [ ] **Peer Count**: Each node has ≥3 connections
  - Target: 5-10 peers per node
  - Alert if: <2 peers for >5 minutes
- [ ] **Network Topology**: Mesh forming correctly
  - Check: No isolated nodes
  - Verify: Cross-region connections active

### SAE Performance

- [ ] **Forward Pass Latency**: p95 < 500ms
  ```bash
  curl -s http://localhost:9090/api/v1/query \
    --data-urlencode 'query=histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m]))'
  ```
- [ ] **Feature Extraction**: Features being generated
  - Check: `sae_features_extracted_total` increasing
  - Verify: No error spikes in `sae_errors_total`
- [ ] **Memory Usage**: WASM sandbox < 200MB per node
  ```bash
  curl -s http://localhost:9090/api/v1/query \
    --data-urlencode 'query=wasm_memory_bytes'
  ```

### Consensus

- [ ] **First Batches Processed**: ≥10 batches completed
  ```bash
  curl -s http://localhost:3030/api/network | jq '.batches_processed'
  ```
- [ ] **Agreement Rate**: ≥70%
  - Target: >85%
  - Alert if: <60% for >10 minutes
- [ ] **ZKP Verification**: Proofs generating and verifying
  - Check: `zkp_proofs_generated_total` > 0
  - Verify: `zkp_verification_success_rate` > 0.8

### Reputation

- [ ] **Initial Scores**: All nodes at 0.5 (genesis default)
- [ ] **Score Updates**: Reputation changing based on contributions
  ```bash
  curl -s http://localhost:3030/api/network | jq '.reputation'
  ```
- [ ] **No Anomalies**: No sudden drops or spikes

---

## T+6h: Six Hour Review

### Sustained Operations

- [ ] **Uptime**: All nodes running ≥99% of last 6 hours
- [ ] **Peer Stability**: No nodes dropped below 2 peers
- [ ] **Consensus Rate**: Sustained ≥70% agreement
- [ ] **SAE Latency**: Sustained p95 < 500ms

### Data Integrity

- [ ] **Feedback Store**: Writing successfully
  ```bash
  curl -s http://localhost:3030/api/feedback | jq '.total_stored'
  ```
- [ ] **No Data Loss**: Batch hashes consistent across nodes
- [ ] **Lease Renewals**: Automatic renewals working
  - Check: No expired leases
  - Verify: Layer assignments stable

### Resource Usage

- [ ] **CPU**: Average < 60% per node
- [ ] **RAM**: Average < 70% per node
- [ ] **Disk**: Growth within expectations (< 1GB/hour)
- [ ] **Network**: Bandwidth within limits (< 50Mbps avg)

### First External Nodes

- [ ] **Join Requests**: Monitor for external node connections
- [ ] **Bootstrap**: Verify new nodes can discover seeds
- [ ] **Reputation**: New nodes start at 0.5, earn from contributions

---

## T+24h: Day 1 Complete

### 24-Hour Metrics Summary

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Node Uptime | ≥99% | __% | ☐ |
| Peer Count (avg) | ≥3/node | __ | ☐ |
| Consensus Rate | ≥70% | __% | ☐ |
| SAE Latency p95 | <500ms | __ms | ☐ |
| WASM Memory | <200MB | __MB | ☐ |
| Batches Processed | ≥1000 | __ | ☐ |
| External Nodes Joined | ≥1 | __ | ☐ |
| Critical Alerts | 0 | __ | ☐ |

### Day 1 Deliverables

- [ ] **Metrics Report**: Export 24h Grafana dashboard to PDF
- [ ] **Incident Log**: Document all issues and resolutions
- [ ] **Operator Feedback**: Collect feedback from seed operators
- [ ] **Community Update**: Post status in #ed2k-announce
- [ ] **Day 2 Plan**: Identify improvements and priorities

### Handoff to Steady-State

- [ ] Switch from launch monitoring to standard ops rotation
- [ ] Update on-call schedule
- [ ] Archive launch-specific alerts
- [ ] Schedule post-launch review meeting (within 48h)
- [ ] Announce "Steady-State Operations" to community

---

## Emergency Contacts

| Situation | Contact | Escalation |
|-----------|---------|------------|
| Node down | Network Ops | Launch Commander |
| Consensus failure | Consensus Watch | Launch Commander |
| Security incident | Security Lead | Launch Commander + Legal |
| Community panic | Community Liaison | Launch Commander |

## Quick Reference Commands

```bash
# Health check all nodes
for node in alpha bravo charlie delta echo; do
  echo -n "$node: "
  curl -sf http://$node.ed2kIA:3030/api/health | jq -r '.status'
done

# Consensus rate
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=sum(rate(consensus_votes_total{result="agree"}[5m])) / sum(rate(consensus_votes_total[5m]))'

# SAE latency p95
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=histogram_quantile(0.95, rate(sae_latency_seconds_bucket[5m]))'

# Peer counts
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=peer_count'

# View all alerts
curl -s http://localhost:9093/api/v1/alerts
```
