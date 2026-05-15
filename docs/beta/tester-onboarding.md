# Beta Tester Onboarding Guide — ed2kIA v1.8.0-beta.1

**Welcome!** You're joining the ed2kIA beta testing program. This guide will help you set up your environment, run nodes, and report feedback effectively.

---

## Requirements

### Minimum
- **OS:** Linux (Ubuntu 20.04+), macOS 12+, Windows 10+ (WSL2 recommended)
- **RAM:** 4GB minimum, 8GB recommended
- **Disk:** 2GB free space
- **Network:** Stable internet connection
- **Rust:** 1.70+ ([rustup.rs](https://rustup.rs))
- **Git:** 2.30+

### Recommended
- **Docker + Docker Compose** (for multi-node testing)
- **Just** command runner: `cargo install just`
- **Disk:** 10GB free (for benchmarks and logs)

---

## Installation

### 1. Clone & Checkout Beta

```bash
git clone https://github.com/Stuartemk/ed2kIA.git
cd ed2kIA
git checkout v1.8.0-beta.1
```

### 2. Build

```bash
# Build with all beta features
cargo build --features v1.8-sprint2

# Or build stable only
cargo build --features stable
```

### 3. Quick Validation

```bash
# Run tests
cargo test --features v1.8-sprint2

# Run linter
cargo clippy --features v1.8-sprint2
```

---

## Running Nodes

### Single Node (Local Dev)

```bash
# Using Just (recommended)
just dev

# Or directly
cargo run --features v1.8-sprint2
```

### Multi-Node Mesh (Docker)

```bash
# Start 3-node mesh + Prometheus + Grafana
just docker-compose

# View logs
docker compose -f devtools/docker-compose.yml logs -f

# Stop
docker compose -f devtools/docker-compose.yml down
```

### Manual Multi-Node

```bash
# Terminal 1: Node A (port 9001)
RUST_LOG=info cargo run --features v1.8-sprint2 -- --port 9001

# Terminal 2: Node B (port 9002)
RUST_LOG=info cargo run --features v1.8-sprint2 -- --port 9002

# Terminal 3: Node C (port 9003)
RUST_LOG=info cargo run --features v1.8-sprint2 -- --port 9003
```

---

## Feature Testing Checklist

### Sprint 1 Features (`--features v1.8-sprint1`)

- [ ] **API Explorer v1** — Test REST endpoints for 3D concept visualization
- [ ] **Reputation Proof Schema** — Verify Ed25519 proof creation and validation
- [ ] **QuantConfig** — Test FP8/INT4 quantization with clamp ranges
- [ ] **Async Steering v1** — Verify late correction signals in tensor pipelines

### Sprint 2 Features (`--features v1.8-sprint2`)

- [ ] **Geographic Routing** — Test peer selection with lat/lon data
- [ ] **WASM Mobile Bridge** — Verify memory limits (64MB) and task scheduling
- [ ] **KAD Fallback** — Test fallback when geo data is insufficient

### Developer Experience

- [ ] **Justfile** — Run `just` to see available recipes
- [ ] **Docker Compose** — Start dev mesh with `just docker-compose`
- [ ] **Setup Script** — Run `devtools/setup.sh --full`

---

## Reporting Bugs

### GitHub Issue Template

Use the **Beta Bug Report** template:
1. Go to [GitHub Issues](https://github.com/Stuartemk/ed2kIA/issues)
2. Click "New Issue"
3. Select "Beta Bug Report" template
4. Fill in all required fields

### Required Information

- **Environment:** OS, Rust version, feature flags used
- **Reproduction Steps:** Clear, numbered steps to reproduce
- **Expected Behavior:** What should happen
- **Actual Behavior:** What actually happened
- **Logs:** Relevant log output (use `RUST_LOG=debug` for detailed logs)
- **Severity:** P0 (critical) / P1 (high) / P2 (medium) / P3 (low)

### Severity Definitions

| Severity | Definition | Response Time |
|----------|-----------|---------------|
| **P0** | Data loss, security vulnerability, complete crash | 2 hours |
| **P1** | Major feature broken, no workaround | 12 hours |
| **P2** | Feature degraded, workaround exists | 48 hours |
| **P3** | Minor issue, cosmetic, enhancement | 7 days |

---

## Support Channels

| Channel | Purpose |
|---------|---------|
| **GitHub Issues** | Bug reports, feature requests |
| **GitHub Discussions** | Questions, feedback, ideas |
| **Discord #beta-testing** | Real-time support, chat with team |
| **Discord #performance** | Performance issues, benchmark results |
| **SECURITY.md** | Security vulnerabilities (private disclosure) |

---

## Collecting Logs

### Basic Logs

```bash
# Info level (default)
RUST_LOG=info cargo run --features v1.8-sprint2

# Debug level (detailed)
RUST_LOG=debug cargo run --features v1.8-sprint2

# Trace level (very detailed)
RUST_LOG=trace cargo run --features v1.8-sprint2
```

### Docker Logs

```bash
# All containers
docker compose -f devtools/docker-compose.yml logs

# Specific container
docker compose -f devtools/docker-compose.yml logs ed2kIA-node

# Follow logs
docker compose -f devtools/docker-compose.yml logs -f
```

### Saving Logs

```bash
# Save to file
RUST_LOG=debug cargo run --features v1.8-sprint2 > beta-test.log 2>&1

# Docker logs to file
docker compose -f devtools/docker-compose.yml logs > docker-beta.log
```

---

## Beta Feedback Form

For general feedback (not bugs), use the **Beta Feedback** template on GitHub Issues or post in Discord #beta-testing.

Include:
- What feature you tested
- What worked well
- What could be improved
- Any suggestions or ideas

---

## Next Steps

1. ✅ Complete installation
2. ✅ Run validation tests
3. ✅ Test features from checklist
4. ✅ Report any issues found
5. ✅ Share feedback in Discord

**Thank you for helping make ed2kIA better!** 🚀

---

*v1.8.0-beta.1 — Beta Tester Onboarding Guide*
*Generated: 2026-05-15*
