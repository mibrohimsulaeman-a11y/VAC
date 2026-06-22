#!/usr/bin/env python3
"""Shared deterministic helpers for VAC local validation scripts."""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
from pathlib import Path
from typing import Any, Sequence


def now() -> str:
    return os.environ.get("VAC_SV_GENERATED_AT", "1970-01-01T00:00:00Z")


def recorded_at() -> str:
    return os.environ.get("VAC_EVIDENCE_RECORDED_AT", now())


def sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def canonical_hash(value: Any) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return sha256_bytes(raw)


def file_hash(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git_output(root: Path, *args: str, timeout: int = 30) -> str:
    try:
        proc = subprocess.run(
            ["git", *args],
            cwd=root,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=timeout,
        )
    except Exception:
        return ""
    return proc.stdout.strip() if proc.returncode == 0 else ""


def git_output_args(root: Path, args: Sequence[str], timeout: int = 30) -> str:
    return git_output(root, *args, timeout=timeout)


def read_json(path: Path, default: Any) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return default


def read_json_object(
    path: Path,
    *,
    missing_reason: str,
    invalid_json_prefix: str,
    not_object_reason: str,
) -> tuple[dict[str, Any] | None, list[str]]:
    if not path.is_file():
        return None, [missing_reason]
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return None, [f"{invalid_json_prefix}:{exc}"]
    if not isinstance(value, dict):
        return None, [not_object_reason]
    return value, []


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.exists():
        return rows
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        if isinstance(row, dict):
            rows.append(row)
    return rows


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def proof_path(root: Path, proof_rel: str) -> Path:
    return root / proof_rel


def proof_payload_hash(proof: dict[str, Any]) -> str:
    payload = dict(proof)
    payload.pop("proof_hash", None)
    return canonical_hash(payload)


def source_scope_hash(root: Path, claim_id: str, source_scope_paths: Sequence[str]) -> str:
    files: list[dict[str, Any]] = []
    for rel in source_scope_paths:
        path = root / rel
        if path.is_file():
            files.append({"path": rel, "sha256": file_hash(path)})
        else:
            files.append({"path": rel, "missing": True})
    return canonical_hash({"claim": claim_id, "files": files})
