#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: optional cargo test for vac-control-plane evidence chain unit tests
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [ -f "$1" ] || fail "missing required file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
SRC="vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs"
require_file "$SRC"
require_grep "pub struct EvidenceRecord" "$SRC"
require_grep "EvidenceChainLink" "$SRC"
require_grep "canonical_evidence_payload" "$SRC"
require_grep "compute_evidence_self_hash" "$SRC"
require_grep "verify_evidence_chain" "$SRC"
require_grep "sha256_hex" "$SRC"
require_grep "EvidenceSignatureEnvelope" "$SRC"
require_grep "verify_evidence_ed25519_signature" "$SRC"
require_grep "evidence_signature_payload" "$SRC"
require_grep 'pub mod vac_init_evidence_chain;' vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
require_grep 'verify_evidence_ed25519_signature' vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-evidence-chain.yaml
require_file .vac/workflows/maintenance.vac-init-evidence-chain.yaml
require_grep 'bash scripts/check-vac-init-evidence-chain-contract.sh' .vac/surfaces/cli.yaml
if command -v cargo >/dev/null 2>&1; then
  cargo test --manifest-path vac-rs/Cargo.toml -p vac-control-plane vac_init_evidence_chain --lib -- --nocapture
  rc=$?
  [ "$rc" -eq 0 ] || exit "$rc"
  echo 'vac-init evidence-chain contract: PASS (cargo unit)'
else
  echo 'vac-init evidence-chain contract: PASS (static); cargo unit NotEvaluated (cargo not found)'
fi
