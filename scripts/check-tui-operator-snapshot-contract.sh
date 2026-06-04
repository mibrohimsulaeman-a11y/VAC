#!/usr/bin/env bash
# Validate operator-console snapshot contracts. When cargo is available this
# also executes the ratatui-backed widget snapshot test; source-only fallback is
# retained for environments that cannot build the renderer.
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

TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-operator-snapshots.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
SNAPSHOT_DIR="$TMPROOT/tui-operator-ui-snapshots"
operator="vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc"
widget="vac-rs/crates/surfaces/tui/src/operator_widget_render.rs"
test_file="vac-rs/crates/surfaces/tui/tests/operator_ui_snapshot_contract.rs"
render_script="scripts/render-tui-operator-snapshots.sh"

require_file "$operator"
require_file "$widget"
require_file "$test_file"
require_file "$render_script"
bash "$render_script" "$SNAPSHOT_DIR" >/dev/null
[ -d "$SNAPSHOT_DIR" ] || fail "missing generated snapshot dir $SNAPSHOT_DIR"

require_contains "$operator" "crate::operator_widget_render::render_snapshot_lines"
require_contains "$widget" "IdleViewState::live"
require_contains "$widget" "Buffer::empty"
require_contains "$widget" "cell.style()"
require_contains "$widget" "current_text.push_str"
require_contains "$widget" "buffer_conversion_preserves_cell_style_and_gauge_fill"
require_contains "$widget" "BorderType::Rounded"
require_contains "$widget" "Tabs::new"
require_contains "$widget" "Gauge::default"
require_contains "$widget" "Table::new"
require_contains "$widget" "Layout::default"
require_contains "$test_file" "operator_widget_render"
require_contains "$test_file" "ratatui panel geometry"
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
done

for snapshot in \
  agent-working-120x36.txt agent-working-140x40.txt agent-working-180x48.txt \
  approval-popup-120x36.txt approval-popup-140x40.txt approval-popup-180x48.txt \
  runtime-jobs-120x36.txt runtime-jobs-140x40.txt runtime-jobs-180x48.txt \
  capability-dashboard-120x36.txt capability-dashboard-140x40.txt capability-dashboard-180x48.txt; do
  require_contains "$SNAPSHOT_DIR/$snapshot" "╭"
done

require_contains "$SNAPSHOT_DIR/first-launch-120x36.txt" "Vastar Agentic CLI"
require_contains "$SNAPSHOT_DIR/first-launch-120x36.txt" "hydrating startup snapshot"
require_contains "$SNAPSHOT_DIR/first-launch-120x36.txt" "VAC operator console"
require_contains "$SNAPSHOT_DIR/idle-120x36.txt" "no persisted recent task loaded"
require_contains "$SNAPSHOT_DIR/idle-120x36.txt" "VAC"
require_contains "$SNAPSHOT_DIR/agent-working-120x36.txt" "tool timeline"
require_contains "$SNAPSHOT_DIR/agent-working-120x36.txt" "context"
require_contains "$SNAPSHOT_DIR/approval-popup-120x36.txt" "DESTRUCTIVE"
require_contains "$SNAPSHOT_DIR/approval-popup-120x36.txt" "approve+remember"
require_contains "$SNAPSHOT_DIR/runtime-jobs-120x36.txt" "autopilot ● running"
require_contains "$SNAPSHOT_DIR/runtime-jobs-120x36.txt" "node progress"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "TUI Capability Dashboard"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "Diagnostics"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "no YAML/control-plane errors detected"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "VALIDATION / DOCS"
require_contains "$SNAPSHOT_DIR/capability-dashboard-180x48.txt" "Policy gates"

reject_tree_contains "$SNAPSHOT_DIR" "gpt-4o"
reject_tree_contains "$SNAPSHOT_DIR" "327 files indexed"
reject_tree_contains "$SNAPSHOT_DIR" "VAC 0.4.3 available"
reject_tree_contains "$SNAPSHOT_DIR" "metrics  ["
reject_tree_contains "$SNAPSHOT_DIR" "layout  left:"
reject_tree_contains "$SNAPSHOT_DIR" "right / Diagnostics"
reject_tree_contains "$SNAPSHOT_DIR" "Diagnostics fallback"
reject_tree_contains "$SNAPSHOT_DIR" ".vac/capabilities/tui.yml:42:13"
reject_tree_contains "$SNAPSHOT_DIR" "almost_ready"
reject_tree_contains "$SNAPSHOT_DIR" "vac.yaml.parser"
reject_tree_contains "$SNAPSHOT_DIR" "VIL-native"
reject_tree_contains "$SNAPSHOT_DIR" "composer"

if command -v cargo >/dev/null 2>&1; then
  cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui --test operator_ui_snapshot_contract -- --nocapture
  echo "cargo-backed widget snapshot test: PASS"
elif command -v rustc >/dev/null 2>&1; then
  echo "cargo-backed widget snapshot test: TV-Pending (rustc exists, but direct rustc harness is intentionally disabled because renderer depends on ratatui)"
else
  echo "cargo-backed widget snapshot test: NotEvaluated (cargo/rustc not found)"
fi

ok "operator TUI widget snapshot source/static contract validated"
