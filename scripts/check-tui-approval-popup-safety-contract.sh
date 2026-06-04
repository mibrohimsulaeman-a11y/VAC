#!/usr/bin/env bash
# Hardening 6: approval popup must be policy-backed and explicit for destructive operations.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }

ui="vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc"
overlay="vac-rs/crates/surfaces/tui/src/bottom_pane/approval_overlay.rs.inc"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-approval-popup.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
snapshot="$TMPROOT/tui-operator-ui-snapshots/approval-popup-120x36.txt"

[ -f "$ui" ] || fail "missing $ui"
[ -f "$overlay" ] || fail "missing $overlay"
bash scripts/render-tui-operator-snapshots.sh "$TMPROOT/tui-operator-ui-snapshots" >/dev/null
[ -f "$snapshot" ] || fail "missing generated snapshot $snapshot"

grep -q 'enum ApprovalRisk' "$ui" || fail "approval risk enum missing"
grep -q 'ApprovalRisk::Destructive' "$ui" || fail "destructive approval risk missing"
grep -q 'struct ApprovalSafetyContext' "$ui" || fail "approval safety context missing"
grep -q 'policy_gate_summary' "$ui" || fail "approval policy gate summary missing"
grep -q 'requires_policy_gate' "$ui" || fail "approval risk policy gate method missing"
grep -q 'policy-gated approval queue' "$ui" || fail "approval UI not tied to policy-gated queue"
grep -q 'does not evaluate whether an action is safe' "$overlay" || fail "approval overlay must not evaluate policy itself"
if grep -Riq 'auto.approve\|auto_approve\|bypass approval' vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc vac-rs/crates/surfaces/tui/src/bottom_pane/approval_overlay.rs; then
  fail "approval renderer contains auto-approve/bypass wording"
fi
for required in 'approval required' 'DESTRUCTIVE' 'cwd' 'context' 'risk' 'policy' 'approve once' 'approve+remember' 'reject with reason'; do
  grep -q "$required" "$snapshot" || fail "approval snapshot missing $required"
done

printf 'tui approval popup safety contract ok\n'
