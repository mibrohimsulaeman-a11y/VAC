#!/usr/bin/env python3
"""Validate evidence-log mirrors are bound to the current VAC snapshot.

Historical logs with old extracted roots/counts are useful in external artifact
bundles, but not inside the current source checkpoint. Inside `.vac/evidence` and
`.vac/registry/evidence`, log files must be regenerated against the current
compiled/index/assessment snapshot or removed.
"""
from __future__ import annotations
import json, pathlib, re, sys
from typing import Any

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv)>1 else '.').resolve()
errors: list[str] = []

def read_json(rel: str) -> Any:
    try:
        return json.loads((ROOT/rel).read_text())
    except Exception:
        return {}

status = read_json('.vac/registry/status.json')
index = read_json('.vac/index/index_manifest.json')
assessment = read_json('.vac/assessment/baseline.json')
current_markers = [
    str(status.get('compiled_snapshot_hash') or ''),
    str(index.get('snapshot_hash') or ''),
    str(assessment.get('snapshot_hash') or ''),
]
current_markers = [m for m in current_markers if m]
for directory in [ROOT/'.vac/evidence', ROOT/'.vac/registry/evidence']:
    if not directory.exists():
        continue
    for path in sorted(directory.glob('*.log')):
        text = path.read_text(errors='replace')
        rel = path.relative_to(ROOT).as_posix()
        if not any(marker in text for marker in current_markers):
            errors.append(f'{rel}: missing current compiled/index/assessment snapshot marker')
        stale_patterns = [
            r'/mnt/data/vac_(?:state|audit|wt|v1).*',
            r'old extracted root path',
            r'files[=:] ?686',
            r'old readiness ready=1',
            r'ready=1 but current ready=0',
        ]
        for pattern in stale_patterns:
            if re.search(pattern, text):
                errors.append(f'{rel}: stale evidence marker matched {pattern!r}')
                break

if errors:
    print('VAC evidence log freshness: FAIL')
    for err in errors:
        print(f'- {err}')
    sys.exit(1)
print('VAC evidence log freshness: PASS')
print('evidence_logs_bound_to_current_snapshot=true')
