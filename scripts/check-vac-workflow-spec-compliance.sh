#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
import pathlib
import yaml

errors = []
for path in sorted(pathlib.Path('.vac/workflows').glob('*.yaml')):
    data = yaml.safe_load(path.read_text()) or {}
    if data.get('kind') != 'workflow':
        continue
    if not data.get('validation'):
        errors.append(f'{path}: missing top-level validation')
    if not data.get('ui'):
        errors.append(f'{path}: missing top-level ui')
    for index, step in enumerate(data.get('steps') or []):
        if not isinstance(step, dict):
            errors.append(f'{path}: steps[{index}] is not a mapping')
            continue
        if 'uses' not in step:
            errors.append(f'{path}: steps[{index}] missing uses')
        if 'command' in step:
            errors.append(f'{path}: steps[{index}] has forbidden command field')
        if step.get('kind') == 'shell':
            errors.append(f'{path}: steps[{index}] uses legacy kind=shell')
        if not step.get('ui'):
            errors.append(f'{path}: steps[{index}] missing ui block')

if errors:
    print('workflow spec compliance: FAIL')
    for error in errors:
        print('ERROR:', error)
    raise SystemExit(1)

print('workflow spec compliance: PASS')
PY
