#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/durable-stores.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_durable_stores.rs
require_file scripts/check-vac-init-durable-stores-contract.sh
require_file .vac/capabilities/durable-stores.yaml
require_file .vac/workflows/maintenance.durable-stores.yaml

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_durable_stores.rs -o "$TMPROOT/vac_init_durable_stores_test"
"$TMPROOT/vac_init_durable_stores_test" --nocapture

PY_STDERR="$TMPROOT/python-stderr.log"
if ! python3 - <<'PY' 2>"$PY_STDERR"
import pathlib
import yaml

cap = yaml.safe_load(pathlib.Path('.vac/capabilities/durable-stores.yaml').read_text())
wf = yaml.safe_load(pathlib.Path('.vac/workflows/maintenance.durable-stores.yaml').read_text())
for field in ('owner', 'ownership', 'policy', 'surfaces', 'validation'):
    assert cap.get(field), f'capability missing {field}'
for command in cap['validation']['commands'] + wf['validation']['commands']:
    assert isinstance(command, dict), 'validation command must be structured'
    for field in ('id', 'runner', 'args', 'risk', 'approval'):
        assert field in command, f'command missing {field}'
print('durable-stores manifest wiring: PASS')
PY
then
  cat "$PY_STDERR" >&2
  exit 1
fi


for required in \
  .vac/.init/source_inventory.yaml \
  .vac/.init/risk_findings.yaml \
  .vac/registry/ownership/report.yaml \
  .vac/registry/approvals/approval.production-rc-example.yaml \
  .vac/registry/memory/semantic/mem.semantic.production-rc.yaml \
  .vac/registry/trajectory/index.yaml; do
  require_file "$required"
done

printf 'durable-stores contract: PASS\n'
