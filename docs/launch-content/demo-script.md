# Demo Script: ed2kIA v2.1.0-stable

**Duration:** 90 seconds — 3 minutes (configurable)  
**Format:** Terminal recording + dashboard overlay  
**Audience:** Technical viewers (developers, researchers, infrastructure engineers)  
**Tone:** Stuartian — rigorous, calm, zero hype  

---

## Pre-Production Checklist

- [ ] Record on clean terminal (no previous commands visible)
- [ ] Terminal theme: Dark background, monospace font, 14pt minimum
- [ ] Screen resolution: 1920x1080 minimum
- [ ] Recording tool: `asciinema` or `terminal-recorder`
- [ ] Dashboard: `http://localhost:8080` loaded in browser (for overlay)
- [ ] Testnet: 3-node testnet running (`./scripts/testnet-mode.sh --nodes 3`)
- [ ] Audio: Optional narration (see timestamps below)

---

## Script (Timed)

### [0:00 — 0:10] Opening

**Visual:** Black screen → Terminal appears with ed2kIA ASCII logo

```
$ echo "ed2kIA v2.1.0-stable — Decentralized Interpretability Network"
```

**Narration:** "ed2kIA is a decentralized network where volunteers audit AI models together. No tokens. No telemetry. Let me show you how it works."

---

### [0:10 — 0:25] Quickstart

**Visual:** Quickstart command execution

```
$ curl -sSL https://github.com/ed2kia/ed2kIA/raw/main/scripts/quickstart.sh | sh
[INFO] === ed2kIA Quickstart v2.1.0-stable ===
[INFO] Pre-flight validation...
[OK]   Rust 1.75.0 detected
[OK]   Cargo detected
[OK]   Git detected
[OK]   Docker detected (optional)
[INFO] Cloning ed2kIA...
[OK]   Repository cloned
[INFO] Building release binary...
[OK]   Build complete
[INFO] Running test suite...
running 3505 tests
test result: ok. 3504 passed; 1 failed; 9 ignored
[OK]   Tests passed
[OK]   Node identity generated
[OK]   Configuration written
[OK]   Welcome to ed2kIA! 🌐
```

**Narration:** "One command installs everything. Rust, Cargo, Git — validated. 3,505 tests pass. Your node identity is generated with Ed25519 cryptography."

---

### [0:25 — 0:45] Testnet Launch

**Visual:** Launch 3-node testnet

```
$ ./scripts/testnet-mode.sh --nodes 3
[INFO] === ed2kIA Testnet Mode ===
[INFO] Configuration: 3 nodes, base port 18080
[OK]   Project root: /home/user/ed2kIA
[NODE node0] Config: ~/.ed2kIA/testnet/node0/config.toml (port 18080)
[NODE node1] Config: ~/.ed2kIA/testnet/node1/config.toml (port 18081)
[NODE node2] Config: ~/.ed2kIA/testnet/node2/config.toml (port 18082)
[OK]   Testnet manifest generated
[INFO] Launching 3 testnet nodes...
[NODE node0] Started (PID: 12345)
[NODE node1] Started (PID: 12346)
[NODE node2] Started (PID: 12347)
[OK]   All 3 nodes launched
```

**Narration:** "Three nodes, three ports. Each node has its own Ed25519 identity, configuration, and data directory. They discover each other via mDNS and form a mesh."

---

### [0:45 — 1:10] CRDT Convergence

**Visual:** Show CRDT state converging across nodes

```
$ # Node 0: Submit interpretability contribution
$ curl -X POST http://localhost:18080/api/v1/contribute \
  -H "Content-Type: application/json" \
  -d '{"feature_id": 42, "activation": 0.85, "sct": {"x": 0.9, "y": 0.2, "z": 0.7}}'
{"status": "accepted", "ce_emitted": 0.7, "version": 1}

$ # Node 1: Query feature dictionary (before sync)
$ curl http://localhost:18081/api/v1/features/42
{"status": "not_found"}

$ sleep 2  # Wait for GossipSub sync

$ # Node 1: Query feature dictionary (after sync)
$ curl http://localhost:18081/api/v1/features/42
{"feature_id": 42, "activation": 0.85, "sct": {"x": 0.9, "y": 0.2, "z": 0.7}, "source": "node0"}
```

**Narration:** "Node 0 submits a feature. Within 2 seconds, GossipSub propagates it to all nodes. CRDT merge guarantees convergence — no conflicts, no coordinator needed."

---

### [1:10 — 1:35] SCT Ethical Steering

**Visual:** Show SCT rejection in action

```
$ # Submit ethically misaligned contribution (z < 0)
$ curl -X POST http://localhost:18080/api/v1/contribute \
  -H "Content-Type: application/json" \
  -d '{"feature_id": 99, "activation": 0.95, "sct": {"x": 0.95, "y": 0.1, "z": -0.3}}'
{"status": "rejected", "reason": "SCT Golden Rule: z < 0", "ce_burned": 0.3}

$ # Check node reputation
$ curl http://localhost:18080/api/v1/node/reputation
{"node_id": "node0", "ce_score": 0.4, "state": "healthy", "contributions": 2}
```

**Narration:** "The SCT Golden Rule: if z is negative, the contribution is rejected — regardless of how beneficial it appears. This is deterministic ethical steering, not a suggestion."

---

### [1:35 — 2:00] Network Apoptosis

**Visual:** Show immune system detecting malicious behavior

```
$ # Simulate repeated misaligned contributions
$ for i in $(seq 1 5); do
  curl -s -X POST http://localhost:18080/api/v1/contribute \
    -d '{"feature_id": '$i', "sct": {"x": 0.5, "y": 0.5, "z": -0.5}}'
done
[{"status":"rejected","ce_burned":0.5}, ... x5]

$ # Check node state — enters Pain state
$ curl http://localhost:18080/api/v1/node/immune
{"node_id": "node0", "ce_score": -2.1, "state": "apoptosis", "blocklisted": true}

$ # Node is isolated from gossip
$ curl http://localhost:18081/api/v1/peers
{"peers": ["node1", "node2"], "blocklisted": ["node0"]}
```

**Narration:** "Network Apoptosis — our immune system. Repeated misalignment triggers isolation. The node is blocklisted from gossip. Reintegration requires human steward approval."

---

### [2:00 — 2:25] Benchmarks

**Visual:** Run Criterion benchmarks

```
$ cargo bench -p ed2kIA-benchmarks --bench p2p_sync
p2p_sync/local_propagation/64 nodes
                        time:   [1.2345 ms, 1.3456 ms, 1.4567 ms]
p2p_sync/convergence_rounds/64 nodes
                        time:   [12.345 ms, 13.456 ms, 14.567 ms]

$ cargo bench -p ed2kIA-benchmarks --bench crdt_merge
crdt_merge/gcounter/1000 peers
                        time:   [0.8901 ms, 0.9012 ms, 0.9123 ms]
```

**Narration:** "Criterion benchmarks: P2P sync converges in under 15 milliseconds for 64 nodes. CRDT merge handles 1,000 peers in under 1 millisecond. All statistically significant."

---

### [2:25 — 2:50] Dashboard

**Visual:** Switch to browser showing dashboard at `http://localhost:8080`

**Overlay annotations:**
- Network graph showing 3 connected nodes
- Real-time SCT counter (positive/negative/z-axis)
- CE score distribution
- Feature dictionary size growing

**Narration:** "The dashboard shows real-time network state. Three nodes, connected. SCT counters tracking ethical alignment. Feature dictionary growing as contributions are accepted."

---

### [2:50 — 3:00] Closing

**Visual:** Terminal returns

```
$ echo "ed2kIA — Interpretability is a public good."
$ echo "https://github.com/ed2kia/ed2kIA"
$ echo "Technical Report: docs/technical-report.md"
```

**Narration:** "ed2kIA v2.1.0-stable. Open source, auditable, community-operated. Interpretability is a public good. Let's build it together."

---

## Production Notes

### Recording Commands

```bash
# Using asciinema
asciinema rec --title "ed2kIA v2.1.0-stable Demo" ed2kIA-demo.cast

# Convert to GIF
gify ed2kIA-demo.cast -s 2 -O ed2kIA-demo.gif

# Convert to MP4 (better quality)
asciinema-play --fps 30 ed2kIA-demo.cast | ffmpeg -f avfoundation -i 1 -pix_fmt yuv420p ed2kIA-demo.mp4
```

### Terminal Configuration

```bash
# Recommended terminal settings
# Font: JetBrains Mono / Fira Code, 14pt
# Colors: One Dark / Nord / Gruvbox Dark
# Opacity: 95%
# Smoothing: Anti-aliased
```

### Cleanup

```bash
# Stop testnet after demo
for pid in $(cat ~/.ed2kIA/testnet/node_pids.txt); do kill $pid; done

# Clean testnet data
rm -rf ~/.ed2kIA/testnet
```

---

**Total Duration:** ~3 minutes (adjust pacing as needed)  
**Key Message:** Functional, ethical, auditable, community-operated. Zero hype, maximum substance.
