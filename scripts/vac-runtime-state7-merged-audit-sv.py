#!/usr/bin/env python3
from __future__ import annotations
import json
import pathlib
import re
import sys
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 local TV gate compatibility.
    import tomli as tomllib

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
failures: list[str] = []

def read(rel: str) -> str:
    p = ROOT / rel
    try:
        return p.read_text()
    except Exception as exc:
        failures.append(f"missing/unreadable {rel}: {exc}")
        return ""

def require(cond: bool, msg: str) -> None:
    if not cond:
        failures.append(msg)

def require_text(rel: str, needles: list[str]) -> str:
    text = read(rel)
    for needle in needles:
        require(needle in text, f"{rel} missing {needle!r}")
    return text

# A1: approval v2 must be propagated to MCP and bound to actual payload/diff.
bound_tool = require_text("vac-rs/crates/runtime/vac-agent-loop/src/bound_tool.rs", [
    "tool_arguments_without_bound_approval",
    '"approval_request_id"',
    '"tool_call_id"',
    '"plan_hash"',
    '"diff_hash"',
    '"policy_snapshot_hash"',
    '"nonce"',
    '"expires_at"',
    '"binding_hash"',
])
require('"arguments": tool_arguments_without_bound_approval(tool_call)' in bound_tool, "approval diff_hash must exclude vac_bound_approval and include real tool arguments")
require('"schema_version": 2' in bound_tool, "agent stamp must emit vac_bound_approval schema_version=2")

local_tools = require_text("vac-rs/crates/integrations/vac-mcp-server/src/local_tools.rs", [
    "approval_boundary",
    "require_vac_bound_approval",
    "VacBoundApproval",
])
approval_boundary = require_text("vac-rs/crates/integrations/vac-mcp-server/src/approval_boundary.rs", [
    "pub struct VacBoundApproval",
    "schema_version: u32",
    "approval_request_id",
    "tool_call_id",
    "plan_hash",
    "diff_hash",
    "policy_snapshot_hash",
    "nonce",
    "expires_at",
    "vac_diff_hash_matches",
    "arguments_without_vac_bound_approval",
    "vac_jcs::canonical_json_sha256",
])
require('approval.schema_version != 2' in approval_boundary, "MCP verifier must reject non-v2 approvals")
require('actual_arguments' in approval_boundary and 'diff_hash mismatch against actual MCP tool arguments' in approval_boundary, "MCP verifier must recompute approval against actual payload")
require('strip_nulls' in approval_boundary, "MCP verifier should tolerate null-vs-omitted canonical optional fields without allowing argument drift")

# A3: needs_discussion must be operator pause, not technical run error.
types = require_text("vac-rs/crates/runtime/vac-agent-loop/src/types.rs", [
    "NeedsOperatorInput",
    "PausedForDiscussion",
])
agent = require_text("vac-rs/crates/runtime/vac-agent-loop/src/agent.rs", [
    "GateDecision::NeedsDiscussion",
    "AgentEvent::PausedForDiscussion",
    "StopReason::NeedsOperatorInput",
])
require('blocking_reason' in agent and 'operator_visible_status' in agent, "PausedForDiscussion event must carry operator-visible blocking context")

# A4/A2: readiness weakest semantics and fixture field freshness.
bound_runtime = require_text("vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs", [
    "effective_lowered_to_declared_computed_weakest",
    "policy_rules: Vec::new()",
    "policy_snapshot: None",
    "command_registry: Vec::new()",
    "read_plan_tickets: Vec::new()",
])
require("self.declared.min(self.computed)" in bound_runtime, "runtime readiness must use weakest(declared, computed) before effective")
runtime_e2e = require_text("vac-rs/crates/runtime/vac-agent-loop/src/runtime_e2e.rs", [
    "policy_rules: Vec::new()",
    "policy_snapshot: None",
    "command_registry: Vec::new()",
    "read_plan_tickets: Vec::new()",
])

# A5/single source: JCS utility and path/glob utility must be reused by product paths.
require((ROOT / "vac-rs/crates/foundation/vac-jcs/src/lib.rs").is_file(), "vac-jcs crate missing")
for rel in [
    "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs",
    "vac-rs/crates/integrations/vac-mcp-server/src/approval_boundary.rs",
    "vac-rs/crates/control-plane/vac-evidence/src/lib.rs",
    "vac-rs/crates/control-plane/vac-registry-compiler/src/lib.rs",
]:
    require("vac_jcs::" in read(rel), f"{rel} must use centralized vac-jcs hashing")
require_text("vac-rs/crates/foundation/vac-paths/src/lib.rs", ["pub fn path_matches", "pub fn globish_matches"])
require("vac_paths::path_matches" in read("vac-rs/crates/control-plane/vac-spec-sync/src/lib.rs"), "SpecSync must use centralized vac-paths matcher")
require("vac_paths::path_matches" in bound_runtime and "vac_paths::globish_matches" in bound_runtime, "Bound runtime must use centralized vac-paths matcher")

# B1: keystone crates must be called by product CLI/doctor path.
cli_toml = tomllib.loads(read("vac-rs/crates/surfaces/vac-cli/Cargo.toml"))
cli_deps = cli_toml.get("dependencies", {})
for crate in ["vac-assessment", "vac-index", "vac-spec-sync", "vac-doctor", "vac-evidence", "vac-registry-compiler", "vac-readiness", "vac-jcs"]:
    require(crate in cli_deps, f"vac-cli missing product dependency {crate}")
control_plane = require_text("vac-rs/crates/surfaces/vac-cli/src/commands/control_plane.rs", [
    "vac_registry_compiler::compile_registry_from_disk",
    "vac_assessment::validate_gap_report_value",
    "vac_index::parser_mode_truth",
    "vac_spec_sync::detect_symbol_invariant_drift",
    "vac_doctor::product_path_gate",
    "vac_evidence::classify_evidence_authority",
    "doctor_enforcement",
    "fixtures_and_sandbox_sv_helpers_only",
])

# B2/B6/B7: index/assessment must be honest about Rust AST default and remaining partiality.
index_script = require_text("scripts/generate-deterministic-index-sv.py", [
    "rust_ast_default_with_fail_closed_fallback",
    "fail_closed_static_heuristic",
    "ast_grounded_default_index",
    "calls_lightweight",
    "complete_call_graph",
])
require("rust_ast_default" in index_script, "index generator must wire rust_ast as the Rust default lane")
require("Not claimed" in index_script and "SV-Partial" in index_script, "index generator must keep call graph partial/non-claim wording")
require("not_parsed_fail_closed" in index_script, "index generator must downgrade Rust parse failures instead of claiming clean absence")
assessment_script = require_text("scripts/generate-assessment-report-sv.py", [
    "sv_static_heuristic_scaffold",
    "heuristic_index_with_deterministic_rubric",
    "Static heuristic severity must not be marketed as AST-grounded product scoring",
])
assessment_crate = require_text("vac-rs/crates/control-plane/vac-assessment/src/lib.rs", [
    "AssessmentAuthoritySummary",
    "heuristic_severity",
    "p1_blocks_without_waiver",
])
index_crate = require_text("vac-rs/crates/control-plane/vac-index/src/lib.rs", [
    "ParserModeTruth",
    "do not advertise AST-grounded product scoring",
])

# B3/B8: lint inheritance and secret panic hardening.
for rel in [
    "vac-rs/core/Cargo.toml",
    "vac-rs/crates/foundation/vac-foundation/Cargo.toml",
    "vac-rs/crates/integrations/vac-mcp-proxy/Cargo.toml",
    "vac-rs/crates/integrations/vac-remote-service/Cargo.toml",
    "vac-rs/crates/providers/vac-provider-core/Cargo.toml",
    "vac-rs/crates/runtime/vac-agent-loop/Cargo.toml",
    "vac-rs/crates/runtime/vac-broker/Cargo.toml",
]:
    data = tomllib.loads(read(rel))
    require(data.get("lints", {}).get("workspace") is True, f"{rel} missing [lints] workspace = true")
secret_prod = read("vac-rs/crates/foundation/vac-foundation/src/secrets/mod.rs").split("#[cfg(test)]")[0]
gitleaks_prod = read("vac-rs/crates/foundation/vac-foundation/src/secrets/gitleaks.rs").split("#[cfg(test)]")[0]
for token in ["panic!", ".unwrap()", ".expect("]:
    require(token not in secret_prod, f"secret redaction production path still contains {token}")
    require(token not in gitleaks_prod, f"gitleaks production path still contains {token}")

# B4: legacy destructive classifier must be marked non-authority.
legacy = require_text("vac-rs/crates/runtime/vac-shell-approval/src/vac_destructive.rs", [
    "legacy_heuristic_fixture_only_not_runtime_authority",
    "Use structured command + vac-policy + BoundRuntimeController",
])

# A6/A8/A9: SpecSync semantic drift, L1 honesty, and generated/recorded timestamp split.
require_text("vac-rs/crates/control-plane/vac-spec-sync/src/lib.rs", [
    "SymbolIntentInvariant",
    "detect_symbol_invariant_drift",
])
require("enforcement_level={level}; VAC remains L1 advisory/cooperative" in control_plane, "doctor enforcement must preserve L1 honesty")
require("deterministic_generated_at" in index_script and "evidence_recorded_at" in index_script, "index generator must split deterministic and event timestamps")
require("deterministic_generated_at" in assessment_script and "evidence_recorded_at" in assessment_script, "assessment generator must split deterministic and event timestamps")

if failures:
    print("VAC runtime State7 merged-audit SV: FAIL")
    for failure in failures:
        print(f"- {failure}")
    sys.exit(1)
print("VAC runtime State7 merged-audit SV: PASS")
print("closed_runtime_findings=A1,A2,A3,A4,A5,A6,A9")
print("closed_assessment_findings=B1,B2,B3,B4,B5,B6,B7,B8")
print("tv=NotEvaluated")
print("l2=NotImplemented")
