#!/usr/bin/env python3
"""Validate VAC v1.9 clean-source and state-export ZIP split."""
from __future__ import annotations
import fnmatch, pathlib, subprocess, sys, zipfile

if len(sys.argv) < 3:
    print("usage: check-v19-package-hygiene.py SOURCE_CLEAN_ZIP STATE_EXPORT_ZIP")
    sys.exit(2)
source_zip = pathlib.Path(sys.argv[1]).resolve()
state_zip = pathlib.Path(sys.argv[2]).resolve()
errors: list[str] = []
SOURCE_FORBIDDEN_PREFIXES = (
    ".git/", "target/", "node_modules/", ".vac/index/", ".vac/assessment/", ".vac/registry/", ".vac/cache/compiled/",
    ".vac/evidence/", ".vac/plans/", ".vac/ledger/", ".vac/memories/", ".vac/session/",
)
SOURCE_FORBIDDEN_EXACT = {".vac/db/runtime.db", "SV_VALIDATION.log", "SV_POST_EVIDENCE_VALIDATION.log"}
SOURCE_ALLOWED_MARKERS = {".vac/db/.gitignore", ".vac/cache/.gitkeep", ".vac/cache/README.md", ".vac/exports/.gitkeep", ".vac/exports/README.md"}
SOURCE_FORBIDDEN_SESSION_PATTERNS = ("secrets.json", "auto_approve.json", "prompt_*.txt")
SOURCE_REQUIRED = {
    "README.md", "GETTING-STARTED.md", ".vac/vac.toml", ".vac/db/.gitignore", ".vac/migrations/runtime-db/0001_runtime_journal.sql",
    "vac-rs/Cargo.toml", "scripts/package-v19-checkpoint.py", "scripts/check-v19-storage-classes.py",
}
STATE_REQUIRED_PREFIXES = (".vac/index/", ".vac/assessment/", ".vac/cache/compiled/")
STATE_FORBIDDEN_DEFAULT_PREFIXES = (".vac/registry/compiled/",)

def path_parts(member: str) -> tuple[str, ...]:
    return tuple(part for part in pathlib.PurePosixPath(member).parts if part not in ("", "."))

def has_path_segment(member: str, *segments: str) -> bool:
    parts = path_parts(member)
    width = len(segments)
    return any(parts[index:index + width] == segments for index in range(len(parts) - width + 1))

def is_session_member(member: str) -> bool:
    return has_path_segment(member, ".vac", "session")

def is_sensitive_session_member(member: str) -> bool:
    name = pathlib.PurePosixPath(member).name
    return any(fnmatch.fnmatchcase(name, pattern) for pattern in SOURCE_FORBIDDEN_SESSION_PATTERNS)

if not source_zip.exists(): errors.append(f"missing source zip: {source_zip}")
if not state_zip.exists(): errors.append(f"missing state zip: {state_zip}")
if errors:
    print("VAC v1.9 package hygiene: FAIL")
    for e in errors: print("-", e)
    sys.exit(1)
with zipfile.ZipFile(source_zip) as zf:
    source = set(zf.namelist())
with zipfile.ZipFile(state_zip) as zf:
    state = set(zf.namelist())
for member in source:
    if member in SOURCE_FORBIDDEN_EXACT:
        errors.append(f"source ZIP contains forbidden generated/local file: {member}")
    if is_session_member(member):
        if is_sensitive_session_member(member):
            errors.append(f"source ZIP contains forbidden runtime/session artifact: {member}")
        else:
            errors.append(f"source ZIP contains forbidden runtime/session path: {member}")
    if any(member.startswith(prefix) for prefix in SOURCE_FORBIDDEN_PREFIXES):
        # allow marker files that document ignored dirs
        if member not in SOURCE_ALLOWED_MARKERS:
            errors.append(f"source ZIP contains forbidden generated/local prefix: {member}")
for req in SOURCE_REQUIRED:
    if req not in source:
        errors.append(f"source ZIP missing required source/authority file: {req}")
for prefix in STATE_REQUIRED_PREFIXES:
    if not any(member.startswith(prefix) for member in state):
        errors.append(f"state export ZIP missing generated prefix: {prefix}")
for prefix in STATE_FORBIDDEN_DEFAULT_PREFIXES:
    if any(member.startswith(prefix) for member in state):
        errors.append(f"state export ZIP contains duplicate legacy compiled mirror by default: {prefix}")
if len(source) >= len(state) and source_zip.stat().st_size > state_zip.stat().st_size * 2:
    errors.append("source ZIP is unexpectedly larger than state export; split may be wrong")
if not errors:
    pair_checker = pathlib.Path(__file__).resolve().parent / "check-v19-pair-integrity.py"
    pair = subprocess.run([sys.executable, str(pair_checker), str(source_zip), str(state_zip)], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if pair.returncode != 0:
        errors.append("source/state pair integrity failed: " + pair.stdout.replace("\n", "; ")[:1200])
if errors:
    print("VAC v1.9 package hygiene: FAIL")
    for e in errors: print("-", e)
    sys.exit(1)
print("VAC v1.9 package hygiene: PASS")
print(f"source_files={len(source)} source_bytes={source_zip.stat().st_size}")
print(f"state_files={len(state)} state_bytes={state_zip.stat().st_size}")
print("pair_integrity=PASS")
