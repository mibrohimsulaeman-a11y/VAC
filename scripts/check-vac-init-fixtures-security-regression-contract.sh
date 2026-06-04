#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/fixtures-security-regression.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_fixture_security_regression.rs
require_file scripts/check-vac-init-fixtures-security-regression-contract.sh
require_file .vac/capabilities/fixtures-security-regression.yaml
require_file .vac/workflows/maintenance.fixtures-security-regression.yaml

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_fixture_security_regression.rs -o "$TMPROOT/vac_init_fixture_security_regression_test"
"$TMPROOT/vac_init_fixture_security_regression_test" --nocapture

PY_STDERR="$TMPROOT/python-stderr.log"
if ! python3 - <<'PY' 2>"$PY_STDERR"
import pathlib
import yaml

cap = yaml.safe_load(pathlib.Path('.vac/capabilities/fixtures-security-regression.yaml').read_text())
wf = yaml.safe_load(pathlib.Path('.vac/workflows/maintenance.fixtures-security-regression.yaml').read_text())
for field in ('owner', 'ownership', 'policy', 'surfaces', 'validation'):
    assert cap.get(field), f'capability missing {field}'
for command in cap['validation']['commands'] + wf['validation']['commands']:
    assert isinstance(command, dict), 'validation command must be structured'
    for field in ('id', 'runner', 'args', 'risk', 'approval'):
        assert field in command, f'command missing {field}'
print('fixtures-security-regression manifest wiring: PASS')
PY
then
  cat "$PY_STDERR" >&2
  exit 1
fi


for dir in \
  tests/fixtures/schema \
  tests/fixtures/state_machine \
  tests/fixtures/policy \
  tests/fixtures/workspace \
  tests/fixtures/evidence \
  tests/fixtures/plans \
  tests/fixtures/patches \
  tests/fixtures/doctor \
  tests/fixtures/memory \
  tests/fixtures/trajectory \
  tests/fixtures/security_regression; do
  if [[ ! -d "$dir" ]]; then
    echo "FAIL: missing fixture dir $dir" >&2
    exit 1
  fi
done

for attack in \
  shell_injection path_traversal absolute_path_write free_form_command \
  approval_replay expired_approval changed_policy_hash broken_evidence_chain \
  secret_memory_content unowned_file_patch undeclared_new_file planned_capability_execution; do
  require_file "tests/fixtures/security_regression/${attack}.yaml"
done

printf 'fixtures-security-regression contract: PASS\n'
