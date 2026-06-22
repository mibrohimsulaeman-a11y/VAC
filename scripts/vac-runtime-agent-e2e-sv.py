#!/usr/bin/env python3
"""SV E2E validation for the VAC v1.9 bounded agent runtime.

No cargo is executed. The script verifies source-level runtime contracts and a
fixture matrix that models happy-path, block, approval-pause, and discussion
pause behavior.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs"
E2E = ROOT / "vac-rs/crates/runtime/vac-agent-loop/src/runtime_e2e.rs"
FIXTURES = ROOT / "tests/fixtures/runtime/bound_agent_e2e_cases.json"


def fail(message: str) -> None:
    print(f"FAIL {message}")
    sys.exit(1)


def read(path: Path) -> str:
    if not path.exists():
        fail(f"missing {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8", errors="ignore")


def check_source() -> None:
    runtime = read(RUNTIME)
    e2e = read(E2E)
    required_runtime = [
        "BoundRuntimePhase", "IntentPlan", "SetupArtifacts", "AgentProcess", "Validate", "UpdateVac",
        "CompletionLock", "RuntimeAuthority::CompiledJson", "compiled JSON snapshot", "deterministic index/read-plan",
        "policy snapshot missing; VAC is fail-closed", "forbidden.files wins over allowed_files",
        "free-form shell runner is forbidden", "integrity-hint, NOT tamper-evident",
        "compiled_json_snapshot_current", "no_critical_spec_drift", "no_code_without_capability",
        "assessment_findings_have_span_evidence",
    ]
    required_e2e = [
        "BoundAgentE2EInput", "BoundAgentE2EReport", "run_bound_agent_e2e", "enforcement_honesty_gate",
        "approve_plan", "setup_artifacts", "pre_patch_gate", "pre_command_gate", "post_validation_gate",
        "complete_session",
    ]
    for token in required_runtime:
        if token not in runtime:
            fail(f"bound_runtime.rs missing token: {token}")
    for token in required_e2e:
        if token not in e2e:
            fail(f"runtime_e2e.rs missing token: {token}")
    if re.search(r"RuntimeAuthority::RawYaml\)\s*=>\s*true", runtime):
        fail("raw YAML appears executable")


def defaults(case: Dict[str, Any]) -> Dict[str, Any]:
    base: Dict[str, Any] = {
        "requested_l2_claim": False,
        "registry": {"authority": "compiled_json", "compiled_snapshot_current": True, "deterministic_index_current": True, "policy_loaded": True, "conformance_level": "L1", "capability_ready": True, "owns_path": True},
        "plan": {"status": "approved", "approval_required": False, "approval_approved": True, "path_forbidden": False, "path_in_plan": True, "range_inside": True, "anchor_matches": True, "budget_ok": True},
        "command": {"runner": "python", "args": ["scripts/vac-runtime-agent-e2e-sv.py"], "approval": "never", "risk": "safe_read", "operator_approved": False},
        "validation": {"exit_zero": True, "no_new_orphans": True},
        "closeout": {"tasks_terminal": True, "spec_terminal": True, "todo_terminal": True, "tasks_complete": True, "spec_complete": True, "todo_complete": True, "discussion_escape": False, "evidence_valid": True, "compiled_json_snapshot_current": True, "no_critical_spec_drift": True, "no_unresolved_readiness_mismatch": True, "no_code_without_capability": True, "no_capability_without_current_intent_spec": True, "assessment_findings_have_span_evidence": True, "operator_visible_status": True},
    }
    for k, v in case.items():
        if isinstance(v, dict) and isinstance(base.get(k), dict):
            base[k].update(v)
        else:
            base[k] = v
    return base


def pre_plan(case: Dict[str, Any]) -> str | None:
    r, p = case["registry"], case["plan"]
    if case.get("requested_l2_claim") and r.get("conformance_level") == "L1":
        return "blocked_enforcement_honesty"
    if r["authority"] != "compiled_json" or not r["compiled_snapshot_current"] or not r["deterministic_index_current"] or not r["policy_loaded"]:
        return "blocked_pre_plan"
    if not r["capability_ready"] or not r["owns_path"]:
        return "blocked_pre_plan"
    if p["approval_required"] and not p["approval_approved"]:
        return "paused_pre_plan"
    return None


def pre_patch(case: Dict[str, Any]) -> str | None:
    p = case["plan"]
    if p["status"] not in {"approved", "executing"}:
        return "blocked_pre_patch"
    if p["path_forbidden"] or not p["path_in_plan"] or not p["range_inside"] or not p["anchor_matches"] or not p["budget_ok"]:
        return "blocked_pre_patch"
    return None


def pre_command(case: Dict[str, Any]) -> str | None:
    c = case["command"]
    runner, args = c["runner"], c.get("args", [])
    if not runner or "/" in runner or " " in runner:
        return "blocked_pre_command"
    if runner in {"sh", "bash", "zsh", "fish", "cmd", "powershell"}:
        return "blocked_pre_command"
    if any(any(meta in arg for meta in ["|", ">", "<", ";", "&", "`", "$", "*", "?"]) or ".." in arg for arg in args):
        return "blocked_pre_command"
    needs_approval = runner in {"rm", "del", "rmdir"} or (runner == "cargo" and "clean" in args) or c.get("approval") == "always" or c.get("risk") in {"high", "critical", "execute_process"}
    if needs_approval and not c.get("operator_approved", False):
        return "paused_pre_command"
    return None


def post_validation(case: Dict[str, Any]) -> str | None:
    v = case["validation"]
    if not v["exit_zero"] or not v["no_new_orphans"]:
        return "blocked_post_validation"
    return None


def completion(case: Dict[str, Any]) -> str:
    c = case["closeout"]
    required = ["tasks_terminal", "spec_terminal", "todo_terminal", "evidence_valid", "compiled_json_snapshot_current", "no_critical_spec_drift", "no_unresolved_readiness_mismatch", "no_code_without_capability", "no_capability_without_current_intent_spec", "assessment_findings_have_span_evidence"]
    blockers = [key for key in required if not c[key]]
    if not blockers and c["tasks_complete"] and c["spec_complete"] and c["todo_complete"]:
        return "done"
    if c.get("discussion_escape") and c.get("operator_visible_status"):
        return "paused_completion_lock"
    return "blocked_completion_lock"


def evaluate(raw: Dict[str, Any]) -> str:
    case = defaults(raw)
    for gate in (pre_plan, pre_patch, pre_command, post_validation):
        outcome = gate(case)
        if outcome:
            return outcome
    return completion(case)


def check_cases() -> List[Tuple[str, str]]:
    matrix = json.loads(read(FIXTURES))
    if matrix.get("kind") != "runtime_e2e_fixture_matrix":
        fail("fixture kind mismatch")
    expected_cases = {
        "happy_path_done", "raw_yaml_authority_blocks_pre_plan", "stale_compiled_snapshot_blocks_pre_plan",
        "missing_policy_blocks_pre_plan", "unowned_path_blocks_pre_plan", "planned_capability_blocks_pre_plan",
        "forbidden_file_blocks_pre_patch", "shell_runner_blocks_pre_command", "destructive_command_pauses_for_approval",
        "validation_failure_blocks_post_validation", "open_todo_blocks_completion_lock", "critical_spec_drift_blocks_completion_lock",
        "needs_discussion_pauses_and_surfaces_question", "l1_runtime_cannot_claim_l2",
    }
    seen, passed = set(), []
    for case in matrix.get("cases", []):
        cid, expect = case.get("id"), case.get("expect")
        if not cid or cid in seen:
            fail(f"invalid/duplicate case id: {cid}")
        seen.add(cid)
        got = evaluate(case)
        if got != expect:
            fail(f"case {cid}: expected {expect}, got {got}")
        passed.append((cid, got))
    missing = expected_cases - seen
    if missing:
        fail(f"missing cases: {sorted(missing)}")
    return passed


def main() -> int:
    check_source()
    passed = check_cases()
    print("VAC runtime agent E2E SV: PASS")
    for cid, got in passed:
        print(f"  - {cid}: {got}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
