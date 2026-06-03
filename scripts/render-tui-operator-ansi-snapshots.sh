#!/usr/bin/env bash
# Generate ANSI-colored snapshots for the VAC operator-console TUI style layer.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${1:-docs/validation/tui-operator-ansi-snapshots}"
HARNESS_BIN="${TMPDIR:-/tmp}/vac-operator-style-snapshot-harness"
RUSTC_BIN="${RUSTC:-rustc}"

mkdir -p "$OUT_DIR"
"$RUSTC_BIN" --edition 2024 vac-rs/tui/tools/operator_style_snapshot_harness.rs -o "$HARNESS_BIN"
"$HARNESS_BIN" "$OUT_DIR"

printf 'generated operator TUI ANSI snapshots in %s\n' "$OUT_DIR"
