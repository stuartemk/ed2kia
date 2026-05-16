#!/bin/bash
# generate_contributor_badges.sh — v2.0.0-stable
# Generates contributor badges based on GitHub contribution data
# Usage: ./scripts/generate_contributor_badges.sh [--output dir] [--tier TIER]

set -euo pipefail

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${PROJECT_ROOT}/badges"
TIER="all"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output) OUTPUT_DIR="$2"; shift 2 ;;
        --tier) TIER="$2"; shift 2 ;;
        --help)
            echo "Usage: $0 [--output dir] [--tier TIER]"
            echo "  --output dir  Output directory (default: ./badges)"
            echo "  --tier TIER   Filter by tier: bronze,silver,gold,platinum,diamond,all"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Helper functions
log() {
    echo "[$(date +%Y-%m-%dT%H:%M:%S%z)] $1"
}

# Create output directory
mkdir -p "$OUTPUT_DIR"

# ============================================
# Badge SVG Templates
# ============================================

generate_badge_svg() {
    local name="$1"
    local tier="$2"
    local achievement="$3"
    local color="$4"
    local filename="ed2kIA-badge-${tier}-${achievement}-${TIMESTAMP}.svg"

    cat > "${OUTPUT_DIR}/${filename}" << SVG
<svg xmlns="http://www.w3.org/2000/svg" width="150" height="45" viewBox="0 0 150 45">
  <defs>
    <linearGradient id="grad-${tier}" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:${color};stop-opacity:1" />
      <stop offset="100%" style="stop-color:${color};stop-opacity:0.7" />
    </linearGradient>
  </defs>
  <rect width="150" height="45" rx="5" ry="5" fill="url(#grad-${tier})" stroke="#333" stroke-width="1"/>
  <text x="75" y="18" font-family="Arial, sans-serif" font-size="10" fill="white" text-anchor="middle" font-weight="bold">ed2kIA</text>
  <text x="75" y="32" font-family="Arial, sans-serif" font-size="12" fill="white" text-anchor="middle">${achievement}</text>
  <text x="75" y="42" font-family="Arial, sans-serif" font-size="8" fill="#ddd" text-anchor="middle">${tier}</text>
</svg>
SVG
    log "Generated badge: $filename"
}

# ============================================
# Color Mapping
# ============================================

get_tier_color() {
    case "$1" in
        bronze) echo "#CD7F32" ;;
        silver) echo "#C0C0C0" ;;
        gold) echo "#FFD700" ;;
        platinum) echo "#E5E4E2" ;;
        diamond) echo "#B9F2FF" ;;
        *) echo "#888888" ;;
    esac
}

# ============================================
# Achievement Definitions
# ============================================

declare -A ACHIEVEMENTS=(
    ["first_commit"]="🌱 Seed"
    ["bug_hunter"]="🐛 Bug Hunter"
    ["feature_builder"]="🔨 Builder"
    ["test_master"]="✅ Test Master"
    ["doc_wizard"]="📚 Wizard"
    ["perf_guru"]="⚡ Guru"
    ["security_sentinel"]="🛡️ Sentinel"
    ["release_engineer"]="🚀 Engineer"
    ["arch_sage"]="🏛️ Sage"
    ["v2_pioneer"]="🌟 Pioneer"
    ["welcome_wagon"]="🤝 Wagon"
    ["discussion_leader"]="💬 Leader"
    ["triage_hero"]="🦸 Hero"
    ["event_organizer"]="🎉 Organizer"
    ["translator"]="🌍 Translator"
    ["advocate"]="📢 Advocate"
    ["grant_writer"]="📝 Writer"
    ["ambassador"]="👑 Ambassador"
)

# ============================================
# Generate Badges by Tier
# ============================================

log "Generating contributor badges..."
log "Output directory: $OUTPUT_DIR"
log "Tier filter: $TIER"

# Read contributors from git log
log "Analyzing git contributions..."

# Get top contributors
TOP_CONTRIBUTORS=$(git log --format='%aN' | sort | uniq -c | sort -rn | head -20)

# Generate badges for each contributor
CONTRIBUTOR_COUNT=0
while IFS= read -r line; do
    COUNT=$(echo "$line" | awk '{print $1}')
    NAME=$(echo "$line" | awk '{print $2}')

    if [[ -z "$NAME" ]]; then continue; fi

    # Determine tier based on commit count
    TIER_DETERMINED="bronze"
    if [[ $COUNT -ge 100 ]]; then
        TIER_DETERMINED="diamond"
    elif [[ $COUNT -ge 50 ]]; then
        TIER_DETERMINED="platinum"
    elif [[ $COUNT -ge 20 ]]; then
        TIER_DETERMINED="gold"
    elif [[ $COUNT -ge 5 ]]; then
        TIER_DETERMINED="silver"
    fi

    # Apply tier filter
    if [[ "$TIER" != "all" && "$TIER" != "$TIER_DETERMINED" ]]; then
        continue
    fi

    COLOR=$(get_tier_color "$TIER_DETERMINED")

    # Generate achievement badges based on contribution type
    # For now, generate a general contribution badge
    generate_badge_svg "$NAME" "$TIER_DETERMINED" "Contributor" "$COLOR"

    # Generate v2.0 Pioneer badge for recent contributors
    if git log --author="$NAME" --since="2026-05-01" --oneline | grep -q .; then
        generate_badge_svg "$NAME" "spec" "v2.0 Pioneer" "#9B59B6"
    fi

    CONTRIBUTOR_COUNT=$((CONTRIBUTOR_COUNT + 1))
done <<< "$TOP_CONTRIBUTORS"

# ============================================
# Generate Special Badges
# ============================================

log "Generating special badges..."

# Security Sentinel badge
generate_badge_svg "security" "sec" "Security Sentinel" "#E74C3C"

# Release Engineer badge
generate_badge_svg "release" "tech" "Release Engineer" "#3498DB"

# Test Master badge
generate_badge_svg "testing" "tech" "Test Master" "#2ECC71"

# ============================================
# Generate Summary
# ============================================

SUMMARY_FILE="${OUTPUT_DIR}/summary_${TIMESTAMP}.json"
cat > "$SUMMARY_FILE" << JSON
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "total_badges_generated": $(ls "${OUTPUT_DIR}"/*.svg 2>/dev/null | wc -l),
  "contributors_processed": $CONTRIBUTOR_COUNT,
  "tier_filter": "$TIER",
  "output_directory": "$OUTPUT_DIR"
}
JSON

log "Summary saved: $SUMMARY_FILE"

# ============================================
# Final Report
# ============================================

echo ""
echo "=========================================="
echo "  Badge Generation Complete"
echo "  Badges generated: $(ls "${OUTPUT_DIR}"/*.svg 2>/dev/null | wc -l)"
echo "  Contributors: $CONTRIBUTOR_COUNT"
echo "  Output: $OUTPUT_DIR"
echo "=========================================="
echo ""
