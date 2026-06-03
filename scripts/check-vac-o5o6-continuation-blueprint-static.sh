#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail(){ echo "continuation blueprint gate: $*" >&2; exit 1; }
require_file(){ [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep(){ grep -qE "$1" "$2" || fail "missing pattern $1 in $2"; }
# Phase 0 ledger
require_file .vac/registry/implementation-blueprint.yaml
require_file .vac/registry/validation-ledger.yaml
require_file .vac/registry/compile-debt-ledger.yaml
require_grep 'o5o6.continuation_blueprint_all_batches' .vac/registry/implementation-blueprint.yaml
require_grep 'cargo_check: TV-Pending' .vac/registry/implementation-blueprint.yaml
# Phase A control-plane depth
cp="vac-rs/crates/control-plane/control-plane/src/control_plane"
cli="vac-rs/crates/surfaces/cli/src"
doctor="$cli/doctor_cli.rs.inc"
require_file "$cp/vac_init_patch_guard.rs"
require_file "$cp/semantic_anchor.rs"
require_grep 'pub enum SemanticAnchorMode' "$cp/vac_init_patch_guard.rs"
require_grep 'StrictAst' "$cp/vac_init_patch_guard.rs"
require_grep 'resolve_semantic_anchor_in_source_strict' "$cp/vac_init_patch_guard.rs"
require_grep 'doc comments and stacked attributes' "$cp/vac_init_patch_guard.rs"
require_grep 'canonical_evidence_field_coverage_report' "$cp/vac_init_evidence_chain.rs"
require_grep 'CanonicalEvidencePayload' "$cp/vac_init_evidence_chain.rs"
require_grep 'EVIDENCE_RECORD_PUBLIC_FIELDS' "$cp/vac_init_evidence_chain.rs"
require_grep 'canonical_field_coverage_report_is_complete' "$cp/vac_init_evidence_chain.rs"
require_grep 'render_evidence_yaml_with_policy' "$cp/vac_init_evidence_writer.rs"
require_grep 'evidence_signing_required_for_root' "$cp/vac_init_evidence_writer.rs"
require_file .vac/policies/evidence-signing.yaml
require_grep 'canonical_coverage' "$doctor"
require_grep 'LayeringDoctorCommand' "$doctor"
require_grep 'DoctorSubcommand::Layering' "$doctor"
require_grep 'ExecutionSandboxProfile' "$cli/plan_cli.rs"
require_grep 'run_sandboxed_command' "$cli/plan_cli.rs"
require_grep 'max_stdout_bytes' "$cli/plan_cli.rs"
require_grep 'sandbox_profile_hash' "$cli/plan_cli.rs"
# Phase B provider prune
bash scripts/check-vac-provider-prune-default-off-static.sh >/dev/null
# Phase C physical layering
bash scripts/check-vac-layering-migration-static.sh >/dev/null
# Phase D/E TUI perf/UX
TUI="vac-rs/crates/surfaces/tui/src"
require_grep 'WindowedTranscriptRenderPlan' "$TUI/chatwidget/height_cache.rs"
require_grep 'windowed_render_plan' "$TUI/chatwidget/split_024_thread_id.rs"
require_file .vac/registry/perf/tui-render.yaml
require_file .vac/registry/perf/startup-ttff.yaml
require_file .vac/registry/perf/event-stream-load.yaml
require_file .vac/registry/perf/runtime-event-persistence.yaml
require_grep 'event-stream-load.yaml' scripts/bench-vac-tui-performance.sh
require_grep 'searchable_shortcut_overlay' "$TUI/key_hint.rs"
require_grep 'feature-off experimental unavailable badge' "$TUI/slash_command.rs"
require_grep 'explicit_auth_state_no_silent_skip' "$TUI/lib.rs"
# Phase F/G debt
require_file .vac/registry/fragmentation-migration-ledger.yaml
require_file .vac/registry/unbounded-channel-allowlist.yaml
require_file .vac/registry/panic-risk-governance.yaml
require_file .vac/registry/technical-debt-markers.yaml
python3 - <<'PY'
from pathlib import Path
files=[
'vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_runner/build_check_parts/build_check_part_006.rs',
'vac-rs/crates/surfaces/tui/src/bottom_pane/textarea.rs',
'vac-rs/crates/capabilities/sessions/src/core_migrated/session/mod.rs',
'vac-rs/crates/foundation/runtime-protocol/src/protocol/thread_history.rs',
'vac-rs/crates/surfaces/tui/src/bottom_pane/request_user_input/mod.rs',
]
for f in files:
    p=Path(f)
    if not p.exists():
        raise SystemExit(f'missing priority dispatcher {f}')
    lines=p.read_text(errors='ignore').splitlines()
    if len(lines)>120:
        raise SystemExit(f'{f} still giant: {len(lines)} lines')
    if 'include!' not in p.read_text(errors='ignore'):
        raise SystemExit(f'{f} is not dispatcher-backed')
PY
bash scripts/check-vac-o5o6-actual-code-closure-static.sh >/dev/null
echo "continuation blueprint gate: PASS"
