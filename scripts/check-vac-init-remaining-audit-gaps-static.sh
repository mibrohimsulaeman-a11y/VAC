#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [ -f "$1" ] || fail "missing required file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
require_no_grep() { ! grep -qE "$1" "$2" || fail "unexpected pattern in $2: $1"; }

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_approval_binding.rs
require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs
require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_migration_runtime.rs
require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_tui_real_data.rs
require_file vac-rs/core/Cargo.toml
require_file THIRD_PARTY_NOTICES.md
require_file NOTICE
require_file .vac/registry/compile-debt-ledger.yaml
require_file tests/fixtures/fixture_matrix.yaml

# GAP-C crypto path: approval + evidence now have Ed25519 verification contracts.
require_grep 'ed25519-dalek = \{ workspace = true \}' vac-rs/core/Cargo.toml
require_grep 'ApprovalSignaturePolicy' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_approval_binding.rs
require_grep 'RequireEd25519' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_approval_binding.rs
require_grep 'verify_approval_ed25519_signature' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_approval_binding.rs
require_grep 'approval_signature_payload' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_approval_binding.rs
require_grep 'EvidenceSignatureEnvelope' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs
require_grep 'verify_evidence_ed25519_signature' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs
require_grep 'evidence_signature_payload' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs

# GAP-C migration depth: full action vocab, rollback, dry-run/apply plan and registry doctor verification.
for symbol in ChangeType VersionBump build_migration_plan build_rollback_plan preview_migration apply_migration_action_to_yaml_text validate_migration_record_depth VerificationMustRunRegistryDoctor RollbackMismatch; do
  require_grep "$symbol" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_migration_runtime.rs
done

# GAP-D UI/scheduler: state fixtures and real-data loaders are present.
require_file tests/fixtures/tui/scheduler_monitor_states.yaml
for symbol in load_capability_dashboard_data_from_workspace load_approval_popup_data_from_store load_autopilot_doctor_monitor_from_workspace CapabilityDashboardData ApprovalPopupData AutopilotDoctorMonitorData destructive_actions_policy_gated; do
  require_grep "$symbol" vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_tui_real_data.rs
done
require_grep 'show_zero_tasks_without_blank_screen' tests/fixtures/tui/scheduler_monitor_states.yaml
require_grep 'approval_bypass_allowed: false' tests/fixtures/tui/scheduler_monitor_states.yaml

# Fixture matrix must carry the new remaining-gap fixtures.
for fixture in \
  tests/fixtures/signatures/approval_ed25519_required.yaml \
  tests/fixtures/signatures/evidence_ed25519_required.yaml \
  tests/fixtures/migrations/runtime_depth_matrix.yaml \
  tests/fixtures/tui/scheduler_monitor_states.yaml; do
  require_file "$fixture"
  grep -q "$fixture" tests/fixtures/fixture_matrix.yaml || fail "fixture matrix missing $fixture"
done

# Legal/provenance must be explicit and cargo-about/cargo-deny remain NotEvaluated, not silently green.
require_grep 'OpenAI Codex CLI' THIRD_PARTY_NOTICES.md
require_grep 'Apache-2.0' THIRD_PARTY_NOTICES.md
require_grep 'Ratatui' THIRD_PARTY_NOTICES.md
require_grep 'MIT' THIRD_PARTY_NOTICES.md
require_grep 'cargo about' THIRD_PARTY_NOTICES.md
require_grep 'dependency_attribution: NotEvaluated' .vac/registry/legal-release-blockers.yaml
require_grep 'cargo_about_generate: NotEvaluated' .vac/registry/legal-release-blockers.yaml

# Truthfulness: this slice must not mark TV/cargo as done.
require_no_grep 'TV-Done|cargo_build_workspace: PASS|cargo_test_workspace: PASS|cargo_clippy_workspace: PASS' .vac/registry/o5-o6-completion-state.yaml
require_no_grep 'TV-Done|cargo_build_workspace: PASS|cargo_test_workspace: PASS|cargo_clippy_workspace: PASS' .vac/registry/o5-o6-monolith-quality-state.yaml

bash scripts/check-vac-init-approval-binding-contract.sh >/tmp/vac_approval_gate.out || { cat /tmp/vac_approval_gate.out >&2; exit 1; }
bash scripts/check-vac-init-evidence-chain-contract.sh >/tmp/vac_evidence_gate.out || { cat /tmp/vac_evidence_gate.out >&2; exit 1; }
bash scripts/check-vac-init-migration-runtime-contract.sh >/tmp/vac_migration_gate.out || { cat /tmp/vac_migration_gate.out >&2; exit 1; }
bash scripts/check-vac-init-tui-real-data-contract.sh >/tmp/vac_tui_gate.out || { cat /tmp/vac_tui_gate.out >&2; exit 1; }

printf 'remaining audit gaps static gate: PASS (cargo-dependent unit tests remain NotEvaluated without cargo)\n'
