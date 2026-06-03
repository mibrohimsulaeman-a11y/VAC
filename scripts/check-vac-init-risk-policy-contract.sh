#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-risk_policy.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_risk_policy_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
SRC="vac-rs/control-plane/src/control_plane/vac_init_risk_policy.rs"
require_file "$SRC"
require_grep "pub struct RiskFinding" "$SRC"
require_grep "pub enum RiskAction" "$SRC"
require_grep "ConfidenceLabel" "$SRC"
require_grep "detect_risk_findings" "$SRC"
require_grep "infer_policy_from_findings" "$SRC"
require_grep "Command::new" "$SRC"
require_grep "CredentialRead" "$SRC"
require_grep 'pub mod vac_init_risk_policy;' vac-rs/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-risk-policy.yaml
require_file .vac/workflows/maintenance.vac-init-risk-policy.yaml
require_grep 'bash scripts/check-vac-init-risk-policy-contract.sh' .vac/surfaces/cli.yaml
"$RUSTC_BIN" --edition 2024 --test "$SRC" -o "$TEST_BIN"
"$TEST_BIN" --nocapture
printf 'vac-init risk-policy contract: PASS\n'
