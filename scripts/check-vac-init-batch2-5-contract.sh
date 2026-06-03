#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: aggregate invokes VAC-Init child gates with standalone rustc tests
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
bash scripts/check-vac-init-manifest-structs-contract.sh
bash scripts/check-vac-init-registry-validator-contract.sh
bash scripts/check-vac-init-lifecycle-contract.sh
bash scripts/check-vac-init-ownership-scanner-contract.sh
bash scripts/check-vac-init-schema-envelope-contract.sh
python3 - <<'PY'
import pathlib, yaml
count=0
for path in sorted(pathlib.Path('.vac').rglob('*.yaml')):
    yaml.safe_load(path.read_text(encoding='utf-8'))
    count += 1
print(f'.vac YAML parse OK: {count} files')
PY
if [[ -x scripts/check-tui-source-artifact-hygiene.sh ]]; then
  bash scripts/check-tui-source-artifact-hygiene.sh
fi
printf 'vac-init batch 2-5 contract: PASS\n'
