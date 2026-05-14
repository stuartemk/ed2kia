# Canary Deployment Execution Checklist: ed2kIA v0.6.0-RC

> **Purpose**: Step-by-step validation checklist for T0 → T+72h rollout.
> **Reference**: `release/v0.6.0-rc/rollout_plan.md`
> **Status**: Template — Fill during execution

---

## Phase T0: Seed Nodes (10%) — Day 0

| # | Step | Responsible | Validation | Sign-Off | Status |
|---|---|---|---|---|---|
| T0-1 | Pre-deployment: Run `cargo test --features "phase6-sprint2"` | Dev Lead | 170 tests pass | [ ] | ⏳ |
| T0-2 | Pre-deployment: Run `cargo clippy --features "phase6-sprint2" -- -D warnings` | Dev Lead | 0 warnings | [ ] | ⏳ |
| T0-3 | Build release binaries for target architectures | DevOps | Binaries available | [ ] | ⏳ |
| T0-4 | Backup seed node state (reputation, governance) | DevOps | Backup verified | [ ] | ⏳ |
| T0-5 | Deploy v0.6.0-RC to seed nodes (3-5 nodes) | DevOps | `canary_deploy.sh --phase canary` | [ ] | ⏳ |
| T0-6 | Verify API v1 health on seed nodes | QA | `curl /api/v1/health` → 200 | [ ] | ⏳ |
| T0-7 | Verify API v2 health on seed nodes | QA | `curl /api/v2/health` → 200 | [ ] | ⏳ |
| T0-8 | Start telemetry simulator (normal scenario) | DevOps | `telemetry_simulator.sh --scenario normal` | [ ] | ⏳ |
| T0-9 | Monitor: Consensus participation ≥ 85% | Monitor | Check every 15min × 24h | [ ] | ⏳ |
| T0-10 | Monitor: Federation round completion ≥ 90% | Monitor | Check every 15min × 24h | [ ] | ⏳ |
| T0-11 | Monitor: API v2 error rate < 0.5% | Monitor | Check every 15min × 24h | [ ] | ⏳ |
| T0-12 | Monitor: No panics or unrecovered errors | Monitor | Log review every 1h | [ ] | ⏳ |
| T0-13 | Run threshold checker on 24h telemetry | QA | `threshold_checker.sh` → exit 0 | [ ] | ⏳ |
| T0-14 | **T0 Sign-Off**: All criteria met? | Release Manager | All T0-1 to T0-13 ✅ | [ ] | ⏳ |

### T0 Rollback Triggers (ANY)
- [ ] Consensus participation drops below 70%
- [ ] API error rate exceeds 2%
- [ ] Any node panic requiring manual restart
- [ ] Federation sync failures > 5 consecutive rounds

**If triggered**: Execute `./ops/rollback_v0.6.0.sh --auto` immediately.

---

## Phase T+24h: Validated Nodes (50%) — Day 1

| # | Step | Responsible | Validation | Sign-Off | Status |
|---|---|---|---|---|---|
| T1-1 | Review T0 results: All criteria passed? | Release Manager | T0-14 signed | [ ] | ⏳ |
| T1-2 | Publish v0.6.0-RC to package registry | DevOps | Image available | [ ] | ⏳ |
| T1-3 | Notify eligible operators (reputation ≥ 0.7) | Comms | Announcement sent | [ ] | ⏳ |
| T1-4 | Deploy to 50% of network | DevOps | `canary_deploy.sh --phase expand --target 50` | [ ] | ⏳ |
| T1-5 | Verify cross-version compatibility | QA | v0.5.0 ↔ v0.6.0 gossip works | [ ] | ⏳ |
| T1-6 | Monitor: Network consensus ≥ 85% | Monitor | Check every 15min × 48h | [ ] | ⏳ |
| T1-7 | Monitor: SAE latency p95 ≤ 400ms | Monitor | Check every 15min × 48h | [ ] | ⏳ |
| T1-8 | Monitor: Federation rounds complete on time | Monitor | Check every 15min × 48h | [ ] | ⏳ |
| T1-9 | Monitor: Staking registry stable | Monitor | No unexpected slashes | [ ] | ⏳ |
| T1-10 | Run threshold checker on 48h telemetry | QA | `threshold_checker.sh` → exit 0 | [ ] | ⏳ |
| T1-11 | **T+24h Sign-Off**: All criteria met? | Release Manager | All T1-1 to T1-10 ✅ | [ ] | ⏳ |

### T+24h Rollback Triggers (ANY)
- [ ] Network consensus drops below 75%
- [ ] SAE latency p95 exceeds 800ms
- [ ] More than 3 node crashes in 1 hour
- [ ] Staking registry corruption detected
- [ ] Cross-version gossip failures

---

## Phase T+72h: Full Network (100%) — Day 3

| # | Step | Responsible | Validation | Sign-Off | Status |
|---|---|---|---|---|---|
| T2-1 | Review T+24h results: All criteria passed? | Release Manager | T1-11 signed | [ ] | ⏳ |
| T2-2 | Update default Docker image tag | DevOps | Tag updated | [ ] | ⏳ |
| T2-3 | Deploy to remaining 50% of network | DevOps | `canary_deploy.sh --phase full --target 100` | [ ] | ⏳ |
| T2-4 | Enable experimental features for research nodes | DevOps | Feature flags set | [ ] | ⏳ |
| T2-5 | Monitor: All T0 + T+24h criteria maintained | Monitor | 7 days continuous | [ ] | ⏳ |
| T2-6 | Monitor: Performance within 10% of v0.5.0 baselines | Monitor | Benchmark comparison | [ ] | ⏳ |
| T2-7 | Collect community feedback | Comms | Governance channel review | [ ] | ⏳ |
| T2-8 | Run threshold checker on 7d telemetry | QA | `threshold_checker.sh` → exit 0 | [ ] | ⏳ |
| T2-9 | **T+72h Sign-Off**: All criteria met? | Release Manager | All T2-1 to T2-8 ✅ | [ ] | ⏳ |

---

## Promotion to v0.6.0 STABLE — Day 10

| # | Step | Responsible | Validation | Sign-Off | Status |
|---|---|---|---|---|---|
| S1-1 | 7 days stable operation confirmed | Release Manager | T2-9 signed + 7d | [ ] | ⏳ |
| S1-2 | No critical bugs reported | QA | Issue review | [ ] | ⏳ |
| S1-3 | Community feedback positive | Comms | Governance channel | [ ] | ⏳ |
| S1-4 | Tag `main` with `v0.6.0` | DevOps | Git tag created | [ ] | ⏳ |
| S1-5 | Update `Cargo.toml` version | Dev Lead | Version = 0.6.0 | [ ] | ⏳ |
| S1-6 | Publish release notes | Comms | GitHub Release | [ ] | ⏳ |
| S1-7 | Archive RC documentation | DevOps | Docs archived | [ ] | ⏳ |
| S1-8 | **STABLE Sign-Off**: v0.6.0 is production-ready | Release Manager | All S1-1 to S1-7 ✅ | [ ] | ⏳ |

---

## Emergency Contacts

| Role | Contact | Escalation |
|---|---|---|
| Release Manager | [TBD] | Primary decision maker |
| Dev Lead | [TBD] | Technical decisions |
| DevOps Lead | [TBD] | Deployment/rollback |
| Security Lead | [TBD] | Security incidents |
| Community Manager | [TBD] | Operator communication |

---

## Notes

- Each sign-off requires explicit approval from the named responsible party.
- If ANY rollback trigger is hit, stop immediately and execute rollback procedure.
- All telemetry data must be preserved for post-mortem analysis.
- This checklist is a living document — update with lessons learned.

---

*Template for ed2kIA v0.6.0-RC canary deployment. Execute with caution, verify at each step.*
