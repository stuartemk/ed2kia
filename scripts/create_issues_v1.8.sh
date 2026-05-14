#!/bin/bash
# create_issues_v1.8.sh — GitHub CLI script for v1.8 "ChatGPT Moment" issue creation
# Usage: bash scripts/create_issues_v1.8.sh

set -e

echo "🚀 Creating v1.8 'ChatGPT Moment' Issues Batch..."

# Ensure gh is installed
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) not found. Install from: https://cli.github.com/"
    exit 1
fi

# Ensure authenticated
if ! gh auth status &> /dev/null; then
    echo "❌ Not authenticated with GitHub CLI. Run: gh auth login"
    exit 1
fi

# Create labels if they don't exist
LABELS=(
    "wasm:1a535c:WebAssembly"
    "browser:0e8a16:Browser Extension"
    "reputation:5319e7:Reputation System"
    "gamification:ff7b14:Gamification"
    "mobile:006b75:Mobile"
    "android:3168dc:Android"
    "api:006b75:API"
    "frontend:0e8a16:Frontend"
    "dashboard:5319e7:Dashboard"
    "security:0e8a16:Security"
    "anti-cheat:006b75:Anti-Cheat"
    "onboarding:0e8a16:Onboarding"
    "core:006b75:Core Module"
)

echo "📋 Creating labels..."
for label_info in "${LABELS[@]}"; do
    IFS=':' read -r name color description <<< "$label_info"
    gh label create "$name" --color "$color" --description "$description" 2>/dev/null || true
done

# Issue definitions
declare -a ISSUES

ISSUES[0]='--title "feat(wasm): extract WASM-compatible core module" --label "good-first-issue,wasm,core" --body "## 🎯 What to Do

Extract the core verification and crypto logic from ed2kIA into a separate WASM-compatible crate.

## 📋 Tasks

- [ ] Create \`ed2kIA-wasm/\` crate with \`cdylib\` crate type
- [ ] Move Ed25519 signing/verification to WASM crate
- [ ] Move SHA-256 hashing to WASM crate
- [ ] Move proof verification logic to WASM crate
- [ ] Add \`wasm-bindgen\` annotations for JS interop
- [ ] Configure \`wasm-pack build\` in CI
- [ ] Ensure WASM output < 2MB

## 📚 Resources

- Architecture: [\`docs/architecture/mobile-browser-expansion.md\`](docs/architecture/mobile-browser-expansion.md)
- \`wasm-pack\` docs: https://rustwasm.github.io/wasm-pack/

## ✅ Acceptance Criteria

- [ ] \`cargo build --target wasm32-unknown-unknown\` succeeds
- [ ] WASM output < 2MB
- [ ] All crypto functions accessible from JS
- [ ] Tests pass in both native and WASM targets

## 🏆 Rewards

- +100 reputation points
- Contributor badge"'

ISSUES[1]='--title "feat(browser): create Chrome/Firefox extension shell" --label "good-first-issue,browser,frontend" --body "## 🎯 What to Do

Create the basic Chrome/Firefox extension shell for ed2kIA Verifier.

## 📋 Tasks

- [ ] Create \`browser-extension/\` directory
- [ ] Add \`manifest.json\` (Manifest V3)
- [ ] Create background service worker
- [ ] Create popup HTML/CSS/JS
- [ ] Add extension icons
- [ ] Wire up WASM module loading
- [ ] Add basic settings page

## 📚 Resources

- Architecture: [\`docs/architecture/mobile-browser-expansion.md\`](docs/architecture/mobile-browser-expansion.md)

## ✅ Acceptance Criteria

- [ ] Extension loads in Chrome without errors
- [ ] Extension loads in Firefox without errors
- [ ] Popup displays status + settings
- [ ] Passes Chrome Web Store validation

## 🏆 Rewards

- +100 reputation points
- Contributor badge"'

ISSUES[2]='--title "feat(reputation): implement Ed25519 proof verification" --label "good-first-issue,reputation,core" --body "## 🎯 What to Do

Implement the Ed25519 proof verification system for the reputation engine.

## 📋 Tasks

- [ ] Create \`src/reputation/proof.rs\` module
- [ ] Implement \`ContributionProof\` struct
- [ ] Implement \`verify_proof()\` function
- [ ] Add nonce monotonicity check
- [ ] Add timestamp expiration check
- [ ] Add Merkle root verification
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [\`docs/architecture/reputation-gamification.md\`](docs/architecture/reputation-gamification.md)

## ✅ Acceptance Criteria

- [ ] Valid proofs verify successfully
- [ ] Invalid signatures rejected
- [ ] 15+ unit tests passing
- [ ] \`cargo clippy -- -D warnings\` passes

## 🏆 Rewards

- +100 reputation points
- Contributor badge"'

ISSUES[3]='--title "feat(reputation): implement badge system & achievements" --label "good-first-issue,reputation,gamification" --body "## 🎯 What to Do

Implement the badge and achievement system for the reputation engine.

## 📋 Tasks

- [ ] Create \`src/reputation/badge.rs\` module
- [ ] Define 15+ badge types
- [ ] Implement badge unlock logic
- [ ] Add badge storage (redb)
- [ ] Create badge query API
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [\`docs/architecture/reputation-gamification.md\`](docs/architecture/reputation-gamification.md)

## ✅ Acceptance Criteria

- [ ] All 15+ badges definable
- [ ] Badges unlock automatically
- [ ] 15+ unit tests passing

## 🏆 Rewards

- +75 reputation points
- Contributor badge"'

ISSUES[4]='--title "feat(api): implement reputation leaderboard API" --label "good-first-issue,api,reputation" --body "## 🎯 What to Do

Create the REST API for reputation leaderboard queries.

## 📋 Tasks

- [ ] Create \`GET /api/v1/reputation/leaderboard\` endpoint
- [ ] Add query parameters (limit, offset, period, tier)
- [ ] Implement caching layer (1hr TTL)
- [ ] Add pagination support
- [ ] Write integration tests
- [ ] Document API in OpenAPI spec

## 📚 Resources

- Architecture: [\`docs/architecture/reputation-gamification.md\`](docs/architecture/reputation-gamification.md)

## ✅ Acceptance Criteria

- [ ] API returns correct leaderboard data
- [ ] Pagination works correctly
- [ ] p95 latency < 200ms
- [ ] Integration tests pass

## 🏆 Rewards

- +75 reputation points
- Contributor badge"'

ISSUES[5]='--title "feat(mobile): implement Android foreground service" --label "good-first-issue,mobile,android" --body "## 🎯 What to Do

Create the Android foreground service for ed2kIA background verification.

## 📋 Tasks

- [ ] Create Android project structure
- [ ] Implement \`Ed2kIAService\` foreground service
- [ ] Add notification management
- [ ] Integrate wasmtime for WASM execution
- [ ] Add battery optimization checks
- [ ] Add WiFi-only mode toggle
- [ ] Write instrumentation tests

## 📚 Resources

- Architecture: [\`docs/architecture/mobile-browser-expansion.md\`](docs/architecture/mobile-browser-expansion.md)

## ✅ Acceptance Criteria

- [ ] Service starts on device boot
- [ ] Notification displays contribution stats
- [ ] Battery impact < 5%/hour

## 🏆 Rewards

- +100 reputation points
- Contributor badge"'

ISSUES[6]='--title "feat(ui): create impact dashboard" --label "good-first-issue,frontend,dashboard" --body "## 🎯 What to Do

Build the real-time impact dashboard showing network-wide verification stats.

## 📋 Tasks

- [ ] Design dashboard layout
- [ ] Create dashboard HTML/CSS/JS
- [ ] Connect to reputation API
- [ ] Add real-time updates
- [ ] Add personal stats section
- [ ] Mobile-responsive design

## 📚 Resources

- Roadmap: [\`docs/roadmap/v1.8-chatgpt-moment.md\`](docs/roadmap/v1.8-chatgpt-moment.md)

## ✅ Acceptance Criteria

- [ ] Dashboard loads < 2s
- [ ] Real-time updates working
- [ ] Mobile-responsive

## 🏆 Rewards

- +75 reputation points
- Contributor badge"'

ISSUES[7]='--title "feat(reputation): implement daily streak system" --label "good-first-issue,reputation,gamification" --body "## 🎯 What to Do

Implement the daily contribution streak system with bonus multipliers.

## 📋 Tasks

- [ ] Create \`src/reputation/streak.rs\` module
- [ ] Implement streak tracking logic
- [ ] Add multiplier calculation
- [ ] Add streak reset on miss
- [ ] Add streak storage (redb)
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [\`docs/architecture/reputation-gamification.md\`](docs/architecture/reputation-gamification.md)

## ✅ Acceptance Criteria

- [ ] Streak increments on daily contribution
- [ ] Streak resets after 24h inactivity
- [ ] 15+ unit tests passing

## 🏆 Rewards

- +50 reputation points
- Contributor badge"'

ISSUES[8]='--title "feat(security): implement Sybil detection heuristics" --label "good-first-issue,security,anti-cheat" --body "## 🎯 What to Do

Implement Sybil detection heuristics to identify multiple accounts from same operator.

## 📋 Tasks

- [ ] Create \`src/reputation/anti_cheat.rs\` module
- [ ] Implement IP overlap detection
- [ ] Implement hardware fingerprint matching
- [ ] Implement behavioral pattern analysis
- [ ] Add flagging system
- [ ] Write 15+ unit tests

## 📚 Resources

- Architecture: [\`docs/architecture/reputation-gamification.md\`](docs/architecture/reputation-gamification.md)

## ✅ Acceptance Criteria

- [ ] IP overlap detection works
- [ ] False positive rate < 5%
- [ ] 15+ unit tests passing

## 🏆 Rewards

- +100 reputation points
- Contributor badge"'

ISSUES[9]='--title "feat(ui): create contributor onboarding wizard" --label "good-first-issue,frontend,onboarding" --body "## 🎯 What to Do

Build a step-by-step onboarding wizard for new contributors.

## 📋 Tasks

- [ ] Design wizard flow (5 steps max)
- [ ] Create wizard HTML/CSS/JS
- [ ] Step 1: Welcome + mission
- [ ] Step 2: Choose contribution type
- [ ] Step 3: Setup guide
- [ ] Step 4: First issue recommendation
- [ ] Step 5: Join community

## 📚 Resources

- Contributor funnel: [\`docs/community/contributor-funnel.md\`](docs/community/contributor-funnel.md)

## ✅ Acceptance Criteria

- [ ] Wizard completes in < 5 minutes
- [ ] All 5 steps functional
- [ ] Mobile-responsive

## 🏆 Rewards

- +75 reputation points
- Contributor badge"'

# Create issues
echo "📝 Creating issues..."
COUNT=0
for i in "${!ISSUES[@]}"; do
    COUNT=$((COUNT + 1))
    echo "  Creating issue $((i + 1))/${#ISSUES[@]}..."
    gh issue create ${ISSUES[$i]} --repo "$GITHUB_REPOSITORY" 2>/dev/null || \
    gh issue create ${ISSUES[$i]} 2>/dev/null || \
    echo "  ⚠️  Failed to create issue $((i + 1))"
done

echo ""
echo "✅ Created $COUNT issues for v1.8 'ChatGPT Moment' Sprint!"
echo ""
echo "📋 View all issues: gh issue list --label good-first-issue"
