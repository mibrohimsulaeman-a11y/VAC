#!/usr/bin/env python3
"""Shared Cargo TV proof status helpers.

The final SV gates must not claim Cargo test/validation coverage from static
strings. They either consume a current proof written by check-cargo-tv.py or keep
the Cargo TV surface at NotEvaluated.
"""
from __future__ import annotations

import json
import os
import pathlib
from typing import Any

from vac_script_common import canonical_hash, read_json, sha256_bytes
from vac_script_common import git_output_args as git_output
from vac_script_common import proof_payload_hash
from vac_script_common import proof_path as common_proof_path

TV_PASS = "TV-Pass"
TV_FAIL = "TV-Fail"
TV_STALE = "TV-Stale"
NOT_EVALUATED = "NotEvaluated"
PROOF_REL = ".vac/evidence/cargo-tv-current.json"
REQUIRED_CHECKS = [
    "cargo_metadata",
    "cargo_fmt",
    "cargo_check",
    "cargo_clippy",
    "cargo_test",
]

WORKSPACE_EXCLUDED_DIRS = {".git", ".vac", "target", "node_modules", "__pycache__"}



def proof_path(root: pathlib.Path) -> pathlib.Path:
    return common_proof_path(root, PROOF_REL)


def cargo_workspace_hash(root: pathlib.Path) -> str:
    """Hash the Rust workspace files Cargo can read, excluding build output."""
    vac_rs = root / "vac-rs"
    if not vac_rs.exists():
        return canonical_hash({"missing": "vac-rs"})
    entries: list[dict[str, Any]] = []
    for dirpath, dirnames, filenames in os.walk(vac_rs):
        dirnames[:] = sorted(
            d for d in dirnames if d not in WORKSPACE_EXCLUDED_DIRS
        )
        for filename in sorted(filenames):
            path = pathlib.Path(dirpath) / filename
            rel = path.relative_to(root).as_posix()
            if path.is_symlink():
                entries.append(
                    {
                        "path": rel,
                        "kind": "symlink",
                        "target": os.readlink(path),
                    }
                )
                continue
            if not path.is_file():
                continue
            entries.append(
                {
                    "path": rel,
                    "kind": "file",
                    "bytes": path.stat().st_size,
                    "sha256": sha256_bytes(path.read_bytes()),
                }
            )
    return canonical_hash({"cargo_workspace_files": entries})



def read_proof(root: pathlib.Path) -> dict[str, Any]:
    proof = read_json(proof_path(root), {})
    return proof if isinstance(proof, dict) else {}


def cargo_tv_proof_consumption_enabled() -> bool:
    value = os.environ.get("VAC_CARGO_TV_CONSUME_PROOF", "")
    return value == "1" or value.lower() == "true"


def not_evaluated_summary() -> dict[str, Any]:
    return {
        "status": NOT_EVALUATED,
        "checks": {check_id: NOT_EVALUATED for check_id in REQUIRED_CHECKS},
        "proof_ref": None,
        "proof_hash": None,
        "cargo_workspace_hash": None,
        "git_head": None,
        "generated_at": None,
        "tv_pending": list(REQUIRED_CHECKS),
        "errors": ["missing_cargo_tv_proof"],
    }



def validate_proof(root: pathlib.Path, proof: dict[str, Any] | None = None) -> list[str]:
    proof = proof if proof is not None else read_proof(root)
    errors: list[str] = []
    if not proof:
        return ["missing_cargo_tv_proof"]
    if proof.get("schema_version") != 1:
        errors.append("invalid_schema_version")
    if proof.get("kind") != "cargo_tv_current_run_proof":
        errors.append("invalid_kind")
    expected_hash = proof_payload_hash(proof)
    if proof.get("proof_hash") != expected_hash:
        errors.append("proof_hash_mismatch")
    if proof.get("cargo_workspace_hash") != cargo_workspace_hash(root):
        errors.append("cargo_workspace_hash_mismatch")
    checks = proof.get("checks")
    if not isinstance(checks, dict):
        errors.append("missing_checks")
        checks = {}
    for check_id in REQUIRED_CHECKS:
        check = checks.get(check_id)
        if not isinstance(check, dict):
            errors.append(f"missing_{check_id}")
            continue
        if check.get("status") != TV_PASS:
            errors.append(f"{check_id}_{check.get('status', NOT_EVALUATED)}")
    if proof.get("proof_status") != TV_PASS:
        errors.append(f"cargo_tv_{proof.get('proof_status', NOT_EVALUATED)}")
    return errors


def cargo_tv_summary(
    root: pathlib.Path,
    *,
    consume_proof: bool | None = None,
) -> dict[str, Any]:
    if consume_proof is None:
        consume_proof = cargo_tv_proof_consumption_enabled()
    if not consume_proof:
        return not_evaluated_summary()

    proof = read_proof(root)
    if not proof:
        return not_evaluated_summary()

    errors = validate_proof(root, proof)
    raw_checks = proof.get("checks") if isinstance(proof.get("checks"), dict) else {}
    checks = {
        check_id: (
            raw_checks.get(check_id, {}).get("status", NOT_EVALUATED)
            if isinstance(raw_checks.get(check_id), dict)
            else NOT_EVALUATED
        )
        for check_id in REQUIRED_CHECKS
    }
    status = TV_PASS if not errors else TV_STALE
    if proof.get("proof_status") == TV_FAIL:
        status = TV_FAIL
    tv_pending = [] if status == TV_PASS else list(REQUIRED_CHECKS)
    return {
        "status": status,
        "checks": checks,
        "proof_ref": PROOF_REL,
        "proof_hash": proof.get("proof_hash"),
        "cargo_workspace_hash": proof.get("cargo_workspace_hash"),
        "git_head": proof.get("git_head"),
        "generated_at": proof.get("generated_at"),
        "tv_pending": tv_pending,
        "errors": errors,
    }


def print_summary(summary: dict[str, Any]) -> None:
    checks = summary.get("checks") if isinstance(summary.get("checks"), dict) else {}
    for check_id in REQUIRED_CHECKS:
        print(f"{check_id}={checks.get(check_id, NOT_EVALUATED)}")
    if summary.get("proof_ref"):
        print(f"cargo_tv_proof={summary['proof_ref']}")
    if summary.get("proof_hash"):
        print(f"cargo_tv_proof_hash={summary['proof_hash']}")
    print(f"cargo_tv={summary.get('status', NOT_EVALUATED)}")
