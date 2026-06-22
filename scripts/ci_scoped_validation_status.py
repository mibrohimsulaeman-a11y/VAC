#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
from typing import Any

from vac_script_common import git_output, proof_payload_hash, read_json_object
from vac_script_common import proof_path as common_proof_path
from vac_script_common import source_scope_hash as common_source_scope_hash

CLAIM_ID = "ci_scoped_validation"
PROOF_REL = ".vac/evidence/ci-scoped-validation-current.json"
PROOF_KIND = "ci_scoped_validation_proof"

TV_PASS = "TV-Pass"
TV_FAIL = "TV-Fail"
TV_STALE = "TV-Stale"
TV_PENDING = "TV-Pending"
OBSERVED_L1 = "observed_l1"
CI_ATTESTED = "ci_attested"
LOCAL_ONLY = "local_only"
L2_NOT_IMPLEMENTED = "NotImplemented"

REQUIRED_CHECKS = [
    "format_check",
    "external_provider_remote_process_io_proof",
    "final_sv_gate",
    "lint",
    "build",
    "tui_agent_tool_lifecycle_smoke",
    "local_provider_io_smoke",
    "workspace_tests",
    "feature_gated_tests",
]

SOURCE_SCOPE_PATHS = [
    ".github/workflows/ci.yml",
    "vac-rs/Cargo.toml",
    "vac-rs/Cargo.lock",
    "scripts/ci_scoped_validation_status.py",
    "scripts/check-ci-scoped-validation-proof.py",
    "scripts/ci-scoped-validation-proof.py",
    "scripts/l2_broker_status.py",
    "scripts/check-l2-broker-status.py",
    "scripts/check-cargo-tv.py",
    "scripts/cargo_tv_status.py",
    "scripts/vac-v19-final-sv-gate.sh",
    "scripts/vac-reaudit-final-sv-gate.sh",
    "scripts/run-final-sv-validation.py",
    "scripts/external_provider_remote_process_io_status.py",
    "scripts/check-external-provider-remote-process-io-e2e.py",
    "scripts/ci-external-provider-remote-process-io-proof.py",
    "docs/audit/VAC_CI_SCOPED_VALIDATION_PROOF.md",
    "docs/audit/VAC_L2_BROKER_P2_FIRST_SLICE.md",
]



def proof_path(root: Path) -> Path:
    return common_proof_path(root, PROOF_REL)


def source_scope_hash(root: Path) -> str:
    return common_source_scope_hash(root, CLAIM_ID, SOURCE_SCOPE_PATHS)


def read_proof(root: Path) -> tuple[dict[str, Any] | None, list[str]]:
    return read_json_object(
        proof_path(root),
        missing_reason="missing_ci_scoped_validation_proof",
        invalid_json_prefix="invalid_ci_scoped_validation_proof_json",
        not_object_reason="ci_scoped_validation_proof_root_not_object",
    )


def validate_proof(root: Path, proof: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    current_head = git_output(root, "rev-parse", "HEAD")
    if proof.get("schema_version") != 1:
        errors.append("schema_version_not_1")
    if proof.get("kind") != PROOF_KIND:
        errors.append("kind_mismatch")
    if proof.get("claim") != CLAIM_ID:
        errors.append("claim_mismatch")
    if proof.get("proof_ref") != PROOF_REL:
        errors.append("proof_ref_mismatch")
    if proof.get("proof_hash") != proof_payload_hash(proof):
        errors.append("proof_hash_mismatch")
    if current_head and proof.get("git_head") != current_head:
        errors.append("git_head_mismatch")
    if proof.get("source_scope_hash") != source_scope_hash(root):
        errors.append("source_scope_hash_mismatch")
    if proof.get("execution") != OBSERVED_L1:
        errors.append("execution_must_remain_observed_l1")
    if proof.get("l2_broker") != L2_NOT_IMPLEMENTED:
        errors.append("l2_broker_must_remain_not_implemented")

    proof_status = proof.get("proof_status")
    if proof_status not in {TV_PASS, TV_PENDING, TV_FAIL}:
        errors.append("proof_status_unsupported")
        return errors
    if proof_status == TV_PENDING:
        return errors
    if proof_status == TV_FAIL:
        errors.append("proof_status_tv_fail")
        return errors

    if proof.get("custody") != CI_ATTESTED:
        errors.append("tv_pass_requires_ci_attested_custody")
    github = proof.get("github")
    if not isinstance(github, dict):
        errors.append("github_context_missing")
    else:
        if github.get("actions") is not True:
            errors.append("github_actions_context_required")
        if not github.get("run_id"):
            errors.append("github_run_id_required")
        if current_head and github.get("sha") != current_head:
            errors.append("github_sha_mismatch")

    checks = proof.get("checks")
    if not isinstance(checks, dict):
        errors.append("checks_missing")
    else:
        for check in REQUIRED_CHECKS:
            if checks.get(check) != TV_PASS:
                errors.append(f"check_not_tv_pass:{check}")
    return errors


def ci_scoped_validation_summary(root: Path) -> dict[str, Any]:
    proof, load_errors = read_proof(root)
    if proof is None:
        return {
            "status": TV_PENDING,
            "execution": OBSERVED_L1,
            "custody": LOCAL_ONLY,
            "proof_ref": None,
            "proof_hash": None,
            "source_scope_hash": source_scope_hash(root),
            "errors": load_errors,
        }
    errors = validate_proof(root, proof)
    raw_status = str(proof.get("proof_status", TV_FAIL))
    if errors:
        status = TV_STALE if raw_status in {TV_PASS, TV_PENDING} else TV_FAIL
    else:
        status = TV_PASS if raw_status == TV_PASS else TV_PENDING
    return {
        "status": status,
        "execution": proof.get("execution", OBSERVED_L1),
        "custody": proof.get("custody", LOCAL_ONLY),
        "proof_ref": PROOF_REL,
        "proof_hash": proof.get("proof_hash"),
        "source_scope_hash": proof.get("source_scope_hash"),
        "errors": errors or list(proof.get("blocked_reasons", [])),
    }


def print_summary(summary: dict[str, Any]) -> None:
    print(f"{CLAIM_ID}={summary['status']}")
    print(f"{CLAIM_ID}_execution={summary['execution']}")
    print(f"{CLAIM_ID}_custody={summary['custody']}")
    if summary.get("proof_ref"):
        print(f"{CLAIM_ID}_proof_ref={summary['proof_ref']}")
    if summary.get("proof_hash"):
        print(f"{CLAIM_ID}_proof_hash={summary['proof_hash']}")
    if summary.get("errors"):
        print(f"{CLAIM_ID}_reasons=" + ",".join(str(item) for item in summary["errors"]))
