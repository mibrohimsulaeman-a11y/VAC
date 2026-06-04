#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-command_gate.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_command_gate_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
SRC="vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_command_gate.rs"
require_file "$SRC"
require_grep "pub struct StructuredCommand" "$SRC"
require_grep "CommandRunnerRegistry" "$SRC"
require_grep "evaluate_structured_command" "$SRC"
require_grep "shell inline" "$SRC"
require_grep "shell metacharacters" "$SRC"
require_grep "CommandGateDecision" "$SRC"
require_grep 'pub mod vac_init_command_gate;' vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-command-gate.yaml
require_file .vac/workflows/maintenance.vac-init-command-gate.yaml
require_grep 'bash scripts/check-vac-init-command-gate-contract.sh' .vac/surfaces/cli.yaml
"$RUSTC_BIN" --edition 2024 --test "$SRC" -o "$TEST_BIN"
"$TEST_BIN" --nocapture
printf 'vac-init command-gate contract: PASS\n'
