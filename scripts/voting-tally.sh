#!/bin/bash
# voting-tally.sh — Community voting tally script
# Reads CSV export from voting dashboard and calculates weighted totals
# Usage: bash scripts/voting-tally.sh <csv_file> [rfc_id]
# License: Apache 2.0 + Ethical Use Clause

set -euo pipefail

# Configuration
QUORUM_THRESHOLD="0.30"
APPROVAL_THRESHOLD="0.60"
TOTAL_ELIGIBLE_MEMBERS="${TOTAL_ELIGIBLE_MEMBERS:-50}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <csv_file> [rfc_id]"
    echo ""
    echo "Arguments:"
    echo "  csv_file  Path to CSV export from voting dashboard"
    echo "  rfc_id    Optional: Filter by specific RFC ID"
    echo ""
    echo "CSV Format:"
    echo "  rfc_id,propositor,status,vote_weight,tier,timestamp"
    echo ""
    echo "Environment Variables:"
    echo "  TOTAL_ELIGIBLE_MEMBERS  Total weighted eligible members (default: 50)"
    exit 1
}

validate_csv() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo -e "${RED}ERROR: File '$file' not found${NC}"
        exit 1
    fi

    # Check header
    local header
    header=$(head -1 "$file")
    if [[ "$header" != "rfc_id,propositor,status,vote_weight,tier,timestamp" ]]; then
        echo -e "${RED}ERROR: Invalid CSV format. Expected header: rfc_id,propositor,status,vote_weight,tier,timestamp${NC}"
        exit 1
    fi
}

calculate_tally() {
    local file="$1"
    local filter_rfc="${2:-}"

    echo "=========================================="
    echo "  Community Voting Tally Report"
    echo "=========================================="
    echo ""
    echo "Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "Total Eligible Members: $TOTAL_ELIGIBLE_MEMBERS"
    echo ""

    # Extract unique RFC IDs
    local rfc_ids
    if [[ -n "$filter_rfc" ]]; then
        rfc_ids="$filter_rfc"
    else
        rfc_ids=$(tail -n +2 "$file" | cut -d',' -f1 | sort -u)
    fi

    for rfc in $rfc_ids; do
        echo "------------------------------------------"
        echo "RFC: $rfc"
        echo "------------------------------------------"

        # Filter votes for this RFC
        local votes
        votes=$(tail -n +2 "$file" | grep "^${rfc},")

        if [[ -z "$votes" ]]; then
            echo -e "${YELLOW}No votes found for $rfc${NC}"
            echo ""
            continue
        fi

        # Calculate weighted totals
        local pro_weight=0
        local contra_weight=0
        local abstain_weight=0
        local total_voters=0

        while IFS=',' read -r _rfc proposer status weight tier _timestamp; do
            total_voters=$((total_voters + 1))
            case "$status" in
                pro) pro_weight=$(echo "$pro_weight + $weight" | bc) ;;
                contra) contra_weight=$(echo "$contra_weight + $weight" | bc) ;;
                abstain) abstain_weight=$(echo "$abstain_weight + $weight" | bc) ;;
                *) echo -e "${RED}WARNING: Unknown status '$status' for voter $proposer${NC}" ;;
            esac
        done <<< "$votes"

        # Calculate totals
        local total_weight
        total_weight=$(echo "$pro_weight + $contra_weight + $abstain_weight" | bc)
        local decision_weight
        decision_weight=$(echo "$pro_weight + $contra_weight" | bc)

        # Calculate quorum
        local quorum
        quorum=$(echo "$total_weight / $TOTAL_ELIGIBLE_MEMBERS" | bc -l)
        local quorum_pct
        quorum_pct=$(echo "$quorum * 100" | bc -l)

        # Calculate majority
        local majority="N/A"
        local majority_pct="N/A"
        if (( $(echo "$decision_weight > 0" | bc -l) )); then
            majority=$(echo "$pro_weight / $decision_weight" | bc -l)
            majority_pct=$(echo "$majority * 100" | bc -l)
        fi

        # Display results
        echo "Total Voters: $total_voters"
        echo "Weighted Pro: $pro_weight"
        echo "Weighted Contra: $contra_weight"
        echo "Weighted Abstain: $abstain_weight"
        echo "Total Weight: $total_weight"
        echo ""
        echo "Quorum: ${quorum_pct}% (Threshold: $(echo "$QUORUM_THRESHOLD * 100" | bc)%)"
        echo "Majority: ${majority_pct}% (Threshold: $(echo "$APPROVAL_THRESHOLD * 100" | bc)%)"
        echo ""

        # Determine result
        local quorum_met=false
        if (( $(echo "$quorum >= $QUORUM_THRESHOLD" | bc -l) )); then
            quorum_met=true
            echo -e "${GREEN}✅ QUORUM MET${NC}"
        else
            echo -e "${RED}❌ QUORUM NOT MET${NC}"
        fi

        if [[ "$quorum_met" == true ]]; then
            if (( $(echo "$majority >= $APPROVAL_THRESHOLD" | bc -l) )); then
                echo -e "${GREEN}✅ PROPOSAL APPROVED${NC}"
            else
                echo -e "${RED}❌ PROPOSAL REJECTED (Insufficient majority)${NC}"
            fi
        else
            echo -e "${YELLOW}⏸️ PROPOSAL DEFERRED (Quorum not reached)${NC}"
        fi
        echo ""
    done

    echo "=========================================="
    echo "  End of Report"
    echo "=========================================="
}

# Main
if [[ $# -lt 1 ]]; then
    usage
fi

CSV_FILE="$1"
RFC_FILTER="${2:-}"

validate_csv "$CSV_FILE"
calculate_tally "$CSV_FILE" "$RFC_FILTER"
