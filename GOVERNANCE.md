# ed2kIA Governance

**Version:** v1.0
**Fecha:** 2026-05-15
**Estado:** ACTIVE
**FASE:** 62 — Post-Beta Retrospective & v1.9 Roadmap

---

## 1. Mission

ed2kIA is a distributed interpretability network using Sparse Autoencoders (SAE) to analyze neural network activations across a federated P2P network. Our mission is to build transparent, verifiable AI through open-source collaboration.

**Core Values:**
- **Transparency:** All decisions documented and accessible
- **Meritocracy:** Influence earned through contribution
- **Safety:** Zero unsafe code, ethical use clause
- **Community:** Inclusive, welcoming, diverse
- **Quality:** High standards for code, docs, and design

---

## 2. Project Roles

### 2.1 Contributor Tiers

| Tier | Requirements | Permissions |
|------|-------------|-------------|
| **Spectator** | Star the repo | Read access, discussions |
| **Beta Tester** | Complete beta onboarding | Beta access, feedback templates |
| **Contributor** | 1 merged PR | Open issues, submit PRs |
| **Active Contributor** | 5 merged PRs | Label issues, review PRs |
| **Maintainer** | 20 merged PRs + nomination | Merge PRs, manage releases |
| **Tech Lead** | Maintainer + election | Architecture decisions, RFC approval |
| **Guardian** | Tech Lead + 1yr tenure | Governance changes, final approval |

### 2.2 Current Roles

| Role | Member | Responsibilities |
|------|--------|------------------|
| Project Lead | @Stuartemk | Vision, roadmap, final decisions |
| Release Engineer | Qweni (AI) | CI/CD, releases, validation |
| Tech Lead | @Stuartemk | Architecture, code review |
| Community Lead | @Stuartemk | Outreach, mentorship, grants |

---

## 3. Decision Making

### 3.1 Decision Framework

| Decision Type | Process | Approval | Examples |
|--------------|---------|----------|----------|
| **Trivial** | Implement + document | Self | Typos, minor fixes |
| **Technical** | Discussion → Implement | Tech Lead | Refactors, new modules |
| **Architectural** | RFC → Discussion → Vote | Tech Lead + Maintainer | New protocols, major features |
| **Governance** | RFC → Community vote | Guardian + majority | Role changes, policy updates |
| **Release** | Validation → Sign-off | Release Eng + Tech Lead | Version tags, releases |

### 3.2 RFC Process

1. **Draft:** Create `docs/rfcs/rfc-NNN-title.md`
2. **Discuss:** Open GitHub Discussion for community input (≥7 days)
3. **Revise:** Update RFC based on feedback
4. **Vote:** Maintainers + Tech Lead vote
5. **Implement:** Approved RFC becomes implementation task
6. **Retrospective:** Post-implementation review

### 3.3 Release Approval

**Current Process (v1.8-beta):**
- Release Engineer (Qweni) validates + signs off
- Tech Lead (@Stuartemk) reviews + approves

**Proposed Process (v1.9+):**
- Release Engineer validates
- Tech Lead reviews
- Community representative (elected Maintainer) confirms
- 3-signoff model for production releases

---

## 4. Code Standards

### 4.1 Quality Gates

| Gate | Requirement | Enforcement |
|------|-------------|-------------|
| Tests | ≥ 99% pass rate | CI |
| Coverage | ≥ 80% (target) | cargo-llvm-cov |
| Clippy | 0 warnings | CI |
| Security | 0 known vulnerabilities | cargo-audit |
| Docs | All public items documented | CI |

### 4.2 Review Process

1. **Self-review:** Author checks against checklist
2. **Auto-check:** CI runs tests, clippy, coverage
3. **Peer review:** ≥1 Maintainer approval required
4. **Tech review:** Architectural changes need Tech Lead
5. **Merge:** Squash merge with conventional commit message

### 4.3 Conventional Commits

```
type(scope): description

[optional body]

[optional footer]
```

**Types:** feat, fix, docs, style, refactor, test, chore, ops, perf, security, beta

**Examples:**
- `feat(sae): add LZ4 compression to fine-tuning v7`
- `fix(p2p): resolve geographic routing edge case`
- `docs(beta): tester onboarding & feedback pipeline`
- `ops(beta): performance monitoring & bug triage automation`

---

## 5. Beta Program Governance

### 5.1 Beta Testing

| Aspect | Policy |
|--------|--------|
| Eligibility | Open to all registered contributors |
| Duration | 2-4 weeks per beta |
| Feedback SLA | P0: 2h, P1: 12h, P2: 48h, P3: 7d |
| Pause Criteria | ≥2 P0 unresolved 24h, data loss, security vuln |
| Rollback | git checkout previous stable tag |

### 5.2 Bug Triage

- **Triage Lead:** @Stuartemk
- **Frequency:** Daily during beta, weekly post-beta
- **Matrix:** See [`docs/operations/bug-triage-matrix.md`](docs/operations/bug-triage-matrix.md)
- **Escalation:** L1 (Triage) → L2 (Owner) → L3 (Release Lead) → L4 (Project Lead)

### 5.3 Performance Monitoring

- **Script:** [`scripts/beta_monitor.sh`](scripts/beta_monitor.sh)
- **Dashboard:** [`docs/operations/dashboard-v2-spec.md`](docs/operations/dashboard-v2-spec.md)
- **Reports:** `release/v1.8.0-beta.1/monitor-report.md`
- **Frequency:** On-demand + automated (planned)

---

## 6. Community Guidelines

### 6.1 Code of Conduct

- Be respectful and inclusive
- Assume good intent
- Disagree constructively
- No harassment, discrimination, or hate speech
- Violations reported to Project Lead

### 6.2 Communication Channels

| Channel | Purpose | Response Time |
|---------|---------|---------------|
| GitHub Issues | Bugs, features, tasks | Per SLA |
| GitHub Discussions | RFCs, proposals, Q&A | 48h |
| Discord #general | Community chat | Best effort |
| Discord #contributing | Dev coordination | 24h |
| Discord #bug-reports | Beta bugs | Per SLA |
| Discord #governance | Governance discussions | 48h |

### 6.3 Contributor Recognition

- **README:** Active contributors listed
- **Release Notes:** Contributors credited per release
- **Badges:** Tier-based Discord roles
- **Bounties:** Grant-funded bug bounties (planned)

---

## 7. Financial Governance

### 7.1 Funding Sources

| Source | Type | Status |
|--------|------|--------|
| GitHub Sponsors | Monthly donations | Active |
| Gitcoin Grants | Quadratic funding | Active |
| Crypto Donations | ETH, USDC | Active |
| Corporate Sponsorship | Annual contracts | Planned |
| Bug Bounties | Per-bug rewards | Planned |

### 7.2 Fund Allocation

| Category | % | Approval |
|----------|---|----------|
| Developer bounties | 40% | Tech Lead |
| Infrastructure | 25% | Project Lead |
| Community events | 15% | Community Lead |
| Security audits | 10% | Tech Lead |
| Contingency | 10% | Project Lead |

### 7.3 Transparency

- Monthly financial summary in Discord #funding
- Quarterly public report
- All transactions documented

---

## 8. Versioning & Release Policy

### 8.1 Semantic Versioning

| Component | When | Example |
|-----------|------|---------|
| MAJOR | Breaking changes | 1.x → 2.0 |
| MINOR | New features (backward compat) | 1.7 → 1.8 |
| PATCH | Bug fixes | 1.8.0 → 1.8.1 |

### 8.2 Release Cadence

| Release Type | Frequency | Duration | Examples |
|--------------|-----------|----------|----------|
| Beta | Per sprint | 2-4 weeks | v1.8.0-beta.1 |
| Stable | Quarterly | — | v1.8.0-stable |
| Hotfix | As needed | < 48h | v1.8.1 |

### 8.3 Feature Flags

| Flag | Description | Lifecycle |
|------|-------------|-----------|
| `stable` | Production features | Permanent |
| `v1.8-sprint1` | Sprint 1 features | → stable after v1.8 |
| `v1.8-sprint2` | Sprint 2 features | → stable after v1.8 |

---

## 9. Security Policy

### 9.1 Vulnerability Disclosure

1. **Report:** Private GitHub Security Advisory
2. **Acknowledge:** Within 24h
3. **Assess:** Severity + impact within 48h
4. **Fix:** Hotfix within SLA (P0: 24h)
5. **Disclose:** Public after fix + coordination

### 9.2 Security Measures

- Zero unsafe code policy
- Ed25519 signature verification
- Anti-sybil reputation system
- Regular dependency audits (cargo-audit)
- Security review for crypto modules

---

## 10. Amendment Process

This governance document can be amended through:

1. **Proposal:** RFC for governance change
2. **Discussion:** ≥14 days community input
3. **Vote:** Guardian + majority of Maintainers
4. **Publish:** Updated GOVERNANCE.md + changelog entry

---

## 11. References

| Document | Location |
|----------|----------|
| Bug Triage Matrix | [`docs/operations/bug-triage-matrix.md`](docs/operations/bug-triage-matrix.md) |
| Dashboard v2 Spec | [`docs/operations/dashboard-v2-spec.md`](docs/operations/dashboard-v2-spec.md) |
| Beta Retrospective | [`docs/retrospectives/beta-v1.8-retro.md`](docs/retrospectives/beta-v1.8-retro.md) |
| v1.9 Roadmap | [`docs/roadmap/v1.9-roadmap-draft.md`](docs/roadmap/v1.9-roadmap-draft.md) |
| Contributor Funnel | [`CONTRIBUTING.md`](CONTRIBUTING.md) |
| License | [`LICENSE`](LICENSE) |
| Support | [`SUPPORT.md`](SUPPORT.md) |

---

**Estado:** ACTIVE
**Última actualización:** 2026-05-15T21:25:00Z
**Autor:** Qweni (Auto-Push Protocol)
**FASE:** 62 — Post-Beta Retrospective & v1.9 Roadmap
