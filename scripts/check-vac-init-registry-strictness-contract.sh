#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-strictness.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_file vac-rs/control-plane/src/control_plane/vac_init_registry_strictness.rs
require_file vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_file .vac/capabilities/vac-init-registry-strictness.yaml
require_file .vac/workflows/maintenance.vac-init-registry-strictness.yaml
require_file docs/vac-init/VAC_INIT_PRODUCTION_HARDENING_A.md
require_file docs/validation/PRODUCTION_HARDENING_A1_A3_VALIDATION.md

if command -v "$RUSTC_BIN" >/dev/null 2>&1; then
  "$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_registry_strictness.rs -o "$TMPROOT/vac_init_registry_strictness_test"
  "$TMPROOT/vac_init_registry_strictness_test" --nocapture
else
  echo "registry strictness rustc unit gate: NotEvaluated (rustc not found: $RUSTC_BIN)" >&2
fi

PY_STDERR="$TMPROOT/python-stderr.log"
if ! python3 - <<'PY' 2>"$PY_STDERR"
from __future__ import annotations

import pathlib
import re
import sys
from collections import Counter

try:
    import yaml
except Exception as exc:  # pragma: no cover - operator diagnostic
    raise SystemExit(f"FAIL: python yaml module is required for strict .vac scan: {exc}")

ROOT = pathlib.Path(".")
VAC = ROOT / ".vac"
SPEC_KINDS = {
    "capability",
    "policy",
    "workflow",
    "workflow_step",
    "surface",
    "registry_status",
    "domains",
    "init_state",
    "evidence",
    "plan",
    "approval_request",
    "ownership_report",
    "memory_record",
    "risk_finding",
    "migration",
    "runtime_owner_support",
    "trajectory",
    "test_assertion",
}
COMPAT_KINDS = {"product", "status", "donor_inventory"}
READY_FIELDS = ("owner", "ownership", "policy", "surfaces", "validation", "docs")
CMD_FIELDS = ("id", "runner", "args", "risk", "approval")
RISKS = {"safe_read", "low", "medium", "high", "critical", "execute_process"}
APPROVALS = {"policy", "always", "never"}
KNOWN_RUNNERS = {"cargo", "rustc", "rustfmt", "python3", "bash", "sh", "git", "vac", "echo", "rg"}
ID_RE = re.compile(r"^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)+$")
CMD_ID_RE = re.compile(r"^[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)+$")
SHELL_META = ("|", ">", "<", "&&", "||", ";", "`", "$(", "${")

errors: list[str] = []
warnings: list[str] = []
ids: dict[str, pathlib.Path] = {}
capabilities: dict[str, tuple[pathlib.Path, dict]] = {}
surface_refs: list[tuple[pathlib.Path, str, str]] = []
validation_count = 0
kind_counts: Counter[str] = Counter()

def fail(path: pathlib.Path, field: str, message: str, hint: str | None = None) -> None:
    suffix = f" (hint: {hint})" if hint else ""
    errors.append(f"{path}:{field}: {message}{suffix}")

def warn(path: pathlib.Path, field: str, message: str) -> None:
    warnings.append(f"{path}:{field}: {message}")

def truthy_block(value) -> bool:
    return value not in (None, {}, [])

def check_command(path: pathlib.Path, command, index: int) -> None:
    global validation_count
    field = f"validation.commands[{index}]"
    if isinstance(command, str):
        fail(path, field, "free-form validation command is forbidden", "replace it with id/runner/args/risk/approval object")
        return
    if not isinstance(command, dict):
        fail(path, field, "validation command must be a mapping")
        return
    validation_count += 1
    for key in CMD_FIELDS:
        if key not in command:
            fail(path, f"{field}.{key}", "structured command missing required field")
    if not all(key in command for key in CMD_FIELDS):
        return

    command_id = str(command["id"])
    runner = str(command["runner"])
    args = command["args"]
    risk = str(command["risk"])
    approval = str(command["approval"])

    if not CMD_ID_RE.match(command_id):
        fail(path, f"{field}.id", f"command id {command_id!r} is not a lower dotted identifier")
    if "/" in runner or "\\" in runner or any(meta in runner for meta in SHELL_META):
        fail(path, f"{field}.runner", f"runner {runner!r} must be an executable name, not a path or shell fragment")
    if runner not in KNOWN_RUNNERS:
        fail(path, f"{field}.runner", f"runner {runner!r} is not in the strict runner registry")
    if not isinstance(args, list) or not all(isinstance(arg, str) for arg in args):
        fail(path, f"{field}.args", "args must be a list of strings")
        return
    if runner in {"bash", "sh"}:
        if not args:
            fail(path, f"{field}.args", "shell runner must point to a checked-in script")
        elif args[0] in {"-c", "--command"}:
            fail(path, f"{field}.args", "shell inline -c/--command is forbidden")
        elif not args[0].startswith("scripts/"):
            fail(path, f"{field}.args", f"shell runner first arg {args[0]!r} must be a checked-in script path")
    for arg in args:
        if any(meta in arg for meta in SHELL_META):
            fail(path, f"{field}.args", f"arg {arg!r} contains shell metacharacters")
    if risk not in RISKS:
        fail(path, f"{field}.risk", f"unsupported risk {risk!r}")
    if approval not in APPROVALS:
        fail(path, f"{field}.approval", f"unsupported approval mode {approval!r}")

def walk_validation(path: pathlib.Path, node) -> None:
    if isinstance(node, dict):
        validation = node.get("validation")
        if isinstance(validation, dict) and isinstance(validation.get("commands"), list):
            for i, command in enumerate(validation["commands"]):
                check_command(path, command, i)
        for value in node.values():
            walk_validation(path, value)
    elif isinstance(node, list):
        for value in node:
            walk_validation(path, value)

# Generated init risk-finding shards can be multi-megabyte scanner artifacts.
# Strict registry envelope validation covers source-controlled registries and
# operator-facing init state; risk-finding payload shape is owned by the scanner
# gates that produced those artifacts.
paths = sorted(
    path
    for path in VAC.rglob("*.yaml")
    if not str(path).startswith(".vac/.init/risk_findings/")
)
for path in paths:
    try:
        data = yaml.safe_load(path.read_text(encoding="utf-8"))
    except Exception as exc:
        fail(path, "<parse>", f"YAML parse failed: {exc}")
        continue
    if not isinstance(data, dict):
        fail(path, "<root>", "root YAML must be a mapping")
        continue

    for field in ("schema_version", "kind", "id"):
        if field not in data:
            fail(path, field, "missing schema envelope field")

    schema_version = data.get("schema_version")
    kind = str(data.get("kind", ""))
    manifest_id = str(data.get("id", ""))
    kind_counts[kind] += 1

    if schema_version != 1:
        fail(path, "schema_version", f"unsupported schema_version {schema_version!r}")
    if kind in COMPAT_KINDS:
        fail(path, "kind", f"compatibility kind {kind!r} is forbidden in strict mode", "migrate to registry_status or another spec kind")
    elif kind not in SPEC_KINDS:
        fail(path, "kind", f"unknown manifest kind {kind!r}")
    if not ID_RE.match(manifest_id):
        fail(path, "id", f"id {manifest_id!r} must be dotted")

    previous = ids.get(manifest_id)
    if previous is not None:
        fail(path, "id", f"duplicate id {manifest_id!r}; first seen at {previous}")
    ids[manifest_id] = path

    if kind == "capability":
        capabilities[manifest_id] = (path, data)
        if data.get("status") == "ready":
            for field in READY_FIELDS:
                if not truthy_block(data.get(field)):
                    fail(path, field, "ready capability is missing a required production field")
            for doc in data.get("docs") or []:
                if not isinstance(doc, str) or not doc.strip():
                    fail(path, "docs[]", "docs entries must be non-empty relative paths")
                elif not (ROOT / doc).is_file():
                    fail(path, "docs[]", f"doc path {doc!r} does not exist")
        if data.get("status") == "planned" and truthy_block(data.get("surfaces")):
            warn(path, "surfaces", "planned capability declares surfaces; ensure no visible executable route references it")

    if kind == "surface":
        for capability in data.get("capabilities") or []:
            surface_refs.append((path, "capabilities[]", str(capability)))
        for route in data.get("routes") or []:
            if isinstance(route, dict):
                capability = route.get("capability")
                if capability:
                    surface_refs.append((path, "routes[].capability", str(capability)))
                if route.get("visible") is True and route.get("status") == "planned":
                    fail(path, "routes[].status", "planned route must not be visible/executable")

    walk_validation(path, data)

for path, field, capability_id in surface_refs:
    if capability_id not in capabilities:
        fail(path, field, f"surface references unknown capability {capability_id!r}")
    else:
        cap_status = capabilities[capability_id][1].get("status")
        if cap_status == "planned":
            fail(path, field, f"surface references planned capability {capability_id!r}")

if errors:
    print("vac-init registry strictness: FAIL")
    for line in errors:
        print(f"ERROR: {line}")
    for line in warnings:
        print(f"WARN:  {line}")
    print(f"summary: manifests={len(paths)} structured_validation_commands={validation_count} kinds={dict(kind_counts)}")
    raise SystemExit(1)

print("vac-init registry strictness: PASS")
print(f"summary: manifests={len(paths)} structured_validation_commands={validation_count} capabilities={len(capabilities)} kinds={dict(kind_counts)} warnings={len(warnings)}")
for line in warnings:
    print(f"WARN: {line}")
PY
then
  cat "$PY_STDERR" >&2
  exit 1
fi
