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

require_file docs/PLAN_CODEBASE_RECONCILIATION.md
require_file docs/PROJECT_STATE_CURRENT.md
require_file docs/DOCS_INDEX_CURRENT.md
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
    file = Path(row['file'])
    if not file.exists():
        missing.append(f'missing plan file: {file}')
        continue
    text = file.read_text(errors='ignore')
    if '<!-- VAC-PLAN-STATE:BEGIN -->' not in text or '<!-- VAC-PLAN-STATE:END -->' not in text:
        missing.append(f'missing reconciliation block: {file}')
    if not row.get('status'):
        missing.append(f'missing status for {file}')
    for evidence in row.get('code_evidence') or []:
        if not Path(evidence).exists():
            missing.append(f'missing code evidence for {file}: {evidence}')

report = Path('docs/PLAN_CODEBASE_RECONCILIATION.md').read_text(errors='ignore')
for needle in ['Workflow control-plane plans', 'Donor domain plans', 'CONTRACT_IMPLEMENTED_DOC_ONLY_OVERLAY']:
    if needle not in report:
        missing.append(f'report missing {needle}')

project_state = Path('docs/PROJECT_STATE_CURRENT.md').read_text(errors='ignore')
if 'Plan-codebase reconciliation — 2026-05-30' not in project_state:
    missing.append('PROJECT_STATE_CURRENT missing reconciliation section')

if missing:
    for item in missing:
        print('FAIL:', item)
    raise SystemExit(1)
print(f'plan-codebase reconciliation: PASS workflow={len(workflow)} donor={len(donor)}')
PY
