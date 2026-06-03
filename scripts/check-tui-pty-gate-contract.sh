#!/usr/bin/env bash
# PTY gate contract: document and verify live-terminal proof remains explicit when sandbox cannot run it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

grep -q "PTY" docs/tui/OPERATOR_CONSOLE_VISUAL_FIDELITY.md
grep -q "sandbox" docs/tui/OPERATOR_CONSOLE_VISUAL_FIDELITY.md
grep -q "deterministic snapshot" docs/tui/OPERATOR_CONSOLE_VISUAL_FIDELITY.md
grep -q "live adapter" docs/tui/OPERATOR_CONSOLE_LIVE_ADAPTER.md

grep -q "id: vac.tui.operator-visual-gate" .vac/capabilities/tui-operator-visual-gate.yaml
grep -q "id: maintenance.tui-operator-visual-gate" .vac/workflows/maintenance.tui-operator-visual-gate.yaml
grep -q "capability: vac.tui.operator-visual-gate" .vac/surfaces/tui.yaml

printf 'operator PTY gate contract ok\n'
