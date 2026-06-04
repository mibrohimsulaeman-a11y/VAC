#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/scanner-policy-hardening.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_scanner_policy_hardening.rs
require_file scripts/check-vac-init-scanner-policy-hardening-contract.sh
require_file .vac/capabilities/scanner-policy-hardening.yaml
require_file .vac/workflows/maintenance.scanner-policy-hardening.yaml

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_scanner_policy_hardening.rs -o "$TMPROOT/vac_init_scanner_policy_hardening_test"
"$TMPROOT/vac_init_scanner_policy_hardening_test" --nocapture

PY_STDERR="$TMPROOT/python-stderr.log"
if ! python3 - <<'PY' 2>"$PY_STDERR"
import pathlib
import yaml

cap = yaml.safe_load(pathlib.Path('.vac/capabilities/scanner-policy-hardening.yaml').read_text())
wf = yaml.safe_load(pathlib.Path('.vac/workflows/maintenance.scanner-policy-hardening.yaml').read_text())
for field in ('owner', 'ownership', 'policy', 'surfaces', 'validation'):
    assert cap.get(field), f'capability missing {field}'
for command in cap['validation']['commands'] + wf['validation']['commands']:
    assert isinstance(command, dict), 'validation command must be structured'
    for field in ('id', 'runner', 'args', 'risk', 'approval'):
        assert field in command, f'command missing {field}'
print('scanner-policy-hardening manifest wiring: PASS')
PY
then
  cat "$PY_STDERR" >&2
  exit 1
fi

printf 'scanner-policy-hardening contract: PASS\n'
