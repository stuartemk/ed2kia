# Rollback Plan - ed2kIA v0.5.0

## Trigger Conditions

Initiate rollback if ANY of the following conditions persist for the specified duration:

| Condition | Threshold | Duration | Severity |
|-----------|-----------|----------|----------|
| Consensus agreement rate | < 60% | 15 minutes | **CRITICAL** |
| SAE forward latency (p95) | > 1000ms | 10 minutes | **CRITICAL** |
| Node crash rate | > 2 nodes/hour | 30 minutes | **CRITICAL** |
| WASM memory leak | > 500MB growing | 10 minutes | **HIGH** |
| Peer discovery failure | 0 new peers | 30 minutes | **HIGH** |
| GossipSub message loss | > 50% drop | 15 minutes | **HIGH** |
| Reputation system corruption | Scores = 0 or NaN | Immediate | **CRITICAL** |

---

## Rollback Levels

### Level 1: Single Node Recovery

**When**: One node experiencing issues, network stable

**Procedure**:
```bash
# 1. Identify affected node
NODE_ID="seed-alpha-001"

# 2. Graceful shutdown
systemctl stop ed2kia-${NODE_ID}

# 3. Backup current state
cp -r /var/lib/ed2kia/${NODE_ID} /var/lib/ed2kia/${NODE_ID}.backup.$(date +%Y%m%d%H%M)

# 4. Clear corrupted data (if applicable)
rm -rf /var/lib/ed2kia/${NODE_ID}/data/*.redb

# 5. Restart with clean state
systemctl start ed2kia-${NODE_ID}

# 6. Verify recovery
curl -sf http://localhost:3030/api/health | jq '.status'
```

**Expected Recovery Time**: 5-10 minutes

---

### Level 2: Feature Disable (core-only Fallback)

**When**: Network-wide issue caused by specific feature (ZKP, steering, etc.)

**Procedure**:
```bash
# 1. Notify all operators
# Send message to #ed2k-incidents: "ROLLBACK L2: Disabling experimental features"

# 2. Restart all nodes with core-only mode
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "systemctl stop ed2kia"
done

sleep 10

for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "\
    ED2K_FEATURE_FLAGS=core-only \
    systemctl start ed2kia"
done

# 3. Wait for re-bootstrap
sleep 60

# 4. Verify core functionality
for NODE_ID in alpha bravo charlie delta echo; do
  echo -n "$NODE_ID: "
  curl -sf http://${NODE_ID}.ed2kIA:3030/api/health | jq -r '.status'
done

# 5. Check consensus recovery
curl -s http://localhost:9090/api/v1/query \
  --data-urlencode 'query=consensus_agreement_rate'
```

**Expected Recovery Time**: 15-30 minutes

---

### Level 3: Full Network Restart

**When**: Consensus broken, network partition, or data corruption

**Procedure**:
```bash
# 1. EMERGENCY: Notify all operators and stakeholders
# #ed2k-incidents: "ROLLBACK L3: Full network restart initiated"

# 2. Graceful shutdown (if possible)
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "systemctl stop ed2kia" 2>/dev/null || \
  ssh ${NODE_ID}.ed2kIA "killall ed2kia" 2>/dev/null || true
done

sleep 15

# 3. Backup ALL data
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "\
    tar czf /backups/ed2kia_${NODE_ID}_pre_rollback_$(date +%Y%m%d%H%M).tar.gz \
    /var/lib/ed2kia/${NODE_ID}/"
done

# 4. Clear state (preserve keys and config)
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "\
    rm -rf /var/lib/ed2kia/${NODE_ID}/data/*.redb \
    /var/lib/ed2kia/${NODE_ID}/data/*.wal"
done

# 5. Sequential restart (seeds first)
for NODE_ID in alpha bravo charlie; do
  ssh ${NODE_ID}.ed2kIA "systemctl start ed2kia"
  sleep 10
done

# 6. Wait for seed mesh to form
sleep 60

# 7. Start validator nodes
for NODE_ID in delta echo; do
  ssh ${NODE_ID}.ed2kIA "systemctl start ed2kia"
  sleep 10
done

# 8. Verify network recovery
sleep 30
echo "=== Network Status ==="
for NODE_ID in alpha bravo charlie delta echo; do
  echo -n "$NODE_ID health: "
  curl -sf http://${NODE_ID}.ed2kIA:3030/api/health | jq -r '.status' || echo "UNREACHABLE"
  echo -n "$NODE_ID peers: "
  curl -sf http://${NODE_ID}.ed2kIA:3030/api/network | jq '.peer_count' || echo "0"
done
```

**Expected Recovery Time**: 30-60 minutes

---

### Level 4: Version Rollback

**When**: v0.5.0 has critical bug, need to revert to v0.4.x

**Procedure**:
```bash
# 1. EMERGENCY: Announce version rollback
# #ed2k-incidents: "ROLLBACK L4: Reverting to v0.4.2"

# 2. Stop all nodes
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "systemctl stop ed2kia"
done

# 3. Backup v0.5.0 state completely
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "\
    tar czf /backups/ed2kia_v050_full_$(date +%Y%m%d%H%M).tar.gz \
    /var/lib/ed2kia/ /etc/ed2kia/"
done

# 4. Restore v0.4.2 binary
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "\
    cp /usr/local/bin/ed2kia.v0.4.2 /usr/local/bin/ed2kia"
done

# 5. Restore v0.4.2 configuration (if needed)
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "\
    cp /etc/ed2kia/config.v0.4.2.toml /etc/ed2kia/config.toml"
done

# 6. Migrate data (if schema changed)
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "\
    ed2kia migrate --from v0.5.0 --to v0.4.2 --data-dir /var/lib/ed2kia/${NODE_ID}"
done

# 7. Sequential restart
for NODE_ID in alpha bravo charlie delta echo; do
  ssh ${NODE_ID}.ed2kIA "systemctl start ed2kia"
  sleep 15
done

# 8. Extended verification (10 minutes)
sleep 600

echo "=== Post-Rollback Status ==="
echo "Version:"
ssh alpha.ed2kIA "ed2kia --version"
echo ""
echo "Network Health:"
for NODE_ID in alpha bravo charlie delta echo; do
  curl -sf http://${NODE_ID}.ed2kIA:3030/api/health | jq '.'
done
```

**Expected Recovery Time**: 1-2 hours

---

## Lease Restoration

After ANY rollback level, verify and restore layer leases:

```bash
# Check current lease state
curl -s http://alpha.ed2kIA:3030/api/network | jq '.leases'

# If leases are expired or inconsistent, redistribute
ed2kia bootstrap genesis \
  --config launch/genesis/config.toml \
  --seeds launch/genesis/seed_nodes.json

# Verify all 16 layers assigned
curl -s http://alpha.ed2kIA:3030/api/network | jq '.layers_assigned | length'
# Expected: 16
```

---

## Post-Rollback Checklist

- [ ] All nodes running and healthy
- [ ] Consensus rate > 70%
- [ ] SAE latency p95 < 500ms
- [ ] All 16 layers have active leases
- [ ] Peer count stable (≥3 per node)
- [ ] Monitoring stack collecting metrics
- [ ] No active critical alerts
- [ ] Root cause identified (create incident report)
- [ ] Community notified of resolution
- [ ] Post-mortem scheduled (within 48 hours)

---

## Communication Template

### Rollback Initiation
```
[ROLLBACK] Level: L{1-4} | Time: {ISO8601}
Trigger: {condition}
Impact: {affected nodes/features}
ETA Recovery: {minutes}
Action: {brief description}
Updates: #ed2k-incidents
```

### Rollback Complete
```
[ROLLBACK COMPLETE] Level: L{1-4} | Duration: {minutes}
Root Cause: {identified cause}
Resolution: {what fixed it}
Current Status: {metrics}
Follow-up: {next steps}
Post-mortem: {date/time}
```

---

## Contact Escalation

| Level | Notify | Method | Response Time |
|-------|--------|--------|---------------|
| L1 | Network Ops | Slack | 5 min |
| L2 | Network Ops + Launch Commander | Slack + Phone | 5 min |
| L3 | All team + Community | Slack + Email | Immediate |
| L4 | All team + Community + Stakeholders | All channels | Immediate |
