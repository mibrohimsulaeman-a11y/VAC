#!/usr/bin/env python3
"""Create VAC v1.9 split artifacts: clean source ZIP + generated state export ZIP."""
from __future__ import annotations
import hashlib, json, os, pathlib, sys, time, zipfile
from typing import Iterable

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
OUT = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "/mnt/data").resolve()
PREFIX = sys.argv[3] if len(sys.argv) > 3 else "vac-runtime-v19-storage-cleanup"
OUT.mkdir(parents=True, exist_ok=True)

SOURCE_ZIP = OUT / f"{PREFIX}-source-clean.zip"
STATE_ZIP = OUT / f"{PREFIX}-state-export.zip"
MANIFEST = OUT / f"{PREFIX}-package-manifest.json"

EXCLUDE_PARTS = {".git", "target", "node_modules", "__pycache__", ".venv"}
SOURCE_EXCLUDE_PREFIXES = (
    ".vac/index/", ".vac/assessment/", ".vac/registry/", ".vac/cache/compiled/", ".vac/evidence/",
    ".vac/plans/", ".vac/ledger/", ".vac/db/", ".vac/exports/", ".vac/memories/",
)
SOURCE_EXCLUDE_NAMES = {"SV_VALIDATION.log", "SV_POST_EVIDENCE_VALIDATION.log", "CHECKPOINT_MANIFEST.json", "SANDBOX_HANDOFF.md"}
SOURCE_EXCLUDE_SUFFIXES = {".zip", ".db", ".db-wal", ".db-shm", ".pyc"}
STATE_PREFIXES = (
    ".vac/index/", ".vac/assessment/", ".vac/cache/compiled/", ".vac/evidence/",
    ".vac/registry/evidence/", ".vac/plans/", ".vac/ledger/", ".vac/registry/spec-sync/", ".vac/registry/sessions/",
    ".vac/registry/runtime/", ".vac/registry/status.json", ".vac/exports/",
)
LEGACY_COMPILED_MIRROR_PREFIX = ".vac/registry/compiled/"
INCLUDE_LEGACY_COMPILED_MIRROR = os.environ.get("VAC_INCLUDE_LEGACY_COMPILED_MIRROR") == "1"
STATE_ROOT_NAMES = {"CHECKPOINT_MANIFEST.json", "SV_VALIDATION.log", "SV_POST_EVIDENCE_VALIDATION.log", "SANDBOX_HANDOFF.md"}


def rel(path: pathlib.Path) -> str:
    return path.relative_to(ROOT).as_posix()

def sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def all_files() -> Iterable[pathlib.Path]:
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file():
            continue
        parts = set(path.relative_to(ROOT).parts)
        if parts & EXCLUDE_PARTS:
            continue
        yield path

def include_source(path: pathlib.Path) -> bool:
    r = rel(path)
    if path.name in SOURCE_EXCLUDE_NAMES:
        return False
    if any(r.startswith(prefix) for prefix in SOURCE_EXCLUDE_PREFIXES):
        # allow source authority markers under ignored dirs
        if r in {".vac/db/.gitignore", ".vac/cache/.gitkeep", ".vac/cache/README.md", ".vac/exports/.gitkeep", ".vac/exports/README.md"}:
            return True
        return False
    if path.suffix in SOURCE_EXCLUDE_SUFFIXES:
        return False
    return True

def include_state(path: pathlib.Path) -> bool:
    r = rel(path)
    if INCLUDE_LEGACY_COMPILED_MIRROR and r.startswith(LEGACY_COMPILED_MIRROR_PREFIX):
        return True
    return any(r.startswith(prefix) for prefix in STATE_PREFIXES) or path.name in STATE_ROOT_NAMES

def write_zip(zip_path: pathlib.Path, predicate) -> dict:
    if zip_path.exists():
        zip_path.unlink()
    members = []
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
        for path in all_files():
            if not predicate(path):
                continue
            r = rel(path)
            zf.write(path, r)
            members.append({"path": r, "bytes": path.stat().st_size, "sha256": "sha256:" + sha(path)})
    return {"path": str(zip_path), "bytes": zip_path.stat().st_size, "files": len(members), "members": members}

source = write_zip(SOURCE_ZIP, include_source)
state = write_zip(STATE_ZIP, include_state)
manifest = {
    "schema_version": 1,
    "kind": "vac_v19_split_checkpoint_manifest",
    "id": PREFIX,
    "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "source_zip": {k: v for k, v in source.items() if k != "members"},
    "state_export_zip": {k: v for k, v in state.items() if k != "members"},
    "policy": {
        "source_zip_excludes": list(SOURCE_EXCLUDE_PREFIXES),
        "state_export_includes": list(STATE_PREFIXES),
        "legacy_compiled_mirror": "excluded_by_default; set VAC_INCLUDE_LEGACY_COMPILED_MIRROR=1 for compatibility exports",
        "runtime_db": ".vac/db/runtime.db ignored/not packaged",
    },
    "sha256": {
        pathlib.Path(source["path"]).name: hashlib.sha256(pathlib.Path(source["path"]).read_bytes()).hexdigest(),
        pathlib.Path(state["path"]).name: hashlib.sha256(pathlib.Path(state["path"]).read_bytes()).hexdigest(),
    },
}
MANIFEST.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
print(json.dumps(manifest, indent=2, sort_keys=True))
