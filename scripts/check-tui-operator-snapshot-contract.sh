#!/usr/bin/env bash
# Validate operator-console snapshot contracts without requiring a transient
# sandbox rustc. Cargo-backed widget snapshot tests remain TV-Pending when the
# toolchain is unavailable; this gate keeps source/static truth strict.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK:   $*"; }
require_file() { [ -f "$1" ] || fail "missing $1"; }
require_contains() {
  local file="$1" pattern="$2"
  grep -Fq "$pattern" "$file" || fail "missing pattern in $file: $pattern"
}
reject_contains() {
  local file="$1" pattern="$2"
  if grep -Fq "$pattern" "$file"; then
    fail "forbidden stale pattern in $file: $pattern"
  fi
}
reject_tree_contains() {
  local dir="$1" pattern="$2"
  if grep -R -F -n "$pattern" "$dir" >/tmp/vac-tui-snapshot-grep.log 2>/dev/null; then
    cat /tmp/vac-tui-snapshot-grep.log >&2
    fail "forbidden stale snapshot pattern: $pattern"
  fi
}

SNAPSHOT_DIR="docs/validation/tui-operator-ui-snapshots"
operator="vac-rs/tui/src/operator_ui.rs"
widget="vac-rs/tui/src/operator_widget_render.rs"
test_file="vac-rs/tui/tests/operator_ui_snapshot_contract.rs"
render_script="scripts/render-tui-operator-snapshots.sh"

require_file "$operator"
require_file "$widget"
require_file "$test_file"
require_file "$render_script"
[ -d "$SNAPSHOT_DIR" ] || fail "missing snapshot dir $SNAPSHOT_DIR"

require_contains "$operator" "crate::operator_widget_render::render_snapshot_lines"
require_contains "$widget" "IdleViewState::live"
require_contains "$widget" "Buffer::empty"
require_contains "$widget" "cell.style()"
require_contains "$widget" "Span::styled(current_text"
require_contains "$widget" "buffer_conversion_preserves_cell_style_and_gauge_fill"
require_contains "$widget" "BorderType::Rounded"
require_contains "$widget" "Tabs::new"
require_contains "$widget" "Gauge::default"
require_contains "$widget" "Table::new"
require_contains "$widget" "Layout::default"
require_contains "$test_file" "operator_widget_render"
require_contains "$test_file" "rounded ratatui block geometry"
require_contains "$widget" "style.bg, Some(BLUE_BG)"
require_contains "$render_script" "TV-Pending"

for snapshot in \
  first-launch-120x36.txt first-launch-140x40.txt first-launch-180x48.txt \
  idle-120x36.txt idle-140x40.txt idle-180x48.txt \
  agent-working-120x36.txt agent-working-140x40.txt agent-working-180x48.txt \
  approval-popup-120x36.txt approval-popup-140x40.txt approval-popup-180x48.txt \
  runtime-jobs-120x36.txt runtime-jobs-140x40.txt runtime-jobs-180x48.txt \
  capability-dashboard-120x36.txt capability-dashboard-140x40.txt capability-dashboard-180x48.txt; do
  [ -s "$SNAPSHOT_DIR/$snapshot" ] || fail "missing generated snapshot $snapshot"
  require_contains "$SNAPSHOT_DIR/$snapshot" "╭"
done

require_contains "$SNAPSHOT_DIR/first-launch-120x36.txt" "Vastar Agentic CLI"
require_contains "$SNAPSHOT_DIR/first-launch-120x36.txt" "hydrating startup snapshot"
require_contains "$SNAPSHOT_DIR/idle-120x36.txt" "no persisted recent task loaded"
require_contains "$SNAPSHOT_DIR/idle-120x36.txt" "composer"
require_contains "$SNAPSHOT_DIR/agent-working-120x36.txt" "tool timeline"
require_contains "$SNAPSHOT_DIR/agent-working-120x36.txt" "context"
require_contains "$SNAPSHOT_DIR/approval-popup-120x36.txt" "DESTRUCTIVE"
require_contains "$SNAPSHOT_DIR/approval-popup-120x36.txt" "approve+remember"
require_contains "$SNAPSHOT_DIR/runtime-jobs-120x36.txt" "autopilot ● running"
require_contains "$SNAPSHOT_DIR/runtime-jobs-120x36.txt" "node progress"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "TUI Capability Dashboard"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "Diagnostics"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "VALIDATION / DOCS"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "Policy gates"

reject_tree_contains "$SNAPSHOT_DIR" "gpt-4o"
reject_tree_contains "$SNAPSHOT_DIR" "327 files indexed"
reject_tree_contains "$SNAPSHOT_DIR" "VAC 0.4.3 available"
reject_tree_contains "$SNAPSHOT_DIR" "metrics  ["
reject_tree_contains "$SNAPSHOT_DIR" "layout  left:"
reject_tree_contains "$SNAPSHOT_DIR" "right / Diagnostics"
reject_tree_contains "$SNAPSHOT_DIR" "Diagnostics fallback"

if command -v cargo >/dev/null 2>&1; then
  echo "cargo-backed widget snapshot test: TV-Pending (run: cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui operator_ui_snapshot_contract)"
elif command -v rustc >/dev/null 2>&1; then
  echo "cargo-backed widget snapshot test: TV-Pending (rustc exists, but direct rustc harness is intentionally disabled because renderer depends on ratatui)"
else
  echo "cargo-backed widget snapshot test: NotEvaluated (cargo/rustc not found)"
fi

ok "operator TUI widget snapshot source/static contract validated"
