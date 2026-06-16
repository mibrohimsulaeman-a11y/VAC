#!/usr/bin/env python3
"""State4 adversarial SV gate for VAC Runtime v1.5.

This is source/static validation only. It encodes the manual State4 blockers so a
future checkpoint cannot pass SV while reintroducing: tool-supplied policy
authority, tool-supplied patch preimage trust, or ungated read/remote `view`.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()


def read(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8", errors="replace")


def exists(rel: str) -> bool:
    return (ROOT / rel).exists()

bound_tool = read("vac-rs/crates/runtime/vac-agent-loop/src/bound_tool.rs")
bound_runtime = read("vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs")
local_tools = read("vac-rs/crates/integrations/vac-mcp-server/src/local_tools.rs")
read_authorization = read("vac-rs/crates/integrations/vac-mcp-server/src/read_authorization.rs")
file_operations = read("vac-rs/crates/integrations/vac-mcp-server/src/file_operations.rs")
evidence = read("vac-rs/crates/control-plane/vac-evidence/src/lib.rs")
compiler = read("scripts/compile-vac-registry-sv.py")
lib_rs = read("vac-rs/crates/runtime/vac-agent-loop/src/lib.rs")
bootstrap = read("vac-rs/crates/runtime/vac-agent-loop/src/session_bootstrap.rs") if exists("vac-rs/crates/runtime/vac-agent-loop/src/session_bootstrap.rs") else ""
fixture_path = ROOT / "tests/fixtures/runtime/v15-state4-adversarial/cases.json"
fixtures = json.loads(fixture_path.read_text()) if fixture_path.exists() else {"cases": []}
fixture_ids = {case.get("id") for case in fixtures.get("cases", [])}

cases: list[tuple[str, bool, str]] = []
add = cases.append

add((
    "tool_supplied_policy_decision_rejected",
    "tool-supplied policy_decision is not authority" in bound_tool
    and "obj.contains_key(\"policy_decision\")" in bound_tool
    and "pub policy_decision: Option<String>" not in local_tools,
    "structured command must reject policy_decision in tool/MCP payloads",
))
add((
    "compiled_policy_computed_inside_runtime",
    "compute_command_policy_decision" in bound_runtime
    and "command.policy_decision =" in bound_tool
    and "controller.compute_command_policy_decision" in bound_tool
    and "policy_rules" in bound_runtime
    and "command_registry" in bound_runtime,
    "runtime must compute process policy from compiled registry/policy snapshots",
))
add((
    "unregistered_runner_denied",
    "if !(in_plan || in_capability || registered)" in bound_runtime
    and "return PolicyDecision::Deny" in bound_runtime,
    "runner/command id not in plan/capability/registry must deny",
))
add((
    "patch_current_file_text_rejected",
    "current_file_text" in bound_tool
    and "tool-supplied patch preimage field" in bound_tool
    and "#[serde(deny_unknown_fields)]\npub struct StrReplaceRequest" in local_tools,
    "current_file_text must be rejected before pre-patch gate",
))
add((
    "patch_file_text_before_rejected",
    "file_text_before" in bound_tool
    and "tool-supplied patch preimage field" in bound_tool,
    "file_text_before must be rejected before pre-patch gate",
))
add((
    "patch_broker_reads_workspace_preimage",
    "fs::read_to_string(&full_path)" in bound_tool
    and "tool_call.arguments.get(\"current_file_text\")" not in bound_tool
    and "tool_call.arguments.get(\"file_text_before\")" not in bound_tool,
    "patch preimage must be read from workspace storage, not LLM arguments",
))
add((
    "replace_all_requires_separate_bounded_operation",
    "replace_all is a separate explicitly bounded operation" in bound_tool
    and "pub replace_all: Option<bool>" in local_tools,
    "replace_all must not share a single-range approval",
))
add((
    "view_requires_read_plan_or_approval",
    "is_read_tool" in bound_tool
    and "pre_read_gate" in bound_runtime
    and "VAC_READ_GOVERNANCE_REQUIRED" in read_authorization
    and "read_plan_ticket: Option<VacReadPlanTicket>" in local_tools,
    "view must be classified as governed read and require read_plan_ticket or approval",
))
add((
    "view_remote_credentials_require_policy",
    "credential_read" in read_authorization
    and "credential_material_present" in bound_runtime
    and "compute_read_policy_decision" in bound_runtime
    and "network_access" in read_authorization,
    "remote/credential view must pause/block through network/credential policy",
))
add((
    "view_output_redacted_before_model",
    file_operations.count("sanitize_text_output(&result)") >= 3
    and "sanitize_text_output(&markdown_content)" in local_tools,
    "view/web outputs must be redacted before returning to the model",
))
add((
    "broker_session_bootstrap_attaches_vac_runtime",
    "VacRuntimeMetadataBootstrap" in bootstrap
    and "load_compiled_registry_snapshot" in bootstrap
    and "attach_semantic_plan" in bootstrap
    and "attach_mandatory_artifacts" in bootstrap
    and "initialize_closeout" in bootstrap
    and "pub mod session_bootstrap" in lib_rs,
    "session bootstrap must assemble vac_runtime metadata from compiled registry/plan/artifacts",
))
add((
    "compiled_registry_contains_policy_and_read_authority",
    "runtime_registry_snapshot" in compiler
    and "compile_policy_rules" in compiler
    and "compile_command_registry_from_caps" in compiler
    and "compile_read_plan_tickets" in compiler,
    "compiled registry must expose policy rules, command registry, and read-plan tickets",
))
add((
    "git_refs_use_git_blob_oid_not_content_sha",
    "git_hash_object_write" in evidence
    and "git hash-object" not in evidence.lower()  # checked by function body tokens below
    and 'arg("hash-object")' in evidence
    and 'arg("update-ref")' in evidence
    and "is_git_object_id" in evidence
    and "new_hash.trim_start_matches(\"sha256:\")" not in evidence,
    "refs/vac/evidence/* must update to Git object IDs, not VAC content sha256 strings",
))
add((
    "checkpoint_manifest_self_count_hygiene",
    exists("scripts/generate-checkpoint-manifest.py")
    and "CHECKPOINT_MANIFEST.json" in read("scripts/generate-checkpoint-manifest.py"),
    "checkpoint manifest generator must exist for final tree counts",
))

missing_fixtures = sorted(name for name, _ok, _why in cases if name not in fixture_ids)
failed = [(name, why) for name, ok, why in cases if not ok]

report = {
    "kind": "vac_runtime_state4_adversarial_sv",
    "cases": [{"id": name, "pass": ok, "why": why} for name, ok, why in cases],
    "missing_fixture_ids": missing_fixtures,
}
print(json.dumps(report, indent=2))

if missing_fixtures or failed:
    print("VAC runtime State4 adversarial SV: FAIL")
    for name in missing_fixtures:
        print(f"- missing fixture id: {name}")
    for name, why in failed:
        print(f"- {name}: {why}")
    sys.exit(1)
print("VAC runtime State4 adversarial SV: PASS")
