#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
import pathlib

symbols = {
    'evaluate_vac_init_pre_plan_gate': 'pre-plan',
    'evaluate_vac_init_pre_patch_gate': 'pre-patch',
    'evaluate_vac_init_pre_command_gate': 'pre-command',
    'evaluate_vac_init_evidence_completion_gate': 'evidence-completion',
}
exclude_parts = {
    'control_plane/vac_init_runtime_enforcement.rs',
    'control_plane/mod.rs',
}
roots = [pathlib.Path('vac-rs/core/src'), pathlib.Path('vac-rs/exec/src'), pathlib.Path('vac-rs/exec-server/src')]
hits = {symbol: [] for symbol in symbols}
for root in roots:
    if not root.exists():
        continue
    for path in root.rglob('*.rs'):
        text_path = path.as_posix()
        if any(part in text_path for part in exclude_parts):
            continue
        if '/tests/' in text_path or text_path.endswith('_tests.rs'):
            continue
        text = path.read_text(errors='ignore')
        for symbol in symbols:
            if symbol in text:
                hits[symbol].append(text_path)

missing = [f'{symbols[symbol]} ({symbol})' for symbol, paths in hits.items() if not paths]
if missing:
    print('runtime gate callsite integration: FAIL')
    for item in missing:
        print('ERROR: missing production callsite for', item)
    print('hits:', hits)
    raise SystemExit(1)

print('runtime gate callsite integration: PASS')
for symbol, paths in hits.items():
    print(f'  {symbol}: {", ".join(paths)}')
PY
