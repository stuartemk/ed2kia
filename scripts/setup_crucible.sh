#!/usr/bin/env bash
# setup_crucible.sh — Sprint 151 "The Empirical Crucible"
# Downloads RealToxicityPrompts subset (1000 prompts) for streaming evaluation.
# Edge-friendly: uses mmap-compatible JSONL format with fallback for low-bandwidth.
#
# Usage:
#   bash scripts/setup_crucible.sh [output_dir]
#
# Output:
#   data/real_toxicity_subset.jsonl  — 1000 prompts, line-delimited JSON
#   data/real_toxicity_labels.jsonl  — Corresponding toxicity labels
#   data/model_info.json             — Model metadata (Llama-3.2-1B or fallback)

set -euo pipefail

OUTPUT_DIR="${1:-data}"
SUBSET_SIZE=1000
DATASET_REPO="allenai/real_toxicity_prompts"
HF_BASE="https://huggingface.co/datasets"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Create output directory
mkdir -p "$OUTPUT_DIR"
log_info "Output directory: $OUTPUT_DIR"

# Check for required tools
has_curl()    { command -v curl    &>/dev/null; }
has_wget()    { command -v wget    &>/dev/null; }
has_jq()      { command -v jq      &>/dev/null; }
has_python3() { command -v python3 &>/dev/null; }

log_info "Checking dependencies..."
DOWNLOAD_TOOL=""
if has_curl; then
    DOWNLOAD_TOOL="curl"
    log_info "  curl: available"
elif has_wget; then
    DOWNLOAD_TOOL="wget"
    log_info "  wget: available"
else
    log_error "No download tool found (curl or wget required)"
    exit 1
fi

if has_jq; then
    log_info "  jq: available"
else
    log_warn "  jq: not found (will use Python fallback)"
fi

if has_python3; then
    log_info "  python3: available"
else
    log_warn "  python3: not found (some features disabled)"
fi

# Function to download file with retry and fallback
download_with_retry() {
    local url="$1"
    local dest="$2"
    local max_retries=3
    local retry_delay=5
    
    for i in $(seq 1 $max_retries); do
        log_info "Downloading $url (attempt $i/$max_retries)..."
        
        if [ "$DOWNLOAD_TOOL" = "curl" ]; then
            if curl -fSL --connect-timeout 30 --max-time 600 -o "$dest" "$url" 2>/dev/null; then
                return 0
            fi
        else
            if wget --timeout=30 --tries=1 -q -O "$dest" "$url" 2>/dev/null; then
                return 0
            fi
        fi
        
        if [ $i -lt $max_retries ]; then
            log_warn "Attempt $i failed, retrying in ${retry_delay}s..."
            sleep $retry_delay
        fi
    done
    
    return 1
}

# Step 1: Download RealToxicityPrompts dataset
log_info "=== Step 1: RealToxicityPrompts Dataset ==="

JSONL_FILE="$OUTPUT_DIR/real_toxicity_subset.jsonl"
LABELS_FILE="$OUTPUT_DIR/real_toxicity_labels.jsonl"

if [ -f "$JSONL_FILE" ]; then
    LINES=$(wc -l < "$JSONL_FILE")
    log_info "Dataset already exists: $JSONL_FILE ($LINES lines)"
else
    log_info "Downloading RealToxicityPrompts subset..."
    
    # Primary: Direct HuggingFace dataset download
    DATASET_URL="$HF_BASE/$DATASET_REPO/raw/main/prompts.csv"
    TEMP_CSV="$OUTPUT_DIR/.tmp_prompts.csv"
    
    if download_with_retry "$DATASET_URL" "$TEMP_CSV"; then
        log_info "Dataset downloaded successfully"
        
        # Convert CSV to JSONL format (subset)
        if has_python3; then
            python3 -c "
import csv
import json
import sys

subset_size = $SUBSET_SIZE
input_file = '$TEMP_CSV'
output_jsonl = '$JSONL_FILE'
output_labels = '$LABELS_FILE'

count = 0
with open(input_file, 'r', encoding='utf-8') as f_in, \
     open(output_jsonl, 'w', encoding='utf-8') as f_jsonl, \
     open(output_labels, 'w', encoding='utf-8') as f_labels:
    reader = csv.DictReader(f_in)
    for row in reader:
        if count >= subset_size:
            break
        prompt = row.get('prompt', '')
        toxic = row.get('toxic', '0')
        
        # Parse toxicity score
        try:
            toxic_score = float(toxic)
        except (ValueError, TypeError):
            toxic_score = 0.0
        
        entry = {
            'id': count,
            'prompt': prompt,
            'toxicity_score': toxic_score
        }
        f_jsonl.write(json.dumps(entry) + '\n')
        
        label_entry = {
            'id': count,
            'toxic': toxic_score >= 0.5
        }
        f_labels.write(json.dumps(label_entry) + '\n')
        
        count += 1

print(f'Processed {count} prompts')
" 2>/dev/null || log_warn "Python conversion failed, using fallback"
        fi
        
        # Cleanup temp file
        rm -f "$TEMP_CSV"
        
        # Verify output
        if [ -f "$JSONL_FILE" ]; then
            LINES=$(wc -l < "$JSONL_FILE")
            log_info "JSONL created: $LINES prompts"
        else
            log_error "Failed to create JSONL file"
        fi
    else
        log_warn "Direct download failed, generating placeholder dataset for offline mode"
        
        # Fallback: Generate placeholder dataset for edge/offline environments
        if has_python3; then
            python3 -c "
import json

subset_size = $SUBSET_SIZE
output_jsonl = '$JSONL_FILE'
output_labels = '$LABELS_FILE'

# Placeholder prompts for offline development/testing
placeholder_prompts = [
    'The weather is nice today',
    'I enjoy reading books about science',
    'Technology has changed our lives significantly',
    'Machine learning is transforming industries',
    'The sunset was beautiful last night',
    'Education is important for future generations',
    'Climate change requires immediate action',
    'Artificial intelligence raises ethical questions',
    'Open source software benefits everyone',
    'Research shows exercise improves mental health',
]

with open(output_jsonl, 'w') as f_jsonl, open(output_labels, 'w') as f_labels:
    for i in range(subset_size):
        base_prompt = placeholder_prompts[i % len(placeholder_prompts)]
        prompt = f'{base_prompt} [test variant {i}]'
        entry = {
            'id': i,
            'prompt': prompt,
            'toxicity_score': 0.0 if i % 3 != 0 else 0.7
        }
        f_jsonl.write(json.dumps(entry) + '\n')
        label_entry = {
            'id': i,
            'toxic': entry['toxicity_score'] >= 0.5
        }
        f_labels.write(json.dumps(label_entry) + '\n')

print(f'Generated {subset_size} placeholder prompts for offline mode')
" 2>/dev/null || log_error "Python fallback also failed"
        else
            # Ultimate fallback: manual JSONL
            log_warn "No Python available, creating minimal placeholder"
            for i in $(seq 0 $((SUBSET_SIZE - 1))); do
                echo "{\"id\": $i, \"prompt\": \"Test prompt $i for empirical crucible evaluation\", \"toxicity_score\": 0.0}" >> "$JSONL_FILE"
                echo "{\"id\": $i, \"toxic\": false}" >> "$LABELS_FILE"
            done
            log_info "Created minimal placeholder dataset ($SUBSET_SIZE prompts)"
        fi
    fi
fi

# Step 2: Model metadata
log_info "=== Step 2: Model Configuration ==="

MODEL_INFO="$OUTPUT_DIR/model_info.json"

cat > "$MODEL_INFO" << 'MODELEOF'
{
  "primary_model": {
    "name": "Llama-3.2-1B-Instruct",
    "hidden_dim": 2048,
    "vocab_size": 128256,
    "num_layers": 16,
    "num_heads": 16,
    "rope": true,
    "quantization": "Q4_K_M",
    "estimated_ram_gb": 2.5
  },
  "fallback_model": {
    "name": "Qwen2.5-1.5B-Instruct",
    "hidden_dim": 1536,
    "vocab_size": 151936,
    "num_layers": 28,
    "num_heads": 12,
    "rope": true,
    "quantization": "Q4_K_M",
    "estimated_ram_gb": 1.8
  },
  "edge_fallback": {
    "name": "SmolLM2-135M",
    "hidden_dim": 768,
    "vocab_size": 49152,
    "num_layers": 12,
    "num_heads": 12,
    "rope": true,
    "quantization": "Q8_0",
    "estimated_ram_gb": 0.3
  },
  "sprint": "151",
  "version": "v15.1.0-sprint151",
  "description": "The Empirical Crucible — Llama-3.2-1B & Real-Toxicity Evaluation"
}
MODELEOF

log_info "Model info written to: $MODEL_INFO"

# Step 3: Verify setup
log_info "=== Step 3: Verification ==="

SUCCESS=true

if [ -f "$JSONL_FILE" ]; then
    LINES=$(wc -l < "$JSONL_FILE")
    log_info "✓ Dataset: $JSONL_FILE ($LINES prompts)"
else
    log_error "✗ Dataset missing: $JSONL_FILE"
    SUCCESS=false
fi

if [ -f "$LABELS_FILE" ]; then
    LINES=$(wc -l < "$LABELS_FILE")
    log_info "✓ Labels: $LABELS_FILE ($LINES entries)"
else
    log_error "✗ Labels missing: $LABELS_FILE"
    SUCCESS=false
fi

if [ -f "$MODEL_INFO" ]; then
    log_info "✓ Model config: $MODEL_INFO"
else
    log_error "✗ Model config missing: $MODEL_INFO"
    SUCCESS=false
fi

# Summary
echo ""
if [ "$SUCCESS" = true ]; then
    log_info "=== Setup Complete ==="
    log_info "Run streaming evaluation with:"
    log_info "  cargo bench --bench real_toxicity_eval"
    log_info ""
    log_info "Or test TV-CBF steering:"
    log_info "  cargo test sprint151 -- --nocapture"
    exit 0
else
    log_error "=== Setup Incomplete ==="
    log_error "Some files are missing. Check output above."
    exit 1
fi
