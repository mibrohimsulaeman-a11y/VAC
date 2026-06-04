#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: optional cargo test for vac-core TUI real-data unit tests
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [ -f "$1" ] || fail "missing required file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
SRC="vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_tui_real_data.rs"
require_file "$SRC"
require_file scripts/check-vac-init-tui-real-data-contract.sh
require_file .vac/capabilities/tui-real-data.yaml
require_file .vac/workflows/maintenance.tui-real-data.yaml
require_grep "load_capability_dashboard_data_from_workspace" "$SRC"
require_grep "load_approval_popup_data_from_store" "$SRC"
require_grep "load_autopilot_doctor_monitor_from_workspace" "$SRC"
require_grep "CapabilityDashboardData" "$SRC"
require_grep "ApprovalPopupData" "$SRC"
require_grep "AutopilotDoctorMonitorData" "$SRC"
require_grep "destructive_actions_policy_gated" "$SRC"
python3 - <<'PY'
import pathlib, yaml
cap = yaml.safe_load(pathlib.Path('.vac/capabilities/tui-real-data.yaml').read_text())
wf = yaml.safe_load(pathlib.Path('.vac/workflows/maintenance.tui-real-data.yaml').read_text())
for field in ('owner', 'ownership', 'policy', 'surfaces', 'validation'):
    assert cap.get(field), f'capability missing {field}'
for command in cap['validation']['commands'] + wf['validation']['commands']:
    assert isinstance(command, dict), 'validation command must be structured'
    for field in ('id', 'runner', 'args', 'risk', 'approval'):
        assert field in command, f'command missing {field}'
print('tui-real-data manifest wiring: PASS')
PY
if command -v cargo >/dev/null 2>&1; then
  (cd vac-rs && cargo test --offline -p vac-core vac_init_tui_real_data --lib)
  rc=$?
  [ "$rc" -eq 0 ] || exit "$rc"
  echo 'vac-init tui-real-data contract: PASS (cargo unit)'
else
  echo 'vac-init tui-real-data contract: PASS (static); cargo unit NotEvaluated (cargo not found)'
fi
