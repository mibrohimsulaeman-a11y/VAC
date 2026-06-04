#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "[FAIL] $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { local pattern="$1" file="$2"; grep -Eq -- "$pattern" "$file" || fail "missing pattern '$pattern' in $file"; }
reject_grep() { local pattern="$1" file="$2"; ! grep -Eq -- "$pattern" "$file" || fail "forbidden pattern '$pattern' in $file"; }
max_lines() { local max="$1" file="$2"; local count; count=$(wc -l < "$file"); [[ "$count" -le "$max" ]] || fail "$file has $count lines > $max"; }

# Phase 0 ledger.
require_file ".vac/registry/implementation-blueprint.yaml"
require_file ".vac/registry/validation-ledger.yaml"
require_grep "o5o6.structured_blueprint" ".vac/registry/implementation-blueprint.yaml"
require_grep "cargo_check: TV-Pending" ".vac/registry/implementation-blueprint.yaml"

# Phase 1 approval/plan/init safety depth.
require_file "vac-rs/crates/surfaces/cli/src/approve_cli.rs"
require_grep "validate_approval_binding_with_signature_policy" "vac-rs/crates/surfaces/cli/src/approve_cli.rs"
require_grep "VacInitApprovalReplayStore" "vac-rs/crates/surfaces/cli/src/approve_cli.rs"
require_grep "replay-store.yaml" "vac-rs/crates/surfaces/cli/src/approve_cli.rs"
require_grep "missing approval request binding record" "vac-rs/crates/surfaces/cli/src/approve_cli.rs"
reject_grep "nonce-\\{id\\}|nonce-\\$\\{|nonce-\\{" "vac-rs/crates/surfaces/cli/src/approve_cli.rs"
require_file ".vac/registry/approvals/approval.production-rc-example.yaml"
reject_grep "nonce-production-rc|plan_hash: plan|diff_hash: diff|policy_snapshot_hash: policy|content_hash: hash" ".vac/registry/approvals/approval.production-rc-example.yaml"

require_file "vac-rs/crates/surfaces/cli/src/plan_cli.rs"
require_grep "CommandRunnerRegistry::with_defaults" "vac-rs/crates/surfaces/cli/src/plan_cli.rs"
require_grep "evaluate_structured_command" "vac-rs/crates/surfaces/cli/src/plan_cli.rs"
require_grep "PlanExecutionStep" "vac-rs/crates/surfaces/cli/src/plan_cli.rs"
require_grep "Actually execute allowed structured commands" "vac-rs/crates/surfaces/cli/src/plan_cli.rs"
require_grep "dry_run" "vac-rs/crates/surfaces/cli/src/plan_cli.rs"
require_grep "execution.yaml" "vac-rs/crates/surfaces/cli/src/plan_cli.rs"

require_file "vac-rs/crates/surfaces/cli/src/init_cli.rs"
require_grep "render_strategy_yaml\(&root, &timestamp\)" "vac-rs/crates/surfaces/cli/src/init_cli.rs"
require_grep "load_operator_choices" "vac-rs/crates/surfaces/cli/src/init_cli.rs"
require_grep "cli_non_interactive_default" "vac-rs/crates/surfaces/cli/src/init_cli.rs"
require_grep "operator_choices.yaml" "vac-rs/crates/surfaces/cli/src/init_cli.rs"

# Phase 2 evidence integrity.
require_file "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs"
require_grep "CANONICAL_EVIDENCE_INCLUDED_FIELDS" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs"
require_grep "CANONICAL_EVIDENCE_EXCLUDED_FIELDS" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs"
require_grep "canonical_payload_changes_when_each_public_field_changes" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs"
require_file "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs"
require_grep "VAC_EVIDENCE_SIGNING_REQUIRED" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs"
require_grep "missing-required-ed25519-key" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs"
require_grep "signing_required" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs"
require_grep "EvidenceSigning" "vac-rs/crates/surfaces/cli/src/doctor_cli.rs.inc"
require_grep "evidence signing required" "vac-rs/crates/surfaces/cli/src/doctor_cli.rs.inc"
require_grep "evidence:" ".vac/policies/approval.yaml"
require_grep "signing:" ".vac/policies/approval.yaml"
require_file "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs"
require_grep "trait SemanticAnchorResolver" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs"
require_grep "struct RustSynAnchorResolver" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs"
require_grep "Ambiguous" "vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs"

# Phase 3 TUI perf completion.
require_file "vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs"
require_grep "struct CellHeightKey" "vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs"
require_grep "struct HeightPrefixIndex" "vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs"
require_grep "visible_range" "vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs"
require_grep "record_windowed_render" "vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs"
require_grep "render_startup_skeleton" "vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs"
require_grep "skeleton_first_frame_rendered" "vac-rs/crates/surfaces/tui/src/full_tui_runtime.rs"
require_file "scripts/bench-vac-tui-performance.sh"
require_grep "pending_final_benchmark" "scripts/bench-vac-tui-performance.sh"

# Phase 4 operator UX.
require_file "vac-rs/crates/surfaces/tui/src/capability_dashboard.rs"
require_grep "new_capability_dashboard_view" "vac-rs/crates/surfaces/tui/src/capability_dashboard.rs"
require_grep "SelectionViewParams" "vac-rs/crates/surfaces/tui/src/capability_dashboard.rs"
require_grep "new_capability_dashboard_output" "vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_015_impl.rs"
require_grep "show_selection_view\(crate::capability_dashboard::new_capability_dashboard_view" "vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_015_impl.rs"
require_grep "new_runtime_operator_console_view" "vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_015_impl.rs"

# Phase 5 provider prune.
require_file "vac-rs/crates/providers/vac-client/Cargo.toml"
require_grep "provider-chatgpt" "vac-rs/crates/providers/vac-client/Cargo.toml"
require_file "vac-rs/crates/providers/vac-client/src/bin/provider_ca_probe.rs"
require_grep "cfg\(feature = \"provider-chatgpt\"\)" "vac-rs/crates/providers/vac-client/src/bin/provider_ca_probe.rs"
require_file "vac-rs/crates/providers/vac-client/src/lib.rs"
require_grep "compatibility remains default-off" "vac-rs/crates/providers/vac-client/src/lib.rs"
reject_grep "https://chatgpt.com/backend-api/" "vac-rs/crates/providers/vac-client/src/lib.rs"

# Phase 6 layer topology contract.
require_file "vac-rs/crates/layer-map.yaml"
require_grep "crates/surfaces/tui" "vac-rs/crates/layer-map.yaml"
require_grep "crates/foundation/utils/absolute-path" "vac-rs/crates/layer-map.yaml"
require_grep "workspace_layer_contract" ".vac/registry/architecture-layer-map.yaml"
for layer in control-plane surfaces capabilities runtime providers integrations foundation; do
  [[ -d "vac-rs/crates/$layer" ]] || fail "missing layer dir vac-rs/crates/$layer"
done

# Phase 7 debt burn-down: priority files are dispatchers now.
max_lines 80 "vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_runner/split_001_build_check.rs"
max_lines 80 "vac-rs/crates/surfaces/tui/src/bottom_pane/textarea.rs"
max_lines 80 "vac-rs/crates/capabilities/sessions/src/core_migrated/session/mod.rs"
max_lines 80 "vac-rs/crates/foundation/runtime-protocol/src/protocol/thread_history.rs"
max_lines 80 "vac-rs/crates/surfaces/tui/src/bottom_pane/request_user_input/mod.rs"
require_file ".vac/registry/technical-debt-markers.yaml"

# Docs prune should remain intact.
[[ ! -d "docs/donor-migration" ]] || fail "docs/donor-migration should stay pruned"

echo "[PASS] O5/O6 structured blueprint static gate"
