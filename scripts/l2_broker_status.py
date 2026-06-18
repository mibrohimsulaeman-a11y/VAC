#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

CLAIM_ID = "l2_broker"
PROOF_REL = ".vac/evidence/l2-broker-mediated-execution-current.json"
PROOF_KIND = "l2_broker_mediated_execution_proof"

NOT_IMPLEMENTED = "NotImplemented"
IMPLEMENTED = "Implemented"
TV_PASS = "TV-Pass"
TV_FAIL = "TV-Fail"
TV_STALE = "TV-Stale"
TV_PENDING = "TV-Pending"
SV_PASS = "SV-Pass"
SV_FAIL = "SV-Fail"
EXECUTION_NOT_MEDIATED = "not_mediated"
EXECUTION_MEDIATED_L2 = "mediated_l2"
LOCAL_ONLY = "local_only"
BROKER_ATTESTED = "broker_attested"

REQUIRED_CHECKS = [
    "structured_intent_only",
    "broker_process_supervisor",
    "filesystem_mediation",
    "process_spawn_mediation",
    "network_mediation",
    "credential_redaction_before_model",
    "mediated_journal_record",
    "broker_signature_hash",
    "direct_os_bypass_rejected",
    "tool_supplied_policy_decision_rejected",
]

SOURCE_SCOPE_PATHS = [
    "scripts/l2_broker_status.py",
    "scripts/check-l2-broker-status.py",
    "scripts/vac-v19-final-sv-gate.sh",
    "scripts/vac-reaudit-final-sv-gate.sh",
    "scripts/run-final-sv-validation.py",
    "scripts/compile-vac-registry-sv.py",
    "docs/audit/VAC_L2_BROKER_P2_FIRST_SLICE.md",
    "docs/VAC_Init_Control_Plane_Spec_v1_9.md",
    "vac-rs/crates/runtime/vac-broker/src/envelope.rs",
    "vac-rs/crates/foundation/vac-state/src/runtime_journal.rs",
    ".vac/migrations/runtime-db/0001_runtime_journal.sql",
    ".vac/schemas/broker-envelope.schema.json",
]

SCAN_ROOTS = [
    ".github",
    "scripts",
    "docs/audit",
    ".vac/registry",
    ".vac/capabilities",
    ".vac/policies",
    ".vac/workflows",
]

TEXT_SUFFIXES = {
    "",
    ".bash",
    ".cfg",
    ".json",
    ".md",
    ".py",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}


def canonical_hash(value: Any) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def file_hash(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def proof_path(root: Path) -> Path:
    return root / PROOF_REL


def git_output(root: Path, *args: str) -> str:
    try:
        proc = subprocess.run(
            ["git", *args],
            cwd=root,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=30,
        )
    except Exception:
        return ""
    return proc.stdout.strip() if proc.returncode == 0 else ""


def source_scope_hash(root: Path) -> str:
    files: list[dict[str, Any]] = []
    for rel in SOURCE_SCOPE_PATHS:
        path = root / rel
        if path.is_file():
            files.append({"path": rel, "sha256": file_hash(path)})
        else:
            files.append({"path": rel, "missing": True})
    return canonical_hash({"claim": CLAIM_ID, "files": files})


def proof_payload_hash(proof: dict[str, Any]) -> str:
    payload = dict(proof)
    payload.pop("proof_hash", None)
    return canonical_hash(payload)


def forbidden_l2_overclaim_patterns() -> list[str]:
    left = "l2_" + "broker"
    return [
        left + "=" + TV_PASS,
        left + "=" + IMPLEMENTED,
        left + ": " + TV_PASS,
        left + ": " + IMPLEMENTED,
        '"' + left + '": "' + TV_PASS + '"',
        '"' + left + '": "' + IMPLEMENTED + '"',
        left + "_status=" + TV_PASS,
        left + "_status=" + IMPLEMENTED,
    ]


def iter_scannable_files(root: Path) -> list[Path]:
    out: list[Path] = []
    for rel_root in SCAN_ROOTS:
        base = root / rel_root
        if not base.exists():
            continue
        if base.is_file():
            candidates = [base]
        else:
            candidates = [p for p in base.rglob("*") if p.is_file()]
        for path in candidates:
            if any(part in {"__pycache__", "target", ".git"} for part in path.parts):
                continue
            if path.suffix not in TEXT_SUFFIXES:
                continue
            out.append(path)
    return out


def scan_overclaims(root: Path) -> list[str]:
    patterns = forbidden_l2_overclaim_patterns()
    findings: list[str] = []
    for path in iter_scannable_files(root):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        rel = path.relative_to(root).as_posix()
        for pattern in patterns:
            if pattern in text:
                findings.append(f"forbidden_l2_overclaim:{rel}:{pattern}")
    return findings


def read_proof(root: Path) -> tuple[dict[str, Any] | None, list[str]]:
    path = proof_path(root)
    if not path.is_file():
        return None, ["missing_l2_broker_mediated_execution_proof"]
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return None, [f"invalid_l2_broker_proof_json:{exc}"]
    if not isinstance(value, dict):
        return None, ["l2_broker_proof_root_not_object"]
    return value, []


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

    proof_status = proof.get("proof_status")
    if proof_status not in {TV_PASS, TV_PENDING, TV_FAIL}:
        errors.append("proof_status_unsupported")
        return errors
    if proof_status == TV_PENDING:
        if proof.get("l2_broker") != NOT_IMPLEMENTED:
            errors.append("pending_proof_must_keep_l2_broker_not_implemented")
        return errors
    if proof_status == TV_FAIL:
        errors.append("proof_status_tv_fail")
        return errors

    if proof.get("l2_broker") != IMPLEMENTED:
        errors.append("tv_pass_requires_l2_broker_implemented")
    if proof.get("execution") != EXECUTION_MEDIATED_L2:
        errors.append("tv_pass_requires_mediated_l2_execution")
    if proof.get("custody") != BROKER_ATTESTED:
        errors.append("tv_pass_requires_broker_attested_custody")
    checks = proof.get("checks")
    if not isinstance(checks, dict):
        errors.append("checks_missing")
    else:
        for check in REQUIRED_CHECKS:
            if checks.get(check) != TV_PASS:
                errors.append(f"check_not_tv_pass:{check}")
    return errors


def l2_broker_summary(root: Path) -> dict[str, Any]:
    overclaim_errors = scan_overclaims(root)
    proof, load_errors = read_proof(root)
    if proof is None:
        return {
            "status": NOT_IMPLEMENTED,
            "claim_gate": SV_FAIL if overclaim_errors else SV_PASS,
            "execution": EXECUTION_NOT_MEDIATED,
            "custody": LOCAL_ONLY,
            "proof_ref": None,
            "proof_hash": None,
            "source_scope_hash": source_scope_hash(root),
            "errors": overclaim_errors,
            "reasons": load_errors,
        }

    proof_errors = validate_proof(root, proof)
    errors = overclaim_errors + proof_errors
    raw_status = str(proof.get("proof_status", TV_FAIL))
    if errors:
        status = TV_STALE if raw_status in {TV_PASS, TV_PENDING} else TV_FAIL
    elif raw_status == TV_PASS:
        status = IMPLEMENTED
    else:
        status = NOT_IMPLEMENTED
    return {
        "status": status,
        "claim_gate": SV_FAIL if errors else SV_PASS,
        "execution": proof.get("execution", EXECUTION_NOT_MEDIATED),
        "custody": proof.get("custody", LOCAL_ONLY),
        "proof_ref": PROOF_REL,
        "proof_hash": proof.get("proof_hash"),
        "source_scope_hash": proof.get("source_scope_hash"),
        "errors": errors,
        "reasons": [] if errors else list(proof.get("blocked_reasons", [])),
    }


def print_summary(summary: dict[str, Any]) -> None:
    print(f"l2_broker={summary['status']}")
    print(f"l2_broker_claim_gate={summary['claim_gate']}")
    print(f"l2_broker_execution={summary['execution']}")
    print(f"l2_broker_custody={summary['custody']}")
    if summary.get("proof_ref"):
        print(f"l2_broker_proof_ref={summary['proof_ref']}")
    if summary.get("proof_hash"):
        print(f"l2_broker_proof_hash={summary['proof_hash']}")
    if summary.get("errors"):
        print("l2_broker_errors=" + ",".join(str(item) for item in summary["errors"]))
    if summary.get("reasons"):
        print("l2_broker_reasons=" + ",".join(str(item) for item in summary["reasons"]))
