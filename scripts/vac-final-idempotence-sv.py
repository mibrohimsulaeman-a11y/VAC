#!/usr/bin/env python3
"""Two-pass idempotence gate for VAC v1.9 packaged final validation.

It intentionally runs the mutating SV generators in the release order:
compiled registry -> deterministic index -> assessment. The second pass must
reproduce the same authority hashes/counts; otherwise a checkpoint PASS log is
not replayable from a clean extraction.
"""
from __future__ import annotations
import json, pathlib, subprocess, sys
from typing import Any

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else '.').resolve()

def run(label: str, cmd: list[str]) -> None:
    print(f'[idempotence] {label}', flush=True)
    p = subprocess.run(cmd, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=300)
    if p.returncode != 0:
        print(p.stdout)
        raise SystemExit(f"{label} failed with exit={p.returncode}")

def read_json(rel: str) -> Any:
    return json.loads((ROOT/rel).read_text())

def snapshot() -> dict[str, Any]:
    idx = read_json('.vac/index/index_manifest.json')
    ass = read_json('.vac/assessment/baseline.json')
    gap = read_json('.vac/assessment/gap_report.json')
    status = read_json('.vac/registry/status.json')
    caps = read_json('.vac/registry/compiled/capabilities/current.json')
    return {
        'compiled_snapshot_hash': status.get('compiled_snapshot_hash'),
        'capabilities_snapshot_hash': caps.get('snapshot_hash'),
        'index_counts': idx.get('counts'),
        'index_snapshot_hash': idx.get('snapshot_hash'),
        'assessment_index_snapshot_hash': ass.get('index_snapshot_hash'),
        'assessment_counts': ass.get('summary'),
        'gap_index_snapshot_hash': gap.get('index_snapshot_hash'),
        'gap_findings_total': (gap.get('summary') or {}).get('findings_total'),
        'readiness_summary': status.get('readiness_summary'),
    }

def pass_once() -> dict[str, Any]:
    run('deterministic-index', ['python3','scripts/generate-deterministic-index-sv.py','.'])
    run('compile-registry', ['python3','scripts/compile-vac-registry-sv.py','.'])
    run('assessment-report', ['python3','scripts/generate-assessment-report-sv.py','.'])
    run('assessment-freshness', ['python3','scripts/check-assessment-freshness.py','.'])
    # Deep validation is executed by the aggregate release gate before and after this script.
    # This idempotence check focuses on the mutating authority generators so it stays replayable.
    return snapshot()

first = pass_once()
second = pass_once()
if first != second:
    print('VAC final idempotence: FAIL')
    print(json.dumps({'first': first, 'second': second}, indent=2, sort_keys=True))
    sys.exit(1)
print('VAC final idempotence: PASS')
print(json.dumps(first, indent=2, sort_keys=True))
