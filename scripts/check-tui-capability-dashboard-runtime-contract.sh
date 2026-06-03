#!/usr/bin/env bash
# Hardening 4: dashboard must be manifest-record driven and never blank on diagnostics.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }

ui="vac-rs/tui/src/operator_ui.rs"
dash="vac-rs/tui/src/capability_dashboard.rs"
snapshot="docs/validation/tui-operator-ui-snapshots/capability-dashboard-180x48.txt"

[ -f "$ui" ] || fail "missing $ui"
[ -f "$dash" ] || fail "missing $dash"

grep -q 'struct CapabilityDashboardRecord' "$ui" || fail "typed capability dashboard records missing"
grep -q 'enum CapabilityManifestStatus' "$ui" || fail "capability status enum missing"
grep -q 'struct ControlPlaneDiagnostic' "$ui" || fail "typed diagnostics missing"
grep -q 'enum DiagnosticSeverity' "$ui" || fail "diagnostic severity enum missing"
grep -q 'from_manifest_records' "$ui" || fail "manifest-driven dashboard state builder missing"
grep -q 'CapabilityManifestStatus::parse' "$dash" || fail "runtime dashboard does not map real manifest statuses"
grep -q 'CapabilityDashboardRecord::new' "$dash" || fail "runtime dashboard does not build typed records"
grep -q 'diagnostic_records' "$dash" || fail "runtime dashboard does not pass typed diagnostics"
grep -q 'right / Diagnostics' "$ui" || fail "dashboard diagnostics panel missing"
grep -q 'no YAML/control-plane errors detected' "$ui" || fail "dashboard non-blank diagnostics fallback missing"

[ -s "$snapshot" ] || fail "missing capability dashboard snapshot"
grep -q 'TUI Capability Dashboard' "$snapshot" || fail "dashboard snapshot title missing"
grep -q 'right / Diagnostics' "$snapshot" || fail "dashboard snapshot diagnostics column missing"
grep -q 'YAML diagnostic' "$snapshot" || fail "dashboard snapshot YAML diagnostic missing"
grep -q 'owned' "$snapshot" || fail "dashboard snapshot ownership metrics missing"
grep -q 'UNOWNED' "$snapshot" || fail "dashboard snapshot unowned metric missing"
grep -q 'VALID' "$snapshot" || fail "dashboard snapshot valid metric missing"

printf 'tui capability dashboard runtime contract ok\n'
