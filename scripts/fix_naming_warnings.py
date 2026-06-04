#!/usr/bin/env python3
"""Sprint 87: Fix all remaining naming warnings in ed2kIA codebase."""
import os
import re
import glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "src")

# Files to process: all .rs files in src/
rs_files = []
for dirpath, _, filenames in os.walk(SRC):
    for f in filenames:
        if f.endswith('.rs'):
            rs_files.append(os.path.join(dirpath, f))

fixes_applied = 0

def fix_file(filepath):
    """Apply naming fixes to a single file."""
    global fixes_applied
    try:
        with open(filepath, 'r', encoding='utf-8', errors='replace') as fh:
            content = fh.read()
    except Exception:
        return

    original = content

    # Fix Byzantine_Eviction -> ByzantineEviction (type/variant names)
    content = content.replace('Byzantine_EvictionFailed', 'ByzantineEvictionFailed')
    content = content.replace('Byzantine_EvictionError', 'ByzantineEvictionError')
    content = content.replace('Byzantine_EvictionActive', 'ByzantineEvictionActive')
    content = content.replace('Byzantine_EvictionState', 'ByzantineEvictionState')
    content = content.replace('Byzantine_EvictionConfig', 'ByzantineEvictionConfig')
    content = content.replace('Byzantine_EvictionNodeState', 'ByzantineEvictionNodeState')
    content = content.replace('Byzantine_EvictionRecord', 'ByzantineEvictionRecord')
    content = content.replace('GracefulByzantine_Eviction', 'GracefulByzantineEviction')
    content = content.replace('Byzantine_Eviction_triggered', 'byzantine_eviction_triggered')
    content = content.replace('Byzantine_Eviction_counter', 'byzantine_eviction_counter')
    content = content.replace('Byzantine_Eviction_threshold', 'byzantine_eviction_threshold')
    content = content.replace('max_concurrent_Byzantine_Eviction', 'max_concurrent_byzantine_eviction')
    content = content.replace('trigger_Byzantine_Eviction', 'trigger_byzantine_eviction')
    content = content.replace('evaluate_Byzantine_Eviction_trigger', 'evaluate_byzantine_eviction_trigger')

    # Fix Topological_ -> topological_ (method/field/constant names)
    content = content.replace('default_Topological', 'default_topological')
    content = content.replace('inject_Topological_noise', 'inject_topological_noise')
    content = content.replace('inject_Topological_noise_internal', 'inject_topological_noise_internal')
    content = content.replace('to_Topological_tensor', 'to_topological_tensor')
    content = content.replace('from_Topological', 'from_topological')
    content = content.replace('Topological_laws_hash', 'topological_laws_hash')
    content = content.replace('Topological_laws_text', 'topological_laws_text')
    content = content.replace('Topological_LAWS_HASH', 'TOPOLOGICAL_LAWS_HASH')
    content = content.replace('Topological_LAWS_TEXT', 'TOPOLOGICAL_LAWS_TEXT')

    # Fix useless comparison: port > 65535 (u16 can't exceed 65535)
    content = content.replace('if port == 0 || port > 65535', 'if port == 0')

    if content != original:
        try:
            with open(filepath, 'w', encoding='utf-8') as fh:
                fh.write(content)
            fixes_applied += 1
            print(f"  FIXED: {filepath.replace(ROOT, '.')}")
        except Exception as e:
            print(f"  ERROR writing {filepath}: {e}")

print(f"Processing {len(rs_files)} Rust files...")
for f in sorted(rs_files):
    fix_file(f)

print(f"\nTotal files modified: {fixes_applied}")
print("Done. Run 'cargo check' to verify.")
