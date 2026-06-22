#!/usr/bin/env python3
"""Generate CHECKPOINT_MANIFEST.json for the exact current VAC source tree."""
from __future__ import annotations
import hashlib
import json
import os
import pathlib
import sys
from datetime import datetime, timezone
from typing import Any

from cargo_tv_status import REQUIRED_CHECKS, TV_PASS, cargo_tv_summary

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
SLUG = sys.argv[2] if len(sys.argv) > 2 else "vac-runtime-v19-storage-cleanup-source-clean"
EXCLUDED_DIRS = {".git", "target", "node_modules", "__pycache__"}
EXCLUDED_FILES: set[str] = set()  # include the manifest itself to avoid final ZIP self-count off-by-one


def read_json(path: pathlib.Path, default: Any) -> Any:
    try:
        return json.loads(path.read_text())
    except Exception:
        return default


def sha(path: pathlib.Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def rel(path: pathlib.Path) -> str:
    return path.relative_to(ROOT).as_posix()


def actual_jsonl_counts() -> dict[str, int]:
    counts: dict[str, int] = {}
    idx = ROOT / ".vac/index"
    if not idx.exists():
        return counts
    for path in sorted(idx.glob("*.jsonl")):
        counts[path.stem] = sum(1 for line in path.read_text(encoding="utf-8", errors="replace").splitlines() if line.strip())
    return counts


def source_tree_inventory() -> dict[str, Any]:
    total_files = 0
    total_bytes = 0
    by_ext: dict[str, int] = {}
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        parts = set(path.relative_to(ROOT).parts)
        if parts & EXCLUDED_DIRS or path.name in EXCLUDED_FILES:
            continue
        total_files += 1
        total_bytes += path.stat().st_size
        ext = path.suffix or "<none>"
        by_ext[ext] = by_ext.get(ext, 0) + 1
    return {"total_files": total_files, "total_bytes": total_bytes, "includes_checkpoint_manifest": True, "by_extension": dict(sorted(by_ext.items()))}


def main() -> int:
    index_manifest = read_json(ROOT / ".vac/index/index_manifest.json", {})
    index_counts = actual_jsonl_counts()
    status = read_json(ROOT / ".vac/registry/status.json", {})
    caps = read_json(ROOT / ".vac/cache/compiled/capabilities/current.json", {}) or read_json(ROOT / ".vac/registry/compiled/capabilities/current.json", {})
    workspace = read_json(ROOT / ".vac/cache/compiled/workspace.json", {}) or read_json(ROOT / ".vac/registry/compiled/workspace.json", {})
    assessment = read_json(ROOT / ".vac/assessment/baseline.json", {})
    gap = read_json(ROOT / ".vac/assessment/gap_report.json", {})
    cargo_tv = cargo_tv_summary(ROOT)
    cargo_tv_status = cargo_tv.get("status", "NotEvaluated")
    cargo_pending = [] if cargo_tv_status == TV_PASS else list(REQUIRED_CHECKS)
    manifest = {
        "schema_version": 1,
        "kind": "registry_status",
        "id": f"artifact.{SLUG}",
        "created_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "source_zip": f"{SLUG}.zip",
        "baseline": "VAC v1.9 current source tree",
        "runtime_label": "VAC Runtime v1.9 L1 Static Closure Candidate — storage split and runtime-journal scaffold",
        "sv_done": True,
        "tv_pending": cargo_pending + ["tui_pty_visual_qa", "l2_broker_os_sandbox"],
        "l2": "not_implemented",
        "cargo_tv": {
            "status": cargo_tv_status,
            "checks": cargo_tv.get("checks", {}),
            "proof_ref": cargo_tv.get("proof_ref"),
            "proof_hash": cargo_tv.get("proof_hash"),
            "cargo_workspace_hash": cargo_tv.get("cargo_workspace_hash"),
            "git_head": cargo_tv.get("git_head"),
            "generated_at": cargo_tv.get("generated_at"),
            "errors": cargo_tv.get("errors", []),
        },
        "runtime_authority": status.get("runtime_authority"),
        "compiled_snapshot_hash": status.get("compiled_snapshot_hash"),
        "readiness_summary": status.get("readiness_summary"),
        "canonical_capabilities_summary": caps.get("summary"),
        "index_manifest_counts": index_manifest.get("counts"),
        "index_counts": index_counts,
        "checkpoint_index_counts": index_counts,
        "assessment_summary": assessment.get("summary"),
        "assessment_snapshot_hash": assessment.get("snapshot_hash"),
        "gap_summary": gap.get("summary"),
        "compiled_outputs": workspace.get("compiled_outputs", []),
        "source_tree": source_tree_inventory(),
        "storage_class": {
            "tracked_authority": [".vac/capabilities", ".vac/policies", ".vac/workflows", ".vac/surfaces", ".vac/specs/confirmed", ".vac/schemas", ".vac/migrations"],
            "runtime_journal": ".vac/db/runtime.db ignored local SQLite",
            "compiled_snapshot_cache": ".vac/cache/compiled",
            "generated_state_export": [".vac/index", ".vac/assessment", ".vac/registry/compiled", ".vac/evidence", ".vac/plans", ".vac/ledger", ".vac/registry/spec-sync"]
        },
        "source_zip_policy": "generated state excluded; paired state export zip includes reproducibility artifacts",
        "excluded_paths": [".git/", "target/", "node_modules/", ".vac/cache/compiled/", ".vac/index/", ".vac/assessment/", ".vac/registry/compiled/", ".vac/evidence/", ".vac/registry/evidence/", ".vac/plans/", ".vac/ledger/", ".vac/registry/spec-sync/", ".vac/db/*.db", ".vac/memories/*.db", "__pycache__/", "*.pyc"],
        "gates": [
            "scripts/vac-v19-final-sv-gate.sh",
            "scripts/check-v19-storage-classes.py .",
            "scripts/check-v19-runtime-db-schema.py .",
            "scripts/package-v19-checkpoint.py",
            "scripts/check-v19-package-hygiene.py",
            "scripts/vac-static-gate.sh",
            "scripts/check-checkpoint-integrity.py .",
            "fresh-zip-extract integrity replay",
            "unzip -t"
        ],
    }
    (ROOT / "CHECKPOINT_MANIFEST.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"manifest": "CHECKPOINT_MANIFEST.json", "index_counts": index_counts}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
