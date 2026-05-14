#!/bin/sh
# post_community_launch.sh — Automated community launch commands for ed2kIA v1.7
# Usage: bash scripts/post_community_launch.sh
# POSIX compliant — Requires GitHub CLI (gh) for GitHub actions

set -e

echo "🚀 ed2kIA v1.7 Community Launch Script"
echo "======================================="
echo ""

# Check gh availability
if command -v gh &> /dev/null; then
    echo "✅ GitHub CLI detected"
    GH_AVAILABLE=1
else
    echo "⚠️  GitHub CLI not found — GitHub actions will be skipped"
    echo "    Install: https://cli.github.com/"
    GH_AVAILABLE=0
fi

# 1. Create GitHub Release (if gh available)
if [ "$GH_AVAILABLE" = "1" ]; then
    echo ""
    echo "📦 Creating GitHub Release v1.7.0-stable..."
    gh release create v1.7.0-stable \
        --title "ed2kIA v1.7.0-stable: Latency PoC & Auto-Push Active" \
        --notes "## ed2kIA v1.7.0-stable

Decentralized AI Federation with verifiable contributions.

### Highlights
- SAE Fine-Tuning v7: Distributed training with cross-model gradient alignment
- Quantization v3: FP8/INT4 with <2% precision loss
- Async ZKP v14: Adaptive proof batching with Merkle+VRF fallback
- Auto-Push CI/CD: Zero-friction contribution workflow

### Contributing
Good First Issues: https://github.com/Stuartemk/ed2kIA/blob/main/ISSUES_BATCH_V1.8.md
Contributor Funnel: https://github.com/Stuartemk/ed2kIA/blob/main/docs/community/contributor-funnel.md

### Funding
See SUPPORT.md for GitHub Sponsors, Open Collective, and crypto options.

Apache 2.0 Licensed | Zero Telemetry | Ethical AI Focus" \
        --target main 2>/dev/null && echo "✅ Release created" || echo "⚠️  Release already exists or failed"
else
    echo ""
    echo "⏭️  Skipping GitHub Release — run manually:"
    echo "    gh release create v1.7.0-stable --title 'ed2kIA v1.7.0-stable' --notes '...' --target main"
fi

# 2. Pin v1.8 Issues (informational)
echo ""
echo "📋 v1.8 Issues Batch:"
echo "    File: ISSUES_BATCH_V1.8.md"
echo "    Script: scripts/create_issues_v1.8.sh"
echo "    Run: bash scripts/create_issues_v1.8.sh (requires gh)"

# 3. Community Post Templates
echo ""
echo "======================================="
echo "📝 Community Post Templates"
echo "======================================="
echo ""
echo "--- EleutherAI Discord ---"
echo '🚀 ed2kIA v1.7.0-stable — Decentralized AI Federation with Verifiable Contributions'
echo ''
echo 'Key features:'
echo '✅ SAE fine-tuning with cross-model gradient alignment (v7)'
echo '✅ FP8/INT4 quantization with <2% precision loss'
echo '✅ Async ZKP proof batching (v14) with adaptive routing'
echo '✅ Auto-Push CI/CD protocol for zero-friction contributions'
echo ''
echo '👉 Good First Issues: https://github.com/Stuartemk/ed2kIA/blob/main/ISSUES_BATCH_V1.8.md'
echo '📖 Docs: https://github.com/Stuartemk/ed2kIA'
echo ''
echo "--- r/rust ---"
echo 'Title: ed2kIA v1.7 — Decentralized AI Federation in Rust (SAE fine-tuning, ZKP proofs, FP8 quantization)'
echo ''
echo 'We released ed2kIA v1.7.0-stable, a Rust framework for decentralized AI training with cryptographic verification.'
echo 'Built entirely in Rust with zero unsafe code.'
echo ''
echo '👉 https://github.com/Stuartemk/ed2kIA'
echo ''
echo "--- Hugging Face Discord ---"
echo '🔬 ed2kIA v1.7 — Verifiable AI Training Framework'
echo ''
echo 'Building transparent AI training with cryptographic proofs.'
echo '✅ SAE fine-tuning | ✅ FP8/INT4 quantization | ✅ ZKP verification'
echo ''
echo '👉 https://github.com/Stuartemk/ed2kIA'
echo ''
echo "--- Twitter/X ---"
echo '🚀 ed2kIA v1.7.0-stable is live!'
echo 'Decentralized AI training with cryptographic verification.'
echo '✅ SAE fine-tuning | ✅ FP8/INT4 | ✅ ZKP | ✅ Auto-Push CI/CD'
echo '👉 https://github.com/Stuartemk/ed2kIA'
echo '#OpenSource #AI #Rust #DecentralizedAI'

# 4. Verification Commands
echo ""
echo "======================================="
echo "✅ Verification Commands"
echo "======================================="
echo ""
echo "# Check release exists"
echo "gh release view v1.7.0-stable"
echo ""
echo "# List open issues"
echo "gh issue list --label good-first-issue --limit 10"
echo ""
echo "# Check funding channels"
echo "bash scripts/verify_funding_channels.sh"
echo ""
echo "# View contributor funnel"
echo "cat docs/community/contributor-funnel.md"

echo ""
echo "======================================="
echo "🎉 Launch script complete!"
echo "======================================="
echo ""
echo "Next steps:"
echo "1. Copy/paste post templates to each platform"
echo "2. Monitor responses for 24h"
echo "3. Run: bash scripts/verify_funding_channels.sh"
echo "4. Review: COMMUNITY_LAUNCH_CHECKLIST.md"
