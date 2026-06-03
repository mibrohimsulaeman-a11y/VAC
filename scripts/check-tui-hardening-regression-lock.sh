#!/usr/bin/env bash
# Hardening 1 regression lock for the operator TUI Batch 1-3 foundation.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

bash scripts/check-tui-status-output-contract.sh
bash scripts/check-tui-status-deep-hardening-contract.sh
bash scripts/check-tui-operator-ui-contract.sh
bash scripts/check-tui-operator-ui-visual-contract.sh
bash scripts/check-tui-operator-snapshot-contract.sh
bash scripts/check-tui-operator-ansi-contract.sh
bash scripts/check-tui-operator-live-adapter.sh
bash scripts/check-tui-renderer-semantic-contract.sh
bash scripts/check-tui-pty-gate-contract.sh
bash scripts/check-autopilot-scheduler-contract.sh

printf 'tui hardening regression lock ok\n'
