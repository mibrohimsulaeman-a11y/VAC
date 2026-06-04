#!/usr/bin/env bash
# Generate fixed-size text snapshots for VAC operator-console TUI surfaces.
# The widget renderer depends on the vac-tui Cargo dependency graph (ratatui), so
# direct rustc generation is no longer valid. In source-only sandboxes without
# cargo, this command validates the committed snapshots and reports TV-Pending.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${1:-docs/validation/tui-operator-ui-snapshots}"
mkdir -p "$OUT_DIR"

BIN="vac-rs/target/debug/operator-ui-snapshot-harness"
is_fresh_binary() {
  [ -x "$BIN" ] && ! find \
    vac-rs/crates/surfaces/tui/Cargo.toml \
    vac-rs/crates/surfaces/tui/build.rs \
    vac-rs/crates/surfaces/tui/src \
    vac-rs/crates/surfaces/tui/tools \
    -newer "$BIN" -print -quit | grep -q .
}

if is_fresh_binary; then
  "$BIN" "$OUT_DIR"
  printf 'generated operator TUI snapshots in %s\n' "$OUT_DIR"
  exit 0
fi

if command -v cargo >/dev/null 2>&1; then
  cargo build --manifest-path vac-rs/crates/surfaces/tui/Cargo.toml --bin operator-ui-snapshot-harness
  "$BIN" "$OUT_DIR"
  printf 'generated operator TUI snapshots in %s\n' "$OUT_DIR"
  exit 0
fi

if [ "${VAC_TUI_RENDER_SNAPSHOTS_REQUIRE_TOOLCHAIN:-0}" = "1" ]; then
  echo "missing cargo; cannot regenerate ratatui widget snapshots" >&2
  exit 1
fi

for snapshot in \
  first-launch-120x36.txt idle-120x36.txt agent-working-120x36.txt \
  approval-popup-120x36.txt runtime-jobs-120x36.txt capability-dashboard-180x48.txt; do
  [ -s "$OUT_DIR/$snapshot" ] || { echo "missing committed snapshot $OUT_DIR/$snapshot" >&2; exit 1; }
done

printf 'operator TUI snapshots: committed source snapshots retained; regeneration TV-Pending (cargo not found)\n'
