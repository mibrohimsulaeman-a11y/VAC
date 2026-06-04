#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-evidence-why.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() { [[ -f "$1" ]] || { echo "FAIL: missing $1" >&2; exit 1; }; }

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs
require_file vac-rs/crates/surfaces/cli/src/why_cli.rs

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs -o "$TMPROOT/vac_init_evidence_writer_test"
"$TMPROOT/vac_init_evidence_writer_test" --nocapture

grep -q '.vac/registry/trajectory/index.yaml' vac-rs/crates/surfaces/cli/src/why_cli.rs
grep -q 'raw_chain_of_thought: excluded' vac-rs/crates/surfaces/cli/src/why_cli.rs
grep -q 'write_vac_init_live_evidence_and_trajectory' vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs

printf 'vac evidence/why live index gate: PASS\n'
