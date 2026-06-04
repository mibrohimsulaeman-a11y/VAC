#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "technical debt marker static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
require_file .vac/registry/technical-debt-markers.yaml
require_grep '^kind: registry_status$' .vac/registry/technical-debt-markers.yaml
require_grep '^marker_inventory_sha256:' .vac/registry/technical-debt-markers.yaml
python3 - <<'PY'
from pathlib import Path
import hashlib
import re
registry = Path('.vac/registry/technical-debt-markers.yaml').read_text()
expected_count = re.search(r'^marker_count: (\d+)$', registry, re.M)
expected_hash = re.search(r'^marker_inventory_sha256: ([0-9a-f]{64})$', registry, re.M)
if not expected_count or not expected_hash:
    raise SystemExit('technical debt registry missing count/hash')
pat=re.compile(r'\b(TODO|FIXME|HACK|XXX)\b')
entries=[]
for root in [Path('vac-rs/core/src'), Path('vac-rs/crates/surfaces/tui/src')]:
    for p in sorted(root.rglob('*.rs')):
        rel=p.as_posix()
        if '/tests/' in rel or rel.endswith('_tests.rs'):
            continue
        for i,line in enumerate(p.read_text(errors='ignore').splitlines(),1):
            m=pat.search(line)
            if m:
                digest=hashlib.sha256(line.strip().encode()).hexdigest()[:16]
                entries.append((rel,i,m.group(1),digest))
canon='\n'.join(f'{p}:{ln}:{m}:{d}' for p,ln,m,d in entries)
actual_hash=hashlib.sha256(canon.encode()).hexdigest()
if int(expected_count.group(1)) != len(entries):
    raise SystemExit(f'technical debt marker count drift: expected {expected_count.group(1)} actual {len(entries)}')
if expected_hash.group(1) != actual_hash:
    raise SystemExit(f'technical debt marker hash drift: expected {expected_hash.group(1)} actual {actual_hash}')
print(f'technical debt marker inventory: PASS count={len(entries)} hash={actual_hash}')
PY
printf 'technical debt marker static gate: PASS\n'
