#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-tui-real-data.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() { [[ -f "$1" ]] || { echo "FAIL: missing $1" >&2; exit 1; }; }

require_file vac-rs/control-plane/src/control_plane/vac_init_tui_real_data.rs
require_file vac-rs/tui/src/capability_dashboard.rs

"$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_tui_real_data.rs -o "$TMPROOT/vac_init_tui_real_data_test"
"$TMPROOT/vac_init_tui_real_data_test" --nocapture

grep -q 'load_control_plane_registry_report' vac-rs/tui/src/capability_dashboard.rs
grep -q 'load_ownership_scan_report' vac-rs/tui/src/capability_dashboard.rs
grep -q 'load_policy_doctor_report_for_path' vac-rs/tui/src/capability_dashboard.rs
grep -q 'load_capability_dashboard_data_from_workspace' vac-rs/control-plane/src/control_plane/vac_init_tui_real_data.rs
grep -q 'load_approval_popup_data_from_store' vac-rs/control-plane/src/control_plane/vac_init_tui_real_data.rs
grep -q 'load_autopilot_doctor_monitor_from_workspace' vac-rs/control-plane/src/control_plane/vac_init_tui_real_data.rs

printf 'vac TUI real-data adapters gate: PASS\n'
