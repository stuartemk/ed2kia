#!/bin/sh
# grant_execution_support.sh — Grant Execution Support Script (POSIX)
# Usage: ./scripts/grant_execution_support.sh [--verify] [--package] [--manifest]
#
# Modes:
#   --verify    Verify grant files exist and are complete
#   --package   Create submission package with checksums
#   --manifest  Generate submission-manifest.json
#
# IMPORTANT: This script PREPARES grant packages. It does NOT submit grants.
# Submission requires human authentication on external portals.
#
# Exit codes:
#   0 = Success
#   1 = Validation failure
#   2 = Script error

set -e

GRANTS_DIR="docs/grants"
OUTPUT_DIR="grants-submission-v1.9"
MANIFEST_FILE="${OUTPUT_DIR}/submission-manifest.json"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "2026-05-16T00:00:00Z")

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; }
log_info() { echo "[INFO] $1"; }

# Verify grant files exist
verify_grants() {
    log_info "Verifying grant files..."

    MISSING=0

    # Gitcoin Grant
    if [ -f "${GRANTS_DIR}/gitcoin-quadratic-funding-draft.md" ]; then
        log_pass "Gitcoin Quadratic Funding draft exists"
    else
        log_fail "Missing: ${GRANTS_DIR}/gitcoin-quadratic-funding-draft.md"
        MISSING=$((MISSING + 1))
    fi

    # NSF Grant
    if [ -f "${GRANTS_DIR}/nsf-ai-safety-draft.md" ]; then
        log_pass "NSF AI Safety draft exists"
    else
        log_fail "Missing: ${GRANTS_DIR}/nsf-ai-safety-draft.md"
        MISSING=$((MISSING + 1))
    fi

    # OSSF Grant
    if [ -f "${GRANTS_DIR}/ossf-draft.md" ]; then
        log_pass "OSSF draft exists"
    else
        log_fail "Missing: ${GRANTS_DIR}/ossf-draft.md"
        MISSING=$((MISSING + 1))
    fi

    # Submission tracker
    if [ -f "${GRANTS_DIR}/submission-tracker.md" ]; then
        log_pass "Submission tracker exists"
    else
        log_warn "Missing: ${GRANTS_DIR}/submission-tracker.md (optional)"
    fi

    # Follow-up tracker
    if [ -f "${GRANTS_DIR}/follow-up-tracker.md" ]; then
        log_pass "Follow-up tracker exists"
    else
        log_warn "Missing: ${GRANTS_DIR}/follow-up-tracker.md (optional)"
    fi

    if [ "${MISSING}" -gt 0 ]; then
        log_fail "${MISSING} required grant file(s) missing"
        return 1
    fi

    log_pass "All required grant files present"
    return 0
}

# Generate submission manifest
generate_manifest() {
    log_info "Generating submission manifest..."

    mkdir -p "${OUTPUT_DIR}"

    # Compute checksums
    GITCOIN_CHECKSUM=""
    NSF_CHECKSUM=""
    OSSF_CHECKSUM=""

    if command -v sha256sum >/dev/null 2>&1; then
        GITCOIN_CHECKSUM=$(sha256sum "${GRANTS_DIR}/gitcoin-quadratic-funding-draft.md" | cut -d' ' -f1)
        NSF_CHECKSUM=$(sha256sum "${GRANTS_DIR}/nsf-ai-safety-draft.md" | cut -d' ' -f1)
        OSSF_CHECKSUM=$(sha256sum "${GRANTS_DIR}/ossf-draft.md" | cut -d' ' -f1)
    elif command -v shasum >/dev/null 2>&1; then
        GITCOIN_CHECKSUM=$(shasum -a 256 "${GRANTS_DIR}/gitcoin-quadratic-funding-draft.md" | cut -d' ' -f1)
        NSF_CHECKSUM=$(shasum -a 256 "${GRANTS_DIR}/nsf-ai-safety-draft.md" | cut -d' ' -f1)
        OSSF_CHECKSUM=$(shasum -a 256 "${GRANTS_DIR}/ossf-draft.md" | cut -d' ' -f1)
    else
        log_warn "No SHA256 tool found — checksums will be empty"
    fi

    cat > "${MANIFEST_FILE}" << EOF
{
  "version": "v1.9.0-stable",
  "generated": "${TIMESTAMP}",
  "grants": [
    {
      "name": "Gitcoin Quadratic Funding",
      "file": "gitcoin-quadratic-funding-draft.md",
      "portal": "https://gitcoin.co/grants",
      "checksum_sha256": "${GITCOIN_CHECKSUM}",
      "requirements": [
        "Gitcoin account with verified wallet",
        "Project page with description and media",
        "Grant application form completion",
        "Community voting campaign (during round)"
      ],
      "auth_method": "MetaMask/WalletConnect",
      "status": "ready_for_review"
    },
    {
      "name": "NSF AI Safety",
      "file": "nsf-ai-safety-draft.md",
      "portal": "https://www.nsfcareer.org/Apply/",
      "checksum_sha256": "${NSF_CHECKSUM}",
      "requirements": [
        "NSF account (or institutional affiliation)",
        "Proposal through Grants.gov",
        "Budget justification document",
        "Letters of support from partners",
        "IRB approval (if applicable)"
      ],
      "auth_method": "Grants.gov account + NSF login",
      "status": "ready_for_review"
    },
    {
      "name": "OSSF Scorecard Improvement",
      "file": "ossf-draft.md",
      "portal": "https://openssf.org/projects/community-profile-development/",
      "checksum_sha256": "${OSSF_CHECKSUM}",
      "requirements": [
        "GitHub organization with active project",
        "OSSF Scorecard ≥ 7.0/10",
        "Security policy and disclosure process",
        "Diverse maintainer team"
      ],
      "auth_method": "GitHub OAuth + email verification",
      "status": "ready_for_review"
    }
  ],
  "execution_checklist": [
    "1. Review all grant drafts for accuracy",
    "2. Update placeholder contact information",
    "3. Verify project metrics (GitHub stars, contributors, etc.)",
    "4. Prepare supporting documents (budget, timeline, team bios)",
    "5. Create accounts on grant portals (if not already done)",
    "6. Submit applications through official portals",
    "7. Track submission status in submission-tracker.md",
    "8. Follow up according to follow-up-tracker.md"
  ],
  "disclaimer": "This manifest is for preparation only. Grant submission requires human authentication on external portals. Never share credentials or sign transactions without verification."
}
EOF

    log_pass "Manifest generated: ${MANIFEST_FILE}"
}

# Package grants for submission
package_grants() {
    log_info "Packaging grants for submission..."

    # Verify first
    verify_grants || return 1

    # Generate manifest
    generate_manifest

    # Copy grant files to output directory
    cp "${GRANTS_DIR}/gitcoin-quadratic-funding-draft.md" "${OUTPUT_DIR}/"
    cp "${GRANTS_DIR}/nsf-ai-safety-draft.md" "${OUTPUT_DIR}/"
    cp "${GRANTS_DIR}/ossf-draft.md" "${OUTPUT_DIR}/"

    # Copy trackers if they exist
    [ -f "${GRANTS_DIR}/submission-tracker.md" ] && cp "${GRANTS_DIR}/submission-tracker.md" "${OUTPUT_DIR}/"
    [ -f "${GRANTS_DIR}/follow-up-tracker.md" ] && cp "${GRANTS_DIR}/follow-up-tracker.md" "${OUTPUT_DIR}/"

    # Create execution checklist
    cat > "${OUTPUT_DIR}/EXECUTION_CHECKLIST.md" << 'EOF'
# Grant Execution Checklist

> **IMPORTANT:** This checklist is for HUMAN execution. Do not automate grant submissions.

## Pre-Submission

- [ ] Review all grant drafts for accuracy and completeness
- [ ] Update placeholder contact information with real data
- [ ] Verify project metrics (GitHub stars, contributors, downloads)
- [ ] Prepare supporting documents:
  - [ ] Budget justification
  - [ ] Project timeline
  - [ ] Team bios
  - [ ] Letters of support (if required)

## Gitcoin Quadratic Funding

- [ ] Create/verify Gitcoin account
- [ ] Connect wallet (MetaMask/WalletConnect)
- [ ] Navigate to: https://gitcoin.co/grants
- [ ] Fill application form using draft as reference
- [ ] Add project media (logo, banner, video)
- [ ] Review and submit
- [ ] Track in submission-tracker.md

## NSF AI Safety

- [ ] Create/verify NSF account (or use institutional)
- [ ] Register on Grants.gov
- [ ] Search for relevant AI safety funding opportunities
- [ ] Prepare proposal according to NSF guidelines
- [ ] Submit through Grants.gov
- [ ] Track in submission-tracker.md

## OSSF

- [ ] Verify OSSF Scorecard (current: 8.5/10)
- [ ] Ensure security policy is public
- [ ] Prepare project profile
- [ ] Submit application through OSSF portal
- [ ] Track in submission-tracker.md

## Post-Submission

- [ ] Record submission dates in tracker
- [ ] Set follow-up reminders
- [ ] Prepare for potential interviews/reviews
- [ ] Continue project development
- [ ] Update community on progress
EOF

    # Create tar.gz archive
    if command -v tar >/dev/null 2>&1; then
        tar -czf grants-submission-v1.9.tar.gz "${OUTPUT_DIR}/"
        log_pass "Archive created: grants-submission-v1.9.tar.gz"
    else
        log_warn "tar not available — archive not created"
    fi

    log_pass "Package ready in ${OUTPUT_DIR}/"
}

# Print execution steps
print_steps() {
    echo "========================================"
    echo "  Grant Execution Support"
    echo "  Generated: ${TIMESTAMP}"
    echo "========================================"
    echo ""
    echo "IMPORTANT: This script PREPARES grant packages."
    echo "Submission requires HUMAN authentication."
    echo ""
    echo "Portal Links:"
    echo "  Gitcoin: https://gitcoin.co/grants"
    echo "  NSF:     https://www.nsfcareer.org/Apply/"
    echo "  OSSF:    https://openssf.org/projects/community-profile-development/"
    echo ""
    echo "Authentication Methods:"
    echo "  Gitcoin: MetaMask/WalletConnect"
    echo "  NSF:     Grants.gov account + NSF login"
    echo "  OSSF:    GitHub OAuth + email verification"
    echo ""
    echo "Next Steps:"
    echo "  1. Review grant drafts in docs/grants/"
    echo "  2. Update placeholder information"
    echo "  3. Follow EXECUTION_CHECKLIST.md"
    echo "  4. Submit through official portals"
    echo "  5. Track progress in submission-tracker.md"
    echo ""
}

# Main
main() {
    MODE="${1:---verify}"

    case "${MODE}" in
        --verify)
            verify_grants
            ;;
        --manifest)
            verify_grants
            generate_manifest
            ;;
        --package)
            package_grants
            ;;
        *)
            echo "Usage: $0 [--verify|--manifest|--package]"
            print_steps
            ;;
    esac
}

main "$@"
