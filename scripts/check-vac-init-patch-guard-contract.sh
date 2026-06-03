#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-patch_guard.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_patch_guard_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
SRC="vac-rs/control-plane/src/control_plane/vac_init_patch_guard.rs"
require_file "$SRC"
require_grep "pub struct ApprovedPatchScope" "$SRC"
require_grep "pub struct PatchAttempt" "$SRC"
require_grep "PatchGuardContext" "$SRC"
require_grep "validate_patch_attempt" "$SRC"
require_grep "patch.file.outside_plan" "$SRC"
require_grep "patch.anchor.unresolved" "$SRC"
require_grep 'pub mod vac_init_patch_guard;' vac-rs/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-patch-guard.yaml
require_file .vac/workflows/maintenance.vac-init-patch-guard.yaml
require_grep 'bash scripts/check-vac-init-patch-guard-contract.sh' .vac/surfaces/cli.yaml
"$RUSTC_BIN" --edition 2024 --test "$SRC" -o "$TEST_BIN"
"$TEST_BIN" --nocapture
printf 'vac-init patch-guard contract: PASS\n'
