#!/usr/bin/env bash
set -u

root="${1:-.}"
cd "$root" || exit 2
fail=0

require_file() {
  if [ ! -f "$1" ]; then
    echo "MISSING $1"
    fail=1
  fi
}
require_contains() {
  file="$1"; pattern="$2"
  if ! grep -Fq "$pattern" "$file"; then
    echo "MISSING_PATTERN $file :: $pattern"
    fail=1
  fi
}
reject_contains() {
  file="$1"; pattern="$2"
  if grep -Fq "$pattern" "$file"; then
    echo "FORBIDDEN_PATTERN $file :: $pattern"
    fail=1
  fi
}

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_runtime_bridge.rs
require_contains vac-rs/crates/capabilities/build/src/core_migrated/apply_patch.rs "evaluate_vac_init_runtime_patch_contract"
require_contains vac-rs/crates/capabilities/build/src/core_migrated/apply_patch.rs "write_vac_init_runtime_patch_evidence"
require_contains vac-rs/crates/capabilities/build/src/core_migrated/apply_patch.rs "VacInitRuntimePatchFileChange"
reject_contains vac-rs/crates/capabilities/build/src/core_migrated/apply_patch.rs "plan_loaded"
reject_contains vac-rs/crates/capabilities/build/src/core_migrated/apply_patch.rs "patch_guard_allowed: plan_loaded"
reject_contains vac-rs/crates/capabilities/build/src/core_migrated/apply_patch.rs "RuntimePlanTargetFile::complete"

require_contains vac-rs/crates/capabilities/tools-domain/src/core_migrated/tools/handlers/unified_exec.rs "evaluate_vac_init_runtime_command_contract"
require_contains vac-rs/crates/capabilities/tools-domain/src/core_migrated/tools/handlers/unified_exec.rs "write_vac_init_runtime_command_evidence"
reject_contains vac-rs/crates/capabilities/tools-domain/src/core_migrated/tools/handlers/unified_exec.rs "runner_registered: true"
reject_contains vac-rs/crates/capabilities/tools-domain/src/core_migrated/tools/handlers/unified_exec.rs "approval_request_persisted: approval_dir.exists"
reject_contains vac-rs/crates/capabilities/tools-domain/src/core_migrated/tools/handlers/unified_exec.rs "evaluate_vac_init_pre_command_gate"
reject_contains vac-rs/crates/capabilities/tools-domain/src/core_migrated/tools/handlers/unified_exec.rs "evaluate_vac_init_evidence_completion_gate"

bridge=vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_runtime_bridge.rs
require_contains "$bridge" "validate_patch_attempt_with_semantic_source"
require_contains "$bridge" "evaluate_structured_command"
require_contains "$bridge" "validate_approval_binding"
require_contains "$bridge" "write_live_evidence_and_trajectory"
require_contains "$bridge" "write_runtime_patch_evidence"
require_contains "$bridge" "validate_semantic_plan"
require_contains "$bridge" "preview_migration(&record)"
require_contains "$bridge" "trajectory_index_from_yaml"
require_contains "$bridge" "migration_actions_from_yaml"

require_contains vac-rs/crates/surfaces/cli/src/init_cli.rs "advance_init_state"
require_contains vac-rs/crates/surfaces/cli/src/init_cli.rs "write_vac_init_live_evidence_and_trajectory"
require_contains vac-rs/crates/surfaces/cli/src/plan_cli.rs "validate_vac_init_plan_yaml_with_engine"
require_contains vac-rs/crates/surfaces/cli/src/why_cli.rs "lookup_vac_init_safe_rationale_with_engine"
require_contains vac-rs/crates/surfaces/cli/src/registry_cli.rs "preview_vac_init_registry_migrations_with_engine"

if [ "$fail" -ne 0 ]; then
  echo "FAIL runtime engine wiring static gate"
  exit 1
fi

echo "PASS runtime engine wiring uses VAC-Init engines instead of file-existence stubs"
echo "TV-PENDING: cargo build/test not evaluated"
