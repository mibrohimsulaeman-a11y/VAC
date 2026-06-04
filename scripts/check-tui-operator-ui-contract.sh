#!/usr/bin/env bash
# Static contract checks for the VAC operator-console TUI batch.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK:   $*"; }

operator_ui="vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc"
status_card="vac-rs/crates/surfaces/tui/src/status/card.rs"
approval_overlay="vac-rs/crates/surfaces/tui/src/bottom_pane/approval_overlay.rs"
approval_overlay_impl="vac-rs/crates/surfaces/tui/src/bottom_pane/approval_overlay.rs.inc"
exec_render="vac-rs/crates/surfaces/tui/src/exec_cell/render.rs"
chatwidget_ops="vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_015_impl.rs"

[ -f "$operator_ui" ] || fail "missing $operator_ui"
[ -f "$status_card" ] || fail "missing $status_card"
[ -f "$approval_overlay" ] || fail "missing $approval_overlay"
[ -f "$approval_overlay_impl" ] || fail "missing $approval_overlay_impl"
[ -f "$exec_render" ] || fail "missing $exec_render"
[ -f "$chatwidget_ops" ] || fail "missing $chatwidget_ops"

bash scripts/check-tui-status-output-contract.sh >/dev/null
grep -q 'StatusDisplayField::TokenUsage.label()' "$status_card" || fail "/status token usage row missing"
grep -q 'StatusDisplayField::ContextWindow.label()' "$status_card" || fail "/status context window row missing"

grep -q 'TOOL_TIMELINE_LIMIT: usize = 5' "$operator_ui" || fail "tool timeline limit is not 5"
grep -q 'tool timeline limited to last 5' "$exec_render" || fail "active tool timeline does not expose last-five cap"
for state in Queued Running Streaming Passed Failed Cancelled; do
  grep -q "ToolTimelineState::$state" "$operator_ui" || fail "missing tool timeline state $state"
done

grep -q 'render_first_launch_lines' "$operator_ui" || fail "first launch renderer missing"
grep -q 'OperatorViewport' "$operator_ui" || fail "operator viewport contract missing"
grep -q 'render_operator_snapshot' "$operator_ui" || fail "operator fixed snapshot renderer missing"
grep -q 'Vastar Agentic CLI' "$operator_ui" || fail "operator header missing"
grep -q 'VAC operator console' "$operator_ui" || fail "VAC operator console first-launch tag missing"
grep -q 'local runtime connected' "$operator_ui" || fail "idle runtime status missing"
if grep -q 'VIL-native' "$operator_ui"; then
  fail "VIL-native must not be the default active TUI mode"
fi
grep -q 'shift+tab' "$operator_ui" || fail "idle keyboard hint missing"
grep -q '/runtime' "$operator_ui" || fail "runtime route hint missing"

grep -q 'render_capability_dashboard_shell' "$operator_ui" || fail "capability dashboard shell renderer missing"
grep -q 'no YAML/control-plane errors detected' "$operator_ui" || fail "dashboard diagnostics fallback missing"
grep -q 'fn capability_dashboard_right_column' "$operator_ui" || fail "dashboard diagnostics column missing"
grep -q 'render_dashboard_right' vac-rs/crates/surfaces/tui/src/operator_widget_render.rs || fail "dashboard diagnostics widget missing"

grep -q 'render_agent_streaming_lines' "$operator_ui" || fail "agent streaming renderer missing"
grep -q 'thinking' "$operator_ui" || fail "agent thinking/status line missing"
grep -q 'context_bar' "$operator_ui" || fail "context usage bar missing"

grep -q 'approval required' "$approval_overlay_impl" || fail "approval popup label missing"
grep -q 'DESTRUCTIVE' "$approval_overlay_impl" || fail "destructive bash label missing"
grep -q 'approve once' "$approval_overlay_impl" || fail "approve-once hotkey label missing"
grep -q 'approve+remember' "$approval_overlay_impl" || fail "approve+remember hotkey label missing"
grep -q 'reject with reason' "$approval_overlay_impl" || fail "reject-with-reason hotkey label missing"
grep -q 'policy-gated approval queue' "$approval_overlay_impl" || fail "policy-gated approval queue label missing"

bash scripts/check-autopilot-scheduler-contract.sh >/dev/null
bash scripts/check-tui-operator-snapshot-contract.sh >/dev/null

grep -q 'Runtime' vac-rs/crates/surfaces/tui/src/slash_command.rs || fail "/runtime slash command enum missing"
grep -q 'add_runtime_jobs_output' "$chatwidget_ops" || fail "runtime jobs output wiring missing"
grep -q 'SlashCommand::Runtime' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs || fail "/runtime slash dispatch missing"

grep -q '"cli_only" | "cli-only"' vac-rs/crates/control-plane/control-plane/src/control_plane/surface_manifest.rs || fail "surface status cli-only alias missing"

ok "operator TUI status/idle/streaming/approval/dashboard/autopilot contracts registered"
