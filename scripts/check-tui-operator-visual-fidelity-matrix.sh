#!/usr/bin/env bash
# Hardening 8: render all operator screenshots across the fixed terminal viewport matrix.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }

ui="vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-operator-visual.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
snapshot_dir="$TMPROOT/tui-operator-ui-snapshots"
ansi_dir="$TMPROOT/tui-operator-ansi-snapshots"

grep -q 'VISUAL_FIDELITY_MATRIX' "$ui" || fail "visual fidelity viewport matrix missing"
grep -q '120,.*36\|width: 120' "$ui" || fail "120x36 viewport missing"
grep -q '140,.*40\|width: 140' "$ui" || fail "140x40 viewport missing"
grep -q '180,.*48\|width: 180' "$ui" || fail "180x48 viewport missing"

bash scripts/render-tui-operator-snapshots.sh "$snapshot_dir" >/tmp/vac-hardening-visual-snapshots.log
bash scripts/render-tui-operator-ansi-snapshots.sh "$ansi_dir" >/tmp/vac-hardening-ansi-snapshots.log

for scenario in first-launch idle agent-working approval-popup runtime-jobs capability-dashboard; do
  for viewport in 120x36 140x40 180x48; do
    [ -s "$snapshot_dir/${scenario}-${viewport}.txt" ] || fail "missing ${scenario}-${viewport}.txt"
    [ -s "$ansi_dir/${scenario}-${viewport}.ansi.txt" ] || fail "missing ${scenario}-${viewport}.ansi.txt"
  done
done

grep -q 'Vastar Agentic CLI' "$snapshot_dir/first-launch-180x48.txt" || fail "first launch large snapshot missing header"
grep -q 'VIL-native.*ready' "$snapshot_dir/idle-140x40.txt" || fail "idle medium snapshot missing ready tag"
grep -Eq 'tool timeline.*last 5' "$snapshot_dir/agent-working-120x36.txt" || fail "agent small snapshot missing timeline cap"
grep -q 'DESTRUCTIVE' "$snapshot_dir/approval-popup-120x36.txt" || fail "approval small snapshot missing destructive label"
grep -q 'runtime jobs' "$snapshot_dir/runtime-jobs-140x40.txt" || fail "runtime medium snapshot missing route tab"
grep -q 'Diagnostics' "$snapshot_dir/capability-dashboard-180x48.txt" || fail "dashboard large snapshot missing diagnostics"

printf 'tui operator visual fidelity matrix ok\n'
