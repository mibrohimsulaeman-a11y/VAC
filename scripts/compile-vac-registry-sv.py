#!/usr/bin/env python3
"""Compile `.vac` YAML authoring manifests into deterministic JSON runtime snapshots.

SV contract for sandbox checkpoints:
- YAML remains the operator-editable authoring plane.
- `.vac/cache/compiled/**.json` is the v1.9 runtime cache for static gates.
- `.vac/registry/compiled/**.json` may be mirrored only for legacy/export compatibility.
- Every compiled artifact that references source YAML carries fresh `path+sha256`.
- `capabilities.json`, `capabilities/current.json`, and `registry/status.json`
  share one readiness authority.
- Dynamic timestamps are kept outside the hashed source-reference checks.
"""
from __future__ import annotations

import hashlib
import os
import json
import pathlib
import sys
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 local TV gate compatibility.
    import tomli as tomllib
from collections import Counter
from datetime import datetime, timezone
from typing import Any

import yaml

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
NOW = os.environ.get("VAC_SV_GENERATED_AT", "1970-01-01T00:00:00Z")
RANK = {"deprecated": 0, "planned": 0, "partial": 1, "ready": 2}
COMPILED_ROOTS = [
    pathlib.Path(".vac/cache/compiled"),
    pathlib.Path(".vac/registry/compiled"),  # legacy mirror; excluded from v1.9 source zips
]

AUTHORING_GLOBS = {
    "capabilities": [".vac/capabilities/*.yaml"],
    "policies": [".vac/policies/*.yaml"],
    "specs": [".vac/specs/confirmed/*.yaml", ".vac/specs/*.yaml"],
    "surfaces": [".vac/surfaces/*.yaml"],
    "workflows": [".vac/workflows/*.yaml"],
}


def rel(path: pathlib.Path) -> str:
    return path.relative_to(ROOT).as_posix()


def sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())


def canonical_hash(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return sha256_bytes(payload)


def write_json(path: pathlib.Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def write_compiled(rel_path: str, data: Any) -> None:
    for root_rel in COMPILED_ROOTS:
        write_json(ROOT / root_rel / rel_path, data)


def authoring_paths(name: str) -> list[pathlib.Path]:
    paths: list[pathlib.Path] = []
    for pattern in AUTHORING_GLOBS[name]:
        paths.extend(ROOT.glob(pattern))
    return sorted({path.resolve(): path for path in paths}.values())


def read_json(path: pathlib.Path, default: Any) -> Any:
    try:
        return json.loads(path.read_text())
    except Exception:
        return default


def yaml_load(path: pathlib.Path) -> dict[str, Any]:
    data = yaml.safe_load(path.read_text()) or {}
    if not isinstance(data, dict):
        raise SystemExit(f"YAML manifest is not a mapping: {rel(path)}")
    for key in ["schema_version", "kind", "id"]:
        if key not in data:
            raise SystemExit(f"YAML manifest missing {key}: {rel(path)}")
    return data


def readiness_triplet(manifest: dict[str, Any]) -> dict[str, Any]:
    raw = manifest.get("readiness")
    if not isinstance(raw, dict):
        status = str(manifest.get("status") or "partial")
        raw = {
            "declared": status,
            "computed": status,
            "effective": status,
            "reason": "legacy status fallback",
            "blockers": [],
        }
    declared = str(raw.get("declared") or raw.get("effective") or manifest.get("status") or "partial")
    computed = str(raw.get("computed") or declared)
    effective = str(raw.get("effective") or computed)
    blockers = [str(item) for item in (raw.get("blockers") or [])]
    # Until cargo/TV gates are evaluated, ready cannot be stronger than partial.
    if effective == "ready" and "tv_cargo_gates_pending" not in blockers:
        blockers.append("tv_cargo_gates_pending")
        effective = "partial"
        computed = min_level(computed, "partial")
    if RANK.get(effective, 1) > RANK.get(computed, 1):
        blockers.append("effective_lowered_by_sv_compiler")
        effective = computed
    return {
        "declared": declared,
        "computed": computed,
        "effective": effective,
        "reason": str(raw.get("reason") or "compiled from YAML authoring plane"),
        "blockers": sorted(set(blockers)),
        "last_checked_at": str(raw.get("last_checked_at") or NOW),
    }


def min_level(a: str, b: str) -> str:
    return a if RANK.get(a, 1) <= RANK.get(b, 1) else b


def first_surface(manifest: dict[str, Any]) -> str | None:
    surfaces = manifest.get("surfaces") or {}
    if isinstance(surfaces, dict):
        tui = surfaces.get("tui") or {}
        if isinstance(tui, dict) and tui.get("route"):
            return str(tui["route"])
        cli = surfaces.get("cli") or {}
        if isinstance(cli, dict):
            commands = cli.get("commands") or []
            if commands:
                return str(commands[0])
    return None


def source_ref(path: pathlib.Path) -> dict[str, str]:
    return {"path": rel(path), "sha256": sha256_file(path)}


def compiled_manifest_projection(path: pathlib.Path, manifest: dict[str, Any]) -> dict[str, Any]:
    src = source_ref(path)
    projection = {
        "schema_version": 1,
        "kind": "compiled_manifest_ref",
        "id": f"compiled.{manifest.get('id')}",
        "runtime_truth": "compiled-json",
        "audit_truth": "jcs-json",
        "manifest_kind": str(manifest.get("kind")),
        "manifest_id": str(manifest.get("id")),
        "source": src,
        "source_hashes": [src],
    }
    if manifest.get("kind") == "capability":
        projection["readiness"] = readiness_triplet(manifest)
    projection["snapshot_hash"] = canonical_hash({k: v for k, v in projection.items() if k != "snapshot_hash"})
    return projection


def compile_manifest_group(name: str, out_dir: str) -> dict[str, Any]:
    paths = authoring_paths(name)
    items: list[dict[str, Any]] = []
    for path in paths:
        manifest = yaml_load(path)
        item = compiled_manifest_projection(path, manifest)
        items.append(item)
        file_name = pathlib.Path(path).with_suffix("").name + ".json"
        write_compiled(f"{out_dir}/{file_name}", item)
    source_hashes = [item["source"] for item in items]
    aggregate = {
        "schema_version": 1,
        "kind": "compiled_manifest_group",
        "id": f"compiled.{name}.current",
        "runtime_truth": "compiled-json",
        "audit_truth": "jcs-json",
        "source_hashes": source_hashes,
        "items": items,
        "summary": {"total": len(items)},
    }
    aggregate["snapshot_hash"] = canonical_hash({k: v for k, v in aggregate.items() if k != "snapshot_hash"})
    write_compiled(f"{out_dir}/current.json", aggregate)
    return aggregate


def compile_capabilities() -> tuple[dict[str, Any], Counter[str]]:
    cap_dir = ROOT / ".vac/capabilities"
    caps: list[dict[str, Any]] = []
    ids: set[str] = set()
    for path in sorted(cap_dir.glob("*.yaml")):
        manifest = yaml_load(path)
        cid = str(manifest.get("id") or "")
        if not cid:
            raise SystemExit(f"capability missing id: {rel(path)}")
        if cid in ids:
            raise SystemExit(f"duplicate capability id: {cid}")
        ids.add(cid)
        readiness = readiness_triplet(manifest)
        owner = manifest.get("owner") if isinstance(manifest.get("owner"), dict) else {}
        validation = manifest.get("validation") if isinstance(manifest.get("validation"), dict) else {}
        commands = validation.get("commands") if isinstance(validation.get("commands"), list) else []
        cap = {
            "id": cid,
            "title": str(manifest.get("title") or cid),
            "readiness": readiness,
            "owner": {"crate": str(owner.get("crate") or ""), "module": str(owner.get("module") or "")},
            "ownership": manifest.get("ownership") or {},
            "surface": first_surface(manifest),
            "policy": manifest.get("policy") or {},
            "policy_risk": (manifest.get("policy") or {}).get("risk") if isinstance(manifest.get("policy"), dict) else None,
            "validation": validation,
            "validation_commands": [str(c.get("id")) for c in commands if isinstance(c, dict) and c.get("id")],
            "docs": list(manifest.get("docs") or []),
            "source": source_ref(path),
        }
        cap["snapshot_hash"] = canonical_hash({k: v for k, v in cap.items() if k != "snapshot_hash"})
        caps.append(cap)
        write_compiled(f"capabilities/{cid}.json", {
            "schema_version": 1,
            "kind": "compiled_capability",
            "id": f"compiled.{cid}",
            "runtime_truth": "compiled-json",
            "audit_truth": "jcs-json",
            "source": cap["source"],
            "source_hashes": [cap["source"]],
            "capability": cap,
            "snapshot_hash": cap["snapshot_hash"],
        })
    counts = Counter(c["readiness"]["effective"] for c in caps)
    summary = {
        "total": len(caps),
        "ready": counts.get("ready", 0),
        "partial": counts.get("partial", 0),
        "planned": counts.get("planned", 0),
        "deprecated": counts.get("deprecated", 0),
    }
    data = {
        "schema_version": 1,
        "kind": "compiled_manifest",
        "id": "compiled.capabilities.current",
        "runtime_truth": "compiled-json",
        "audit_truth": "jcs-json",
        "readiness_semantics": "effective_never_stronger_than_computed; tv_pending_lowers_ready_to_partial",
        "source_hashes": [c["source"] for c in caps],
        "summary": summary,
        "capabilities": caps,
    }
    data["snapshot_hash"] = canonical_hash({k: v for k, v in data.items() if k != "snapshot_hash"})
    write_compiled("capabilities.json", data)
    write_compiled("capabilities/current.json", data)
    return data, counts


def authoring_sources() -> list[dict[str, str]]:
    paths: list[pathlib.Path] = []
    for patterns in AUTHORING_GLOBS.values():
        for pattern in patterns:
            paths.extend(ROOT.glob(pattern))
    return [source_ref(path) for path in sorted(set(paths))]



def ownership_patterns(manifest: dict[str, Any]) -> list[str]:
    out: list[str] = []
    ownership = manifest.get("ownership") if isinstance(manifest.get("ownership"), dict) else {}
    targets = ownership.get("targets") if isinstance(ownership.get("targets"), list) else []
    for target in targets:
        if not isinstance(target, dict):
            continue
        include = target.get("include")
        if isinstance(include, list):
            out.extend(str(item) for item in include if item)
        elif isinstance(include, str):
            out.append(include)
    return sorted(set(out))



def policy_action_enum(action: str) -> str:
    return {
        "filesystem_read": "filesystem_read",
        "filesystem_write": "filesystem_write",
        "filesystem_delete": "filesystem_delete",
        "execute_process": "execute_process",
        "network_access": "network_access",
        "session_write": "session_write",
        "checkpoint_write": "checkpoint_write",
        "credential_read": "credential_read",
        "memory_write": "memory_write",
        "plan_modify": "plan_modify",
        "capability_register": "capability_register",
        "readiness_declare": "readiness_declare",
    }.get(action, action or "filesystem_read")


def compile_policy_rules() -> list[dict[str, Any]]:
    """Legacy flat rule projection kept for compatibility and audit diffing."""
    rules: list[dict[str, Any]] = []
    for path in sorted(ROOT.glob(".vac/policies/*.yaml")):
        manifest = yaml_load(path)
        default = str(manifest.get("default_decision") or "deny")
        for rule in manifest.get("rules") or []:
            if not isinstance(rule, dict):
                continue
            match = rule.get("match") if isinstance(rule.get("match"), dict) else {}
            network = match.get("network") if isinstance(match.get("network"), dict) else {}
            rules.append({
                "id": str(rule.get("id") or f"{manifest.get('id')}.rule.{len(rules)+1}"),
                "action": str(match.get("action") or ""),
                "decision": str(rule.get("decision") or default),
                "path": match.get("path"),
                "runner": match.get("runner"),
                "host": network.get("host"),
                "protocol": network.get("protocol"),
                "data_class": match.get("data_class"),
            })
    return rules


def compile_policy_snapshot() -> dict[str, Any]:
    """Compile YAML policy manifests into the single vac-policy::PolicySnapshot authority.

    State5 closure: BoundRuntimeController must not duplicate policy semantics.
    This projection mirrors vac_policy::PolicySnapshot field names so Rust can
    deserialize and evaluate the same snapshot used by scripts/TUI/doctor.
    """
    workspace_rules: list[dict[str, Any]] = []
    source_hashes: list[dict[str, str]] = []
    default_decision = "deny"
    for path in sorted(ROOT.glob(".vac/policies/*.yaml")):
        manifest = yaml_load(path)
        source_hashes.append(source_ref(path))
        default_decision = min_decision(default_decision, str(manifest.get("default_decision") or "deny"))
        for rule in manifest.get("rules") or []:
            if not isinstance(rule, dict):
                continue
            match = rule.get("match") if isinstance(rule.get("match"), dict) else {}
            network = match.get("network") if isinstance(match.get("network"), dict) else {}
            workspace_rules.append({
                "id": str(rule.get("id") or f"{manifest.get('id')}.rule.{len(workspace_rules)+1}"),
                "matcher": {
                    "action": policy_action_enum(str(match.get("action") or "filesystem_read")),
                    "path": match.get("path"),
                    "data_class": match.get("data_class"),
                    "network_host": network.get("host"),
                    "network_protocol": network.get("protocol"),
                },
                "decision": str(rule.get("decision") or manifest.get("default_decision") or "deny"),
                "reason": str(rule.get("reason") or "compiled from .vac policy manifest"),
            })
    snapshot = {
        "id": "vac.policy.snapshot.current",
        "default_decision": default_decision,
        "hardcoded_safety_denials": [
            {
                "id": "vac.policy.hard.deny_credentials",
                "matcher": {"action": "credential_read", "path": "any", "data_class": "secret_like", "network_host": None, "network_protocol": None},
                "decision": "deny",
                "reason": "Credential/secret-like reads are fail-closed unless a future L2 broker grants a narrower redacted path.",
            }
        ],
        "workspace_rules": workspace_rules,
        "capability_rules": [],
        "workflow_rules": [],
        "plan_rules": [],
        "scoped_grants": [],
        "source_hashes": source_hashes,
    }
    snapshot["snapshot_hash"] = canonical_hash({k: v for k, v in snapshot.items() if k != "snapshot_hash"})
    return snapshot


def min_decision(left: str, right: str) -> str:
    order = {"allow": 0, "approval_required": 1, "deny": 2}
    return left if order.get(left, 2) >= order.get(right, 2) else right


def script_runner_binding_for_command(command: dict[str, Any]) -> dict[str, Any] | None:
    if str(command.get("runner") or "") != "vac-script-runner":
        return None
    args = [str(arg) for arg in (command.get("args") or [])]
    script_id = None
    for idx, arg in enumerate(args):
        if arg == "--script-id" and idx + 1 < len(args):
            script_id = args[idx + 1]
            break
    if not script_id:
        return None
    script_registry = {
        "vac.static.gate": "scripts/vac-static-gate.sh",
        "vac.reaudit.final": "scripts/vac-reaudit-final-sv-gate.sh",
        "vac.final.sv": "scripts/run-final-sv-validation.py",
        "vac.state5.operational": "scripts/vac-runtime-state5-operational-sv.py",
    }
    script_path = script_registry.get(script_id)
    if not script_path:
        return {"script_id": script_id, "error": "script_id_not_registered"}
    p = ROOT / script_path
    return {
        "script_id": script_id,
        "script_path": script_path,
        "script_sha256": sha256_file(p) if p.exists() else None,
        "broker_owned_runner": "vac-script-runner",
        "immutable_binding": True,
    }



def compile_command_registry_from_caps(caps: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    seen: set[str] = set()
    for cap in caps:
        validation = cap.get("validation") if isinstance(cap.get("validation"), dict) else {}
        for cmd in validation.get("commands") or []:
            if not isinstance(cmd, dict) or not cmd.get("id") or not cmd.get("runner"):
                continue
            key = str(cmd["id"])
            if key in seen:
                continue
            seen.add(key)
            record = {
                "id": key,
                "runner": str(cmd["runner"]),
                "args": [str(arg) for arg in (cmd.get("args") or [])],
                "risk": str(cmd.get("risk") or "safe_read"),
                "approval": str(cmd.get("approval") or "policy"),
            }
            binding = script_runner_binding_for_command(cmd)
            if binding:
                record["script_runner_binding"] = binding
            out.append(record)
    return out


def script_runner_bindings(command_registry: list[dict[str, Any]]) -> list[dict[str, Any]]:
    bindings = []
    for command in command_registry:
        binding = command.get("script_runner_binding")
        if isinstance(binding, dict):
            bindings.append({"command_id": command.get("id"), **binding})
    return bindings


def compile_read_plan_tickets() -> list[dict[str, Any]]:
    path = ROOT / ".vac/index/read_plans.jsonl"
    records: list[dict[str, Any]] = []
    if not path.exists():
        return records
    for line in path.read_text().splitlines():
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except Exception:
            continue
        if not isinstance(row, dict):
            continue
        tid = row.get("ticket_id") or row.get("id")
        p = row.get("path")
        if tid and p:
            records.append({
                "id": str(tid),
                "path": str(p),
                "spans": [str(item) for item in (row.get("spans") or row.get("span_ids") or [])],
                "capability": row.get("capability"),
            })
    return records


def runtime_capability_records(caps: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for cap in caps:
        manifest_path = ROOT / cap["source"]["path"]
        manifest = yaml_load(manifest_path) if manifest_path.exists() else {}
        surface = cap.get("surface")
        out.append({
            "id": cap["id"],
            "readiness": cap["readiness"],
            "owned_paths": ownership_patterns(manifest),
            "validation_commands": cap.get("validation_commands") or [],
            "surfaces": [surface] if surface else [],
        })
    return out

def rust_crate_count() -> int:
    cargo_path = ROOT / "vac-rs/Cargo.toml"
    data = tomllib.loads(cargo_path.read_text())
    return len(data.get("workspace", {}).get("members", []))


def source_file_count() -> int:
    excluded_parts = {".git", "target", "node_modules", "__pycache__"}
    generated_prefixes = (
        ".vac/index/", ".vac/assessment/", ".vac/registry/compiled/", ".vac/cache/",
        ".vac/evidence/", ".vac/registry/evidence/", ".vac/plans/", ".vac/ledger/",
        ".vac/registry/spec-sync/", ".vac/registry/sessions/", ".vac/registry/runtime/",
        ".vac/registry/approvals/", ".vac/registry/ledger/", ".vac/registry/vector-cache/",
        ".vac/registry/status.json", ".vac/db/", ".vac/memories/", ".vac/exports/",
    )
    generated_names = {"CHECKPOINT_MANIFEST.json", "SANDBOX_HANDOFF.md", "SV_VALIDATION.log", "SV_POST_EVIDENCE_VALIDATION.log"}
    count = 0
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel_path = rel(path)
        if path.name in generated_names:
            continue
        if any(part in excluded_parts for part in path.parts):
            continue
        if any(rel_path.startswith(prefix) for prefix in generated_prefixes):
            continue
        count += 1
    return count


def refresh_runtime_registry() -> dict[str, Any]:
    path = ROOT / ".vac/registry/runtime/jobs.json"
    data = read_json(path, {})
    jobs = data.get("jobs") if isinstance(data.get("jobs"), list) else data.get("records")
    if not isinstance(jobs, list):
        jobs = []
    queued = sum(1 for j in jobs if isinstance(j, dict) and j.get("state") == "queued")
    running = sum(1 for j in jobs if isinstance(j, dict) and j.get("state") == "running")
    failed = sum(1 for j in jobs if isinstance(j, dict) and j.get("state") == "failed")
    out = {
        "schema_version": 1,
        "kind": "runtime_jobs",
        "id": "runtime.jobs.current",
        "generated_at": NOW,
        "source": "local_runtime_registry",
        "mode": data.get("mode") or "monitor-only",
        "autopilot": data.get("autopilot") or {"state": "not_running", "pid": None, "mode": "monitor_only", "env": "host"},
        "queue": {"queued": queued, "running": running},
        "summary": {"queued": queued, "running": running, "failed": failed, "queue": queued + running},
        "jobs": jobs,
        "records": jobs,
        "empty_state": data.get("empty_state") or "No runtime jobs detected in this sandbox checkpoint. VAC will show an honest empty state until one-shot/cron/filewatch jobs are recorded.",
    }
    write_json(path, out)
    return out


def refresh_sessions_index() -> None:
    sessions_root = ROOT / ".vac/registry/sessions"
    sessions_root.mkdir(parents=True, exist_ok=True)
    sessions = []
    for d in sorted(p for p in sessions_root.iterdir() if p.is_dir()):
        close = next(iter(sorted(d.glob("close*.yaml"))), None)
        task = next(iter(sorted(d.glob("task*.yaml"))), None)
        status = "open"
        if close:
            try:
                c = yaml_load(close)
                status = str(c.get("status") or c.get("state") or "closed")
            except Exception:
                status = "closed"
        elif task:
            try:
                t = yaml_load(task)
                status = str(t.get("state") or "open")
            except Exception:
                status = "open"
        sessions.append({"session": d.name, "status": status})
    write_json(sessions_root / "index.json", {"schema_version": 1, "kind": "session_index", "id": "sessions.current", "generated_at": NOW, "sessions": sessions})


def compile_workspace(compiled_caps: dict[str, Any], runtime_jobs: dict[str, Any], groups: dict[str, dict[str, Any]]) -> None:
    compiled_path = ROOT / ".vac/cache/compiled/capabilities/current.json"
    compiled_caps_hash = sha256_file(compiled_path)
    sources = authoring_sources()
    policy_rules = compile_policy_rules()
    policy_snapshot = compile_policy_snapshot()
    command_registry = compile_command_registry_from_caps(compiled_caps.get("capabilities", []))
    bindings = script_runner_bindings(command_registry)
    read_plan_tickets = compile_read_plan_tickets()
    runtime_registry_snapshot = {
        "authority": "compiled_json",
        "compiled_snapshot_current": True,
        "source_hashes": sources,
        "capabilities": runtime_capability_records(compiled_caps.get("capabilities", [])),
        "deterministic_index_current": True,
        "policy_loaded": bool(policy_rules) and bool(policy_snapshot.get("id")),
        "policy_rules": policy_rules,
        "policy_snapshot": policy_snapshot,
        "command_registry": command_registry,
        "script_runner_bindings": bindings,
        "read_plan_tickets": read_plan_tickets,
        "conformance_level": "l1",
    }
    workspace = {
        "schema_version": 1,
        "kind": "compiled_manifest",
        "id": "compiled.registry.current",
        "runtime_truth": "compiled-json",
        "audit_truth": "jcs-json",
        "readiness_semantics": "effective_never_stronger_than_computed",
        "source_hashes": sources,
        "runtime_registry_snapshot": runtime_registry_snapshot,
        "compiled_outputs": [
            {"path": ".vac/cache/compiled/capabilities/current.json", "sha256": compiled_caps_hash},
            *[
                {"path": f".vac/cache/compiled/{name}/current.json", "sha256": sha256_file(ROOT / f".vac/cache/compiled/{name}/current.json")}
                for name in sorted(groups)
            ],
        ],
    }
    workspace["snapshot_hash"] = canonical_hash({k: v for k, v in workspace.items() if k != "snapshot_hash"})
    write_compiled("workspace.json", workspace)
    workspace_hash = sha256_file(ROOT / ".vac/cache/compiled/workspace.json")
    summary = compiled_caps["summary"]
    total = int(summary.get("total", 0))
    ready = int(summary.get("ready", 0))
    valid_percent = int((ready * 100) / total) if total else 0
    status = {
        "schema_version": 1,
        "kind": "registry_status",
        "id": "status.workspace.current",
        "generated_at": NOW,
        "updated_at": NOW,
        "product": "VAC",
        "control_plane_version": "1.9",
        "rust_workspace": "vac-rs",
        "profile": "default",
        "provider": None,
        "model": None,
        "rulebook": "vac.core",
        "enforcement_level": "L1",
        "workspace": {"enforcement_level": "L1", "conformance_level": "L1", "rust_crate_count": rust_crate_count(), "file_count": source_file_count()},
        "control_plane": {
            "authoring": "yaml",
            "runtime": "compiled_json",
            "audit": "jcs_json",
            "compiled_snapshots_current": True,
            "compiled_snapshot_hash": workspace_hash,
            "capabilities_snapshot_hash": compiled_caps_hash,
            "compiled_snapshot_location": ".vac/cache/compiled",
            "generated_mirror_location": ".vac/registry/compiled",
        },
        "compiled_snapshot_hash": workspace_hash,
        "runtime_authority": "compiled_json",
        "runtime_mode": "local-control-plane",
        "runtime": {"server_default": False, "gateway_default": False, "jobs": ".vac/registry/runtime/jobs.json", "journal": ".vac/db/runtime.db", "compiled_cache": ".vac/cache/compiled"},
        "runtime_jobs": {"queued": runtime_jobs["queue"]["queued"], "running": runtime_jobs["queue"]["running"]},
        "readiness": {
            "source": "canonical compiled capabilities/current.json; cargo gates not evaluated",
            "valid_percent": valid_percent,
            "effective_percent": valid_percent,
            "ready": summary.get("ready", 0),
            "partial": summary.get("partial", 0),
            "planned": summary.get("planned", 0),
            "deprecated": summary.get("deprecated", 0),
        },
        "readiness_summary": summary,
        "spec_sync": {"unresolved_critical_drift": 0},
        "storage_class": {"authority_manifests": "tracked", "runtime_journal": "local_sqlite_ignored", "compiled_snapshot": "cache_or_db", "generated_artifacts": "export_only_by_default"},
        "tv_pending": ["cargo_metadata", "cargo_fmt", "cargo_check", "cargo_clippy", "cargo_test"],
    }
    write_json(ROOT / ".vac/registry/status.json", status)


def main() -> int:
    compiled_caps, counts = compile_capabilities()
    groups = {
        "policies": compile_manifest_group("policies", "policies"),
        "specs": compile_manifest_group("specs", "specs"),
        "surfaces": compile_manifest_group("surfaces", "surfaces"),
        "workflows": compile_manifest_group("workflows", "workflows"),
    }
    runtime_jobs = refresh_runtime_registry()
    refresh_sessions_index()
    compile_workspace(compiled_caps, runtime_jobs, groups)
    print("compiled_caps=" + str(len(compiled_caps["capabilities"])))
    print("readiness=" + json.dumps(compiled_caps["summary"], sort_keys=True))
    print("workspace_snapshot=.vac/cache/compiled/workspace.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
