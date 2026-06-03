#!/usr/bin/env bash
# Hardening 7: autopilot scheduler remains monitor-only and policy-gated.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }

bash scripts/check-autopilot-scheduler-contract.sh >/dev/null
ui="vac-rs/tui/src/operator_ui.rs"
capability=".vac/capabilities/autopilot-scheduler.yaml"
workflow=".vac/workflows/maintenance.autopilot-scheduler-readiness.yaml"
snapshot="docs/validation/tui-operator-ui-snapshots/runtime-jobs-120x36.txt"

grep -q 'struct AutopilotActionSpec' "$ui" || fail "autopilot action contract missing"
grep -q 'default_monitor_actions' "$ui" || fail "default monitor actions missing"
grep -q 'actions policy-gated' "$ui" || fail "operator panel must show policy-gated actions"
grep -q 'monitor-only scheduler' "$ui" || fail "operator panel must state monitor-only scheduler"
grep -q 'destructive_task' "$capability" || fail "destructive autopilot tasks must be approval-gated"
grep -q 'monitor-only' "$capability" || fail "capability must state monitor-only"
grep -q 'policy-gated' "$workflow" || fail "readiness workflow must state policy-gated safety"
for required in autopilot 'runtime jobs' queue running 'c cancel' 'r retry' 'open in editor' attach 'approval gates'; do
  grep -q "$required" "$snapshot" || fail "runtime jobs snapshot missing $required"
done

printf 'autopilot scheduler monitor-only contract ok\n'
