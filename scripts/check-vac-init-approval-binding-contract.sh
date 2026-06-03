#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: optional cargo test for vac-core approval binding unit tests
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [ -f "$1" ] || fail "missing required file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
SRC="vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_approval_binding.rs"
require_file "$SRC"
require_grep "pub struct ApprovalRequestRecord" "$SRC"
require_grep "pub struct ApprovalBinding" "$SRC"
require_grep "ApprovalReplayStore" "$SRC"
require_grep "validate_approval_binding" "$SRC"
require_grep "consume_approval" "$SRC"
require_grep "NonceReplay" "$SRC"
require_grep "ApprovalSignaturePolicy" "$SRC"
require_grep "RequireEd25519" "$SRC"
require_grep "verify_approval_ed25519_signature" "$SRC"
require_grep "approval_signature_payload" "$SRC"
require_grep "MissingResponseSignature" "$SRC"
require_grep "ed25519-dalek = \{ workspace = true \}" vac-rs/core/Cargo.toml
require_grep 'pub mod vac_init_approval_binding;' vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
require_grep 'validate_approval_binding_with_signature_policy' vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-approval-binding.yaml
require_file .vac/workflows/maintenance.vac-init-approval-binding.yaml
require_grep 'bash scripts/check-vac-init-approval-binding-contract.sh' .vac/surfaces/cli.yaml
if command -v cargo >/dev/null 2>&1; then
  (cd vac-rs && cargo test --offline -p vac-core vac_init_approval_binding --lib)
  rc=$?
  [ "$rc" -eq 0 ] || exit "$rc"
  echo 'vac-init approval-binding contract: PASS (cargo unit)'
else
  echo 'vac-init approval-binding contract: PASS (static); cargo unit NotEvaluated (cargo not found)'
fi
