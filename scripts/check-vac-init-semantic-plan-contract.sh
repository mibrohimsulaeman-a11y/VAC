#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-semantic_plan.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_semantic_plan_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
SRC="vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_semantic_plan.rs"
require_file "$SRC"
require_grep "pub struct SemanticPlan" "$SRC"
require_grep "PlanAllowedFile" "$SRC"
require_grep "PlanValidationContext" "$SRC"
require_grep "validate_semantic_plan" "$SRC"
require_grep "can_transition_plan_status" "$SRC"
require_grep "plan.allowed_files" "$SRC"
require_grep 'pub mod vac_init_semantic_plan;' vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-semantic-plan.yaml
require_file .vac/workflows/maintenance.vac-init-semantic-plan.yaml
require_grep 'bash scripts/check-vac-init-semantic-plan-contract.sh' .vac/surfaces/cli.yaml
"$RUSTC_BIN" --edition 2024 --test "$SRC" -o "$TEST_BIN"
"$TEST_BIN" --nocapture
printf 'vac-init semantic-plan contract: PASS\n'
