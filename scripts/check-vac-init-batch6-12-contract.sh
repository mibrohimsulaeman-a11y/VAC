#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: aggregate invokes VAC-Init child gates with standalone rustc tests
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

bash scripts/check-vac-init-risk-policy-contract.sh
bash scripts/check-vac-init-policy-evaluator-contract.sh
bash scripts/check-vac-init-command-gate-contract.sh
bash scripts/check-vac-init-semantic-plan-contract.sh
bash scripts/check-vac-init-approval-binding-contract.sh
bash scripts/check-vac-init-patch-guard-contract.sh
bash scripts/check-vac-init-evidence-chain-contract.sh

python3 - <<'PYAML'
import pathlib, yaml
count = 0
for path in sorted(pathlib.Path('.vac').rglob('*.yaml')):
    yaml.safe_load(path.read_text(encoding='utf-8'))
    count += 1
print(f'.vac YAML parse OK: {count} files')
PYAML

if [[ -x scripts/check-vac-init-batch2-5-contract.sh ]]; then
  bash scripts/check-vac-init-batch2-5-contract.sh
fi
if [[ -x scripts/check-tui-source-artifact-hygiene.sh ]]; then
  bash scripts/check-tui-source-artifact-hygiene.sh
fi
printf 'vac-init batch 6-12 aggregate contract: PASS
'
