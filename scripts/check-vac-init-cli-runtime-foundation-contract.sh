#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-cli-runtime.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_grep() {
  local pattern="$1"
  local path="$2"
  if ! grep -qE "$pattern" "$path"; then
    echo "FAIL: missing pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_cli_runtime.rs
require_file vac-rs/crates/surfaces/cli/src/init_cli.rs
require_file vac-rs/crates/surfaces/cli/src/plan_cli.rs
require_file vac-rs/crates/surfaces/cli/src/why_cli.rs
require_file vac-rs/crates/surfaces/cli/src/doctor_cli.rs
require_file .vac/capabilities/vac-init-cli-runtime-foundation.yaml
require_file .vac/workflows/maintenance.vac-init-cli-runtime-foundation.yaml
require_file .vac/.init/state.yaml
require_file .vac/registry/trajectory/index.yaml

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_cli_runtime.rs -o "$TMPROOT/vac_init_cli_runtime_test"
"$TMPROOT/vac_init_cli_runtime_test" --nocapture
"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_lifecycle.rs -o "$TMPROOT/vac_init_lifecycle_test"
"$TMPROOT/vac_init_lifecycle_test" --nocapture
"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_semantic_plan.rs -o "$TMPROOT/vac_init_semantic_plan_test"
"$TMPROOT/vac_init_semantic_plan_test" --nocapture
"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_safe_rationale.rs -o "$TMPROOT/vac_init_safe_rationale_test"
"$TMPROOT/vac_init_safe_rationale_test" --nocapture
"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_doctor_release.rs -o "$TMPROOT/vac_init_doctor_release_test"
"$TMPROOT/vac_init_doctor_release_test" --nocapture

require_grep 'mod init_cli;' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'mod plan_cli;' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'mod why_cli;' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'Init\(init_cli::InitCommand\)' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'Plan\(plan_cli::PlanCommand\)' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'Why\(why_cli::WhyCommand\)' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'Evidence\(EvidenceDoctorCommand\)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
require_grep 'Memory\(MemoryDoctorCommand\)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
require_grep 'Init\(InitDoctorCommand\)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
require_grep 'VAC Doctor Taxonomy Aggregate' vac-rs/crates/surfaces/cli/src/doctor_cli.rs

for command in \
  'vac init' \
  'vac plan' \
  'vac plan validate' \
  'vac why' \
  'vac doctor evidence' \
  'vac doctor memory' \
  'vac doctor init' \
  'vac doctor release'; do
  require_grep "command: ${command}$" .vac/surfaces/cli.yaml
done

require_grep '^kind: init_state$' .vac/.init/state.yaml
require_grep '^id: init.state$' .vac/.init/state.yaml
require_grep '^kind: trajectory$' .vac/registry/trajectory/index.yaml
# Structured validation command coverage is enforced by registry strictness.
printf 'vac-init CLI runtime foundation static audit: PASS\n'

printf 'vac-init CLI runtime foundation contract: PASS\n'
