#!/usr/bin/env bash
# Static gate for the live ratatui adapter that connects the snapshot render model to the TUI runtime.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

grep -q "mod operator_style;" vac-rs/tui/src/lib.rs
grep -q "mod operator_ui_styles;" vac-rs/tui/src/lib.rs
grep -q "style_operator_lines_from_specs" vac-rs/tui/src/operator_ui_styles.rs
grep -q "classify_operator_line" vac-rs/tui/src/operator_ui_styles.rs
grep -q "crate::operator_ui_styles::style_operator_lines_from_specs" vac-rs/tui/src/capability_dashboard.rs
grep -q "crate::operator_ui_styles::style_operator_lines_from_specs" vac-rs/tui/src/chatwidget.rs
grep -q "DESTRUCTIVE" vac-rs/tui/src/operator_ui.rs
grep -q "approval required" vac-rs/tui/src/operator_ui.rs
grep -q "runtime jobs" vac-rs/tui/src/operator_ui.rs
grep -q "capability dashboard" vac-rs/tui/src/operator_ui.rs
grep -q "OperatorLineSpec" vac-rs/tui/src/operator_style.rs
grep -q "OperatorSpanSpec" vac-rs/tui/src/operator_style.rs
grep -q "render_autopilot_scheduler_line_specs" vac-rs/tui/src/operator_ui.rs
grep -q "render_capability_dashboard_shell_specs" vac-rs/tui/src/operator_ui.rs

printf 'operator live adapter contract ok\n'
