# ed2kIA Steward Program

**Version:** v2.1.0-stable  
**Date:** 2026-05-22  
**Status:** Active  

---

## Mission

The ed2kIA Steward Program exists to ensure that interpretability remains a public good. Stewards are trusted community members who provide ethical oversight, resolve conflicts, and guide the network toward its mission of transparent, distributed AI understanding.

This is not a governance token system. Stewards receive no financial compensation. Participation is motivated by commitment to ethical AI, technical excellence, and community service.

---

## Roles

### Observer

**Requirements:** None  
**Privileges:**
- Read access to feature dictionary
- Access to public documentation and metrics
- Participate in community discussions
- Submit feature requests and bug reports

**Path to Contributor:** Run a node for 7 consecutive days with CE score > 0.

### Contributor

**Requirements:**
- Functional node running for â‰¥7 days
- Valid Ed25519 identity
- CE score > 0 (positive ethical standing)
- No blocklist history in last 30 days

**Privileges:**
- Submit interpretability contributions
- Participate in consensus validation
- Access to contributor Discord/channel
- Vote on RFCs (Request for Comments)

**Path to Steward:** Nominated by existing Steward + 30 days as Contributor + CE score > 5.

### Steward

**Requirements:**
- Nominated by existing Steward
- â‰¥30 days as active Contributor
- CE score > 5
- Demonstrated understanding of SCT ethics
- Passed Steward orientation (documentation review)

**Privileges:**
- Review and resolve Byzantine_Eviction cases
- Approve/deny node reintegration
- Participate in Steward Council meetings
- Nominate new Stewards
- Access to governance dashboard
- Sign release candidates

**Responsibilities:**
- Review â‰¥5 Byzantine_Eviction cases per week
- Respond to escalation within 48 hours
- Document all decisions with rationale
- Participate in monthly Steward Council

### Steward Council

**Composition:** 7 Stewards (elected quarterly by Steward body)  
**Term:** 3 months (renewable once, then 6-month cooldown)  
**Responsibilities:**
- Final arbitration on contested decisions
- Approve major version releases
- Modify ethical thresholds (requires 5/7 quorum)
- Publish quarterly transparency reports
- Represent ed2kIA in external partnerships

---

## Code of Conduct

### Principles

1. **Rigorous over Rapid:** Technical accuracy matters more than speed. Take time to verify.
2. **Ethical over Convenient:** Ethical alignment takes priority over network growth or feature velocity.
3. **Transparent over Opaque:** All decisions are documented and publicly accessible.
4. **Inclusive over Exclusive:** Welcome diverse perspectives. Assume good faith.
5. **Human over Automated:** Machines detect; humans decide. Always.

### Expected Behavior

- Review contributions with intellectual honesty
- Document rejection rationale with specific references
- Engage constructively with disputed decisions
- Respect privacy of node operators
- Disclose conflicts of interest immediately

### Prohibited Behavior

- Use Steward position for personal gain
- Share private node operator data
- Manipulate CE scores or SCT evaluations
- Collude to bypass ethical thresholds
- Make decisions under influence or duress

### Violations

| Severity | Example | Consequence |
|----------|---------|-------------|
| Minor | Late response (>48h) without notice | Warning from Council |
| Medium | Undocumented decision | Mandatory re-orientation |
| Major | CE score manipulation | Immediate Steward removal |
| Critical | Data breach, collusion | Permanent ban, public disclosure |

---

## Steward Orientation

New Stewards complete the following orientation before receiving privileges:

### Week 1: Documentation Review

- [ ] Read [Technical Report](technical-report.md)
- [ ] Read [Production Threat Model](security/production-threat-model.md)
- [ ] Read [GOVERNANCE.md](../GOVERNANCE.md)
- [ ] Read [SCT Core Documentation](../src/alignment/sct_core.rs)
- [ ] Read [Network Byzantine_Eviction Documentation](../src/federated/network_Byzantine_Eviction.rs)

### Week 2: Practical Exercises

- [ ] Run a local testnet (`./scripts/testnet-mode.sh --nodes 3`)
- [ ] Submit 10 contributions (5 positive z, 5 negative z)
- [ ] Observe CRDT convergence across nodes
- [ ] Trigger and resolve 1 Byzantine_Eviction case
- [ ] Review Prometheus metrics dashboard

### Week 3: Shadow Review

- [ ] Shadow an experienced Steward for 5 Byzantine_Eviction cases
- [ ] Document decisions and compare with mentor
- [ ] Receive feedback on decision quality
- [ ] Pass orientation assessment (â‰¥80% agreement with mentor)

### Week 4: Provisional Stewardship

- [ ] Review 5 Byzantine_Eviction cases independently (with mentor review)
- [ ] Participate in 1 Steward Council meeting (observer)
- [ ] Complete self-assessment
- [ ] Receive Steward status

---

## Decision Framework

Stewards use the following framework for Byzantine_Eviction review:

### Step 1: Context Assessment

- What contribution triggered Byzantine_Eviction?
- What is the node's CE history (last 30 days)?
- Is this a first offense or pattern?

### Step 2: Intent Evaluation

- Was the misalignment intentional or accidental?
- Does the node operator respond to warnings?
- Is there evidence of coordinated attack?

### Step 3: Proportional Response

| Pattern | CE Impact | Response |
|---------|-----------|----------|
| First offense, minor | -0.5 to -1 | Warning, monitoring |
| Repeated minor | -1 to -2 | Temporary isolation (24h) |
| Single major | -2 to -3 | Isolation (7d), review required |
| Coordinated attack | < -3 | Permanent blocklist, Council review |

### Step 4: Documentation

Every decision requires:
- Case ID (auto-generated)
- Node ID (redacted for privacy if requested)
- CE score at time of decision
- Decision rationale (â‰¥3 sentences)
- Steward signature (Ed25519)
- Timestamp (UTC)

---

## Rewards (Non-Financial)

Stewards receive:

- **Recognition:** Listed in release notes and transparency reports
- **Access:** Early access to new features for testing
- **Influence:** Vote on RFCs and ethical threshold changes
- **Community:** Invitation to annual Steward gathering (virtual or in-person)
- **Development:** Mentorship opportunities for Contributors

Stewards do NOT receive:
- Financial compensation
- Token rewards
- Priority processing
- Exemption from ethical thresholds

---

## Transparency

### Quarterly Reports

The Steward Council publishes quarterly reports including:
- Total Byzantine_Eviction cases reviewed
- Average response time
- Decision distribution (approve/reject/reintegrate)
- CE score distribution across network
- Steward participation metrics

### Public Ledger

All Steward decisions are recorded in the public governance ledger:
- Case ID
- Decision type
- Timestamp
- Steward ID (public key)
- Rationale summary

Node operator identities are NOT disclosed without consent.

---

## Contact

- **Steward Applications:** Open a GitHub Issue with label `steward-application`
- **Escalations:** `#steward-escalations` in community Discord
- **Council Meetings:** First Monday of each month, 18:00 UTC (recorded)
- **Documentation:** https://github.com/ed2kia/ed2kIA/tree/main/docs

---

*This program is governed by the ed2kIA Project Constitution and the Apache 2.0 + Ethical Use Clause license.*
