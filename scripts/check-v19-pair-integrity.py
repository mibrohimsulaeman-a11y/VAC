#!/usr/bin/env python3
"""Verify a VAC v1.9 source-clean ZIP and state-export ZIP as one pair.

This closes the State-8 false-pass class: a state export can be internally
consistent while its deterministic index is stale against the source-clean ZIP.
The gate extracts both artifacts, checks every FileRecord path+sha256 against
source-clean bytes, regenerates index/assessment from the extracted clean source
view, and compares exported hashes/counts with the recomputed ones.
"""
from __future__ import annotations

import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import zipfile
from typing import Any

TEXT_SUFFIXES = {
    ".rs", ".toml", ".yaml", ".yml", ".json", ".jsonl", ".md", ".txt", ".js", ".ts", ".sh", ".lock", ".schema", ".sql"
}
INDEX_EXCLUDED_PREFIXES = (
    ".vac/index/",
    ".vac/cache/",
    ".vac/registry/compiled/",
    ".vac/registry/runtime/",
    ".vac/registry/status.json",
    ".vac/registry/ledger/",
    ".vac/registry/vector-cache/",
    ".vac/memories/",
    ".vac/exports/",
    ".vac/assessment/",
    ".vac/registry/evidence/",
    ".vac/evidence/",
    ".vac/plans/",
    ".vac/ledger/",
    ".vac/registry/spec-sync/",
    ".vac/registry/sessions/",
    ".vac/registry/approvals/",
    ".vac/db/",
)
INDEX_EXCLUDED_NAMES = {"SV_VALIDATION.log", "SV_POST_EVIDENCE_VALIDATION.log", "CHECKPOINT_MANIFEST.json"}
INDEX_EXCLUDED_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".zip", ".db", ".msgpack", ".ico"}


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text())


def load_jsonl(path: pathlib.Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line_no, line in enumerate(path.read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except Exception as exc:  # pragma: no cover - used as CLI gate
            raise SystemExit(f"invalid JSONL {path}:{line_no}: {exc}") from exc
        if not isinstance(row, dict):
            raise SystemExit(f"invalid JSONL row {path}:{line_no}: expected object")
        rows.append(row)
    return rows


def sha_file(path: pathlib.Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def has_rel_segments(rel: str, *segments: str) -> bool:
    parts = pathlib.PurePosixPath(rel).parts
    if len(segments) > len(parts):
        return False
    return any(
        tuple(parts[index : index + len(segments)]) == segments
        for index in range(len(parts) - len(segments) + 1)
    )


def source_index_scope(rel: str) -> tuple[bool, str]:
    if rel in INDEX_EXCLUDED_NAMES:
        return False, "root_checkpoint_or_validation_log"
    if has_rel_segments(rel, ".vac", "session"):
        return False, "generated_or_local_state_session"
    if any(rel.startswith(prefix) for prefix in INDEX_EXCLUDED_PREFIXES):
        return False, "generated_or_local_state_prefix"
    suffix = pathlib.Path(rel).suffix.lower()
    if suffix in INDEX_EXCLUDED_SUFFIXES:
        return False, "binary_or_archive_suffix"
    if suffix not in TEXT_SUFFIXES:
        return False, "unsupported_non_text_suffix"
    return True, "indexed_text_source"


def run(cmd: list[str], cwd: pathlib.Path, log: list[str]) -> None:
    log.append("$ " + " ".join(cmd))
    proc = subprocess.run(cmd, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=240)
    log.append(proc.stdout.rstrip())
    log.append(f"exit={proc.returncode}")
    if proc.returncode != 0:
        raise SystemExit("command failed during pair integrity: " + " ".join(cmd) + "\n" + proc.stdout)


def compare_json_value(path: str, state_root: pathlib.Path, source_root: pathlib.Path, errors: list[str]) -> None:
    state_path = state_root / path
    source_path = source_root / path
    if not state_path.exists():
        errors.append(f"state export missing recomputed artifact: {path}")
        return
    if not source_path.exists():
        errors.append(f"recomputed source artifact missing: {path}")
        return
    if load_json(state_path) != load_json(source_path):
        errors.append(f"state export artifact differs from recomputed source-clean view: {path}")


def compare_jsonl_hash(path: str, state_root: pathlib.Path, source_root: pathlib.Path, errors: list[str]) -> None:
    state_path = state_root / path
    source_path = source_root / path
    if not state_path.exists() or not source_path.exists():
        errors.append(f"missing JSONL pair for {path}")
        return
    if hashlib.sha256(state_path.read_bytes()).hexdigest() != hashlib.sha256(source_path.read_bytes()).hexdigest():
        errors.append(f"state export JSONL differs from recomputed source-clean view: {path}")


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: check-v19-pair-integrity.py SOURCE_CLEAN_ZIP STATE_EXPORT_ZIP")
        return 2
    source_zip = pathlib.Path(sys.argv[1]).resolve()
    state_zip = pathlib.Path(sys.argv[2]).resolve()
    errors: list[str] = []
    log: list[str] = []
    with tempfile.TemporaryDirectory(prefix="vac-v19-pair-") as td:
        tmp = pathlib.Path(td)
        source_root = tmp / "source"
        state_root = tmp / "state"
        source_root.mkdir()
        state_root.mkdir()
        with zipfile.ZipFile(source_zip) as zf:
            zf.extractall(source_root)
        with zipfile.ZipFile(state_zip) as zf:
            zf.extractall(state_root)

        files_path = state_root / ".vac/index/files.jsonl"
        if not files_path.exists():
            errors.append("state export missing .vac/index/files.jsonl")
        else:
            file_records = load_jsonl(files_path)
            seen = set()
            for row in file_records:
                rel = str(row.get("path") or "")
                if not rel:
                    errors.append("FileRecord missing path")
                    continue
                if rel in seen:
                    errors.append(f"duplicate FileRecord path: {rel}")
                seen.add(rel)
                actual_path = source_root / rel
                if not actual_path.exists():
                    errors.append(f"FileRecord path missing from source-clean ZIP: {rel}")
                    continue
                expected_sha = row.get("sha256")
                actual_sha = sha_file(actual_path)
                if expected_sha != actual_sha:
                    errors.append(f"FileRecord sha mismatch: {rel} expected={expected_sha} actual={actual_sha}")

            for path in sorted(p for p in source_root.rglob("*") if p.is_file()):
                rel = path.relative_to(source_root).as_posix()
                in_scope, reason = source_index_scope(rel)
                if in_scope and rel not in seen:
                    errors.append(f"source-clean text file missing FileRecord: {rel}")
                elif not in_scope:
                    # The explicit reason is intentionally not emitted on pass; it documents the gate policy.
                    _ = reason

        # Recompute generated state from the exact source-clean view.  Copy the
        # exported generated state is deliberately avoided; source must stand on
        # its own authority manifests and scripts.
        required_scripts = [
            "scripts/generate-deterministic-index-sv.py",
            "scripts/compile-vac-registry-sv.py",
            "scripts/generate-assessment-report-sv.py",
            "scripts/generate-spec-sync-report-sv.py",
            "scripts/refresh-evidence-logs-sv.py",
            "scripts/refresh-confirmed-intent-status.py",
            "scripts/refresh-rust-ast-index-status.py",
        ]
        for script in required_scripts:
            if not (source_root / script).is_file():
                errors.append(f"source-clean ZIP missing required regeneration script: {script}")
        if not errors:
            run([sys.executable, "scripts/generate-deterministic-index-sv.py", "."], source_root, log)
            run([sys.executable, "scripts/compile-vac-registry-sv.py", "."], source_root, log)
            run([sys.executable, "scripts/generate-assessment-report-sv.py", "."], source_root, log)
            run([sys.executable, "scripts/generate-spec-sync-report-sv.py", "."], source_root, log)
            run([sys.executable, "scripts/refresh-evidence-logs-sv.py", "."], source_root, log)
            run([sys.executable, "scripts/refresh-confirmed-intent-status.py", "."], source_root, log)
            run([sys.executable, "scripts/refresh-rust-ast-index-status.py", "."], source_root, log)

            compare_json_value(".vac/index/index_manifest.json", state_root, source_root, errors)
            compare_json_value(".vac/assessment/baseline.json", state_root, source_root, errors)
            compare_json_value(".vac/assessment/gap_report.json", state_root, source_root, errors)
            compare_json_value(".vac/cache/compiled/workspace.json", state_root, source_root, errors)
            compare_json_value(".vac/cache/compiled/capabilities/current.json", state_root, source_root, errors)
            compare_json_value(".vac/registry/status.json", state_root, source_root, errors)
            for rel in [
                ".vac/index/files.jsonl",
                ".vac/index/spans.jsonl",
                ".vac/index/symbols.jsonl",
                ".vac/index/relations.jsonl",
                ".vac/index/risks.jsonl",
                ".vac/index/read_plans.jsonl",
            ]:
                compare_jsonl_hash(rel, state_root, source_root, errors)

    if errors:
        print("VAC v1.9 source/state pair integrity: FAIL")
        for error in errors:
            print("-", error)
        return 1
    exported_manifest = load_json(state_root / ".vac/index/index_manifest.json") if False else None
    print("VAC v1.9 source/state pair integrity: PASS")
    print("file_records_bound_to_source_clean=true")
    print("source_clean_recomputed_index_matches_state_export=true")
    print("source_clean_recomputed_assessment_matches_state_export=true")
    print("source_clean_recomputed_compiled_cache_matches_state_export=true")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
