#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }

require_file scripts/measure-vac-o6-1-runtime-panic-surface.py
require_file .vac/registry/o6-panic-surface.yaml
require_file .vac/registry/o6-quality-triage.yaml

if [[ "${VAC_O6_1_RESCAN:-1}" == "0" ]]; then
  require_grep 'claim_scope: static_tokenizer_measurement_not_refactor' .vac/registry/o6-panic-surface.yaml
  require_grep 'claim_scope: static_tokenizer_measurement_not_refactor' .vac/registry/o6-quality-triage.yaml
  require_grep 'runtime_files_scanned: 1247' .vac/registry/o6-panic-surface.yaml
  require_grep 'cfg_test_lines_removed: 133182' .vac/registry/o6-panic-surface.yaml
  require_grep 'total_runtime_panic_surface: 252' .vac/registry/o6-panic-surface.yaml
  printf 'O6.1 de-panic surface: SKIP_RESCAN aggregate mode; dedicated scanner gate must run separately\n'
  exit 0
fi

python3 - <<'PYO61'
from pathlib import Path
import json
import subprocess
import sys
import yaml

scan = subprocess.check_output(['python3', 'scripts/measure-vac-o6-1-runtime-panic-surface.py', '--json'], text=True)
result = json.loads(scan)
registry = yaml.safe_load(Path('.vac/registry/o6-panic-surface.yaml').read_text())
triage = yaml.safe_load(Path('.vac/registry/o6-quality-triage.yaml').read_text())
counts = result['pattern_counts']
expected = {
    'runtime_files_scanned': result['runtime_files_scanned'],
    'cfg_test_lines_removed': result['cfg_test_lines_removed'],
    'total_runtime_panic_surface': result['total_runtime_panic_surface'],
    'unwrap': counts['.unwrap()'],
    'expect': counts['.expect('],
    'panic_macro': counts['panic!'],
    'todo_macro': counts['todo!'],
    'unimplemented_macro': counts['unimplemented!'],
    'unreachable_macro': counts['unreachable!'],
}
for key in ['runtime_files_scanned', 'cfg_test_lines_removed', 'total_runtime_panic_surface']:
    if int(registry[key]) != int(expected[key]):
        print(f'FAIL: registry {key}={registry[key]} scanner={expected[key]}', file=sys.stderr)
        sys.exit(1)
registry_counts = registry['counts']
for key in ['unwrap','expect','panic_macro','todo_macro','unimplemented_macro','unreachable_macro']:
    if int(registry_counts[key]) != int(expected[key]):
        print(f'FAIL: registry counts.{key}={registry_counts[key]} scanner={expected[key]}', file=sys.stderr)
        sys.exit(1)
triage_depanic = triage['depanic']
triage_map = {
    'unwrap_runtime': expected['unwrap'],
    'expect_runtime': expected['expect'],
    'panic_runtime': expected['panic_macro'],
    'todo_runtime': expected['todo_macro'],
    'unimplemented_runtime': expected['unimplemented_macro'],
    'unreachable_runtime': expected['unreachable_macro'],
    'total_runtime_panic_surface': expected['total_runtime_panic_surface'],
}
for key, val in triage_map.items():
    if int(triage_depanic[key]) != int(val):
        print(f'FAIL: triage depanic.{key}={triage_depanic[key]} scanner={val}', file=sys.stderr)
        sys.exit(1)
if registry.get('claim_scope') != 'static_tokenizer_measurement_not_refactor':
    print('FAIL: registry must mark claim_scope static_tokenizer_measurement_not_refactor', file=sys.stderr)
    sys.exit(1)
if triage_depanic.get('claim_scope') != 'static_tokenizer_measurement_not_refactor':
    print('FAIL: triage must mark claim_scope static_tokenizer_measurement_not_refactor', file=sys.stderr)
    sys.exit(1)
print('O6.1 panic surface reproducibility: PASS total={} files={} cfg_test_lines_removed={}'.format(
    expected['total_runtime_panic_surface'], expected['runtime_files_scanned'], expected['cfg_test_lines_removed']
))
PYO61

require_grep 'previous_grep_upper_bound_retired: true' .vac/registry/o6-quality-triage.yaml
require_grep 'cargo_clippy_status: NotEvaluated' .vac/registry/o6-quality-triage.yaml
require_grep 'method: runtime paths minus tests/cfg\(test\), comments, strings, and char/raw literals' .vac/registry/o6-quality-triage.yaml
printf 'O6.1 de-panic surface: PASS scanner output reproduced against registry; refactor remains TV-Pending\n'
