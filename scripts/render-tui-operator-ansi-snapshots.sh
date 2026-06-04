#!/usr/bin/env bash
# Generate ANSI-colored snapshots for the VAC operator-console TUI style layer.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${1:-docs/validation/tui-operator-ansi-snapshots}"

mkdir -p "$OUT_DIR"
BIN="vac-rs/target/debug/operator-style-snapshot-harness"
is_fresh_binary() {
  [ -x "$BIN" ] && ! find \
    vac-rs/crates/surfaces/tui/Cargo.toml \
    vac-rs/crates/surfaces/tui/build.rs \
    vac-rs/crates/surfaces/tui/src \
    vac-rs/crates/surfaces/tui/tools \
    -newer "$BIN" -print -quit | grep -q .
}

if ! is_fresh_binary; then
  cargo build --manifest-path vac-rs/crates/surfaces/tui/Cargo.toml --bin operator-style-snapshot-harness
fi
"$BIN" "$OUT_DIR"

printf 'generated operator TUI ANSI snapshots in %s\n' "$OUT_DIR"
