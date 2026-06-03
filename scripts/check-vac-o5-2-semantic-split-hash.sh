#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
python3 - <<'PYHASH'
from pathlib import Path
import hashlib
import re
import sys

manifests = sorted(Path('vac-rs').rglob('split_manifest.yaml'))
if not manifests:
    print('FAIL: no O5.2 split manifests found', file=sys.stderr)
    sys.exit(1)
checked = 0
for manifest in manifests:
    text = manifest.read_text(encoding='utf-8')
    if 'kind: o5_godfile_semantic_split' not in text:
        continue
    expected = None
    shard_paths = []
    shard_expected = {}
    current_path = None
    for line in text.splitlines():
        m = re.match(r'^original_full_byte_sha256:\s*([0-9a-f]{64})\s*$', line)
        if m:
            expected = m.group(1)
            continue
        m = re.match(r'^\s*- path:\s*(\S+)\s*$', line)
        if m:
            current_path = m.group(1)
            shard_paths.append(current_path)
            continue
        m = re.match(r'^\s+sha256:\s*([0-9a-f]{64})\s*$', line)
        if m and current_path:
            shard_expected[current_path] = m.group(1)
            current_path = None
    if not expected:
        print(f'FAIL: {manifest} missing original_full_byte_sha256', file=sys.stderr)
        sys.exit(1)
    if not shard_paths:
        print(f'FAIL: {manifest} has no shard paths', file=sys.stderr)
        sys.exit(1)
    blob = b''
    for raw in shard_paths:
        p = Path(raw)
        if not p.is_file():
            # Core decomposition relocated several O5.2 shard manifests under
            # crates/capabilities/<domain>/src/core_migrated while preserving
            # the original pre-relocation shard metadata for audit traceability.
            # Static verification must therefore resolve missing historical
            # paths against the manifest directory by filename instead of
            # forcing the old core/src layout back into the source tree.
            relocated = manifest.parent / Path(raw).name
            if relocated.is_file():
                p = relocated
            else:
                print(f'FAIL: {manifest} shard missing: {p}', file=sys.stderr)
                sys.exit(1)
        data = p.read_bytes()
        actual_shard = hashlib.sha256(data).hexdigest()
        expected_shard = shard_expected.get(raw)
        if expected_shard and actual_shard != expected_shard:
            print(f'FAIL: {manifest} shard sha mismatch: {raw} expected={expected_shard} actual={actual_shard}', file=sys.stderr)
            sys.exit(1)
        blob += data
    actual = hashlib.sha256(blob).hexdigest()
    if actual != expected:
        print(f'FAIL: {manifest} reconstructed sha mismatch expected={expected} actual={actual}', file=sys.stderr)
        sys.exit(1)
    checked += 1
    print(f'OK: {manifest} reconstructed={actual} shards={len(shard_paths)}')
if checked != 7:
    print(f'FAIL: expected 7 semantic split manifests, checked={checked}', file=sys.stderr)
    sys.exit(1)
print(f'O5.2 semantic split hash verification: PASS ({checked}/7 manifests)')
PYHASH
