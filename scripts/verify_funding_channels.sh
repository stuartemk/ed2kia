#!/bin/sh
# verify_funding_channels.sh — Validates funding channel setup for ed2kIA
# Usage: bash scripts/verify_funding_channels.sh
# POSIX compliant

set -e

PASS=0
FAIL=0
WARN=0

pass() {
    echo "✅ PASS: $1"
    PASS=$((PASS + 1))
}

fail() {
    echo "❌ FAIL: $1"
    FAIL=$((FAIL + 1))
}

warn() {
    echo "⚠️  WARN: $1"
    WARN=$((WARN + 1))
}

echo "🔍 ed2kIA Funding Channel Verification"
echo "======================================"
echo ""

# 1. Check SUPPORT.md exists
if [ -f "SUPPORT.md" ]; then
    pass "SUPPORT.md exists"
else
    fail "SUPPORT.md missing"
fi

# 2. Check SUPPORT.md contains required keywords
KEYWORDS="GitHub Sponsors Open Collective Gitcoin"
for kw in $KEYWORDS; do
    if grep -q "$kw" SUPPORT.md 2>/dev/null; then
        pass "SUPPORT.md contains '$kw'"
    else
        fail "SUPPORT.md missing '$kw'"
    fi
done

# 3. Check for crypto addresses (BTC/ETH)
if grep -q "BTC\|bitcoin" SUPPORT.md 2>/dev/null; then
    pass "SUPPORT.md mentions BTC/bitcoin"
else
    warn "SUPPORT.md missing BTC reference"
fi

if grep -q "ETH\|ethereum" SUPPORT.md 2>/dev/null; then
    pass "SUPPORT.md mentions ETH/ethereum"
else
    warn "SUPPORT.md missing ETH reference"
fi

# 4. Check funding-strategy.md exists
if [ -f "docs/funding-strategy.md" ]; then
    pass "docs/funding-strategy.md exists"
else
    fail "docs/funding-strategy.md missing"
fi

# 5. Check funding-strategy.md contains key sections
STRATEGY_KEYWORDS="multisig treasury grants revenue"
for kw in $STRATEGY_KEYWORDS; do
    if grep -qi "$kw" docs/funding-strategy.md 2>/dev/null; then
        pass "funding-strategy.md contains '$kw'"
    else
        warn "funding-strategy.md missing '$kw'"
    fi
done

# 6. Check funding-setup-checklist.md exists
if [ -f "docs/funding-setup-checklist.md" ]; then
    pass "docs/funding-setup-checklist.md exists"
else
    fail "docs/funding-setup-checklist.md missing"
fi

# 7. Check funding-setup-checklist.md contains required sections
CHECKLIST_KEYWORDS="multisig Gitcoin Open Collective"
for kw in $CHECKLIST_KEYWORDS; do
    if grep -q "$kw" docs/funding-setup-checklist.md 2>/dev/null; then
        pass "funding-setup-checklist.md contains '$kw'"
    else
        fail "funding-setup-checklist.md missing '$kw'"
    fi
done

# 8. Verify keyword count in funding-setup-checklist.md
COUNT=$(grep -c "multisig\|Gitcoin\|Open Collective" docs/funding-setup-checklist.md 2>/dev/null || echo "0")
if [ "$COUNT" -ge 3 ]; then
    pass "funding-setup-checklist.md has $COUNT references to key terms (>=3 required)"
else
    fail "funding-setup-checklist.md has only $COUNT references to key terms (>=3 required)"
fi

# 9. Check README.md links to SUPPORT.md
if grep -q "SUPPORT" README.md 2>/dev/null; then
    pass "README.md references SUPPORT"
else
    warn "README.md missing SUPPORT reference"
fi

# 10. Check for funding badges in README.md
if grep -q "sponsors\|sponsor" README.md 2>/dev/null; then
    pass "README.md contains sponsor badge/reference"
else
    warn "README.md missing sponsor badge"
fi

# 11. Check CONTRIBUTING.md references funding
if grep -qi "fund\|sponsor\|donate" CONTRIBUTING.md 2>/dev/null; then
    pass "CONTRIBUTING.md references funding"
else
    warn "CONTRIBUTING.md missing funding reference"
fi

# 12. Check for placeholder warnings in funding-setup-checklist.md
if grep -q "PLACEHOLDER\|VERIFICAR" docs/funding-setup-checklist.md 2>/dev/null; then
    warn "funding-setup-checklist.md contains placeholders — replace before public launch"
else
    pass "funding-setup-checklist.md has no placeholders"
fi

# Summary
echo ""
echo "======================================"
echo "📊 Summary"
echo "======================================"
echo "✅ Passed: $PASS"
echo "❌ Failed: $FAIL"
echo "⚠️  Warnings: $WARN"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "❌ VERIFICATION FAILED — Fix issues above before proceeding"
    exit 1
else
    echo "✅ VERIFICATION PASSED — Funding channels properly documented"
    if [ "$WARN" -gt 0 ]; then
        echo "⚠️  Review $WARN warning(s) above"
    fi
    exit 0
fi
