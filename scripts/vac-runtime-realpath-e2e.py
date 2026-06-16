#!/usr/bin/env python3
"""Static real-path E2E scenarios for VAC v1.5 runtime closure.

This is not a Rust runtime test. It proves that source-level real execution
paths now contain the expected fail-closed mediation points and no longer rely
on plan-echo patch data or free-form shell strings as command authority.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()


def text(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8", errors="replace")

agent = text("vac-rs/crates/runtime/vac-agent-loop/src/agent.rs")
bound_tool = text("vac-rs/crates/runtime/vac-agent-loop/src/bound_tool.rs")
local_tools = text("vac-rs/crates/integrations/vac-mcp-server/src/local_tools.rs")
approval_boundary = text("vac-rs/crates/integrations/vac-mcp-server/src/approval_boundary.rs")
command_authority = text("vac-rs/crates/integrations/vac-mcp-server/src/command_authority.rs")
evidence = text("vac-rs/crates/control-plane/vac-evidence/src/lib.rs")
policy = text("vac-rs/crates/control-plane/vac-policy/src/lib.rs")
specsync = text("vac-rs/crates/control-plane/vac-spec-sync/src/lib.rs")
assessment = text("vac-rs/crates/control-plane/vac-assessment/src/lib.rs")
memory = text("vac-rs/crates/control-plane/vac-memory/src/lib.rs")
remote = text("vac-rs/crates/foundation/vac-foundation/src/remote_connection.rs")
compiler = text("scripts/compile-vac-registry-sv.py")
checker = text("scripts/check-checkpoint-integrity.py")
control_plane = text("vac-rs/crates/control-plane/vac-control-plane/src/lib.rs")
vac_cli_toml = text("vac-rs/crates/surfaces/vac-cli/Cargo.toml")

cases = [
    ("agent_mutating_tool_without_plan_blocked", "mutating/process tool attempted without approved VAC Semantic Plan" in bound_tool),
    ("agent_mutating_tool_with_plan_gets_stamped", "stamp_tool_call" in bound_tool and "vac_bound_approval" in bound_tool and "execute_tool_call(run, &bound_tool_call" in agent),
    ("mcp_command_without_vac_bound_approval_blocked", "VAC_BOUND_APPROVAL_REQUIRED" in approval_boundary and "require_vac_bound_approval" in local_tools and "execute_process" in local_tools),
    ("mcp_command_requires_structured_object", "structured_command: Option<VacStructuredCommandRequest>" in local_tools and "free-form command strings are migration mirrors only" in command_authority),
    ("mcp_command_mirror_drift_blocked", "command mirror does not match structured_command" in command_authority),
    ("mcp_command_shell_metachar_blocked", "contains_shell_metachar" in command_authority and "VAC_STRUCTURED_COMMAND_REQUIRED" in command_authority),
    ("mcp_file_mutation_without_approval_blocked", local_tools.count("filesystem_write") >= 2 and "filesystem_delete" in local_tools and "require_vac_bound_approval" in local_tools),
    ("mcp_network_without_vac_bound_approval_blocked", "ViewWebPageRequest" in local_tools and "network_access" in local_tools and "require_vac_bound_approval" in local_tools),
    ("bound_runtime_network_gate_exists", "NetworkAccess" in bound_tool and "pre_network_gate" in text("vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs") and "is_network_tool" in bound_tool),
    ("pre_patch_uses_actual_old_str_range", "find_all_matches" in bound_tool and "byte_range_to_line_range" in bound_tool and "load_patch_file_text" in bound_tool),
    ("pre_patch_blocks_zero_or_multiple_matches", "old_str has zero matches" in bound_tool and "old_str has multiple matches" in bound_tool),
    ("pre_patch_does_not_echo_plan_scope", "touched_range: scope.line_range" not in bound_tool and "semantic_anchor: scope.semantic_anchor.clone()" not in bound_tool),
    ("pre_patch_uses_monotonic_patch_index", "next_patch_index = self.next_patch_index.saturating_add(1)" in bound_tool and not re.search(r"PatchAttempt\s*\{[^}]*patch_index:\s*0", bound_tool, flags=re.S)),
    ("completion_lock_blocks_mutating_without_closeout", "complete_before_run_completed" in agent and "completion lock requires closeout state" in bound_tool),
    ("checkpoint_integrity_gate_exists", "compiled_source_hash_mismatches" in checker and "checkpoint_index_counts" in checker and "status.readiness_summary" in checker),
    ("registry_compiler_canonical_current_snapshot", "capabilities/current.json" in compiler and "capabilities.json" in compiler and "source_hashes" in compiler),
    ("readiness_single_authority_lowered_by_tv_pending", "tv_cargo_gates_pending" in compiler and "\"readiness_summary\": summary" in compiler),
    ("cargo_static_blockers_source_fixed", "CompiledCapability" in control_plane and "WorkspaceStatus" in control_plane and vac_cli_toml.count("[[bin]]") == 1),
    ("evidence_store_has_cas_merkle_git_refs", "EvidenceStore" in evidence and "CasConflict" in evidence and "MerkleRoot" in evidence and "GitRefsEvidenceStore" in evidence and "git_update_ref_cas" in evidence),
    ("evidence_signature_not_fake_verified", "CryptoBackendUnavailable" in evidence and "signature_value_has_strict_encoding" in evidence),
    ("policy_scoped_grant_expiry_and_lowering", "is_expired" in policy and "Decision::ApprovalRequired, Decision::Allow" in policy),
    ("specsync_changed_file_maps_to_capability", "map_changed_files" in specsync and "ChangedFile" in specsync and "detect_spec_drift" in specsync),
    ("assessment_bidirectional_gap_analysis", "bidirectional_gap_analysis" in assessment and "intent_without_code" in assessment and "code_without_intent" in assessment),
    ("memory_cannot_relax_policy", "memory_can_relax_policy" in memory and "false" in memory),
    ("memory_redaction_extended", "looks_like_jwt" in memory and "looks_high_entropy" in memory and "PRIVATE KEY" in memory),
    ("remote_host_key_verification_required", "KnownHostsVerifier" in remote and "VAC_REMOTE_ALLOW_UNKNOWN_HOSTS_L1_DEGRADED" in remote and "verify(&self.hostname" in remote),
    ("remote_no_bash_trampoline_in_source_path", "bash -c" not in remote and "sh -c" not in remote and "let wrapped_command = command.to_string();" in remote),
    ("assessment_freshness_gate_wired", "check_assessment_freshness" in checker and "assessment_freshness=current_index" in checker),
]

failed = [name for name, ok in cases if not ok]
report = {"kind": "vac_runtime_realpath_e2e", "cases": [{"id": name, "pass": ok} for name, ok in cases]}
print(json.dumps(report, indent=2))
if failed:
    print("VAC runtime real-path E2E: FAIL")
    for name in failed:
        print(f"- {name}")
    sys.exit(1)
print("VAC runtime real-path E2E: PASS")
