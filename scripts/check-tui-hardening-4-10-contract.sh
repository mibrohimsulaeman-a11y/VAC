#!/usr/bin/env bash
# Aggregate hardening 4-10 validation gate.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

bash scripts/check-tui-hardening-regression-lock.sh
bash scripts/check-tui-capability-dashboard-runtime-contract.sh
bash scripts/check-tui-agent-streaming-runtime-contract.sh
bash scripts/check-tui-approval-popup-safety-contract.sh
bash scripts/check-autopilot-scheduler-monitor-only-contract.sh
bash scripts/check-tui-operator-visual-fidelity-matrix.sh
bash scripts/check-tui-spec-driven-consolidation.sh
bash scripts/check-tui-source-artifact-hygiene.sh

printf 'tui hardening 4-10 contract ok\n'
