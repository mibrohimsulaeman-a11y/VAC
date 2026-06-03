#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-runtime-gates.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_grep() {
  local pattern="$1"
  local path="$2"
  if ! grep -qE "$pattern" "$path"; then
    echo "FAIL: missing pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_file vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs
require_file vac-rs/control-plane/src/control_plane/vac_init_semantic_plan.rs
require_file vac-rs/control-plane/src/control_plane/vac_init_patch_guard.rs
require_file vac-rs/control-plane/src/control_plane/vac_init_command_gate.rs
require_file vac-rs/control-plane/src/control_plane/vac_init_evidence_chain.rs
require_file .vac/capabilities/vac-init-runtime-gate-enforcement.yaml
require_file .vac/workflows/maintenance.vac-init-runtime-gate-enforcement.yaml
require_file docs/vac-init/VAC_INIT_PRODUCTION_HARDENING_C_RUNTIME_GATES.md
require_file docs/validation/PRODUCTION_HARDENING_C1_C4_VALIDATION.md

require_grep 'pub enum RuntimeGateKind' vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs
require_grep 'PrePlanGateContext' vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs
require_grep 'PrePatchGateContext' vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs
require_grep 'PreCommandGateContext' vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs
require_grep 'EvidenceCompletionGateContext' vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs
require_grep 'RuntimeEnforcementCoverage' vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs
require_grep 'evaluate_pre_plan_gate' vac-rs/control-plane/src/control_plane/mod.rs
require_grep 'evaluate_pre_patch_gate' vac-rs/control-plane/src/control_plane/mod.rs
require_grep 'evaluate_pre_command_gate' vac-rs/control-plane/src/control_plane/mod.rs
require_grep 'evaluate_evidence_completion_gate' vac-rs/control-plane/src/control_plane/mod.rs

"$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_runtime_enforcement.rs -o "$TMPROOT/vac_init_runtime_enforcement_test"
"$TMPROOT/vac_init_runtime_enforcement_test" --nocapture

PY_STDERR="$TMPROOT/python-stderr.log"
if ! python3 - <<'PY' 2>"$PY_STDERR"
import pathlib
import yaml

capability_path = pathlib.Path('.vac/capabilities/vac-init-runtime-gate-enforcement.yaml')
workflow_path = pathlib.Path('.vac/workflows/maintenance.vac-init-runtime-gate-enforcement.yaml')
surface_path = pathlib.Path('.vac/surfaces/cli.yaml')

capability = yaml.safe_load(capability_path.read_text())
workflow = yaml.safe_load(workflow_path.read_text())
surface = yaml.safe_load(surface_path.read_text())

assert capability['kind'] == 'capability'
assert capability['status'] == 'ready'
for field in ('owner', 'ownership', 'policy', 'surfaces', 'validation', 'docs'):
    assert capability.get(field), f'capability missing {field}'

commands = capability['validation']['commands'] + workflow['validation']['commands']
for command in commands:
    assert isinstance(command, dict), 'validation command must be structured'
    for field in ('id', 'runner', 'args', 'risk', 'approval'):
        assert field in command, f'command missing {field}'

expected_steps = {
    'pre_plan_gate',
    'pre_patch_gate',
    'pre_command_gate',
    'evidence_completion_gate',
}
actual_steps = {step['id'] for step in workflow['steps']}
missing = expected_steps - actual_steps
assert not missing, f'missing workflow steps: {sorted(missing)}'

assert 'vac.init.runtime-gate-enforcement' in surface['capabilities']
print('vac-init runtime gate manifest wiring: PASS')
PY
then
  cat "$PY_STDERR" >&2
  exit 1
fi

printf 'vac-init runtime gate enforcement contract: PASS\n'
