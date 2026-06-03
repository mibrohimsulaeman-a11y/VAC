#!/usr/bin/env bash
# Hardening 9: every operator TUI hardening surface is declared in .vac manifests/workflows/surfaces.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }

for cap in \
  tui-capability-dashboard-runtime \
  tui-agent-streaming-runtime \
  tui-approval-popup-safety \
  autopilot-scheduler-monitor-only \
  tui-operator-visual-fidelity-matrix \
  tui-spec-driven-consolidation \
  tui-source-artifact-hygiene; do
  [ -f ".vac/capabilities/${cap}.yaml" ] || fail "missing capability ${cap}"
  grep -q 'status: ready' ".vac/capabilities/${cap}.yaml" || fail "capability ${cap} must be ready"
  grep -q 'validation:' ".vac/capabilities/${cap}.yaml" || fail "capability ${cap} missing validation"
done

for wf in \
  maintenance.tui-capability-dashboard-runtime \
  maintenance.tui-agent-streaming-runtime \
  maintenance.tui-approval-popup-safety \
  maintenance.autopilot-scheduler-monitor-only \
  maintenance.tui-operator-visual-fidelity-matrix \
  maintenance.tui-spec-driven-consolidation \
  maintenance.tui-source-artifact-hygiene \
  maintenance.tui-hardening-4-10; do
  [ -f ".vac/workflows/${wf}.yaml" ] || fail "missing workflow ${wf}"
  grep -q 'status: ready' ".vac/workflows/${wf}.yaml" || fail "workflow ${wf} must be ready"
done

for id in \
  vac.tui.capability-dashboard-runtime \
  vac.tui.agent-streaming-runtime \
  vac.tui.approval-popup-safety \
  vac.autopilot.scheduler-monitor-only \
  vac.tui.operator-visual-fidelity-matrix \
  vac.tui.spec-driven-consolidation \
  vac.tui.source-artifact-hygiene; do
  grep -q "$id" .vac/surfaces/tui.yaml || fail "surface.tui missing $id"
  grep -q "$id" .vac/capabilities/tui.yaml || fail "vac.tui missing hardening reference $id"
done

printf 'tui spec-driven consolidation ok\n'
