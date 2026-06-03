#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "three-audit findings gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
require_absent() { [[ ! -e "$1" ]] || fail "retired path still exists: $1"; }
reject_grep() { ! grep -qE "$1" "$2" || fail "forbidden pattern in $2: $1"; }

# Audit pack 1: server-bound prune map and local-agent defaults.
require_file vac-rs/crates/providers/vac-client/Cargo.toml
require_grep '^provider-chatgpt = \[\"vac-provider-http/provider-chatgpt\"\]' vac-rs/crates/providers/vac-client/Cargo.toml
require_file vac-rs/crates/providers/vac-client/src/bin/provider_ca_probe.rs
require_grep '#!\[cfg\(feature = "provider-chatgpt"\)\]' vac-rs/crates/providers/vac-client/src/bin/provider_ca_probe.rs
require_grep 'compatibility remains default-off' vac-rs/crates/providers/vac-client/src/lib.rs
require_grep 'LEGACY_CHATGPT_ACCOUNT_ENV' vac-rs/crates/foundation/protocol/src/auth.rs
require_grep 'allowed_for_local_coding_agent' vac-rs/crates/foundation/protocol/src/auth.rs
require_grep 'legacy_chatgpt_account_enabled' vac-rs/crates/capabilities/local-runtime-owner/src/startup.rs
require_grep 'legacy ChatGPT account local-runtime startup is disabled' vac-rs/crates/capabilities/local-runtime-owner/src/startup.rs
grep -R -qE 'legacy-chatgpt-cyber-verify-disabled' vac-rs/crates/capabilities/sessions/src/core_migrated/session && true || fail 'missing pattern in vac-rs/crates/capabilities/sessions/src/core_migrated/session: legacy-chatgpt-cyber-verify-disabled'
require_grep 'VAC_ENABLE_LEGACY_CHATGPT_AGENT_IDENTITY_JWKS' vac-rs/crates/providers/agent-identity/src/lib.rs
require_grep 'legacy-chatgpt-agent-identity-jwks-disabled' vac-rs/crates/providers/agent-identity/src/lib.rs
require_grep 'VAC_ENABLE_PROVIDER_REALTIME' vac-rs/crates/providers/provider-http/src/endpoint/realtime_call.rs
reject_grep 'chatgpt.com/explore|chatgpt.com/vac/settings' vac-rs/crates/foundation/protocol/src/error.rs
require_absent vac-rs/crates/providers/vac-api/tests

# Audit pack 2: perf/UX TUI findings beyond earlier quick wins.
require_file vac-rs/crates/surfaces/tui/src/perf_bench_harness.rs
require_grep 'perf_bench_harness_markdown_large_transcript' vac-rs/crates/surfaces/tui/src/perf_bench_harness.rs
require_grep 'perf_bench_harness' scripts/bench-vac-tui-performance.sh
require_file vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs
require_grep 'DesiredHeightCache' vac-rs/crates/surfaces/tui/src/chatwidget/height_cache.rs
require_grep 'desired_height_cache' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_010_impl.rs
require_grep 'get_or_compute' vac-rs/crates/surfaces/tui/src/chatwidget/chatwidget_group_025_epilogue.rs
require_grep 'Height-only changes do' vac-rs/crates/surfaces/tui/src/app/resize_reflow.rs
require_grep 'let should_rebuild_transcript = reflow_needed;' vac-rs/crates/surfaces/tui/src/app/resize_reflow.rs
require_grep 'runner: executed directly from manifest' vac-rs/crates/surfaces/tui/src/workflow_browser.rs
require_grep 'format_rules_requirements' vac-rs/crates/surfaces/tui/src/debug_config.rs
reject_grep 'TODO\(gt\): Expand this debug output' vac-rs/crates/surfaces/tui/src/debug_config.rs

# Audit pack 3: control-plane product surface and contract-depth findings.
require_file vac-rs/crates/surfaces/cli/src/approve_cli.rs
require_file vac-rs/crates/surfaces/cli/src/workflow_cli.rs
require_grep 'Approve\(approve_cli::ApproveCommand\)' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'Workflow\(workflow_cli::WorkflowCommand\)' vac-rs/crates/surfaces/cli/src/main.rs
require_grep 'Create\(PlanCreateCommand\)' vac-rs/crates/surfaces/cli/src/plan_cli.rs
require_grep 'Approve\(PlanApproveCommand\)' vac-rs/crates/surfaces/cli/src/plan_cli.rs
require_grep 'Execute\(PlanExecuteCommand\)' vac-rs/crates/surfaces/cli/src/plan_cli.rs
require_grep 'Abandon\(PlanAbandonCommand\)' vac-rs/crates/surfaces/cli/src/plan_cli.rs
require_grep 'operator_choices.yaml' vac-rs/crates/surfaces/cli/src/init_cli.rs
require_grep 'prompt_with_default' vac-rs/crates/surfaces/cli/src/init_cli.rs
require_grep 'requires_cli_interactive_flag: false' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'canonical_evidence_yaml' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs
require_grep 'commands_executed:' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_chain.rs
require_grep 'VAC_EVIDENCE_ED25519_SIGNING_KEY_BASE64' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs
require_grep 'optional_ed25519_signature_with_policy' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_evidence_writer.rs
require_grep 'RustSynAnchorResolver|StrictAst' vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_patch_guard.rs

printf 'three-audit findings gate: PASS\n'
