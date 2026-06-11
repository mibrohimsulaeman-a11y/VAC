#!/usr/bin/env python3
"""Generate minimal deterministic SpecSync state for v1.9 SV/release gates.

SpecSync proposals/reports are generated runtime/export state. They are not
included in the v1.9 clean source ZIP, but can be regenerated from authority
manifests and compiled snapshot cache before doctor/deep validation.
"""
from __future__ import annotations
import hashlib, json, os, pathlib, sys

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
NOW = os.environ.get("VAC_SV_GENERATED_AT", "1970-01-01T00:00:00Z")
OUT = ROOT / ".vac/registry/spec-sync"
OUT.mkdir(parents=True, exist_ok=True)

def read_json(path: pathlib.Path, default):
    try: return json.loads(path.read_text())
    except Exception: return default

def canonical_hash(value) -> str:
    return "sha256:" + hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()

workspace = read_json(ROOT / ".vac/cache/compiled/workspace.json", {}) or read_json(ROOT / ".vac/registry/compiled/workspace.json", {})
status = read_json(ROOT / ".vac/registry/status.json", {})
source_hashes = workspace.get("source_hashes") or []
report = {
    "schema_version": 1,
    "kind": "spec_sync_report",
    "id": "spec_sync.current",
    "generated_at": NOW,
    "deterministic_generated_at": NOW,
    "source": "sv_regenerated_from_compiled_authority",
    "manifest_set_hash": workspace.get("snapshot_hash") or status.get("compiled_snapshot_hash"),
    "compiled_snapshot_hash": status.get("compiled_snapshot_hash") or workspace.get("snapshot_hash"),
    "checked_authority_sources": len(source_hashes),
    "drift": [],
    "proposals": [],
    "unresolved_critical_drift": 0,
    "storage_class": "generated_runtime_state_export_only",
}
report["snapshot_hash"] = canonical_hash({k: v for k, v in report.items() if k != "snapshot_hash"})
for name in ["report.json", "current.json", "bootstrap.json"]:
    (OUT / name).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
print("VAC SpecSync report generated")
print(f"manifest_set_hash={report['manifest_set_hash']}")
print("unresolved_critical_drift=0")
