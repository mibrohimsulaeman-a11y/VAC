#!/usr/bin/env bash
# Snapshot-level visual contract checks for the VAC operator-console TUI.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK:   $*"; }

operator_ui="vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc"
[ -f "$operator_ui" ] || fail "missing $operator_ui"

grep -q 'struct OperatorViewport' "$operator_ui" || fail "viewport-aware operator renderer missing"
grep -q 'enum SnapshotScenario' "$operator_ui" || fail "snapshot scenarios missing"
grep -q 'render_operator_snapshot' "$operator_ui" || fail "snapshot renderer entrypoint missing"
grep -q 'render_screen' "$operator_ui" || fail "screen chrome renderer missing"
grep -q 'render_composer_lines' "$operator_ui" || fail "bottom composer renderer missing"
grep -q 'render_statusline' "$operator_ui" || fail "bottom statusline renderer missing"
grep -q 'render_columns' "$operator_ui" || fail "dashboard column renderer missing"
grep -q 'CapabilityDashboard' "$operator_ui" || fail "capability dashboard scenario missing"
grep -q 'ApprovalPopup' "$operator_ui" || fail "approval popup scenario missing"
grep -q 'RuntimeJobs' "$operator_ui" || fail "runtime jobs scenario missing"

TMP_DIR="/tmp/vac-tui-operator-snapshots-check"
rm -rf "$TMP_DIR"
bash scripts/render-tui-operator-snapshots.sh "$TMP_DIR" >/tmp/vac-tui-operator-snapshots-check.log

for file in \
  first-launch-120x36.txt \
  idle-120x36.txt \
  agent-working-120x36.txt \
  approval-popup-120x36.txt \
  runtime-jobs-120x36.txt \
  capability-dashboard-180x48.txt; do
  [ -s "$TMP_DIR/$file" ] || fail "snapshot $file was not rendered"
done

grep -q 'Vastar Agentic CLI' "$TMP_DIR/first-launch-120x36.txt" || fail "first launch snapshot missing header"
grep -q 'VAC.*ready' "$TMP_DIR/idle-120x36.txt" || fail "idle snapshot missing VAC ready state"
if grep -q 'VIL-native' "$TMP_DIR/idle-120x36.txt"; then
  fail "idle snapshot must not default to VIL-native"
fi
grep -q 'tool timeline' "$TMP_DIR/agent-working-120x36.txt" || fail "agent snapshot missing tool timeline"
grep -q 'last 5' "$TMP_DIR/agent-working-120x36.txt" || fail "agent snapshot missing last-five marker"
grep -q 'DESTRUCTIVE' "$TMP_DIR/approval-popup-120x36.txt" || fail "approval snapshot missing destructive risk label"
grep -q 'approve+remember' "$TMP_DIR/approval-popup-120x36.txt" || fail "approval snapshot missing approve+remember hotkey"
grep -q 'autopilot' "$TMP_DIR/runtime-jobs-120x36.txt" || fail "runtime jobs snapshot missing autopilot status"
grep -q 'enter attach' "$TMP_DIR/runtime-jobs-120x36.txt" || fail "runtime jobs snapshot missing attach action"
grep -q 'Diagnostics' "$TMP_DIR/capability-dashboard-180x48.txt" || fail "dashboard snapshot missing diagnostics panel"
grep -q 'no YAML/control-plane errors detected' "$TMP_DIR/capability-dashboard-180x48.txt" || fail "dashboard snapshot missing healthy diagnostics fallback"
if grep -q '.vac/capabilities/tui.yml:42:13\|almost_ready\|vac.yaml.parser' "$TMP_DIR/capability-dashboard-180x48.txt"; then
  fail "dashboard snapshot leaked stale fake YAML diagnostic"
fi
grep -q 'profile ' "$TMP_DIR/idle-120x36.txt" || fail "snapshot statusline missing profile"
grep -q 'rulebook runtime' "$TMP_DIR/idle-120x36.txt" || fail "idle snapshot statusline missing rulebook"
grep -q 'rulebook runtime' "$TMP_DIR/agent-working-120x36.txt" || fail "agent snapshot statusline missing runtime rulebook label"
grep -q 'rulebook runtime' "$TMP_DIR/runtime-jobs-120x36.txt" || fail "snapshot statusline missing runtime rulebook"

ok "operator TUI viewport snapshots match required visual contract"
