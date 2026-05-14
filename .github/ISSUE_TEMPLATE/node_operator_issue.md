---
name: Node Operator Issue
about: Report an issue specific to running an ed2kIA node
title: '[NODE] '
labels: ['node-operator']
assignees: []
---

## Node Environment

- **Node Role**: <!-- Full / Light / Validator -->
- **OS**: <!-- e.g., Ubuntu 24.04, Debian 12 -->
- **CPU/RAM**: <!-- e.g., 4 vCPU / 8GB RAM -->
- **Storage**: <!-- e.g., 50GB SSD -->
- **Network**: <!-- e.g., 100Mbps symmetric -->
- **ed2kIA Version**: <!-- e.g., v0.5.0 -->
- **Deployment Method**: <!-- systemd / Docker / Manual -->

## Node Metrics (at time of issue)

- **Peers Connected**: <!-- Number of active peers -->
- **SAE Latency (avg)**: <!-- e.g., 120ms -->
- **Consensus Rate**: <!-- e.g., 95% -->
- **WASM Memory Usage**: <!-- e.g., 150MB -->
- **Reputation Score**: <!-- e.g., 0.85 -->

## Lease State

<!-- List affected layer leases if applicable -->
| Layer ID | Owner | Status | Time Remaining |
|----------|-------|--------|----------------|
|          |       |        |                |

## Error Details

### Consensus/ZKP Errors

```
<!-- Paste consensus or ZKP verification errors -->
```

### SAE Forward Errors

```
<!-- Paste SAE loading or forward pass errors -->
```

### P2P/Gossipsub Errors

```
<!-- Paste libp2p or gossipsub errors -->
```

## Reproduction Steps

1. <!-- Step 1 -->
2. <!-- Step 2 -->
3. <!-- Step 3 -->

## Expected Behavior

<!-- What did you expect the node to do? -->

## Actual Behavior

<!-- What actually happened? -->

## Logs

```
<!-- Paste relevant node logs (redact peer IDs and sensitive data) -->
```

## Additional Context

<!-- Configuration files, network topology, or other relevant details -->
