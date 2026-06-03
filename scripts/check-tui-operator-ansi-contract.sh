#!/usr/bin/env bash
# Validate that the operator console has a semantic ANSI style layer without layout drift.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
HARNESS_TEST="${TMPDIR:-/tmp}/vac-operator-style-tests"
OUT_DIR="${TMPDIR:-/tmp}/vac-operator-ansi-contract"

"$RUSTC_BIN" --edition 2024 --test vac-rs/tui/src/operator_style.rs -o "$HARNESS_TEST"
"$HARNESS_TEST"

bash scripts/render-tui-operator-ansi-snapshots.sh "$OUT_DIR"

sample="$OUT_DIR/approval-popup-120x36.ansi.txt"
test -s "$sample"
grep -q $'\033' "$sample"
grep -q "style_roles: plain, chrome, muted, accent, success, warning, danger, user, agent, status" "$sample"
grep -q "approval required" "$sample"
grep -q "DESTRUCTIVE" "$sample"

for role in plain chrome muted accent success warning danger user agent status; do
  grep -q "$role" vac-rs/tui/src/operator_style.rs
done

printf 'operator ANSI style contract ok\n'
