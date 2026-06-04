#!/usr/bin/env bash
# PTY gate contract: document and verify live-terminal proof remains explicit when sandbox cannot run it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

grep -q "id: vac.tui.operator-visual-gate" .vac/capabilities/tui-operator-visual-gate.yaml
grep -q "id: maintenance.tui-operator-visual-gate" .vac/workflows/maintenance.tui-operator-visual-gate.yaml
grep -q "capability: vac.tui.operator-visual-gate" .vac/surfaces/tui.yaml
grep -q "operator-ui-snapshot-harness" vac-rs/crates/surfaces/tui/Cargo.toml
grep -q "VISUAL_FIDELITY_MATRIX" vac-rs/crates/surfaces/tui/src/operator_ui.rs.inc
grep -q "render_operator_snapshot_text" vac-rs/crates/surfaces/tui/tools/operator_ui_snapshot_harness.rs
grep -q "render_operator_snapshot_ansi_text" vac-rs/crates/surfaces/tui/tools/operator_style_snapshot_harness.rs

printf 'operator PTY gate contract ok\n'
