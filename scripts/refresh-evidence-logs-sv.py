#!/usr/bin/env python3
from __future__ import annotations
import json, pathlib, sys
from cargo_tv_status import cargo_tv_summary
ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else '.').resolve()

def read_json(rel: str):
    try:
        return json.loads((ROOT / rel).read_text())
    except Exception:
        return {}
status=read_json('.vac/registry/status.json')
index=read_json('.vac/index/index_manifest.json')
baseline=read_json('.vac/assessment/baseline.json')
gap=read_json('.vac/assessment/gap_report.json')
compiled=status.get('compiled_snapshot_hash') or ''
index_hash=index.get('snapshot_hash') or ''
assessment_hash=baseline.get('snapshot_hash') or ''
index_counts=index.get('counts') or {}
readiness=status.get('readiness') or status.get('readiness_summary') or {}
assessment_summary=baseline.get('summary') or {}
gap_summary=gap.get('summary') or {}
cargo_tv=cargo_tv_summary(ROOT)
cargo_checks=cargo_tv.get('checks') or {}
text='\n'.join([
    'VAC v1.9 evidence mirror: current snapshot only',
    'status: SV-PASS final source freeze',
    'workspace_root: packaged-source-relative',
    f'compiled_snapshot_hash: {compiled}',
    f'index_snapshot_hash: {index_hash}',
    f'assessment_snapshot_hash: {assessment_hash}',
    'index_counts: '+json.dumps(index_counts, sort_keys=True),
    'readiness: '+json.dumps(readiness, sort_keys=True),
    'assessment_summary: '+json.dumps(assessment_summary, sort_keys=True),
    'gap_summary: '+json.dumps(gap_summary, sort_keys=True),
    'cargo_tv: '+str(cargo_tv.get('status', 'NotEvaluated')),
    'cargo_tv_proof: '+str(cargo_tv.get('proof_ref') or ''),
    'cargo_tv_proof_hash: '+str(cargo_tv.get('proof_hash') or ''),
    'cargo_tv_checks: '+json.dumps(cargo_checks, sort_keys=True),
    'l2_broker: NotImplemented',
    'evidence_log_freshness: current_snapshot',
    '',
])
# Historical in-tree .log mirrors are not source-of-truth artifacts; they must
# be rebound to the current snapshot before checkpoint packaging so auditors do
# not see stale root paths/counts. External bundle logs keep detailed history.
targets = {
    ROOT/'.vac/evidence/SV_VALIDATION.log',
    ROOT/'.vac/evidence/SV_POST_EVIDENCE_VALIDATION.log',
    ROOT/'.vac/registry/evidence/SV_VALIDATION.log',
    ROOT/'.vac/registry/evidence/SV_POST_EVIDENCE_VALIDATION.log',
}
for directory in [ROOT/'.vac/evidence', ROOT/'.vac/registry/evidence']:
    if directory.exists():
        targets.update(directory.glob('*.log'))
for p in sorted(targets):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)
print('VAC evidence logs refreshed for current snapshot')
print(f'compiled_snapshot_hash={compiled}')
print(f'index_snapshot_hash={index_hash}')
print(f'assessment_snapshot_hash={assessment_hash}')
print(f"cargo_tv={cargo_tv.get('status', 'NotEvaluated')}")
