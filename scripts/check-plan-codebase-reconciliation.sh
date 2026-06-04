#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_file docs/workflow-control-plane/INDEX.md
require_file docs/workflow-control-plane/IMPLEMENTATION_PLAN.md
require_file docs/workflow-control-plane/plans/INDEX.md
require_file .vac/registry/plan-state.yaml

python3 - <<'PY'
from pathlib import Path
import yaml

state = yaml.safe_load(Path('.vac/registry/plan-state.yaml').read_text())
assert state['schema_version'] == 1
assert state['kind'] == 'registry_status'
assert state['id'] == 'vac.plan_state'
workflow = state.get('workflow_control_plane_plans') or []
donor = state.get('donor_domain_plans') or []
assert len(workflow) >= 43, len(workflow)
assert len(donor) == 11, len(donor)

missing = []
for row in workflow + donor:
    if not row.get('status'):
        missing.append(f"missing status for archived plan row: {row.get('plan_id')}")
    if 'file' not in row:
        missing.append(f"archived plan row missing file for {row.get('plan_id')}")

plan_index = Path('docs/workflow-control-plane/plans/INDEX.md').read_text(errors='ignore')
if 'Retired generated projections' not in plan_index:
    missing.append('plans index must record that detailed historical plans were pruned')

if missing:
    for item in missing:
        print('FAIL:', item)
    raise SystemExit(1)
print(f'plan-codebase reconciliation: PASS workflow={len(workflow)} donor={len(donor)}')
PY
