#!/usr/bin/env python3
"""Static E2E validator for the VAC v1.5 bounded runtime.

This gate intentionally does not invoke Cargo. It validates the runtime contract
with deterministic JSON fixtures plus source/registry checks that can run inside
sandbox checkpoints.
"""
from __future__ import annotations

import copy
import json
import pathlib
import re
import sys
from typing import Any

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
FIXTURES = ROOT / "tests/fixtures/runtime/v15-e2e"
ERRORS: list[str] = []
WARNINGS: list[str] = []


def fail(message: str) -> None:
    ERRORS.append(message)


def warn(message: str) -> None:
    WARNINGS.append(message)


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text())
    except Exception as exc:  # noqa: BLE001 - gate reports exact fixture issue
        fail(f"invalid JSON fixture {path.relative_to(ROOT)}: {exc}")
        return {}


def rel(path: pathlib.Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def path_matches(pattern: str, path: str) -> bool:
    if pattern in (path, "**"):
        return True
    if pattern.endswith("/**"):
        prefix = pattern[:-3]
        return path == prefix or path.startswith(prefix + "/")
    if "*" not in pattern:
        return False
    regex = "^" + re.escape(pattern).replace("\\*", ".*") + "$"
    return re.match(regex, path) is not None


def decision_with_warnings(blockers: list[str], warnings: list[str]) -> str:
    if blockers:
        return "block"
    if warnings:
        return "pass_with_warnings"
    return "pass"


def capability_for(registry: dict[str, Any], cap_id: str) -> dict[str, Any] | None:
    for capability in registry.get("capabilities", []):
        if capability.get("id") == cap_id:
            return capability
    return None


def pre_plan(registry: dict[str, Any], plan: dict[str, Any]) -> tuple[str, list[str], list[str]]:
    blockers: list[str] = []
    warnings: list[str] = []
    if registry.get("authority") != "compiled_json":
        blockers.append("runtime authorization attempted without compiled JSON snapshot")
    if not registry.get("compiled_snapshot_current"):
        blockers.append("compiled JSON snapshot is stale or missing")
    if not registry.get("deterministic_index_current"):
        blockers.append("deterministic index/read-plan tickets are stale or missing")
    if not registry.get("policy_loaded"):
        blockers.append("policy snapshot missing; VAC is fail-closed")

    capability = capability_for(registry, plan.get("capability", ""))
    if capability is None:
        blockers.append(f"capability not present in compiled registry: {plan.get('capability')}")
        return "block", blockers, warnings

    readiness = capability.get("readiness", {})
    computed = readiness.get("computed")
    effective = readiness.get("effective")
    # Fail-closed: effective cannot be stronger than computed. Order is intentionally conservative.
    order = {"deprecated": 0, "planned": 1, "partial": 2, "ready": 3}
    if order.get(effective, -1) > order.get(computed, -1):
        warnings.append("effective readiness was stronger than computed and must be lowered")
        effective = computed
    if effective == "ready":
        pass
    elif effective == "partial":
        warnings.append(f"capability {capability['id']} is partial; execution remains bounded")
    else:
        blockers.append(f"capability {capability['id']} is not executable: {effective}")

    owned_paths = capability.get("owned_paths", [])
    for scope in plan.get("allowed_files", []):
        if scope.get("ownership") != capability.get("id"):
            blockers.append(f"plan file {scope.get('path')} ownership mismatch")
        if not any(path_matches(pattern, scope.get("path", "")) for pattern in owned_paths):
            blockers.append(f"file is not owned by capability {capability.get('id')}: {scope.get('path')}")
    approval = plan.get("approval", {})
    if approval.get("required") and not approval.get("approved") and not blockers:
        return "approval_required", blockers, [*warnings, "operator approval is required"]
    return decision_with_warnings(blockers, warnings), blockers, warnings


def plan_executable(plan: dict[str, Any]) -> bool:
    approval = plan.get("approval", {})
    approved = (not approval.get("required")) or bool(approval.get("approved"))
    return plan.get("status") in {"approved", "executing"} and approved


def approve_plan(registry: dict[str, Any], plan: dict[str, Any]) -> tuple[str, list[str], list[str]]:
    decision, blockers, warnings = pre_plan(registry, plan)
    blockers = list(blockers)
    warnings = list(warnings)
    if not plan_executable(plan):
        blockers.append("Semantic Plan must be approved and operator-bound before agent processing")
    return decision_with_warnings(blockers, warnings), blockers, warnings


def artifact_lock(session_id: str, plan: dict[str, Any], artifacts: dict[str, Any]) -> tuple[str, list[str], list[str]]:
    blockers: list[str] = []
    warnings: list[str] = []
    task = artifacts.get("task", {})
    spec = artifacts.get("spec", {})
    todo = artifacts.get("todo", {})
    for name, artifact in (("task", task), ("spec", spec), ("todo", todo)):
        if artifact.get("session") != session_id:
            blockers.append(f"{name} artifact is not bound to active session")
        if not artifact.get("id"):
            blockers.append(f"{name} artifact missing stable id")
    if task.get("plan") != plan.get("id"):
        blockers.append("task artifact must reference the active Semantic Plan")
    if task.get("capability") != plan.get("capability"):
        blockers.append("task artifact capability must match active plan capability")
    if plan.get("capability") not in spec.get("touched_capabilities", []):
        blockers.append("spec artifact must list the active capability")
    if not task.get("acceptance_criteria"):
        blockers.append("task artifact must include acceptance criteria")
    if not any(item.get("blocking") for item in todo.get("items", [])):
        blockers.append("todo artifact must include at least one blocking closeout item")
    return decision_with_warnings(blockers, warnings), blockers, warnings


def pre_patch(plan: dict[str, Any], patch: dict[str, Any], patch_count: int, new_file_count: int, line_delta: int) -> tuple[str, list[str], list[str]]:
    blockers: list[str] = []
    warnings: list[str] = []
    path = patch.get("path", "")
    if not plan_executable(plan):
        blockers.append("Semantic Plan is not approved/executable")
        return "block", blockers, warnings
    if any(path_matches(pattern, path) for pattern in plan.get("forbidden_files", [])):
        blockers.append("forbidden.files wins over allowed_files")
        return "block", blockers, warnings
    scope = next((item for item in plan.get("allowed_files", []) if item.get("path") == path), None)
    if scope is None:
        blockers.append(f"file is outside Semantic Plan: {path}")
        return "block", blockers, warnings
    if scope.get("operation") != patch.get("operation"):
        blockers.append("patch operation does not match plan operation")
    approved_range = scope.get("line_range", {})
    touched_range = patch.get("touched_range", {})
    if touched_range.get("start", 0) < approved_range.get("start", 0) or touched_range.get("end", 0) > approved_range.get("end", 0):
        blockers.append("patch range is outside approved line range")
    scope_anchor = scope.get("semantic_anchor", {})
    patch_anchor = patch.get("semantic_anchor", {})
    for key in ("symbol", "kind"):
        if scope_anchor.get(key) != patch_anchor.get(key):
            blockers.append("semantic anchor mismatch; plan refresh required")
            break
    expected_fingerprint = scope_anchor.get("normalized_fingerprint")
    if expected_fingerprint and patch_anchor.get("normalized_fingerprint") != expected_fingerprint:
        blockers.append("semantic anchor fingerprint drifted; plan refresh required")
    if patch.get("creates_new_file") and scope.get("operation") != "create":
        blockers.append("new file creation was not declared in the plan")
    if patch.get("patch_index") != patch_count:
        blockers.append("patch attempt index is not monotonic for the active session")
    bounds = plan.get("bounds", {})
    if patch_count >= bounds.get("max_patches", 0):
        blockers.append("patch budget exceeded")
    if new_file_count + int(bool(patch.get("creates_new_file"))) > bounds.get("max_new_files", 0):
        blockers.append("new file budget exceeded")
    delta = int(patch.get("lines_added", 0)) + int(patch.get("lines_removed", 0))
    if line_delta + delta > bounds.get("max_line_delta", 0):
        blockers.append("line delta budget exceeded")
    return decision_with_warnings(blockers, warnings), blockers, warnings


def contains_shell_metachar(arg: str) -> bool:
    return any(ch in arg for ch in "|><;&`$") or ".." in arg or "*" in arg or "?" in arg


def pre_command(command: dict[str, Any]) -> tuple[str, list[str], list[str]]:
    blockers: list[str] = []
    warnings: list[str] = []
    runner = command.get("runner", "")
    if not command.get("id"):
        blockers.append("structured command id is required")
    if "/" in runner or " " in runner or not runner.strip():
        blockers.append("runner must be a registry executable name, not a shell/path string")
    if runner in {"sh", "bash", "zsh", "fish", "cmd", "powershell"}:
        blockers.append("free-form shell runner is forbidden by structured command contract")
    if any(contains_shell_metachar(str(arg)) for arg in command.get("args", [])):
        blockers.append("shell interpolation, wildcard, pipe, or redirection detected in structured args")
    if command.get("policy_decision") == "deny":
        blockers.append("policy snapshot denies this structured command")
    if blockers:
        return "block", blockers, warnings
    if runner in {"rm", "del", "rmdir"} or (runner == "cargo" and "clean" in command.get("args", [])):
        return "approval_required", blockers, ["destructive filesystem command requires explicit approval"]
    if command.get("approval") == "always" or command.get("policy_decision") == "approval_required":
        return "approval_required", blockers, ["command must pause for policy/operator approval"]
    if command.get("risk") in {"high", "critical", "execute_process"}:
        warnings.append("high/process risk command is allowed only because compiled policy allowed it")
    return decision_with_warnings(blockers, warnings), blockers, warnings


def post_validation(post: dict[str, Any]) -> tuple[str, list[str], list[str]]:
    blockers: list[str] = []
    warnings: list[str] = []
    if not post.get("commands_exit_zero"):
        blockers.append("one or more validation commands failed")
    if not post.get("no_new_orphans"):
        blockers.append("post-validation detected new orphan/unowned files")
    return decision_with_warnings(blockers, warnings), blockers, warnings


def artifacts_all_complete(artifacts: dict[str, Any]) -> bool:
    task = artifacts.get("task", {})
    spec = artifacts.get("spec", {})
    todo = artifacts.get("todo", {})
    return (
        task.get("state") == "done"
        and all(item.get("met") for item in task.get("acceptance_criteria", []))
        and spec.get("state") == "finalized"
        and todo.get("state") == "all_checked"
        and all(item.get("checked") or not item.get("blocking") for item in todo.get("items", []))
    )


def artifacts_all_terminal(artifacts: dict[str, Any]) -> bool:
    return (
        artifacts.get("task", {}).get("state") in {"done", "dropped", "needs_discussion"}
        and artifacts.get("spec", {}).get("state") in {"finalized", "needs_discussion"}
        and artifacts.get("todo", {}).get("state") in {"all_checked", "needs_discussion"}
    )


def artifacts_discussion_escape(artifacts: dict[str, Any]) -> bool:
    return any(
        artifacts.get(name, {}).get("state") == "needs_discussion" and artifacts.get(name, {}).get("open_questions")
        for name in ("task", "spec", "todo")
    )


def completion_lock(artifacts: dict[str, Any], closeout: dict[str, Any]) -> tuple[str, list[str], list[str]]:
    blockers: list[str] = []
    warnings: list[str] = []
    if artifacts.get("task", {}).get("state") not in {"done", "dropped", "needs_discussion"}:
        blockers.append("tasks_terminal")
    if artifacts.get("spec", {}).get("state") not in {"finalized", "needs_discussion"}:
        blockers.append("spec_terminal")
    if artifacts.get("todo", {}).get("state") not in {"all_checked", "needs_discussion"}:
        blockers.append("todo_terminal")
    if artifacts_all_terminal(artifacts) and not artifacts_all_complete(artifacts):
        if artifacts_discussion_escape(artifacts):
            warnings.append("mandatory artifacts use needs_discussion escape hatch")
        else:
            blockers.append("mandatory artifacts terminal but not complete and no explicit discussion question")
    evidence = closeout.get("evidence", {})
    if not evidence.get("valid"):
        blockers.append("evidence_valid")
    if evidence.get("broker_sig_algorithm") == "none":
        warnings.append("evidence is in integrity-hint mode, NOT tamper-evident")
    if not closeout.get("compiled_json_snapshot_current"):
        blockers.append("compiled_json_snapshot_current")
    spec_sync = closeout.get("spec_sync", {})
    if not spec_sync.get("no_critical_spec_drift") or spec_sync.get("unresolved_critical_drift", 0) > 0:
        blockers.append("no_critical_spec_drift")
    readiness = closeout.get("readiness", {})
    if not readiness.get("no_unresolved_readiness_mismatch") or readiness.get("unresolved_mismatches", 0) > 0:
        blockers.append("no_unresolved_readiness_mismatch")
    ownership = closeout.get("ownership", {})
    if not ownership.get("no_code_without_capability") or ownership.get("code_without_capability", 0) > 0:
        blockers.append("no_code_without_capability")
    if not ownership.get("no_capability_without_current_intent_spec") or ownership.get("capabilities_without_intent", 0) > 0:
        blockers.append("no_capability_without_current_intent_spec")
    assessment = closeout.get("assessment", {})
    if not assessment.get("findings_have_span_evidence") or assessment.get("unresolved_critical_findings", 0) > 0:
        blockers.append("assessment_findings_have_span_evidence")
    blockers = sorted(set(blockers))
    discussion_escape = bool(closeout.get("explicit_open_question")) and bool(closeout.get("blocking_reason")) and bool(closeout.get("operator_visible_status"))
    if not blockers and artifacts_all_complete(artifacts):
        return "done", blockers, warnings
    if discussion_escape and artifacts_all_terminal(artifacts) and (artifacts_discussion_escape(artifacts) or blockers):
        return "paused_for_discussion", blockers, warnings
    return "blocked", blockers, warnings


def apply_patch_object(target: dict[str, Any], patch: dict[str, Any]) -> dict[str, Any]:
    out = copy.deepcopy(target)
    for key, value in patch.items():
        out[key] = copy.deepcopy(value)
    return out


def assert_decision(name: str, actual: str, expected: str, blockers: list[str] | None = None, contains: str | None = None) -> None:
    if actual != expected:
        fail(f"{name}: expected {expected}, got {actual}")
    if contains and blockers is not None and not any(contains in blocker for blocker in blockers):
        fail(f"{name}: expected blocker containing {contains!r}, got {blockers}")


def run_happy_path(fixture: dict[str, Any]) -> None:
    registry = fixture["registry"]
    plan = fixture["plan"]
    artifacts = fixture["artifacts"]
    expected = fixture["expect"]
    decision, blockers, warnings = pre_plan(registry, plan)
    assert_decision("happy pre_plan", decision, expected["pre_plan"], blockers)
    decision, blockers, warnings = approve_plan(registry, plan)
    if blockers:
        fail(f"happy approve_plan unexpectedly blocked: {blockers}")
    decision, blockers, warnings = artifact_lock("session-happy", plan, artifacts)
    assert_decision("happy artifact_lock", decision, expected["artifact_lock"], blockers)
    patch_count = 0
    new_file_count = 0
    line_delta = 0
    patch_decisions: list[str] = []
    for patch in fixture.get("patches", []):
        decision, blockers, warnings = pre_patch(plan, patch, patch_count, new_file_count, line_delta)
        patch_decisions.append(decision)
        if decision == "pass":
            patch_count += 1
            new_file_count += int(bool(patch.get("creates_new_file")))
            line_delta += int(patch.get("lines_added", 0)) + int(patch.get("lines_removed", 0))
    if patch_decisions != expected["patches"]:
        fail(f"happy patches: expected {expected['patches']}, got {patch_decisions}")
    command_decisions = []
    for command in fixture.get("commands", []):
        decision, blockers, warnings = pre_command(command)
        command_decisions.append(decision)
    if command_decisions != expected["commands"]:
        fail(f"happy commands: expected {expected['commands']}, got {command_decisions}")
    decision, blockers, warnings = post_validation(fixture["post_validation"])
    assert_decision("happy post_validation", decision, expected["post_validation"], blockers)
    disposition, blockers, warnings = completion_lock(artifacts, fixture["closeout"])
    assert_decision("happy completion", disposition, expected["completion"], blockers)


def run_blocked_cases(base: dict[str, Any], fixture: dict[str, Any]) -> None:
    for case in fixture.get("cases", []):
        name = case["name"]
        registry = copy.deepcopy(base["registry"])
        plan = copy.deepcopy(base["plan"])
        artifacts = copy.deepcopy(base["artifacts"])
        closeout = copy.deepcopy(base["closeout"])
        patch = copy.deepcopy(base["patches"][0])
        command = copy.deepcopy(base["commands"][0])
        if "registry_patch" in case:
            registry = apply_patch_object(registry, case["registry_patch"])
        if "capability_readiness_patch" in case:
            registry["capabilities"][0]["readiness"] = apply_patch_object(registry["capabilities"][0]["readiness"], case["capability_readiness_patch"])
        if "plan_patch" in case:
            plan = apply_patch_object(plan, case["plan_patch"])
        if "patch_patch" in case:
            patch = apply_patch_object(patch, case["patch_patch"])
        if "command_patch" in case:
            command = apply_patch_object(command, case["command_patch"])
        if "todo_patch" in case:
            artifacts["todo"] = apply_patch_object(artifacts["todo"], case["todo_patch"])
        if "expect_pre_plan" in case:
            decision, blockers, warnings = pre_plan(registry, plan)
            assert_decision(name, decision, case["expect_pre_plan"], blockers, case.get("expect_blocker_contains"))
        if "expect_approve_plan" in case:
            decision, blockers, warnings = approve_plan(registry, plan)
            assert_decision(name, decision, case["expect_approve_plan"], blockers, case.get("expect_blocker_contains"))
        if "expect_patch" in case:
            decision, blockers, warnings = approve_plan(registry, plan)
            if blockers and case["expect_patch"] != "block":
                fail(f"{name}: setup approve_plan blocked unexpectedly: {blockers}")
            decision, blockers, warnings = pre_patch(plan, patch, 0, 0, 0)
            assert_decision(name, decision, case["expect_patch"], blockers, case.get("expect_blocker_contains"))
        if "expect_command" in case:
            decision, blockers, warnings = pre_command(command)
            assert_decision(name, decision, case["expect_command"], blockers, case.get("expect_blocker_contains"))
        if "expect_completion" in case:
            disposition, blockers, warnings = completion_lock(artifacts, closeout)
            assert_decision(name, disposition, case["expect_completion"], blockers, case.get("expect_blocker_contains"))
        if "expect_enforcement_honesty" in case:
            level = registry.get("conformance_level", "L1")
            blockers = []
            if case.get("claim_l2") and level == "L1":
                blockers.append("L2 fail-closed claim is invalid while runtime is L1 advisory")
            decision = "block" if blockers else "pass"
            assert_decision(name, decision, case["expect_enforcement_honesty"], blockers, case.get("expect_blocker_contains"))


def run_needs_discussion(base: dict[str, Any], fixture: dict[str, Any]) -> None:
    artifacts = copy.deepcopy(base["artifacts"])
    closeout = copy.deepcopy(base["closeout"])
    patch = fixture.get("closeout_patch", {})
    if "task" in patch:
        artifacts["task"] = apply_patch_object(artifacts["task"], patch["task"])
    for key in ("spec", "todo"):
        if key in patch:
            artifacts[key] = apply_patch_object(artifacts[key], patch[key])
    for key, value in patch.items():
        if key not in {"task", "spec", "todo"}:
            closeout[key] = copy.deepcopy(value)
    disposition, blockers, warnings = completion_lock(artifacts, closeout)
    assert_decision("needs_discussion completion", disposition, fixture["expect_completion"], blockers)


def validate_source_contracts() -> None:
    source_path = ROOT / "vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs"
    source = source_path.read_text(errors="ignore") if source_path.is_file() else ""
    required_needles = [
        "BoundRuntimeController",
        "RuntimeAuthority::CompiledJson",
        "compiled JSON snapshot",
        "forbidden.files wins over allowed_files",
        "PolicyDecision",
        "CompletionLockResult",
        "evaluate_completion_lock_v1_5",
        "tasks_terminal",
        "no_critical_spec_drift",
        "assessment_findings_have_span_evidence",
        "L2 fail-closed claim is invalid while runtime is L1 advisory",
        "semantic anchor fingerprint drifted",
    ]
    for needle in required_needles:
        if needle not in source:
            fail(f"runtime source missing contract marker: {needle}")
    lib = ROOT / "vac-rs/crates/runtime/vac-agent-loop/src/lib.rs"
    lib_text = lib.read_text(errors="ignore") if lib.is_file() else ""
    for export in ["BoundRuntimeController", "PolicyDecision", "evaluate_completion_lock_v1_5"]:
        if export not in lib_text:
            fail(f"vac-agent-loop lib.rs does not export {export}")


def validate_registry_runtime_truth() -> None:
    status = load_json(ROOT / ".vac/registry/status.json")
    if status.get("runtime_authority") != "compiled_json":
        fail(".vac/registry/status.json runtime_authority must be compiled_json")
    if status.get("control_plane", {}).get("runtime") != "compiled_json":
        fail("control_plane.runtime must be compiled_json")
    if status.get("runtime", {}).get("server_default") is not False:
        fail("server_default must be false; broker is optional, not default runtime")
    if status.get("runtime", {}).get("gateway_default") is not False:
        fail("gateway_default must be false; gateway is optional integration, not default runtime")
    if status.get("enforcement_level") == "L1" and status.get("workspace", {}).get("conformance_level") != "L1":
        fail("L1 enforcement/conformance label mismatch")
    compiled = load_json(ROOT / ".vac/registry/compiled/workspace.json")
    if compiled.get("runtime_truth") not in {"compiled-json", "compiled_json"}:
        fail("compiled workspace snapshot must identify compiled-json runtime truth")
    caps = load_json(ROOT / ".vac/registry/compiled/capabilities.json")
    if not caps.get("capabilities"):
        fail("compiled capabilities snapshot is empty")
    for cap in caps.get("capabilities", []):
        readiness = cap.get("readiness", {})
        declared = readiness.get("declared")
        computed = readiness.get("computed")
        effective = readiness.get("effective")
        order = {"deprecated": 0, "planned": 1, "partial": 2, "ready": 3}
        if order.get(effective, -1) > order.get(computed, -1):
            fail(f"{cap.get('id')} readiness.effective stronger than computed")


def main() -> int:
    happy = load_json(FIXTURES / "happy-path.json")
    blocked = load_json(FIXTURES / "blocked-cases.json")
    discussion = load_json(FIXTURES / "needs-discussion.json")
    if happy:
        run_happy_path(happy)
    if happy and blocked:
        run_blocked_cases(happy, blocked)
    if happy and discussion:
        run_needs_discussion(happy, discussion)
    validate_source_contracts()
    validate_registry_runtime_truth()
    if ERRORS:
        print("VAC runtime v1.5 E2E: FAIL")
        for error in ERRORS:
            print(f"ERROR: {error}")
        for warning in WARNINGS:
            print(f"WARN: {warning}")
        return 1
    print("VAC runtime v1.5 E2E: PASS")
    print("fixtures: happy-path, blocked-cases, needs-discussion")
    print("coverage: compiled-registry intake, plan/artifact/patch/command/validation gates, v1.5 completion lock")
    for warning in WARNINGS:
        print(f"WARN: {warning}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
