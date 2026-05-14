# Issues Batch v1.8 — "ChatGPT Moment" Sprint

This document contains the batch of GitHub issues for the v1.8 "ChatGPT Moment" sprint. Each issue follows the `good-first-issue` template format for maximum contributor onboarding success.

---

## Issue 1: WASM Core Extraction

```markdown
title: "feat(wasm): extract WASM-compatible core module"
labels: good-first-issue, wasm, core
assignee: ""

## 🎯 What to Do

Extract the core verification and crypto logic from ed2kIA into a separate WASM-compatible crate that can be compiled to `wasm32-unknown-unknown` for browser extension use.

## 📋 Tasks

- [ ] Create `ed2kIA-wasm/` crate with `cdylib` crate type
- [ ] Move Ed25519 signing/verification to WASM crate
- [ ] Move SHA-256 hashing to WASM crate
- [ ] Move proof verification logic to WASM crate
- [ ] Add `wasm-bindgen` annotations for JS interop
- [ ] Configure `wasm-pack build` in CI
- [ ] Ensure WASM output < 2MB

## 📚 Resources

- Architecture: [`docs/architecture/mobile-browser-expansion.md`](docs/architecture/mobile-browser-expansion.md)
- Existing crypto: [`src/security/`](src/security/)
- `wasm-pack` docs: https://rustwasm.github.io/wasm-pack/

## ✅ Acceptance Criteria

- [ ] `cargo build --target wasm32-unknown-unknown` succeeds
- [ ] WASM output < 2MB
- [ ] All crypto functions accessible from JS via `wasm-bindgen`
- [ ] Tests pass in both native and WASM targets

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +100 reputation points
- Contributor badge
- Core module author credit
```

---

## Issue 2: Browser Extension Shell

```markdown
title: "feat(browser): create Chrome/Firefox extension shell"
labels: good-first-issue, browser, frontend
assignee: ""

## 🎯 What to Do

Create the basic Chrome/Firefox extension shell for ed2kIA Verifier, including manifest v3, background service worker, and popup UI.

## 📋 Tasks

- [ ] Create `browser-extension/` directory
- [ ] Add `manifest.json` (Manifest V3)
- [ ] Create background service worker (`background.js`)
- [ ] Create popup HTML/CSS/JS
- [ ] Add extension icons (48x48, 128x128)
- [ ] Wire up WASM module loading
- [ ] Add basic settings page

## 📚 Resources

- Architecture: [`docs/architecture/mobile-browser-expansion.md`](docs/architecture/mobile-browser-expansion.md)
- Manifest V3 docs: https://developer.chrome.com/docs/extensions/mv3/intro/
- Design reference: See architecture doc §4.3

## ✅ Acceptance Criteria

- [ ] Extension loads in Chrome without errors
- [ ] Extension loads in Firefox without errors
- [ ] Popup displays status + settings
- [ ] WASM module loads successfully
- [ ] Passes Chrome Web Store validation

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +100 reputation points
- Contributor badge
- Browser extension author credit
```

---

## Issue 3: Reputation Engine — Ed25519 Proof Verification

```markdown
title: "feat(reputation): implement Ed25519 proof verification"
labels: good-first-issue, reputation, crypto
assignee: ""

## 🎯 What to Do

Implement the Ed25519 proof verification system for the reputation engine, allowing contributors to sign contribution proofs cryptographically.

## 📋 Tasks

- [ ] Create `src/reputation/proof.rs` module
- [ ] Implement `ContributionProof` struct
- [ ] Implement `verify_proof()` function
- [ ] Add nonce monotonicity check
- [ ] Add timestamp expiration check
- [ ] Add Merkle root verification
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [`docs/architecture/reputation-gamification.md`](docs/architecture/reputation-gamification.md)
- Existing Ed25519: `ed25519-dalek` crate
- Proof structure: See architecture doc §2.2

## ✅ Acceptance Criteria

- [ ] Valid proofs verify successfully
- [ ] Invalid signatures rejected
- [ ] Non-monotonic nonces rejected
- [ ] Expired proofs rejected
- [ ] 15+ unit tests passing
- [ ] `cargo clippy -- -D warnings` passes

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +100 reputation points
- Contributor badge
- Reputation system author credit
```

---

## Issue 4: Badge System & Achievements

```markdown
title: "feat(reputation): implement badge system & achievements"
labels: good-first-issue, reputation, gamification
assignee: ""

## 🎯 What to Do

Implement the badge and achievement system for the reputation engine, allowing contributors to unlock badges based on milestones.

## 📋 Tasks

- [ ] Create `src/reputation/badge.rs` module
- [ ] Define 15+ badge types (see architecture doc §5.1)
- [ ] Implement badge unlock logic
- [ ] Add badge storage (redb)
- [ ] Create badge query API
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [`docs/architecture/reputation-gamification.md`](docs/architecture/reputation-gamification.md)
- Badge definitions: See architecture doc §5.1
- Storage: `redb` crate

## ✅ Acceptance Criteria

- [ ] All 15+ badges definable
- [ ] Badges unlock automatically on milestone
- [ ] Badge query returns correct results
- [ ] 15+ unit tests passing
- [ ] `cargo clippy -- -D warnings` passes

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +75 reputation points
- Contributor badge
- Gamification author credit
```

---

## Issue 5: Leaderboard API

```markdown
title: "feat(api): implement reputation leaderboard API"
labels: good-first-issue, api, reputation
assignee: ""

## 📋 Tasks

- [ ] Create `GET /api/v1/reputation/leaderboard` endpoint
- [ ] Add query parameters (limit, offset, period, tier)
- [ ] Implement caching layer (1hr TTL)
- [ ] Add pagination support
- [ ] Write integration tests
- [ ] Document API in OpenAPI spec

## 📚 Resources

- Architecture: [`docs/architecture/reputation-gamification.md`](docs/architecture/reputation-gamification.md)
- API spec: See architecture doc §5.2
- Existing API patterns: `src/api/`

## ✅ Acceptance Criteria

- [ ] API returns correct leaderboard data
- [ ] Pagination works correctly
- [ ] Caching reduces DB load
- [ ] p95 latency < 200ms
- [ ] Integration tests pass

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +75 reputation points
- Contributor badge
- API author credit
```

---

## Issue 6: Android Foreground Service

```markdown
title: "feat(mobile): implement Android foreground service"
labels: good-first-issue, mobile, android
assignee: ""

## 🎯 What to Do

Create the Android foreground service for ed2kIA background verification, using wasmtime for WASM execution.

## 📋 Tasks

- [ ] Create Android project structure
- [ ] Implement `Ed2kIAService` foreground service
- [ ] Add notification management
- [ ] Integrate wasmtime for WASM execution
- [ ] Add battery optimization checks
- [ ] Add WiFi-only mode toggle
- [ ] Write instrumentation tests

## 📚 Resources

- Architecture: [`docs/architecture/mobile-browser-expansion.md`](docs/architecture/mobile-browser-expansion.md)
- Android service docs: https://developer.android.com/guide/components/services
- wasmtime Android: https://docs.wasmtime.dev/

## ✅ Acceptance Criteria

- [ ] Service starts on device boot
- [ ] Notification displays contribution stats
- [ ] WASM module executes correctly
- [ ] Battery impact < 5%/hour
- [ ] WiFi-only mode functional

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +100 reputation points
- Contributor badge
- Mobile author credit
```

---

## Issue 7: Impact Dashboard

```markdown
title: "feat(ui): create impact dashboard"
labels: good-first-issue, frontend, dashboard
assignee: ""

## 🎯 What to Do

Build the real-time impact dashboard showing network-wide verification stats, contributor rankings, and personal contribution metrics.

## 📋 Tasks

- [ ] Design dashboard layout (Figma or wireframe)
- [ ] Create dashboard HTML/CSS/JS
- [ ] Connect to reputation API
- [ ] Add real-time updates (WebSocket or polling)
- [ ] Add personal stats section
- [ ] Add network health visualization
- [ ] Mobile-responsive design

## 📚 Resources

- Architecture: [`docs/roadmap/v1.8-chatgpt-moment.md`](docs/roadmap/v1.8-chatgpt-moment.md)
- API endpoints: Leaderboard API (Issue 5)
- Existing web: `web/` directory

## ✅ Acceptance Criteria

- [ ] Dashboard loads < 2s
- [ ] Real-time updates working
- [ ] Mobile-responsive
- [ ] Personal stats accurate
- [ ] Network health visualization clear

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +75 reputation points
- Contributor badge
- UI author credit
```

---

## Issue 8: Streak System

```markdown
title: "feat(reputation): implement daily streak system"
labels: good-first-issue, reputation, gamification
assignee: ""

## 🎯 What to Do

Implement the daily contribution streak system with bonus multipliers for consecutive days of activity.

## 📋 Tasks

- [ ] Create `src/reputation/streak.rs` module
- [ ] Implement streak tracking logic
- [ ] Add multiplier calculation (see architecture §5.3)
- [ ] Add streak reset on miss
- [ ] Add streak storage (redb)
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [`docs/architecture/reputation-gamification.md`](docs/architecture/reputation-gamification.md)
- Multiplier table: See architecture doc §5.3

## ✅ Acceptance Criteria

- [ ] Streak increments on daily contribution
- [ ] Streak resets after 24h inactivity
- [ ] Multipliers apply correctly
- [ ] 15+ unit tests passing
- [ ] `cargo clippy -- -D warnings` passes

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +50 reputation points
- Contributor badge
- Gamification author credit
```

---

## Issue 9: Anti-Cheat — Sybil Detection

```markdown
title: "feat(security): implement Sybil detection heuristics"
labels: good-first-issue, security, anti-cheat
assignee: ""

## 🎯 What to Do

Implement Sybil detection heuristics to identify multiple accounts operated by the same person.

## 📋 Tasks

- [ ] Create `src/reputation/anti_cheat.rs` module
- [ ] Implement IP overlap detection
- [ ] Implement hardware fingerprint matching
- [ ] Implement behavioral pattern analysis
- [ ] Add flagging system (not auto-ban)
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [`docs/architecture/reputation-gamification.md`](docs/architecture/reputation-gamification.md)
- Detection methods: See architecture doc §4.1

## ✅ Acceptance Criteria

- [ ] IP overlap detection works
- [ ] Behavioral analysis flags suspicious patterns
- [ ] False positive rate < 5%
- [ ] 15+ unit tests passing
- [ ] `cargo clippy -- -D warnings` passes

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +100 reputation points
- Contributor badge
- Security author credit
```

---

## Issue 10: Contributor Onboarding Wizard

```markdown
title: "feat(ui): create contributor onboarding wizard"
labels: good-first-issue, frontend, onboarding
assignee: ""

## 🎯 What to Do

Build a step-by-step onboarding wizard for new contributors, guiding them from signup to first contribution.

## 📋 Tasks

- [ ] Design wizard flow (5 steps max)
- [ ] Create wizard HTML/CSS/JS
- [ ] Step 1: Welcome + mission explanation
- [ ] Step 2: Choose contribution type
- [ ] Step 3: Setup guide (fork, clone, etc.)
- [ ] Step 4: First issue recommendation
- [ ] Step 5: Join community (Discord, etc.)

## 📚 Resources

- Contributor funnel: [`docs/community/contributor-funnel.md`](docs/community/contributor-funnel.md)
- Existing web: `web/` directory

## ✅ Acceptance Criteria

- [ ] Wizard completes in < 5 minutes
- [ ] All 5 steps functional
- [ ] Mobile-responsive
- [ ] Links to resources work
- [ ] Conversion rate tracking

## 🤝 Need Help?

Ask in #contributing Discord channel or tag @advocates

## 🏆 Rewards

- +75 reputation points
- Contributor badge
- Onboarding author credit
```

---

## Usage

To create these issues via GitHub CLI:

```bash
# Run the creation script
bash scripts/create_issues_v1.8.sh
```

Or create manually using the templates above.

## Labels Required

Ensure these labels exist in the repository:
- `good-first-issue`
- `wasm`
- `browser`
- `reputation`
- `gamification`
- `mobile`
- `android`
- `api`
- `frontend`
- `dashboard`
- `security`
- `anti-cheat`
- `onboarding`
- `core`

---

*Generated for v1.8 "ChatGPT Moment" Sprint*
