#!/usr/bin/env bash
# Hardening 5: agent streaming surface must expose timeline, turn metadata, context, and last-five tools.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }

ui="vac-rs/tui/src/operator_ui.rs"
snapshot="docs/validation/tui-operator-ui-snapshots/agent-working-120x36.txt"
exec_render="vac-rs/tui/src/exec_cell/render.rs"

[ -f "$ui" ] || fail "missing $ui"
[ -f "$snapshot" ] || fail "missing $snapshot"

grep -q 'TOOL_TIMELINE_LIMIT: usize = 5' "$ui" || fail "tool timeline limit is not five"
grep -q 'visible_tools' "$ui" || fail "tail-limited visible tool helper missing"
grep -q 'render_agent_turn_metadata_line' "$ui" || fail "agent turn metadata renderer missing"
grep -q 'metadata context' "$ui" || fail "agent turn metadata does not include context"
grep -q 'context_bar' "$ui" || fail "context usage bar missing"
grep -q 'composer: {}' "$ui" || fail "agent composer line missing"
for state in Queued Running Streaming Passed Failed Cancelled; do
  grep -q "ToolTimelineState::$state" "$ui" || fail "missing tool state $state"
done
[ -f "$exec_render" ] && grep -q 'tool timeline limited to last 5' "$exec_render" || fail "runtime exec render last-five contract missing"

grep -q 'tool timeline (last 5;' "$snapshot" || fail "snapshot missing last-five timeline label"
grep -q 'thinking' "$snapshot" || fail "snapshot missing thinking/status line"
grep -q 'context' "$snapshot" || fail "snapshot missing context bar"
grep -q 'composer:' "$snapshot" || fail "snapshot missing composer"
if grep -q 'glob' "$snapshot"; then
  fail "agent snapshot leaked omitted older tool"
fi

printf 'tui agent streaming runtime contract ok\n'
