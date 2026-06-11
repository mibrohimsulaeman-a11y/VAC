#!/usr/bin/env python3
"""Validate VAC v1.9 storage-class discipline in the working tree."""
from __future__ import annotations
import json, pathlib, re, sys
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 local TV gate compatibility.
    import tomli as tomllib

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors: list[str] = []
warnings: list[str] = []

def fail(msg: str) -> None: errors.append(msg)

def read_json(path: pathlib.Path):
    try: return json.loads(path.read_text())
    except Exception as exc:
        fail(f"invalid json {path.relative_to(ROOT)}: {exc}"); return {}

vac_toml = ROOT / ".vac/vac.toml"
if not vac_toml.exists():
    fail("missing .vac/vac.toml")
else:
    try:
        cfg = tomllib.loads(vac_toml.read_text())
    except Exception as exc:
        fail(f"invalid .vac/vac.toml: {exc}"); cfg = {}
    ws = cfg.get("workspace", {}) if isinstance(cfg, dict) else {}
    storage = cfg.get("storage", {}) if isinstance(cfg, dict) else {}
    registry = cfg.get("registry", {}) if isinstance(cfg, dict) else {}
    if ws.get("control_plane_version") != "1.9":
        fail(f".vac/vac.toml control_plane_version must be 1.9, got {ws.get('control_plane_version')!r}")
    if storage.get("runtime_journal") != ".vac/db/runtime.db":
        fail(".vac/vac.toml [storage].runtime_journal must be .vac/db/runtime.db")
    if storage.get("compiled_cache") != ".vac/cache/compiled":
        fail(".vac/vac.toml [storage].compiled_cache must be .vac/cache/compiled")
    if registry.get("compiled") != ".vac/cache/compiled":
        fail(".vac/vac.toml [registry].compiled must be .vac/cache/compiled")

required_authority_dirs = [
    ".vac/capabilities", ".vac/policies", ".vac/workflows", ".vac/surfaces", ".vac/specs/confirmed", ".vac/schemas", ".vac/migrations/runtime-db",
]
for rel in required_authority_dirs:
    if not (ROOT / rel).is_dir():
        fail(f"missing v1.9 authority directory: {rel}")

if list((ROOT / ".vac/specs").glob("*.yaml")):
    fail("confirmed specs must live under .vac/specs/confirmed; root .vac/specs/*.yaml is legacy")

ignored = (ROOT / ".gitignore").read_text(errors="ignore") if (ROOT / ".gitignore").exists() else ""
for token in [
    ".vac/db/*.db", ".vac/cache/**", ".vac/index/**", ".vac/assessment/**", ".vac/registry/compiled/**", ".vac/evidence/**",
    ".vac/registry/evidence/**", ".vac/plans/**", ".vac/ledger/**", ".vac/registry/spec-sync/**",
]:
    if token not in ignored:
        fail(f".gitignore missing generated/local-state pattern: {token}")

status = read_json(ROOT / ".vac/registry/status.json") if (ROOT / ".vac/registry/status.json").exists() else {}
if status:
    if status.get("control_plane_version") != "1.9":
        fail("registry status must report control_plane_version=1.9")
    if status.get("storage_class", {}).get("runtime_journal") != "local_sqlite_ignored":
        fail("registry status must classify runtime_journal as local_sqlite_ignored")
    if status.get("control_plane", {}).get("compiled_snapshot_location") != ".vac/cache/compiled":
        fail("registry status must point compiled_snapshot_location to .vac/cache/compiled")

if not (ROOT / "scripts/package-v19-checkpoint.py").is_file():
    fail("missing v1.9 source/state package splitter")
if not (ROOT / "scripts/check-v19-package-hygiene.py").is_file():
    fail("missing v1.9 package hygiene gate")
if not (ROOT / "scripts/check-v19-pair-integrity.py").is_file():
    fail("missing v1.9 source/state pair-integrity gate")
if not (ROOT / "scripts/check-v19-runtime-db-schema.py").is_file():
    fail("missing v1.9 runtime DB schema gate")

if errors:
    print("VAC v1.9 storage classes: FAIL")
    for e in errors: print("-", e)
    sys.exit(1)
print("VAC v1.9 storage classes: PASS")
print("authority_dirs=" + ",".join(required_authority_dirs))
print("generated_state_policy=source_zip_excluded,state_export_included")
for w in warnings: print("warning:", w)
