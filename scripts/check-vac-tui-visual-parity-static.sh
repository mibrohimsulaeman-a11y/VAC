#!/usr/bin/env bash
set -u

root="${1:-.}"
cd "$root" || exit 2
fail=0
require_contains() {
  file="$1"; pattern="$2"
  if ! grep -Fq "$pattern" "$file"; then
    echo "MISSING_PATTERN $file :: $pattern"
    fail=1
  fi
}
reject_contains() {
  file="$1"; pattern="$2"
  if grep -Fq "$pattern" "$file"; then
    echo "FORBIDDEN_PATTERN $file :: $pattern"
    fail=1
  fi
}

operator=vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc
adapter=vac-rs/crates/surfaces/tui/src/operator_ui_styles.rs
widget=vac-rs/crates/surfaces/tui/src/operator_widget_render.rs
runtime=vac-rs/crates/surfaces/tui/src/chatwidget/split_015_set_thread_rename_block_message.rs
operator_console=vac-rs/crates/surfaces/tui/src/operator_console.rs
idle=vac-rs/crates/surfaces/tui/src/history_cell/split_003_approvaldecisionactor.rs

reject_contains "$operator" "layout  left: command/nav"
reject_contains "$operator" "metrics  ["
reject_contains "$operator" "327 files indexed"
reject_contains "$operator" "gpt-4o"
reject_contains "$adapter" "without changing line count"
require_contains "$operator" "crate::operator_widget_render::render_snapshot_lines"
require_contains "$operator" "render_capability_dashboard_visual_lines"
require_contains "$operator" "render_autopilot_scheduler_visual_lines"
require_contains "$operator" "render_first_launch_visual_lines"
require_contains "$operator" "fn from_workspace"
require_contains "$operator" "fn live"
require_contains "$widget" "use ratatui::widgets::{"
require_contains "$widget" "Block"
require_contains "$widget" "Tabs"
require_contains "$widget" "Gauge"
require_contains "$widget" "Table"
require_contains "$widget" "Layout"
require_contains "$widget" "BorderType::Rounded"
require_contains "$widget" "Buffer::empty"
require_contains "$widget" "cell.style()"
require_contains "$widget" "Span::styled(current_text"
reject_contains "$widget" "lines.push(Line::from(row))"
require_contains "$widget" "render_capability_dashboard_lines"
require_contains "$widget" "render_runtime_jobs_lines"
require_contains "$widget" "render_agent_streaming_lines"
require_contains "$widget" "render_startup_lines"
require_contains "$runtime" "new_runtime_operator_console_view"
require_contains "$operator_console" "render_autopilot_scheduler_visual_lines"
require_contains "$operator_console" "execute_vac_init_autopilot_action"
reject_contains "$runtime" "monitor_only_sample"
require_contains "$idle" "IdleViewState::live"
require_contains "$idle" "render_idle_visual_lines"
require_contains "$idle" "render_first_launch_visual_lines"
reject_contains "$idle" "IdleViewState::ready"
require_contains "$adapter" "legacy callers"

if [ "$fail" -ne 0 ]; then
  echo "FAIL TUI visual parity static gate"
  exit 1
fi

echo "PASS TUI visual parity static contract: ratatui widget geometry + live runtime operator console sources"
echo "TV-PENDING: terminal screenshot/pixel gate not evaluated"
