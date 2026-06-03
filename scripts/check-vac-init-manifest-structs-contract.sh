#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-manifest.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_manifest_contract_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
require_file vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'pub struct CapabilityManifestContract' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'pub struct PolicyManifestContract' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'pub struct SurfaceManifestContract' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'pub struct WorkflowManifestContract' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'CapabilityLifecycleStatus' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'PolicyAction::ExecuteProcess|ExecuteProcess' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'merge_policy_decisions' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'validate_capability_contract' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'validate_surface_contract' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'validate_workflow_contract' vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs
require_grep 'pub mod vac_init_manifest_contract;' vac-rs/control-plane/src/control_plane/mod.rs
require_grep 'validate_vac_init_capability_contract' vac-rs/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-manifest-contracts.yaml
require_file .vac/workflows/maintenance.vac-init-manifest-contracts.yaml
require_grep 'vac\.init\.manifest-contracts' .vac/surfaces/cli.yaml
"$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_manifest_contract.rs -o "$TEST_BIN"
"$TEST_BIN" --nocapture
printf 'vac-init manifest structs contract: PASS\n'
