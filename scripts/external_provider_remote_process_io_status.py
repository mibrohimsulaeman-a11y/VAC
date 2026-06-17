#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

CLAIM_ID = "external_provider_remote_process_io_e2e"
REMOTE_PROCESS_CLAIM_ID = "remote_process_io_e2e"
PROOF_REL = ".vac/evidence/external-provider-remote-process-io-e2e.json"
PROOF_KIND = "external_provider_remote_process_io_e2e_proof"

TV_PASS = "TV-Pass"
TV_FAIL = "TV-Fail"
TV_STALE = "TV-Stale"
TV_PENDING = "TV-Pending"

OBSERVED_L1 = "observed_l1"
CI_ATTESTED = "ci_attested"
LOCAL_ONLY = "local_only"
L2_NOT_IMPLEMENTED = "NotImplemented"

REQUIRED_CHECKS = [
    "external_provider_authenticated",
    "remote_process_command_invoked",
    "remote_stdout_observed",
    "remote_exit_status_observed",
    "remote_credential_material_redacted",
    "negative_governance_fail_closed",
]

REQUIRED_NEGATIVE_GOVERNANCE = [
    "missing_bound_approval",
    "structured_command_reject",
    "binding_mismatch",
    "non_loopback_http_reject",
]

SOURCE_SCOPE_PATHS = [
    ".github/workflows/ci.yml",
    "scripts/external_provider_remote_process_io_status.py",
    "scripts/check-external-provider-remote-process-io-e2e.py",
    "scripts/ci-external-provider-remote-process-io-proof.py",
    "scripts/pty-vac-cli-real-io-e2e.py",
    "scripts/check-confirmed-intent-coverage.py",
    "scripts/check-vac-tui-e2e-coverage.py",
    "scripts/refresh-confirmed-intent-status.py",
    "tests/fixtures/confirmed-intent/domain-map.json",
    "docs/audit/VAC_EXTERNAL_PROVIDER_REMOTE_PROCESS_IO_E2E.md",
]


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def canonical_hash(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def file_sha256(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def git_output(root: Path, *args: str) -> str | None:
    try:
        proc = subprocess.run(
            ["git", *args],
            cwd=root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return proc.stdout.strip()


def source_scope_hash(root: Path) -> str:
    files: list[dict[str, Any]] = []
    for rel in SOURCE_SCOPE_PATHS:
        path = root / rel
        if path.is_file():
            files.append({"path": rel, "sha256": file_sha256(path)})
        else:
            files.append({"missing": True, "path": rel})
    return canonical_hash({"claim": CLAIM_ID, "files": files})


def proof_path(root: Path) -> Path:
    return root / PROOF_REL


def proof_hash_payload(proof: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in proof.items() if key != "proof_hash"}


def proof_hash(proof: dict[str, Any]) -> str:
    return canonical_hash(proof_hash_payload(proof))


def load_proof(root: Path) -> tuple[dict[str, Any] | None, list[str]]:
    path = proof_path(root)
    if not path.is_file():
        return None, ["missing_external_provider_remote_process_io_proof"]
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return None, [f"invalid_external_provider_remote_process_io_proof_json:{exc}"]
    if not isinstance(value, dict):
        return None, ["external_provider_remote_process_io_proof_root_not_object"]
    return value, []


def _require_bool_true(proof: dict[str, Any], path: tuple[str, ...], errors: list[str]) -> None:
    value: Any = proof
    for key in path:
        if not isinstance(value, dict) or key not in value:
            errors.append("missing_" + ".".join(path))
            return
        value = value[key]
    if value is not True:
        errors.append("not_true_" + ".".join(path))


def validate_proof(root: Path, proof: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    current_head = git_output(root, "rev-parse", "HEAD")
    current_source_hash = source_scope_hash(root)

    if proof.get("schema_version") != 1:
        errors.append("schema_version_not_1")
    if proof.get("kind") != PROOF_KIND:
        errors.append("kind_not_external_provider_remote_process_io_e2e_proof")
    if proof.get("claim") != CLAIM_ID:
        errors.append("claim_mismatch")
    if proof.get("remote_process_claim") != REMOTE_PROCESS_CLAIM_ID:
        errors.append("remote_process_claim_mismatch")
    if proof.get("proof_ref") != PROOF_REL:
        errors.append("proof_ref_mismatch")
    if proof.get("proof_hash") != proof_hash(proof):
        errors.append("proof_hash_mismatch")
    if current_head and proof.get("git_head") != current_head:
        errors.append("git_head_mismatch")
    if proof.get("source_scope_hash") != current_source_hash:
        errors.append("source_scope_hash_mismatch")
    if proof.get("execution") != OBSERVED_L1:
        errors.append("execution_axis_must_remain_observed_l1")
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

    if proof.get("provider_kind") != "external_provider":
        errors.append("provider_kind_must_be_external_provider")
    if proof.get("target_kind") != "external_remote_process":
        errors.append("target_kind_must_be_external_remote_process")

    checks = proof.get("checks")
    if not isinstance(checks, dict):
        errors.append("checks_missing")
    else:
        for check in REQUIRED_CHECKS:
            if checks.get(check) != TV_PASS:
                errors.append(f"check_not_tv_pass:{check}")

    remote_process = proof.get("remote_process")
    if not isinstance(remote_process, dict):
        errors.append("remote_process_missing")
    else:
        for key in [
            "remote_boundary_observed",
            "command_invoked",
            "stdout_observed",
            "exit_status_observed",
        ]:
            _require_bool_true(proof, ("remote_process", key), errors)
        if remote_process.get("exit_status") != 0:
            errors.append("remote_process_exit_status_not_zero")
        stdout_hash = remote_process.get("stdout_sha256")
        if not isinstance(stdout_hash, str) or not stdout_hash.startswith("sha256:"):
            errors.append("remote_process_stdout_sha256_missing")

    negative_governance = proof.get("negative_governance")
    if not isinstance(negative_governance, dict):
        errors.append("negative_governance_missing")
    else:
        for case in REQUIRED_NEGATIVE_GOVERNANCE:
            if negative_governance.get(case) is not True:
                errors.append(f"negative_governance_not_fail_closed:{case}")

    credential_redaction = proof.get("credential_redaction")
    if not isinstance(credential_redaction, dict):
        errors.append("credential_redaction_missing")
    else:
        if credential_redaction.get("secret_material_observed") is not False:
            errors.append("credential_secret_material_observed")
        redacted_fields = credential_redaction.get("redacted_fields")
        if not isinstance(redacted_fields, list) or not redacted_fields:
            errors.append("credential_redacted_fields_missing")

    return errors


def external_provider_remote_process_io_summary(root: Path) -> dict[str, Any]:
    proof, load_errors = load_proof(root)
    if proof is None:
        return {
            "status": TV_PENDING,
            "remote_process_status": TV_PENDING,
            "execution": OBSERVED_L1,
            "custody": LOCAL_ONLY,
            "proof_ref": None,
            "proof_hash": None,
            "source_scope_hash": source_scope_hash(root),
            "errors": load_errors,
        }

    validation_errors = validate_proof(root, proof)
    proof_status = str(proof.get("proof_status", TV_FAIL))
    if validation_errors:
        status = TV_STALE if proof_status in {TV_PASS, TV_PENDING} else TV_FAIL
    else:
        status = TV_PASS if proof_status == TV_PASS else TV_PENDING

    return {
        "status": status,
        "remote_process_status": status if status == TV_PASS else TV_PENDING,
        "execution": proof.get("execution", OBSERVED_L1),
        "custody": proof.get("custody", LOCAL_ONLY),
        "proof_ref": PROOF_REL,
        "proof_hash": proof.get("proof_hash"),
        "source_scope_hash": proof.get("source_scope_hash"),
        "errors": validation_errors or list(proof.get("blocked_reasons", [])),
    }


def print_summary(summary: dict[str, Any]) -> None:
    print(f"{CLAIM_ID}={summary['status']}")
    print(f"{REMOTE_PROCESS_CLAIM_ID}={summary['remote_process_status']}")
    print(f"{CLAIM_ID}_execution={summary['execution']}")
    print(f"{CLAIM_ID}_custody={summary['custody']}")
    if summary.get("proof_ref"):
        print(f"{CLAIM_ID}_proof_ref={summary['proof_ref']}")
    if summary.get("proof_hash"):
        print(f"{CLAIM_ID}_proof_hash={summary['proof_hash']}")
    if summary.get("errors"):
        print(f"{CLAIM_ID}_reasons=" + ",".join(str(item) for item in summary["errors"]))
