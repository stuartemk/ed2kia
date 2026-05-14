# Reputation & Gamification Architecture — ed2kIA v1.8

This document defines the reputation and gamification system for ed2kIA, providing cryptographic proof of contribution, anti-cheat mechanisms, and motivational incentives for network participants.

## 1. System Overview

The reputation system transforms anonymous network contributions into verifiable, non-transferable reputation scores that unlock privileges, recognition, and governance weight within the ed2kIA ecosystem.

### 1.1 Core Principles

| Principle | Description |
|-----------|-------------|
| **Cryptographic Proof** | All reputation earned via Ed25519-signed contribution proofs |
| **Non-Transferable** | Reputation bound to contributor identity (Soulbound) |
| **Decay on Inactivity** | Reputation decays over time if contributor becomes inactive |
| **Anti-Cheat** | Multi-signal detection: Sybil resistance, bot detection, quality thresholds |
| **Transparent Scoring** | All scoring formulas public and auditable |
| **Progressive Unlock** | Reputation unlocks increasing privileges (badges, governance, rewards) |

### 1.2 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Contributor Node                         │
│  ┌─────────────┐  ┌─────────────┐  ┌────────────────────┐  │
│  │ Compute     │  │ Verification│  │ Data Contribution  │  │
│  │ Tasks       │  │ Proofs      │  │ (SAE shards)       │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬───────────┘  │
│         │                │                   │              │
│         ▼                ▼                   ▼              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           Contribution Collector                     │   │
│  │  - Aggregate tasks completed                        │   │
│  │  - Generate proof batch                             │   │
│  │  - Sign with Ed25519 private key                    │   │
│  └──────────────────────┬──────────────────────────────┘   │
└─────────────────────────┼──────────────────────────────────┘
                          │ Ed25519 Signed Proof
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  Reputation Oracle                          │
│  ┌─────────────┐  ┌─────────────┐  ┌────────────────────┐  │
│  │ Sybil       │  │ Quality     │  │ Anti-Bot           │  │
│  │ Detection   │  │ Scoring     │  │ Heuristics         │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬───────────┘  │
│         │                │                   │              │
│         ▼                ▼                   ▼              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           Reputation Ledger (redb)                   │   │
│  │  - Immutable append-only log                        │   │
│  │  - Per-contributor score tracking                   │   │
│  │  - Badge/achievement state                          │   │
│  └──────────────────────┬──────────────────────────────┘   │
└─────────────────────────┼──────────────────────────────────┘
                          │ Reputation Score
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                   Gamification Layer                        │
│  ┌─────────────┐  ┌─────────────┐  ┌────────────────────┐  │
│  │ Badges &    │  │ Leaderboard │  │ Governance         │  │
│  │ Achievements│  │ API         │  │ Weight Multiplier  │  │
│  └─────────────┘  └─────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 2. Cryptographic Identity

### 2.1 Ed25519 Key Pairs

Each contributor generates an Ed25519 key pair:

```rust
// Contributor identity
pub struct ContributorIdentity {
    pub public_key: [u8; 32],   // Ed25519 public key
    pub signature: [u8; 64],    // Ed25519 signature over contribution proof
    pub nonce: u64,             // Monotonically increasing nonce
}
```

### 2.2 Contribution Proof Structure

```rust
pub struct ContributionProof {
    pub contributor_id: [u8; 32],     // Ed25519 public key hash
    pub task_type: TaskType,           // Compute, Verification, Data
    pub task_id: String,               // Unique task identifier
    pub work_units: u64,               // Computed work units
    pub timestamp_ms: u64,             // Unix timestamp
    pub merkle_root: String,           // Merkle root of batch
    pub signature: [u8; 64],           // Ed25519 signature
}

pub enum TaskType {
    Compute,       // Tensor computation / fine-tuning
    Verification,  // ZKP verification
    Data,          // SAE shard contribution
    Review,        // Code review / documentation
}
```

### 2.3 Proof Verification

```rust
pub fn verify_proof(proof: &ContributionProof) -> Result<(), ReputationError> {
    // 1. Verify Ed25519 signature
    let message = encode_proof_message(proof);
    ed25519_verify(&proof.contributor_id, &message, &proof.signature)?;

    // 2. Check nonce monotonicity
    let last_nonce = get_last_nonce(proof.contributor_id)?;
    if proof.nonce <= last_nonce {
        return Err(ReputationError::NonceNotMonotonic);
    }

    // 3. Verify timestamp within acceptable window
    let now = current_timestamp_ms();
    if proof.timestamp_ms > now || (now - proof.timestamp_ms) > MAX_PROOF_AGE_MS {
        return Err(ReputationError::ProofExpired);
    }

    // 4. Verify Merkle root matches batch
    verify_merkle_inclusion(proof.task_id, &proof.merkle_root)?;

    Ok(())
}
```

## 3. Reputation Scoring

### 3.1 Base Score Formula

Reputation score is computed as a weighted sum of contribution signals:

```
Reputation = Σ (signal_weight_i × normalized_signal_i) × decay_factor
```

| Signal | Weight | Description |
|--------|--------|-------------|
| `compute_units` | 0.30 | Tensor computation work completed |
| `verification_count` | 0.25 | ZKP proofs verified |
| `data_shards` | 0.15 | SAE shards contributed |
| `code_reviews` | 0.15 | Pull requests reviewed |
| `documentation` | 0.05 | Documentation contributions |
| `community_help` | 0.10 | Community support (issues, discussions) |

### 3.2 Normalization

Each signal is normalized to [0, 1] range using percentile ranking:

```rust
pub fn normalize_signal(value: f64, percentile_data: &[f64]) -> f64 {
    let sorted = &mut percentile_data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let rank = sorted.binary_search_by(|&x| x.partial_cmp(&value).unwrap()).unwrap_or_else(|i| i);
    rank as f64 / sorted.len() as f64
}
```

### 3.3 Decay Function

Reputation decays exponentially based on inactivity:

```
decay_factor = e^(-λ × days_since_last_contribution)
```

Where `λ = 0.01` (half-life ≈ 69 days).

```rust
pub fn compute_decay_factor(days_inactive: u64, lambda: f64) -> f64 {
    (-lambda * days_inactive as f64).exp()
}
```

| Days Inactive | Decay Factor | Reputation Retained |
|---------------|-------------|-------------------|
| 0 | 1.000 | 100% |
| 30 | 0.741 | 74% |
| 60 | 0.549 | 55% |
| 90 | 0.407 | 41% |
| 180 | 0.165 | 17% |

### 3.4 Score Tiers

| Tier | Score Range | Privileges |
|------|-------------|------------|
| **Observer** | 0 - 100 | Read access, basic participation |
| **Contributor** | 100 - 500 | Write access, issue creation |
| **Verifier** | 500 - 2,000 | Proof verification, code review |
| **Advocate** | 2,000 - 5,000 | Governance voting (1x weight) |
| **Steward** | 5,000 - 15,000 | Governance voting (2x weight), mentorship |
| **Guardian** | 15,000 - 50,000 | Governance voting (3x weight), steering committee eligibility |
| **Legend** | 50,000+ | Governance voting (5x weight), core team eligibility |

## 4. Anti-Cheat System

### 4.1 Sybil Detection

Multiple accounts from same operator detected via:

| Signal | Method | Threshold |
|--------|--------|-----------|
| IP Overlap | Shared IP addresses | ≥3 accounts → flag |
| Hardware Fingerprint | CPU/GPU signature match | Exact match → merge |
| Behavioral Patterns | Identical contribution timing | Correlation > 0.95 → flag |
| Key Generation Pattern | Sequential key pairs | Detected via entropy analysis |

### 4.2 Bot Detection

Automated contribution detection:

```rust
pub struct BotScore {
    pub timing_regularity: f64,    // How regular are contribution intervals
    pub work_unit_variance: f64,   // Variance in work units per task
    pub error_rate: f64,           // Error rate (bots often have 0% or 100%)
    pub diversity_score: f64,      // Task type diversity
}

pub fn compute_bot_score(contributor_id: &[u8; 32]) -> BotScore {
    let contributions = fetch_contributions(contributor_id);
    BotScore {
        timing_regularity: analyze_timing_regularity(&contributions),
        work_unit_variance: analyze_work_variance(&contributions),
        error_rate: compute_error_rate(&contributions),
        diversity_score: compute_task_diversity(&contributions),
    }
}

pub fn is_likely_bot(score: &BotScore) -> bool {
    score.timing_regularity > 0.95          // Too regular
        || score.work_unit_variance < 0.05  // Too consistent
        || score.error_rate == 0.0          // Perfect accuracy (suspicious)
        || score.diversity_score < 0.2      // Only one task type
}
```

### 4.3 Quality Thresholds

Minimum quality requirements per task type:

| Task Type | Quality Metric | Minimum Threshold |
|-----------|---------------|-------------------|
| Compute | Gradient alignment score | ≥ 0.85 |
| Verification | Proof correctness rate | ≥ 0.99 |
| Data | Shard completeness | ≥ 0.95 |
| Review | Review depth (comments) | ≥ 3 meaningful comments |

Contributions below threshold are rejected and contribute negatively to reputation.

### 4.4 Penalty System

| Violation | Penalty | Duration |
|-----------|---------|----------|
| First offense (quality below threshold) | Warning + 10% score reduction | 7 days |
| Second offense | 25% score reduction | 30 days |
| Third offense | 50% score reduction + suspension | 90 days |
| Confirmed Sybil/Bot | Permanent ban + score reset | Permanent |

## 5. Gamification Layer

### 5.1 Badges & Achievements

| Badge | Requirement | Reputation Bonus |
|-------|-------------|-----------------|
| **First Step** | Complete first task | +10 |
| **Consistent** | 30 consecutive days | +50 |
| **Diverse** | Contribute all 4 task types | +100 |
| **Speed Demon** | Complete 100 tasks in 24h | +75 |
| **Quality First** | 99% quality rate over 100 tasks | +150 |
| **Mentor** | Help 10 new contributors | +200 |
| **Marathon** | 365 consecutive days | +500 |
| **Guardian** | Reach Guardian tier | +1000 |
| **Legend** | Reach Legend tier | +5000 |

### 5.2 Leaderboard API

REST API for leaderboard queries:

```
GET /api/v1/reputation/leaderboard
Query Parameters:
  - limit: Number of entries (default: 50, max: 1000)
  - offset: Pagination offset
  - period: Time period (daily, weekly, monthly, all_time)
  - tier: Filter by minimum tier
  - task_type: Filter by task type

Response:
{
  "data": [
    {
      "rank": 1,
      "contributor_id": "ed25519_pub_key_hash",
      "display_name": "ContributorName",
      "score": 45230,
      "tier": "Legend",
      "badges": ["First Step", "Consistent", "Legend"],
      "contributions_today": 12,
      "streak_days": 365
    }
  ],
  "total": 15234,
  "period": "all_time"
}
```

### 5.3 Streak System

Daily contribution streaks provide bonus multipliers:

| Streak Length | Multiplier | Bonus |
|--------------|-----------|-------|
| 7 days | 1.05x | +5% reputation |
| 30 days | 1.10x | +10% reputation |
| 90 days | 1.20x | +20% reputation |
| 365 days | 1.50x | +50% reputation |

```rust
pub fn compute_streak_bonus(streak_days: u64) -> f64 {
    match streak_days {
        0..7 => 1.0,
        7..30 => 1.05,
        30..90 => 1.10,
        90..365 => 1.20,
        _ => 1.50,
    }
}
```

### 5.4 Seasonal Competitions

Quarterly competitions with themed challenges:

| Season | Theme | Special Rewards |
|--------|-------|----------------|
| Q1 | Verification Sprint | Extra ZKP weight |
| Q2 | Compute Marathon | Extra compute weight |
| Q3 | Community Builder | Extra review/help weight |
| Q4 | Innovation Challenge | New task type bonus |

## 6. Storage Architecture

### 6.1 Redb Storage Schema

Using [`redb`](https://crates.io/crates/redb) for embedded, ACID-compliant storage:

```rust
// Table definitions
pub const REPUTATION_TABLE: &str = "reputation_scores";
pub const CONTRIBUTION_LOG: &str = "contribution_proofs";
pub const BADGE_TABLE: &str = "badges_awarded";
pub const STREAK_TABLE: &str = "streak_tracking";
pub const PENALTY_TABLE: &str = "penalty_records";

// Schema
pub struct ReputationSchema;

impl redb::TableDefinition for ReputationSchema {
    // contributor_id: [u8; 32] → ReputationRecord
}

pub struct ReputationRecord {
    pub total_score: f64,
    pub tier: Tier,
    pub badges: Vec<String>,
    pub current_streak: u64,
    pub longest_streak: u64,
    pub last_contribution_ms: u64,
    pub penalty_until_ms: u64,
}
```

### 6.2 Index Strategy

| Index | Key | Purpose |
|-------|-----|---------|
| `by_score` | Score (descending) | Leaderboard queries |
| `by_tier` | Tier + Score | Tier-based filtering |
| `by_badge` | Badge name | Badge holders lookup |
| `by_streak` | Streak length | Streak leaderboard |

### 6.3 Data Retention

| Data Type | Retention Period | Reason |
|-----------|-----------------|--------|
| Contribution proofs | Permanent | Audit trail |
| Reputation scores | Permanent | Core state |
| Badge records | Permanent | Achievement history |
| Penalty records | 2 years | Compliance |
| Raw task data | 90 days | Storage optimization |

## 7. Governance Integration

### 7.1 Voting Weight

Reputation score determines governance voting weight:

```
voting_weight = base_weight × tier_multiplier × quality_bonus
```

| Component | Formula |
|-----------|---------|
| `base_weight` | min(reputation_score / 1000, 10) |
| `tier_multiplier` | Observer=0, Contributor=0.5, Verifier=1, Advocate=1, Steward=2, Guardian=3, Legend=5 |
| `quality_bonus` | 1.0 + (quality_rate - 0.9) × 2 (capped at 1.5) |

### 7.2 Proposal Submission

Minimum reputation required to submit governance proposals:

| Proposal Type | Minimum Tier | Minimum Score |
|--------------|--------------|---------------|
| Feature Request | Contributor | 100 |
| Bug Fix Priority | Verifier | 500 |
| Funding Request | Advocate | 2,000 |
| Protocol Change | Steward | 5,000 |
| Constitutional Amendment | Guardian | 15,000 |

### 7.3 Steering Committee Eligibility

Steering committee members selected from Guardian+ tier based on:
- Reputation score (primary)
- Contribution diversity
- Community engagement
- Governance participation

## 8. API Reference

### 8.1 Reputation Endpoints

```
GET    /api/v1/reputation/{contributor_id}     # Get reputation profile
GET    /api/v1/reputation/leaderboard          # Get leaderboard
POST   /api/v1/reputation/submit-proof         # Submit contribution proof
GET    /api/v1/reputation/badges/{contributor_id}  # Get badges
GET    /api/v1/reputation/streak/{contributor_id}   # Get streak info
```

### 8.2 Response Formats

```rust
pub struct ReputationProfile {
    pub contributor_id: String,
    pub display_name: Option<String>,
    pub score: f64,
    pub tier: Tier,
    pub badges: Vec<Badge>,
    pub current_streak: u64,
    pub longest_streak: u64,
    pub total_contributions: u64,
    pub join_date_ms: u64,
    pub last_active_ms: u64,
}

pub struct Badge {
    pub name: String,
    pub description: String,
    pub awarded_at_ms: u64,
    pub icon_url: String,
}
```

## 9. Security Considerations

### 9.1 Key Management

- Private keys never leave contributor device
- Signature verification only (no key storage on servers)
- Optional key backup via social recovery (3-of-5 guardians)

### 9.2 Privacy

- Contributor IDs are public key hashes (no PII)
- Optional display names (not linked to real identity)
- Contribution metadata does not include IP addresses in public API

### 9.3 Attack Vectors & Mitigations

| Attack | Mitigation |
|--------|-----------|
| Sybil attack | Multi-signal detection + proof-of-personhood (optional) |
| Bot farming | Behavioral analysis + quality thresholds |
| Reputation manipulation | Decay function + penalty system |
| DDoS on reputation API | Rate limiting + caching |
| Database corruption | Redb ACID guarantees + regular backups |

## 10. Implementation Roadmap

### Phase 1: Core Reputation Engine (v1.8 Sprint 1)
- [ ] Ed25519 proof generation + verification
- [ ] Redb storage schema
- [ ] Base scoring formula
- [ ] API endpoints (submit-proof, get-profile)

### Phase 2: Anti-Cheat + Badges (v1.8 Sprint 2)
- [ ] Sybil detection heuristics
- [ ] Bot detection scoring
- [ ] Badge system + achievement tracking
- [ ] Leaderboard API

### Phase 3: Gamification + Governance (v1.8 Sprint 3)
- [ ] Streak system
- [ ] Seasonal competitions
- [ ] Governance weight integration
- [ ] Proposal submission thresholds

### Phase 4: Advanced Features (v1.9)
- [ ] Social recovery for key backup
- [ ] Proof-of-personhood (optional, Worldcoin/Spruce integration)
- [ ] Cross-chain reputation (future blockchain integration)
- [ ] Reputation NFT (soulbound token)

## 11. Metrics & Monitoring

### 11.1 Key Metrics

| Metric | Target | Alert Threshold |
|--------|--------|----------------|
| Active contributors (daily) | > 100 | < 50 |
| Average reputation score | 500-2000 | > 5000 (inflation) |
| Bot detection rate | < 5% | > 10% |
| Proof verification time | < 100ms | > 500ms |
| API p95 latency | < 200ms | > 1s |

### 11.2 Dashboard Panels

1. **Reputation Distribution**: Histogram of scores across tiers
2. **Contribution Velocity**: Contributions per hour/day/week
3. **Bot Detection Alerts**: Flagged accounts + false positive rate
4. **Badge Velocity**: New badges awarded per day
5. **Streak Distribution**: Streak length histogram
6. **Governance Participation**: Voting weight distribution

---

*This document is a living architecture spec. Update with each sprint iteration.*
