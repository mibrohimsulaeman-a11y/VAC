#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-status-output.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/surfaces/tui/src/status/output_contract.rs -o "$TMPROOT/vac-status-output-contract-test"
"$TMPROOT/vac-status-output-contract-test" --nocapture

printf 'tui status output rustc contract: PASS\n'
