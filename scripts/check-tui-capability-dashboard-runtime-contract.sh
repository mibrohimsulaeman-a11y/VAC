#!/usr/bin/env bash
# Hardening 4: dashboard must be manifest-record driven and never blank on diagnostics.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }

ui="vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc"
dash="vac-rs/crates/surfaces/tui/src/capability_dashboard.rs"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-capability-dashboard.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
snapshot="$TMPROOT/tui-operator-ui-snapshots/capability-dashboard-180x48.txt"

[ -f "$ui" ] || fail "missing $ui"
[ -f "$dash" ] || fail "missing $dash"
bash scripts/render-tui-operator-snapshots.sh "$TMPROOT/tui-operator-ui-snapshots" >/dev/null

grep -q 'struct CapabilityDashboardRecord' "$ui" || fail "typed capability dashboard records missing"
grep -q 'enum CapabilityManifestStatus' "$ui" || fail "capability status enum missing"
grep -q 'struct ControlPlaneDiagnostic' "$ui" || fail "typed diagnostics missing"
grep -q 'enum DiagnosticSeverity' "$ui" || fail "diagnostic severity enum missing"
grep -q 'from_manifest_records' "$ui" || fail "manifest-driven dashboard state builder missing"
grep -q 'CapabilityManifestStatus::parse' "$dash" || fail "runtime dashboard does not map real manifest statuses"
grep -q 'CapabilityDashboardRecord::new' "$dash" || fail "runtime dashboard does not build typed records"
grep -q 'diagnostic_records' "$dash" || fail "runtime dashboard does not pass typed diagnostics"
grep -q 'Diagnostics' "$ui" || fail "dashboard diagnostics panel missing"
grep -q 'no YAML/control-plane errors detected' "$ui" || fail "dashboard non-blank diagnostics fallback missing"

[ -s "$snapshot" ] || fail "missing capability dashboard snapshot"
grep -q 'TUI Capability Dashboard' "$snapshot" || fail "dashboard snapshot title missing"
grep -q 'Diagnostics' "$snapshot" || fail "dashboard snapshot diagnostics column missing"
grep -q 'no YAML/control-plane errors detected' "$snapshot" || fail "dashboard snapshot healthy diagnostics fallback missing"
if grep -q '.vac/capabilities/tui.yml:42:13\|almost_ready\|vac.yaml.parser' "$snapshot"; then
  fail "dashboard snapshot leaked stale fake YAML diagnostic"
fi
grep -q 'owned' "$snapshot" || fail "dashboard snapshot ownership metrics missing"
grep -q 'UNOWNED' "$snapshot" || fail "dashboard snapshot unowned metric missing"
grep -q 'VALID' "$snapshot" || fail "dashboard snapshot valid metric missing"

printf 'tui capability dashboard runtime contract ok\n'
