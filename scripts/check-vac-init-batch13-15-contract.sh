#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

bash scripts/check-vac-init-why-contract.sh
bash scripts/check-vac-init-memory-governance-contract.sh
bash scripts/check-vac-init-doctor-release-contract.sh

python3 - <<'PYAML'
import pathlib, yaml
count = 0
for path in sorted(pathlib.Path('.vac').rglob('*.yaml')):
    yaml.safe_load(path.read_text(encoding='utf-8'))
    count += 1
print(f'.vac YAML parse OK: {count} files')
PYAML

if [[ -f scripts/check-tui-source-artifact-hygiene.sh ]]; then
  bash scripts/check-tui-source-artifact-hygiene.sh
fi

# Earlier VAC-Init gates may require rustc. They are intentionally not invoked
# here because Batch 13-15 has its own contract scope and sandbox toolchain
# extraction can be unavailable under source-artifact mode.
printf 'vac-init batch 13-15 aggregate contract: PASS\n'
