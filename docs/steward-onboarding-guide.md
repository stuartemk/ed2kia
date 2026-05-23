# Steward Onboarding Guide — ed2kIA Live Testnet

> **Welcome, Steward.** This guide will take you from zero to connected on the ed2kIA live testnet in under 30 minutes. You will learn to launch a node, submit ethical feedback via the Steering Bridge, verify the Octahedron reacts, report issues, and join the steward community channel.

**Version:** v2.1.0-stable | **Feature Gate:** `v2.1-testnet-ops` | **Sprint:** 35

---

## Table of Contents

1. [What is a Steward?](#1-what-is-a-steward)
2. [Requirements](#2-requirements)
3. [Quickstart (5 min)](#3-quickstart-5-min)
4. [Connect to the Testnet](#4-connect-to-the-testnet)
5. [Use the Steering Bridge](#5-use-the-steering-bridge)
6. [Verify Octahedron Reacts](#6-verify-octahedron-reacts)
7. [Report Issues](#7-report-issues)
8. [Join the Steward Channel](#8-join-the-steward-channel)
9. [Troubleshooting](#9-troubleshooting)
10. [Next Steps](#10-next-steps)

---

## 1. What is a Steward?

A **Steward** is a trusted community member who operates an ed2kIA node on the live testnet and participates in **Human Steering** — the process of providing ethical feedback to the network's Sparse Autoencoder (SAE) interpretation layer.

### Steward Responsibilities

| Responsibility | Description |
|---|---|
| **Node Operation** | Run an ed2kIA node 24/7 on the testnet |
| **Ethical Feedback** | Submit steering events via CLI or Web interface |
| **Observation** | Monitor the Stuartian Octahedron for network health |
| **Reporting** | Report anomalies, bugs, or ethical concerns |
| **Community** | Participate in steward discussions and RFC reviews |

### Steward Benefits

- Early access to new features and governance proposals
- Recognition in the network's Steward Registry
- Influence on ethical direction via Steering Bridge
- Technical deep-dive into decentralized interpretability

---

## 2. Requirements

### Hardware

| Component | Minimum | Recommended |
|---|---|---|
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 16 GB |
| Storage | 20 GB SSD | 50 GB NVMe |
| Network | 10 Mbps | 100+ Mbps |

### Software

| Dependency | Version | Install |
|---|---|---|
| Rust | ≥ 1.75 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Cargo | Latest | Included with Rust |
| Git | ≥ 2.30 | `sudo apt install git` or [git-scm.com](https://git-scm.com) |
| Docker (optional) | ≥ 20.10 | [docs.docker.com/get-docker](https://docs.docker.com/get-docker/) |

### Platform Support

- ✅ Linux (Ubuntu 22.04+, Debian 12+, Arch)
- ✅ macOS (Ventura 13+, ARM64/x86_64)
- ✅ Windows (WSL2 required for POSIX scripts)
- ⚠️ Bare Windows (cargo run works, scripts require WSL/Git Bash)

### Verify Installation

```bash
# Check Rust
rustc --version   # Should show 1.75.0 or higher
cargo --version   # Should show 0.69.0 or higher

# Check Git
git --version     # Should show 2.30.0 or higher

# Check Docker (optional)
docker --version  # Should show 20.10.0 or higher
```

---

## 3. Quickstart (5 min)

### Step 1: Clone the Repository

```bash
git clone https://github.com/ed2kia/ed2kIA.git
cd ed2kIA
```

### Step 2: Build the Binary

```bash
# Build with testnet features enabled
cargo build --release --features v2.1-testnet-ops
```

Expected output:
```
   Compiling ed2kIA v2.1.0-stable
    Finished release [optimized] target(s) in 120.5s
```

> **Note:** First build may take 5-15 minutes depending on hardware. Subsequent builds are faster.

### Step 3: Verify the Build

```bash
# Check binary exists
ls -la target/release/ed2kIA-node

# Quick version check
./target/release/ed2kIA-node --version
# Expected: ed2kIA-node v2.1.0-stable
```

### Step 4: Alternative — Quickstart Script

For automated setup (Linux/macOS/WSL):

```bash
./scripts/quickstart.sh
```

This script handles: dependency check → clone → build → test → config → launch.

---

## 4. Connect to the Testnet

### Option A: Join Existing Testnet (Recommended)

If a testnet is already active, use the bootstrap file:

```bash
# Get the bootstrap file from the testnet operator
# Or generate your own if you're the first steward:
./scripts/activate-testnet.sh --nodes 1 --data ~/.ed2kIA/steward-node

# Connect to the active testnet
./target/release/ed2kIA-node \
  --bootstrap ~/.ed2kIA/testnet-live/testnet-bootstrap.json \
  --features v2.1-testnet-ops \
  --data-dir ~/.ed2kIA/steward-node
```

### Option B: Launch Your Own Testnet

If you're the first steward or want an isolated testnet:

```bash
# Launch a 3-node testnet
./scripts/activate-testnet.sh --nodes 3

# Check status
./scripts/activate-testnet.sh --status

# View the public dashboard
# Open: web/testnet-status.html in your browser
```

### Verify Connection

Your node log should show:

```
[INFO] ed2kIA-node v2.1.0-stable starting...
[INFO] Feature gate: v2.1-testnet-ops
[INFO] Loading bootstrap peers from testnet-bootstrap.json
[INFO] P2P: Listening on /ip4/127.0.0.1/tcp/18080
[INFO] Discovery: Found peer 12D3KooWTestNet0000000000001
[INFO] GossipSub: Subscribed to symbol-registry topic
[INFO] SymbolRegistry: CRDT sync initiated
```

### Key Indicators of Success

| Indicator | What to Look For |
|---|---|
| ✅ P2P Listening | `P2P: Listening on /ip4/...` |
| ✅ Peer Discovery | `Discovery: Found peer ...` |
| ✅ GossipSub | `Subscribed to symbol-registry topic` |
| ✅ CRDT Sync | `SymbolRegistry: CRDT sync initiated` |
| ✅ HTTP API | `HTTP: API on :18080/api/v1` |

---

## 5. Use the Steering Bridge

The **Steering Bridge** is the interface for submitting ethical feedback to the network. Your feedback influences the Stuartian Context Tensor (SCT) that guides SAE interpretation.

### CLI Steering Bridge

```bash
# Submit positive feedback (constructive interpretation)
./target/release/ed2kIA-node steering \
  --feedback "This SAE explanation correctly identifies the model's reasoning pattern" \
  --token-id 42 \
  --data-dir ~/.ed2kIA/steward-node

# Submit negative feedback (potentially harmful interpretation)
./target/release/ed2kIA-node steering \
  --feedback "This interpretation appears to hallucinate causal relationships" \
  --token-id 99 \
  --data-dir ~/.ed2kIA/steward-node
```

### Web Steering Bridge

1. Open the Steward Portal: `web/steward-portal.html`
2. Connect to your node's API (default: `http://localhost:18080`)
3. Navigate to "Steering Bridge" tab
4. Write your ethical feedback
5. Submit — your feedback is signed and broadcast via GossipSub

### Feedback Guidelines

| Do | Don't |
|---|---|
| Be specific about what you observed | Submit vague or emotional feedback |
| Reference specific token IDs or layers | Submit feedback without context |
| Explain why an interpretation is ethical/unethical | Spam the bridge with duplicate feedback |
| Consider the Golden Rule (X·Y·Z > 0) | Attempt to manipulate CE scores |

### How Your Feedback Works

```
Your Feedback
    ↓
Steering Bridge (signed with your node's Ed25519 key)
    ↓
GossipSub broadcast to all nodes
    ↓
SymbolRegistry CRDT merge (async conflict resolution)
    ↓
SCT Tensor update (X: benefit, Y: cost, Z: golden rule)
    ↓
SAE interpretation layer adjusts attention weights
    ↓
Octahedron visualization updates in real-time
```

---

## 6. Verify Octahedron Reacts

The **Stuartian Octahedron** is the 3D visualization of the network's collective ethical state. After submitting feedback, verify the Octahedron reacts.

### Steps

1. **Open the Testnet Dashboard:** `web/testnet-status.html`
2. **Locate the Octahedron** section (3D rotating visualization)
3. **Note the current state:**
   - **Green particles (Foco Superior):** Approved trajectories (Z > 0)
   - **Red particles (Foco Inferior):** Rejected trajectories (Z < 0)
   - **Blue particles:** Neutral / converging
4. **Submit feedback** via Steering Bridge (Step 5)
5. **Observe the Octahedron** update within 15-30 seconds

### What to Expect

| Feedback Type | Octahedron Reaction |
|---|---|
| Positive (constructive) | Green particles increase, Z-axis shifts upward |
| Negative (destructive) | Red particles increase, Z-axis shifts downward |
| Mixed (conflicting) | Particles converge toward equator (Z ≈ 0) |
| No feedback | Auto-rotation continues, particles stable |

### Axes Reference

| Axis | Meaning | Range |
|---|---|---|
| **X** | Collective Benefit | 0.0 (none) → 1.0 (maximum) |
| **Y** | Ethical Cost | 0.0 (none) → 1.0 (maximum) |
| **Z** | Golden Rule Evaluation | -1.0 (rejected) → +1.0 (approved) |

### Troubleshooting Octahedron

| Issue | Solution |
|---|---|
| Octahedron not loading | Check browser console for JS errors, verify `js/geometry-bridge.js` exists |
| No particles | Node may not have received feedback yet, wait 30s |
| Static visualization | Check that `status.json` is being updated by your node |
| Wrong colors | Verify SCT values in node logs (`grep SCT node.log`) |

---

## 7. Report Issues

As a Steward, you are the eyes and ears of the network. Report issues promptly and clearly.

### Issue Categories

| Category | Severity | Examples |
|---|---|---|
| **Critical** | P0 — Immediate | Node crash, data corruption, security vulnerability |
| **High** | P1 — Within 24h | P2P discovery failure, CRDT divergence, CE miscalculation |
| **Medium** | P2 — Within 1 week | UI bugs, performance degradation, documentation gaps |
| **Low** | P3 — Backlog | Cosmetic issues, feature requests, typo fixes |

### How to Report

#### Option A: GitHub Issues (Preferred)

1. Go to: https://github.com/ed2kia/ed2kIA/issues
2. Click "New Issue"
3. Select appropriate template:
   - 🐛 Bug Report
   - ✨ Feature Request
   - 📚 Documentation
   - 🔒 Security (use private vulnerability reporting)
4. Fill in the template with:
   - **Title:** Clear, specific description
   - **Steps to Reproduce:** Numbered list
   - **Expected Behavior:** What should happen
   - **Actual Behavior:** What actually happened
   - **Environment:** OS, Rust version, node version
   - **Logs:** Relevant excerpts from node logs

#### Option B: Steward Channel

For quick questions or time-sensitive issues:
1. Join the steward channel (Step 8)
2. Prefix your message with `[BUG]`, `[QUESTION]`, or `[URGENT]`
3. Include relevant details

#### Option C: Automated Reports

The testnet validation CI generates reports automatically:
- Location: `.github/workflows/testnet-validation.yml` artifacts
- Schedule: Weekly + on push to `scripts/deploy/.github`

### Report Template

```markdown
## Issue: [Brief description]

**Severity:** P0/P1/P2/P3
**Category:** Bug/Feature/Docs/Security

### Environment
- OS: [e.g., Ubuntu 22.04]
- Rust: [e.g., 1.75.0]
- ed2kIA: [e.g., v2.1.0-stable]
- Feature: [e.g., v2.1-testnet-ops]

### Steps to Reproduce
1. Step one
2. Step two
3. Step three

### Expected Behavior
What should happen.

### Actual Behavior
What actually happened.

### Logs
```
[Relevant log excerpts]
```

### Additional Context
[Any screenshots, metrics, or other details]
```

---

## 8. Join the Steward Channel

The steward channel is the primary communication hub for testnet stewards.

### Communication Platforms

| Platform | Purpose | Link |
|---|---|---|
| **GitHub Discussions** | RFC reviews, design decisions | [ed2kIA Discussions](https://github.com/ed2kia/ed2kIA/discussions) |
| **Discord** (planned) | Real-time chat, quick questions | TBA |
| **Matrix** (planned) | Decentralized chat | TBA |

### Getting Started

1. **Introduce yourself** in GitHub Discussions → "Steward Introductions"
2. **Share your setup:** OS, hardware, node status
3. **Ask questions:** No question is too basic
4. **Share learnings:** Help other stewards onboard
5. **Participate in RFCs:** Vote on governance proposals

### Steward Community Guidelines

- **Be respectful:** We welcome diverse perspectives
- **Be specific:** Clear questions get clear answers
- **Be patient:** Responses may take hours, not minutes
- **Be constructive:** Critique ideas, not people
- **Be ethical:** Follow the project's Code of Conduct

---

## 9. Troubleshooting

### Common Issues

#### Node Fails to Start

```bash
# Check Rust version
rustc --version  # Must be ≥ 1.75

# Check for port conflicts
sudo lsof -i :18080  # Linux/macOS
netstat -ano | findstr :18080  # Windows

# Try with verbose logging
RUST_LOG=debug ./target/release/ed2kIA-node --features v2.1-testnet-ops

# Check disk space
df -h ~/.ed2kIA
```

#### P2P Discovery Fails

```bash
# Verify bootstrap file exists
cat ~/.ed2kIA/testnet-live/testnet-bootstrap.json

# Check firewall
sudo ufw status  # Ubuntu
# Ensure ports 18080-18090 are open

# Try manual peer connection
./target/release/ed2kIA-node --peer /ip4/127.0.0.1/tcp/18080/p2p/12D3KooWTestNet0000000000001
```

#### CRDT Sync Stuck

```bash
# Check SymbolRegistry logs
grep "SymbolRegistry" ~/.ed2kIA/steward-node/logs/node.log

# Force sync restart
# Restart the node:
kill $(cat ~/.ed2kIA/steward-node/node.pid)
./target/release/ed2kIA-node --bootstrap ~/.ed2kIA/testnet-live/testnet-bootstrap.json
```

#### Steering Bridge Not Responding

```bash
# Check API is running
curl http://localhost:18080/api/v1/health

# Check feature gate
./target/release/ed2kIA-node --help | grep steering

# Verify feedback submission
./target/release/ed2kIA-node steering --feedback "test" --dry-run
```

#### Dashboard Not Loading

```bash
# Check status.json exists
ls -la ~/.ed2kIA/testnet-live/status.json

# Open browser console (F12) for JS errors
# Verify geometry-bridge.js loads
curl http://localhost:8080/js/geometry-bridge.js  # If served via HTTP

# Try opening HTML file directly
file:///path/to/ed2kIA/web/testnet-status.html
```

### Getting Help

1. **Check logs:** `~/.ed2kIA/steward-node/logs/`
2. **Search existing issues:** https://github.com/ed2kia/ed2kIA/issues
3. **Ask in steward channel:** GitHub Discussions
4. **Create a new issue:** Follow the template in Step 7

---

## 10. Next Steps

Congratulations! You are now an ed2kIA Steward. Here's what to do next:

### Immediate (Day 1)

- [ ] Keep your node running 24/7
- [ ] Monitor the Octahedron daily
- [ ] Submit at least one steering event
- [ ] Introduce yourself in the steward channel

### Short-term (Week 1)

- [ ] Read the [Technical Report](./technical-report.md)
- [ ] Review the [Project Constitution](./governance/project-constitution.md)
- [ ] Participate in one RFC discussion
- [ ] Set up automated monitoring (systemd/Docker)

### Medium-term (Month 1)

- [ ] Contribute to documentation
- [ ] Submit a feature proposal
- [ ] Mentor a new steward
- [ ] Review a pull request

### Long-term (Ongoing)

- [ ] Participate in governance voting
- [ ] Contribute code to the project
- [ ] Write technical blog posts
- [ ] Speak at conferences/meetups

---

## Appendix A: Quick Reference

### Essential Commands

```bash
# Start testnet
./scripts/activate-testnet.sh --nodes 3

# Connect to testnet
./target/release/ed2kIA-node --bootstrap ~/.ed2kIA/testnet-live/testnet-bootstrap.json

# Check status
./scripts/activate-testnet.sh --status

# Stop testnet
./scripts/activate-testnet.sh --stop

# Clean testnet
./scripts/activate-testnet.sh --clean

# Submit steering feedback
./target/release/ed2kIA-node steering --feedback "..." --token-id 42

# View dashboard
# Open: web/testnet-status.html
```

### File Locations

| File | Purpose |
|---|---|
| `~/.ed2kIA/testnet-live/` | Testnet data directory |
| `~/.ed2kIA/testnet-live/testnet-bootstrap.json` | Bootstrap peer config |
| `~/.ed2kIA/testnet-live/status.json` | Live status for dashboard |
| `~/.ed2kIA/steward-node/` | Your steward node data |
| `web/testnet-status.html` | Public dashboard |
| `docs/steward-onboarding-guide.md` | This guide |

### Key URLs

| Resource | URL |
|---|---|
| GitHub | https://github.com/ed2kia/ed2kIA |
| Dashboard | web/testnet-status.html |
| Technical Report | docs/technical-report.md |
| Steward Program | docs/steward-program.md |
| Governance | GOVERNANCE.md |
| CHANGELOG | CHANGELOG.md |

---

## Appendix B: Architecture Overview

For stewards who want to understand the internals:

```
┌─────────────────────────────────────────────────────────────┐
│                    ed2kIA Network                            │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐               │
│  │ Node 1   │◄──►│ Node 2   │◄──►│ Node 3   │   ...         │
│  │ (You)    │    │ (Peer)   │    │ (Peer)   │               │
│  └────┬─────┘    └────┬─────┘    └────┬─────┘               │
│       │               │               │                      │
│       └───────────────┴───────────────┘                      │
│                       │                                       │
│              ┌────────▼────────┐                              │
│              │   GossipSub     │                              │
│              │   (libp2p)      │                              │
│              └────────┬────────┘                              │
│                       │                                       │
│       ┌───────────────┼───────────────┐                      │
│       │               │               │                      │
│  ┌────▼─────┐  ┌──────▼──────┐  ┌────▼─────┐               │
│  │ Symbol   │  │  Existential│  │  Stuart- │               │
│  │Registry  │  │  Credit     │  │ ian      │               │
│  │ (CRDT)   │  │  Ledger     │  │ Octahedron│              │
│  └──────────┘  └─────────────┘  └──────────┘               │
│                                                               │
│  Human Steering → Steering Bridge → SCT → SAE → Octahedron   │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

**Last Updated:** Sprint 35 | v2.1.0-stable
**Maintainers:** ed2kIA Core Team
**Questions?** Open an issue or join the steward channel.
