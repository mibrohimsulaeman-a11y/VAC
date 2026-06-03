#!/usr/bin/env bash
set -euo pipefail
fail() { echo "operator draw read-only contract: $*" >&2; exit 1; }
console="vac-rs/crates/surfaces/tui/src/operator_console.rs"
ui="vac-rs/crates/surfaces/tui/src/operator_ui.rs"
[[ -f "$console" ]] || fail "operator console missing"
[[ -f "$ui" ]] || fail "operator UI missing"
grep -q 'refresh_view(false)' "$console" || fail "pre_draw_tick must use read-only refresh"
grep -q 'current_view_signature' "$console" || fail "dirty signature missing"
grep -q 'file_signature' "$console" || fail "file mtime/len signature missing"
! grep -q 'refresh_vac_init_autopilot_scheduler_status' "$console" || fail "draw path still calls scheduler executor"
! grep -q 'execute_vac_init_autopilot_action' "$console" || true
! grep -q 'refresh_vac_init_autopilot_scheduler_status' "$ui" || fail "operator_ui render model must not execute scheduler"
grep -q 'live refresh: dirty-file poll only' "$console" || fail "runtime status line must disclose read-only refresh"
echo "operator draw read-only contract: PASS"
