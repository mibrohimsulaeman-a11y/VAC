#!/usr/bin/env bash
# Static control-plane contract check for the monitor-only autopilot scheduler surface.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK:   $*"; }

capability=".vac/capabilities/autopilot-scheduler.yaml"
workflow=".vac/workflows/maintenance.autopilot-scheduler-readiness.yaml"
operator_ui="vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc"

[ -f "$capability" ] || fail "missing $capability"
[ -f "$workflow" ] || fail "missing $workflow"
[ -f "$operator_ui" ] || fail "missing $operator_ui"

grep -q 'id: vac.autopilot.scheduler' "$capability" || fail "capability id not registered"
grep -q 'status: ready' "$capability" || fail "capability status must be ready"
grep -q '/runtime/jobs' "$capability" || fail "runtime jobs route missing from capability"
grep -q 'approval_required_for:' "$capability" || fail "capability missing approval gate list"
grep -q 'destructive_task' "$capability" || fail "destructive tasks are not policy-gated"

grep -q 'vac.autopilot.scheduler' .vac/surfaces/tui.yaml || fail "tui surface missing scheduler capability"
grep -q 'path: /runtime' .vac/surfaces/tui.yaml || fail "tui /runtime route missing"
grep -q 'path: /runtime/jobs' .vac/surfaces/tui.yaml || fail "tui /runtime/jobs route missing"
grep -q 'command: /runtime' .vac/surfaces/slash.yaml || fail "slash /runtime route missing"
grep -q 'open_runtime_jobs' .vac/surfaces/palette.yaml || fail "palette runtime jobs action missing"

grep -q 'id: maintenance.autopilot-scheduler-readiness' "$workflow" || fail "readiness workflow missing"
grep -q 'approval_required_for:' "$workflow" || fail "workflow missing approval gates"
grep -q 'monitor-only scheduler' "$operator_ui" || fail "operator panel must state monitor-only policy"
grep -q 'AutopilotSchedulerState' "$operator_ui" || fail "operator UI scheduler state missing"

ok "autopilot scheduler manifest/surface/workflow/operator contract registered"
