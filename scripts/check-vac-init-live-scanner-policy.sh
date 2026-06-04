#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-live-scanner.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() { [[ -f "$1" ]] || { echo "FAIL: missing $1" >&2; exit 1; }; }

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs
require_file vac-rs/crates/surfaces/cli/src/init_cli.rs

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_live_scanner_policy.rs -o "$TMPROOT/vac_init_live_scanner_policy_test"
"$TMPROOT/vac_init_live_scanner_policy_test" --nocapture

grep -q 'build_vac_init_live_scanner_reports' vac-rs/crates/surfaces/cli/src/init_cli.rs
grep -q '.vac/.init/source_inventory.yaml' vac-rs/crates/surfaces/cli/src/init_cli.rs
grep -q '.vac/.init/risk_findings.yaml' vac-rs/crates/surfaces/cli/src/init_cli.rs
grep -q '.vac/.init/policy_inference_report.yaml' vac-rs/crates/surfaces/cli/src/init_cli.rs
grep -q 'write_vac_init_store_record_atomic' vac-rs/crates/surfaces/cli/src/init_cli.rs

printf 'vac-init live scanner/policy gate: PASS\n'
