# Beta Feedback Tracker — v1.8.0-beta.1

**Purpose:** Central tracking of all beta feedback, bugs, and feature requests.

---

## Active Issues

| Issue | Module | Severity | Status | Assignee | ETA |
|-------|--------|----------|--------|----------|-----|
| — | — | — | — | — | — |

## Resolved Issues

| Issue | Module | Severity | Resolution | Closed Date |
|-------|--------|----------|------------|-------------|
| — | — | — | — | — |

---

## Severity Definitions

| Severity | Response SLA | Definition |
|----------|-------------|------------|
| P0 | 2 hours | Data loss, security vulnerability, complete crash |
| P1 | 12 hours | Major feature broken, no workaround |
| P2 | 48 hours | Feature degraded, workaround exists |
| P3 | 7 days | Minor issue, cosmetic, enhancement |

## Status Definitions

| Status | Meaning |
|--------|---------|
| New | Just reported, not yet reviewed |
| Triaged | Reviewed and categorized |
| In Progress | Actively being worked on |
| Waiting | Waiting for more info from reporter |
| Resolved | Fix applied, awaiting verification |
| Closed | Verified fixed or rejected |

---

## Module Categories

- **API Explorer** — REST endpoints, 3D visualization
- **Reputation Proof Schema** — Ed25519 proofs, tiers, anti-Sybil
- **Geographic Routing** — Haversine, RTT scoring, KAD fallback
- **WASM Mobile Bridge** — Memory limits, task scheduling, adaptive sync
- **Async Steering** — Late correction signals, priority scheduling
- **QuantConfig** — FP8/INT4 quantization, clamp ranges
- **P2P Swarm** — Network, peer management, gossipsub
- **DX Tools** — Justfile, Docker Compose, setup scripts
- **Other** — Documentation, build system, misc

---

## How to Update

1. Add new issues from GitHub with label `beta-bug` or `beta-feedback`
2. Update status as issues progress
3. Move resolved issues to "Resolved Issues" table
4. Update assignee and ETA as work is planned

---

*v1.8.0-beta.1 — Feedback Tracker*
*Generated: 2026-05-15*
