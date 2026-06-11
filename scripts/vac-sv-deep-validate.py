#!/usr/bin/env python3
"""Static SV validation for the sandbox VAC architecture/TUI slice.

This intentionally avoids cargo build/check/clippy/test. It validates structural
contracts that are observable without a Rust toolchain.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 local TV gate compatibility.
    import tomli as tomllib
from collections import Counter

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
ERRORS: list[str] = []
WARNINGS: list[str] = []


def fail(msg: str) -> None:
    ERRORS.append(msg)


def warn(msg: str) -> None:
    WARNINGS.append(msg)


def read(path: str) -> str:
    try:
        return (ROOT / path).read_text(errors="ignore")
    except FileNotFoundError:
        fail(f"missing file: {path}")
        return ""


def load_json(path: str):
    text = read(path)
    if not text:
        return None
    try:
        return json.loads(text)
    except Exception as exc:  # noqa: BLE001 - validator reports exact parse failure
        fail(f"invalid json {path}: {exc}")
        return None


def ensure_dirs() -> None:
    required = [
        ".vac/capabilities", ".vac/policies", ".vac/surfaces", ".vac/workflows",
        ".vac/specs", ".vac/index", ".vac/assessment", ".vac/cache/compiled",
        ".vac/registry/runtime", ".vac/registry/spec-sync", ".vac/registry/sessions",
        ".vac/schemas", "vac-rs/crates/foundation", "vac-rs/crates/control-plane",
        "vac-rs/crates/runtime", "vac-rs/crates/surfaces", "vac-rs/crates/providers",
        "vac-rs/crates/integrations", "vac-rs/crates/capabilities", "vac-cli/bin",
        "docs/workflow-control-plane", "scripts", "tests/fixtures",
    ]
    for rel in required:
        if not (ROOT / rel).is_dir():
            fail(f"missing required directory: {rel}")
    for rel in ["cli", "tui", "libs"]:
        if (ROOT / rel).exists():
            fail(f"legacy top-level source directory still exists: {rel}")


def validate_workspace_manifest() -> None:
    cargo = ROOT / "vac-rs/Cargo.toml"
    if not cargo.is_file():
        fail("missing vac-rs/Cargo.toml")
        return
    try:
        parsed = tomllib.loads(cargo.read_text())
    except Exception as exc:  # noqa: BLE001
        fail(f"vac-rs/Cargo.toml is not valid TOML: {exc}")
        return
    workspace = parsed.get("workspace", {})
    members = workspace.get("members", [])
    if not members:
        fail("workspace.members is empty")
    seen = Counter(members)
    for member, count in seen.items():
        if count > 1:
            fail(f"duplicate workspace member: {member}")
    expected_prefixes = {
        "core", "crates/foundation/", "crates/control-plane/", "crates/runtime/", "crates/surfaces/",
        "crates/providers/", "crates/integrations/", "crates/capabilities/",
    }
    for member in members:
        if member != "core" and not any(member.startswith(prefix) for prefix in expected_prefixes if prefix != "core"):
            fail(f"workspace member outside VAC layered crate tree: {member}")
        if not (ROOT / "vac-rs" / member / "Cargo.toml").is_file():
            fail(f"workspace member missing Cargo.toml: {member}")
    deps = parsed.get("workspace", {}).get("dependencies", {})
    for name, value in deps.items():
        if isinstance(value, dict) and "path" in value:
            if not (ROOT / "vac-rs" / value["path"] / "Cargo.toml").is_file():
                fail(f"workspace dependency path missing Cargo.toml: {name} -> {value['path']}")


def parse_yaml_id(text: str) -> str | None:
    match = re.search(r"^id:\s*([^\s#]+)", text, flags=re.M)
    return match.group(1) if match else None


def validate_control_plane_files() -> None:
    cap_files = sorted((ROOT / ".vac/capabilities").glob("*.yaml"))
    if len(cap_files) < 8:
        fail(f"too few capability manifests: {len(cap_files)}")
    ids: list[str] = []
    for cap in cap_files:
        text = cap.read_text(errors="ignore")
        cid = parse_yaml_id(text)
        if not cid:
            fail(f"capability missing id: {cap.relative_to(ROOT)}")
            continue
        ids.append(cid)
        for needle in ["readiness:", "declared:", "computed:", "effective:", "owner:", "ownership:", "surfaces:", "validation:"]:
            if needle not in text:
                fail(f"{cap.relative_to(ROOT)} missing {needle.rstrip(':')}")
    for cid, count in Counter(ids).items():
        if count > 1:
            fail(f"duplicate capability id: {cid}")

    compiled = load_json(".vac/cache/compiled/capabilities.json") or load_json(".vac/registry/compiled/capabilities.json") or {}
    compiled_caps = compiled.get("capabilities", [])
    if len(compiled_caps) != len(cap_files):
        fail(f"compiled capabilities count {len(compiled_caps)} != yaml count {len(cap_files)}")
    for cap in compiled_caps:
        readiness = cap.get("readiness", {})
        declared = readiness.get("declared")
        computed = readiness.get("computed")
        effective = readiness.get("effective")
        if not all([declared, computed, effective]):
            fail(f"compiled capability missing readiness triplet: {cap.get('id')}")
        if effective == "ready" and computed != "ready":
            fail(f"effective readiness stronger than computed: {cap.get('id')}")

    workspace = load_json(".vac/cache/compiled/workspace.json") or load_json(".vac/registry/compiled/workspace.json") or {}
    if workspace.get("runtime_truth") not in {"compiled-json", "compiled_json"}:
        fail("compiled workspace does not declare compiled-json runtime truth")
    if workspace.get("audit_truth") not in {"jcs-json", "jcs_json"}:
        fail("compiled workspace does not declare jcs-json audit truth")
    if not workspace.get("source_hashes"):
        fail("compiled workspace missing source_hashes")


def validate_v15_reports() -> None:
    status = load_json(".vac/registry/status.json") or {}
    if status.get("product") != "VAC":
        fail("registry status product is not VAC")
    if status.get("runtime_authority") != "compiled_json":
        fail("registry status runtime_authority is not compiled_json")
    if status.get("runtime", {}).get("server_default") is not False:
        fail("registry status must mark server_default=false")
    if status.get("runtime", {}).get("gateway_default") is not False:
        fail("registry status must mark gateway_default=false")
    if "readiness_summary" not in status:
        fail("registry status missing readiness_summary")

    index = load_json(".vac/index/index_manifest.json") or {}
    if index.get("kind") != "index_manifest":
        fail("index manifest kind mismatch")
    outputs = index.get("outputs", {})
    for rel in ["files", "symbols", "relations", "spans", "risks", "read_plans"]:
        path = outputs.get(rel)
        if not path or not (ROOT / path).is_file():
            fail(f"index output missing or absent: {rel}")

    gap = load_json(".vac/assessment/gap_report.json") or {}
    if gap.get("kind") != "gap_report":
        fail("gap report kind mismatch")
    for finding in gap.get("findings", []):
        evidence = finding.get("evidence", {})
        if finding.get("severity") in {"P0", "P1"} and not evidence.get("span_id") and not evidence.get("matched_spans"):
            fail(f"critical assessment finding lacks span/missing-code evidence: {finding.get('finding_id')}")

    spec_sync = load_json(".vac/registry/spec-sync/report.json") or load_json(".vac/registry/spec-sync/current.json") or {}
    unresolved = spec_sync.get("unresolved_critical_drift", 0)
    if unresolved not in (0, "0", None):
        fail("spec-sync reports unresolved critical drift")



def validate_assessment_freshness_inline() -> None:
    manifest = load_json(".vac/index/index_manifest.json") or {}
    spans_rows: dict[str, dict] = {}
    spans_path = ROOT / ((manifest.get("outputs") or {}).get("spans") or ".vac/index/spans.jsonl")
    if not spans_path.exists():
        fail("missing .vac/index/spans.jsonl for assessment freshness")
        return
    for lineno, line in enumerate(spans_path.read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except Exception as exc:  # noqa: BLE001
            fail(f"invalid spans jsonl row {lineno}: {exc}")
            continue
        if isinstance(row, dict) and isinstance(row.get("span_id"), str):
            spans_rows[row["span_id"]] = row

    gap = load_json(".vac/assessment/gap_report.json") or {}
    for finding in gap.get("findings", []):
        if not isinstance(finding, dict):
            fail("assessment finding must be object")
            continue
        fid = finding.get("finding_id") or "<unknown>"
        evidence = finding.get("evidence", {})
        if not isinstance(evidence, dict):
            fail(f"assessment finding {fid} evidence must be object")
            continue
        span_ids = []
        if isinstance(evidence.get("span_id"), str):
            span_ids.append(evidence["span_id"])
        for item in evidence.get("matched_spans") or []:
            if isinstance(item, str):
                span_ids.append(item)
            elif isinstance(item, dict) and isinstance(item.get("span_id"), str):
                span_ids.append(item["span_id"])
        if not span_ids:
            missing_code = evidence.get("missing_code") or evidence.get("intent_without_code")
            if isinstance(missing_code, dict) and missing_code.get("intent_ref") and evidence.get("matched_spans") == []:
                continue
            fail(f"assessment finding lacks current span evidence: {fid}")
            continue
        for span_id in sorted(set(span_ids)):
            current = spans_rows.get(span_id)
            if current is None:
                fail(f"assessment stale span id: {fid} -> {span_id}")
                continue
            expected = evidence.get("span_sha256")
            if isinstance(expected, str):
                if not expected.startswith("sha256:"):
                    expected = "sha256:" + expected
                if expected != current.get("span_sha256"):
                    fail(f"assessment stale span hash: {fid} -> {span_id}")

    baseline = load_json(".vac/assessment/baseline.json") or {}
    summary = baseline.get("summary", {})
    counts = manifest.get("counts", {})
    for summary_key, count_key in {
        "files_indexed": "files",
        "symbols_indexed": "symbols",
        "relations_indexed": "relations",
        "risk_findings": "risks",
        "spans_indexed": "spans",
        "read_plans_indexed": "read_plans",
    }.items():
        if counts.get(count_key) is not None and summary.get(summary_key) != counts.get(count_key):
            fail(f"assessment baseline stale: {summary_key}={summary.get(summary_key)} current={counts.get(count_key)}")
    if baseline.get("index_snapshot_hash") != manifest.get("snapshot_hash"):
        fail("assessment baseline index_snapshot_hash mismatch")

def validate_runtime_jobs() -> None:
    jobs = load_json(".vac/registry/runtime/jobs.json") or {}
    if jobs.get("kind") != "runtime_jobs":
        fail("runtime jobs registry kind mismatch")
    rows = jobs.get("jobs", [])
    if not isinstance(rows, list):
        fail("runtime jobs field must be a list")
        rows = []
    for row in rows:
        title = row.get("title") or row.get("trigger", "")
        if title == "refactor handlers/input.rs":
            fail("runtime jobs registry contains supplied mock job row")
        if row.get("kind") not in {"one_shot", "oneshot", "cron", "filewatch", "manual", "workflow"}:
            warn(f"runtime job has nonstandard kind: {row.get('id')} -> {row.get('kind')}")


def validate_tui_operator() -> None:
    view = read("vac-rs/crates/surfaces/vac-tui/src/view.rs")
    if "render_vac_operator_console" not in view:
        fail("view.rs does not enter VAC operator renderer")
    operator = read("vac-rs/crates/surfaces/vac-tui/src/services/vac_operator.rs")
    for needle in [STATUS_PATH := ".vac/registry/status.json", JOBS_PATH := ".vac/registry/runtime/jobs.json", "latest_tool_call", "dialog_approval_state", "model_label(state)"]:
        if needle not in operator:
            fail(f"vac_operator.rs missing live-data contract marker: {needle}")
    if "take(5)" not in operator and "truncate(5)" not in operator:
        fail("vac_operator.rs must cap tool timeline to last five events")
    forbidden = [r"VIL-native", r"rulebook\s+vil\.core", r"refactor handlers/input\.rs", r"mutation gate"]
    for path in (ROOT / "vac-rs/crates/surfaces/vac-tui/src").rglob("*.rs"):
        text = path.read_text(errors="ignore")
        for lineno, line in enumerate(text.splitlines(), 1):
            if "assert!" in line or "concat!(" in line:
                continue
            for pattern in forbidden:
                if re.search(pattern, line, re.I):
                    fail(f"TUI mock literal {pattern} at {path.relative_to(ROOT)}:{lineno}")
    commands = read("vac-rs/crates/surfaces/vac-tui/src/services/commands.rs")
    for route in ["/runtime", "/capabilities", "/assessment", "/spec-sync"]:
        if route not in commands:
            fail(f"slash route missing from commands.rs: {route}")
    variants_match = re.search(r"pub enum CommandAction\s*\{(?P<body>.*?)\n\}", commands, flags=re.S)
    if variants_match:
        variants = [v.strip().strip(",") for v in variants_match.group("body").splitlines() if v.strip() and not v.strip().startswith("//")]
        for variant, count in Counter(variants).items():
            if count > 1:
                fail(f"duplicate CommandAction variant: {variant}")
    else:
        fail("CommandAction enum not found")


def validate_brand_scope() -> None:
    legacy_terms = [
        "upstream inherited brand",
        "UPSTREAM_INHERITED_BRAND",
        "stak" + "pak",
        "Stack" + "pak",
        "stack" + "pak",
        "StakAI",
        "Stakai",
        "stakai",
        "stak" + "apk",
        "." + ("stak" + "pak"),
    ]
    forbidden = re.compile("|".join(re.escape(term) for term in legacy_terms), re.I)
    allowed = {
        "scripts/check-brand-allowlist.py",
        "scripts/sv_static_validate.py",
        "scripts/vac-sv-deep-validate.py",
        "vac-rs/Cargo.lock",
        "vac-rs/crates/foundation/vac-brand/src/lib.rs",
        "vac-rs/crates/foundation/vac-paths/src/lib.rs",
    }
    allowed_prefixes = (
        "docs/workflow-control-plane/plans/",
        ".vac/registry/ledger/architecture-migration.yaml",
    )
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel = path.relative_to(ROOT).as_posix()
        if rel in allowed or any(rel.startswith(prefix) for prefix in allowed_prefixes):
            continue
        if any(part in {".git", "target", "node_modules"} for part in path.parts):
            continue
        if any(part in {"scripts", "__pycache__"} for part in path.parts):
            continue
        if path.suffix.lower() in {".png", ".jpg", ".jpeg", ".gif", ".lock", ".zip", ".pyc"}:
            continue
        try:
            with path.open("r", encoding="utf-8", errors="ignore") as fh:
                for line in fh:
                    if forbidden.search(line):
                        fail(f"legacy brand literal outside allowlist: {rel}")
                        break
        except OSError:
            continue

def main() -> int:
    ensure_dirs()
    validate_workspace_manifest()
    validate_control_plane_files()
    validate_v15_reports()
    validate_assessment_freshness_inline()
    validate_runtime_jobs()
    validate_tui_operator()
    validate_brand_scope()
    if ERRORS:
        print("SV DEEP FAIL")
        for err in ERRORS:
            print(f"- {err}")
        if WARNINGS:
            print("WARNINGS")
            for warning in WARNINGS:
                print(f"- {warning}")
        return 1
    print("SV DEEP PASS")
    if WARNINGS:
        print("WARNINGS")
        for warning in WARNINGS:
            print(f"- {warning}")
    print("validated=architecture,tui,runtime_jobs,compiled_registry,index,assessment,assessment_freshness,spec_sync,brand")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
