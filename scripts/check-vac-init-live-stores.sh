#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-live-stores.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

"$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_live_stores.rs -o "$TMPROOT/vac_init_live_stores_test"
"$TMPROOT/vac_init_live_stores_test" --nocapture

grep -q 'write_vac_init_store_record_atomic' vac-rs/cli/src/init_cli.rs
grep -q 'pub use vac_init_live_stores::write_vac_init_store_record_atomic' vac-rs/control-plane/src/control_plane/mod.rs

printf 'vac-init live stores: PASS\n'
