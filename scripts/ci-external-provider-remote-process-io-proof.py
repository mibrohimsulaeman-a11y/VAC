#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from external_provider_remote_process_io_status import (
    CI_ATTESTED,
    CLAIM_ID,
    LOCAL_ONLY,
    L2_NOT_IMPLEMENTED,
    OBSERVED_L1,
    PROOF_KIND,
    PROOF_REL,
    REMOTE_PROCESS_CLAIM_ID,
    REQUIRED_CHECKS,
    REQUIRED_NEGATIVE_GOVERNANCE,
    TV_FAIL,
    TV_PASS,
    TV_PENDING,
    external_provider_remote_process_io_summary,
    git_output,
    print_summary,
    proof_hash,
    proof_path,
    source_scope_hash,
)


def load_json_file(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("result JSON root must be an object")
    return value


def load_result_from_env(root: Path) -> tuple[dict[str, Any] | None, list[str]]:
    path_value = os.environ.get("VAC_EXTERNAL_PROVIDER_REMOTE_PROCESS_IO_RESULT_JSON", "").strip()
    b64_value = os.environ.get("VAC_EXTERNAL_PROVIDER_REMOTE_PROCESS_IO_RESULT_B64", "").strip()
    raw_value = os.environ.get("VAC_EXTERNAL_PROVIDER_REMOTE_PROCESS_IO_RESULT", "").strip()

    if path_value:
        result_path = Path(path_value)
        if not result_path.is_absolute():
            result_path = root / result_path
        if not result_path.is_file():
            return None, ["configured_result_json_missing"]
        try:
            return load_json_file(result_path), []
        except (OSError, json.JSONDecodeError, ValueError) as exc:
            return None, [f"configured_result_json_invalid:{exc}"]

    if b64_value:
        try:
            decoded = base64.b64decode(b64_value, validate=True).decode("utf-8")
            value = json.loads(decoded)
        except (ValueError, json.JSONDecodeError) as exc:
            return None, [f"configured_result_b64_invalid:{exc}"]
        if not isinstance(value, dict):
            return None, ["configured_result_b64_root_not_object"]
        return value, []

    if raw_value:
        try:
            value = json.loads(raw_value)
        except json.JSONDecodeError as exc:
            return None, [f"configured_result_env_invalid:{exc}"]
        if not isinstance(value, dict):
            return None, ["configured_result_env_root_not_object"]
        return value, []

    return None, ["external_provider_remote_process_io_result_not_configured"]


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def sha256_text(text: str) -> str:
    return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()


def build_github_context() -> dict[str, Any]:
    server_url = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    run_url = f"{server_url}/{repository}/actions/runs/{run_id}" if repository and run_id else None
    return {
        "actions": os.environ.get("GITHUB_ACTIONS") == "true",
        "run_id": run_id,
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", ""),
        "workflow": os.environ.get("GITHUB_WORKFLOW", ""),
        "job": os.environ.get("GITHUB_JOB", ""),
        "sha": os.environ.get("GITHUB_SHA", ""),
        "ref": os.environ.get("GITHUB_REF", ""),
        "server_url": server_url,
        "repository": repository,
        "run_url": run_url,
    }


def result_to_checks(result: dict[str, Any]) -> tuple[dict[str, str], list[str]]:
    remote_process = as_dict(result.get("remote_process"))
    negative_governance = as_dict(result.get("negative_governance"))
    credential_redaction = as_dict(result.get("credential_redaction"))
    checks = {
        "external_provider_authenticated": TV_PASS
        if result.get("external_provider_authenticated") is True
        else TV_FAIL,
        "remote_process_command_invoked": TV_PASS
        if remote_process.get("command_invoked") is True
        else TV_FAIL,
        "remote_stdout_observed": TV_PASS if remote_process.get("stdout_observed") is True else TV_FAIL,
        "remote_exit_status_observed": TV_PASS
        if remote_process.get("exit_status_observed") is True
        else TV_FAIL,
        "remote_credential_material_redacted": TV_PASS
        if credential_redaction.get("secret_material_observed") is False
        and isinstance(credential_redaction.get("redacted_fields"), list)
        and bool(credential_redaction.get("redacted_fields"))
        else TV_FAIL,
        "negative_governance_fail_closed": TV_PASS
        if all(negative_governance.get(case) is True for case in REQUIRED_NEGATIVE_GOVERNANCE)
        else TV_FAIL,
    }
    errors = [f"check_failed:{key}" for key in REQUIRED_CHECKS if checks.get(key) != TV_PASS]
    return checks, errors


def build_proof(root: Path, result: dict[str, Any] | None, blocked_reasons: list[str]) -> tuple[dict[str, Any], int]:
    github = build_github_context()
    generated_in_ci = github["actions"] is True
    git_head = git_output(root, "rev-parse", "HEAD") or "unknown"
    custody = CI_ATTESTED if generated_in_ci else LOCAL_ONLY

    base: dict[str, Any] = {
        "schema_version": 1,
        "kind": PROOF_KIND,
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "proof_ref": PROOF_REL,
        "claim": CLAIM_ID,
        "remote_process_claim": REMOTE_PROCESS_CLAIM_ID,
        "execution": OBSERVED_L1,
        "custody": custody,
        "l2_broker": L2_NOT_IMPLEMENTED,
        "git_head": git_head,
        "source_scope_hash": source_scope_hash(root),
        "github": github,
    }

    if result is None:
        proof = {
            **base,
            "proof_status": TV_PENDING,
            "blocked_reasons": blocked_reasons,
            "checks": {check: TV_PENDING for check in REQUIRED_CHECKS},
            "negative_governance": {case: False for case in REQUIRED_NEGATIVE_GOVERNANCE},
            "credential_redaction": {"secret_material_observed": False, "redacted_fields": []},
            "remote_process": {},
            "provider_kind": None,
            "target_kind": None,
        }
        proof["proof_hash"] = proof_hash(proof)
        return proof, 0

    checks, check_errors = result_to_checks(result)
    provider_kind = str(result.get("provider_kind", ""))
    target_kind = str(result.get("target_kind", ""))
    remote_process = as_dict(result.get("remote_process"))
    credential_redaction = as_dict(result.get("credential_redaction"))
    negative_governance = as_dict(result.get("negative_governance"))
    stdout_text = str(remote_process.get("stdout", ""))
    if stdout_text and not remote_process.get("stdout_sha256"):
        remote_process = dict(remote_process)
        remote_process["stdout_sha256"] = sha256_text(stdout_text)
        remote_process.pop("stdout", None)

    proof_status = TV_PASS
    errors = list(check_errors)
    if provider_kind != "external_provider":
        errors.append("provider_kind_not_external_provider")
    if target_kind != "external_remote_process":
        errors.append("target_kind_not_external_remote_process")
    if not generated_in_ci:
        errors.append("ci_context_required_for_tv_pass")
    if errors:
        proof_status = TV_FAIL

    proof = {
        **base,
        "proof_status": proof_status,
        "blocked_reasons": errors,
        "provider_kind": provider_kind,
        "target_kind": target_kind,
        "checks": checks,
        "negative_governance": negative_governance,
        "credential_redaction": credential_redaction,
        "remote_process": remote_process,
    }
    proof["proof_hash"] = proof_hash(proof)
    return proof, 0 if proof_status == TV_PASS else 1


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create CI proof for external-provider remote process IO E2E without local Cargo"
    )
    parser.add_argument("root", nargs="?", default=".")
    args = parser.parse_args()
    root = Path(args.root).resolve()

    result, blocked = load_result_from_env(root)
    proof, exit_code = build_proof(root, result, blocked)
    path = proof_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(proof, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    summary = external_provider_remote_process_io_summary(root)
    print("VAC external provider remote process IO CI proof generated")
    print_summary(summary)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
