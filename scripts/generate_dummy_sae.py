#!/usr/bin/env python3
"""Generate a dummy QwenScope .safetensors model for local testing.

Produces `models/dummy_qwen_scope.safetensors` with tensors:
  - W_enc: (d_sae, d_model)
  - W_dec: (d_model, d_sae)
  - b_enc: (d_sae,)
  - b_dec: (d_model,)

Default dimensions: d_model=64, d_sae=256.
Compatible with QwenScopeLoader.

Usage:
    python3 scripts/generate_dummy_sae.py
    python3 scripts/generate_dummy_sae.py --d-model 128 --d-sae 512
"""

import argparse
import os
import struct
import json
import sys
from pathlib import Path


def generate_dummy_safetensors(output_path: str, d_model: int, d_sae: int) -> None:
    """Write a minimal .safetensors file with dummy QwenScope weights.

    Safetensors format:
      [8 bytes: JSON header length (little-endian u64)]
      [N bytes: JSON header]
      [tensor data...]

    The JSON header is a dict mapping tensor names to {dtype, shape, data_offsets}.
    Data offsets are relative to the start of the tensor data section.
    """
    # Build tensor data (float32 = 4 bytes per element)
    dtype = "F32"  # safetensors dtype string for f32
    dtype_bytes = 4

    tensors = {
        "W_enc": {"shape": (d_sae, d_model), "elements": d_sae * d_model},
        "W_dec": {"shape": (d_model, d_sae), "elements": d_model * d_sae},
        "b_enc": {"shape": (d_sae,), "elements": d_sae},
        "b_dec": {"shape": (d_model,), "elements": d_model},
    }

    # Build binary tensor data (all zeros for simplicity)
    tensor_data = bytearray()
    header = {}

    for name, info in tensors.items():
        offset = len(tensor_data)
        count = info["elements"]
        # Append zeros as float32
        tensor_data.extend(b"\x00" * (count * dtype_bytes))
        header[name] = {
            "dtype": dtype,
            "shape": list(info["shape"]),
            "data_offsets": [offset, offset + count * dtype_bytes],
        }

    # Build JSON header
    json_header = json.dumps(header).encode("utf-8")
    # Pad JSON header to 8-byte alignment
    padding = 8 - (len(json_header) % 8)
    if padding < 8:
        json_header += b" " * padding

    # Write file
    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
    with open(output_path, "wb") as f:
        # Header length (little-endian u64)
        f.write(struct.pack("<Q", len(json_header)))
        # JSON header
        f.write(json_header)
        # Tensor data
        f.write(tensor_data)

    file_size = os.path.getsize(output_path)
    print(f"Generated: {output_path}")
    print(f"  d_model = {d_model}")
    print(f"  d_sae   = {d_sae}")
    print(f"  Size    = {file_size:,} bytes ({file_size / 1024:.1f} KB)")
    print(f"  Tensors: {', '.join(tensors.keys())}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate dummy QwenScope .safetensors model for testing."
    )
    parser.add_argument(
        "--d-model",
        type=int,
        default=64,
        help="Model dimension (default: 64)",
    )
    parser.add_argument(
        "--d-sae",
        type=int,
        default=256,
        help="SAE latent dimension (default: 256)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=None,
        help="Output path (default: models/dummy_qwen_scope.safetensors)",
    )
    args = parser.parse_args()

    # Resolve output path relative to project root
    if args.output:
        output_path = args.output
    else:
        # Find project root (directory containing Cargo.toml)
        script_dir = Path(__file__).parent
        project_root = script_dir.parent
        output_path = str(project_root / "models" / "dummy_qwen_scope.safetensors")

    if not os.path.isabs(output_path):
        output_path = str(project_root / output_path)  # type: ignore[operator]

    generate_dummy_safetensors(output_path, args.d_model, args.d_sae)


if __name__ == "__main__":
    main()
