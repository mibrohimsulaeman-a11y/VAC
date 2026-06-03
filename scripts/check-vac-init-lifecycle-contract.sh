#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-life.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_lifecycle_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
require_file vac-rs/control-plane/src/control_plane/vac_init_lifecycle.rs
require_grep 'pub enum VacInitLifecycleState' vac-rs/control-plane/src/control_plane/vac_init_lifecycle.rs
for state in uninitialized discovered partition_selected policy_inferred manifests_synthesized doctor_verified ready scan_failed operator_cancelled policy_conflict ownership_missing doctor_failed; do
  require_grep "$state" vac-rs/control-plane/src/control_plane/vac_init_lifecycle.rs
done
require_grep 'advance_init_state' vac-rs/control-plane/src/control_plane/vac_init_lifecycle.rs
require_grep 'can_transition' vac-rs/control-plane/src/control_plane/vac_init_lifecycle.rs
require_grep 'VacInitStateRecord' vac-rs/control-plane/src/control_plane/vac_init_lifecycle.rs
require_grep 'pub mod vac_init_lifecycle;' vac-rs/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-lifecycle.yaml
require_file .vac/workflows/maintenance.vac-init-lifecycle.yaml
require_grep 'vac init --dry-run' .vac/surfaces/cli.yaml
"$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_lifecycle.rs -o "$TEST_BIN"
"$TEST_BIN" --nocapture
printf 'vac-init lifecycle contract: PASS\n'
