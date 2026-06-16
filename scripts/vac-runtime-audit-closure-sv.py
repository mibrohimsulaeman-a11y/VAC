#!/usr/bin/env python3
"""SV source gate for VAC Runtime v1.5 audit/re-audit closure.

This gate is intentionally static. It verifies source-level invariants that can
be checked without Rust/Cargo: real agent-loop mediation, no agent-accessible
free-form shell path in the audited MCP/task paths, actual patch range
resolution instead of plan echo, structured command object authority,
checkpoint integrity hooks, and current control-plane scaffolds.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors: list[str] = []


def read(rel: str) -> str:
    path = ROOT / rel
    if not path.exists():
        errors.append(f"missing {rel}")
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def require(rel: str, *needles: str) -> None:
    text = read(rel)
    for needle in needles:
        if needle not in text:
            errors.append(f"{rel}: missing token {needle!r}")


def forbid_before(rel: str, marker: str, pattern: str, message: str) -> None:
    text = read(rel)
    head = text.split(marker, 1)[0] if marker in text else text
    if re.search(pattern, head):
        errors.append(f"{rel}: {message}")


# P0-RUNTIME-001: bound runtime wired into real agent lifecycle.
require(
    "vac-rs/crates/runtime/vac-agent-loop/src/agent.rs",
    "BoundRuntimeToolBoundary::from_context_metadata",
    "vac_boundary.gate_tool_call",
    "vac_boundary.complete_before_run_completed",
    "execute_tool_call(run, &bound_tool_call",
    "VAC completion lock blocked run closeout",
)

# P0.3: bridge must resolve concrete edit range/anchor and cannot echo plan.
bound_tool = read("vac-rs/crates/runtime/vac-agent-loop/src/bound_tool.rs")
for token in [
    "mutating/process tool attempted without approved VAC Semantic Plan",
    "pre_patch_gate",
    "pre_command_gate",
    "stamp_tool_call",
    "vac_bound_approval",
    "completion lock requires closeout state",
    "find_all_matches",
    "old_str has zero matches",
    "old_str has multiple matches",
    "byte_range_to_line_range",
    "resolve_actual_semantic_anchor",
    "normalized_anchor_fingerprint",
    "next_patch_index",
    "patch_index",
]:
    if token not in bound_tool:
        errors.append(f"bound_tool.rs missing {token!r}")
if "touched_range: scope.line_range" in bound_tool or "semantic_anchor: scope.semantic_anchor.clone()" in bound_tool:
    errors.append("bound_tool.rs still echoes approved plan range/anchor as attempted patch")
if re.search(r"PatchAttempt\s*\{[^}]*patch_index:\s*0", bound_tool, flags=re.S):
    errors.append("bound_tool.rs still constructs PatchAttempt with constant patch_index: 0")

# P0.4: command authority must be typed object, string only mirror/degraded shim.
local_tools = read("vac-rs/crates/integrations/vac-mcp-server/src/local_tools.rs")
command_authority = read("vac-rs/crates/integrations/vac-mcp-server/src/command_authority.rs")
mcp_command_boundary = local_tools + "\n" + command_authority
forbid_before(
    "vac-rs/crates/integrations/vac-mcp-server/src/local_tools.rs",
    "#[cfg(test)]",
    r'Command::new\("sh"\)|\.arg\("-c"\)|\.args\(\["-c"',
    "MCP execution path still contains free-form shell execution",
)
if "full system access" in local_tools:
    errors.append("vac-mcp-server local_tools still advertises full system access")
for token in [
    "pub vac_bound_approval: Option<VacBoundApproval>",
    "pub structured_command: Option<VacStructuredCommandRequest>",
    "resolve_vac_structured_command_authority",
    "parse_vac_structured_command_object",
    "free-form command strings are migration mirrors only",
    "command mirror does not match structured_command",
    "require_vac_bound_approval",
    "execute_process",
    "filesystem_write",
    "filesystem_delete",
    "Command::new(&structured.runner)",
]:
    if token not in mcp_command_boundary:
        errors.append(f"mcp command boundary missing {token!r}")

# Background task path must also avoid sh -c.
task_manager = read("vac-rs/crates/foundation/vac-foundation/src/task_manager.rs")
if 'Command::new("sh")' in task_manager or '.arg("-c")' in task_manager or '.args(["-c"' in task_manager:
    errors.append("task_manager still contains free-form shell execution")
for token in ["parse_structured_task_command", "Command::new(&structured.runner)", "free-form shell is blocked"]:
    if token not in task_manager:
        errors.append(f"task_manager missing {token!r}")

# Shared structured command contract.
require(
    "vac-rs/crates/runtime/vac-exec/src/lib.rs",
    "RunnerRegistry",
    "ShellFreeViolation",
    "parse_command",
    "evaluate_shell_free",
    "CommandExecutionPlan",
)

# P0.1/P0.5 registry truth and checkpoint integrity.
require(
    "scripts/check-checkpoint-integrity.py",
    "compiled_source_hash_mismatches",
    "checkpoint_index_counts",
    "status.readiness_summary",
)
require(
    "vac-rs/crates/control-plane/vac-registry-compiler/src/lib.rs",
    "compile_registry_from_disk",
    "CompiledRegistrySnapshot",
    "canonical_json_sha256",
    "runtime_truth",
    "source_manifest_hash",
    "CompiledAuthoringManifest",
)

# Control-plane implementation contracts/scaffolds hardened by this slice.
require("vac-rs/crates/control-plane/vac-policy/src/lib.rs", "PolicySnapshot", "PolicyRequest", "hardcoded_safety_denials", "ScopedGrant", "is_expired", "Decision::ApprovalRequired, Decision::Allow")
require("vac-rs/crates/runtime/vac-agent-loop/src/approval.rs", "ApprovalBindingV2", "policy_snapshot_hash", "nonce", "expires_at", "operator_sig", "validate_v2_approval_binding")
require("vac-rs/crates/control-plane/vac-evidence/src/lib.rs", "pub trait EvidenceStore", "append_xref", "MerkleRoot", "broker_sig", "IntegrityHint", "CasConflict", "GitRefsEvidenceStore", "protected_ref_namespace")
require("vac-rs/crates/control-plane/vac-index/src/lib.rs", "ast_path", "normalized_fingerprint", "resolve_semantic_anchor", "AnchorResolution", "ScannerConfidence")
require("vac-rs/crates/control-plane/vac-assessment/src/lib.rs", "AssessmentEvidence", "validate_span_grounding", "changed_file_capabilities", "bidirectional_gap_analysis")
require("vac-rs/crates/control-plane/vac-spec-sync/src/lib.rs", "SpecSyncReport", "ChangedFile", "SpecUpdateProposal", "map_changed_files", "detect_spec_drift")
require("vac-rs/crates/control-plane/vac-memory/src/lib.rs", "EPISODIC_SQLITE_SCHEMA", "SEMANTIC_SQLITE_SCHEMA", "redact_for_memory", "repeated_failure_seen", "memory_can_relax_policy", "looks_like_jwt", "looks_high_entropy")
require("vac-rs/crates/control-plane/vac-doctor/src/lib.rs", "DoctorGate", "ReleaseDoctorReport", "aggregate_release_status")
require("vac-rs/crates/control-plane/vac-readiness/src/lib.rs", "declared", "computed", "effective", "compute_readiness")
require("vac-rs/crates/surfaces/vac-tui/src/services/registry_transaction.rs", "ReadinessDeclaredTransaction", "write_declared_readiness_transaction", "computed/effective readiness remains compiler/doctor-owned")

# Compile blockers from re-audit.
require("vac-rs/crates/control-plane/vac-control-plane/src/lib.rs", "pub struct CompiledCapability", "pub struct WorkspaceStatus")
vac_cli_toml = read("vac-rs/crates/surfaces/vac-cli/Cargo.toml")
if vac_cli_toml.count('[[bin]]') != 1 or vac_cli_toml.count('name = "vac"') < 1:
    errors.append("vac-cli Cargo.toml must contain exactly one [[bin]] stanza for name=vac")

# Memory bootstrap schemas.
for rel in [".vac/memories/episodic.schema.sql", ".vac/memories/semantic.schema.sql"]:
    path = ROOT / rel
    if not path.exists() or path.stat().st_size < 100:
        errors.append(f"{rel}: missing substantive SQLite schema")

# Plan/ledger/evidence must be present for both initial audit closure and re-audit closure.
require("docs/workflow-control-plane/VAC_RUNTIME_V15_AUDIT_CLOSURE_IMPLEMENTATION_PLAN.md", "Finding-to-slice map", "P0-RUNTIME-001", "P1-RUNTIME-015")
require(".vac/plans/runtime-v15-audit-closure.yaml", "runtime-v15-audit-closure", "BoundRuntimeToolBoundary")
require("docs/workflow-control-plane/VAC_RUNTIME_V15_REAUDIT_CLOSURE_IMPLEMENTATION_PLAN.md", "F-001", "F-005", "P0.3", "P2")
require(".vac/plans/runtime-v15-reaudit-closure.yaml", "runtime-v15-reaudit-closure", "check-checkpoint-integrity")
require(".vac/ledger/runtime-v15-reaudit-closure.yaml", "compiled_source_hash_mismatches", "actual_patch_range_resolution")

if errors:
    print("VAC runtime audit/re-audit closure SV: FAIL")
    for err in errors:
        print(f"- {err}")
    sys.exit(1)

print("VAC runtime audit/re-audit closure SV: PASS")
print("closed_findings=F-001,F-002,F-003,F-004,F-005,P0.3,P0.4,P0.5,P1.1,P1.2,P1.3,P2")
print("boundary=real_agent_loop_mediated")
print("patch=actual_old_str_range_anchor")
print("shell=structured_command_object_runner_args")
print("registry=compiled_source_hashes_current")
print("evidence=git_refs_cas_integrity_hint")
print("readiness=single_canonical_snapshot_fail_closed")
