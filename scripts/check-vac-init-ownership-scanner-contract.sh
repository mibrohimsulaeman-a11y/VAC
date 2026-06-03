#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-own.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_ownership_scanner_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
require_file vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs
require_grep 'pub enum OwnershipCoverageStatus' vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs
for status in complete partial hidden overclaimed unowned; do require_grep "$status" vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs; done
for quarantine in soft_quarantine hard_quarantine auto_quarantine; do require_grep "$quarantine" vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs; done
require_grep 'build_source_inventory_from_paths' vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs
require_grep 'build_ownership_report' vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs
require_grep 'blocks_agent_write' vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs
require_grep 'pub mod vac_init_ownership_scanner;' vac-rs/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-ownership-scanner.yaml
require_file .vac/workflows/maintenance.vac-init-ownership-scanner.yaml
require_grep 'vac doctor ownership' .vac/surfaces/cli.yaml
"$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_ownership_scanner.rs -o "$TEST_BIN"
"$TEST_BIN" --nocapture
printf 'vac-init ownership scanner contract: PASS\n'
