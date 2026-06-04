#!/usr/bin/env bash
# Static gate for the live ratatui adapter that connects the snapshot render model to the TUI runtime.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

grep -q "mod operator_style;" vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs
grep -q "mod operator_ui_styles;" vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs
grep -q "style_operator_lines_from_specs" vac-rs/crates/surfaces/tui/src/operator_ui_styles.rs
grep -q "classify_operator_line" vac-rs/crates/surfaces/tui/src/operator_ui_styles.rs
grep -q "crate::operator_ui::render_first_launch_visual_lines" vac-rs/crates/surfaces/tui/src/history_cell/split_003_approvaldecisionactor.rs
grep -q "crate::operator_ui::render_idle_visual_lines" vac-rs/crates/surfaces/tui/src/history_cell/split_003_approvaldecisionactor.rs
grep -q "crate::operator_ui::render_autopilot_scheduler_visual_lines" vac-rs/crates/surfaces/tui/src/operator_console.rs
grep -q "crate::operator_ui::render_capability_dashboard_visual_lines" vac-rs/crates/surfaces/tui/src/capability_dashboard.rs
grep -q "new_capability_dashboard_output" vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_015_impl.rs
grep -q "DESTRUCTIVE" vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc
grep -q "approval required" vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc
grep -q "runtime jobs" vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc
grep -q "capability dashboard" vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc
grep -q "OperatorLineSpec" vac-rs/crates/surfaces/tui/src/operator_style.rs
grep -q "OperatorSpanSpec" vac-rs/crates/surfaces/tui/src/operator_style.rs
grep -q "render_autopilot_scheduler_line_specs" vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc
grep -q "render_capability_dashboard_shell_specs" vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc

printf 'operator live adapter contract ok\n'
