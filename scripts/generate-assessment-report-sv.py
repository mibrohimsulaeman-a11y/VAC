#!/usr/bin/env python3
"""Generate current VAC assessment baseline + span-grounded gap report.

VAC v1.9 expands the assessment from a six-finding scaffold into deterministic
joins across intent manifests, code index facts, risk findings, and baseline
counts. This is still SV/no-Cargo and honest about TV/L2 pending work, but every
finding is current-span grounded or explicitly marked as intent-without-code.
"""
from __future__ import annotations

import fnmatch
import hashlib
import json
import pathlib
import sys
from datetime import datetime, timezone
from typing import Any

import yaml

from vac_script_common import canonical_hash, now, read_json, read_jsonl, recorded_at, write_json

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
OUT = ROOT / ".vac/assessment"



def load_yaml(path: pathlib.Path) -> dict[str, Any]:
    try:
        data = yaml.safe_load(path.read_text()) or {}
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def path_matches(pattern: str, value: str) -> bool:
    if pattern in {"*", "any", "workspace", "project"}:
        return True
    if pattern.endswith("/**"):
        prefix = pattern[:-3]
        return value == prefix or value.startswith(prefix + "/")
    return fnmatch.fnmatch(value, pattern)


def first_span_by_path(spans_by_path: dict[str, list[dict[str, Any]]], path: str) -> dict[str, Any] | None:
    spans = spans_by_path.get(path) or []
    for span in spans:
        if span.get("ast_path") == "file":
            return span
    return spans[0] if spans else None


def evidence_from_span(span: dict[str, Any]) -> dict[str, Any]:
    return {
        "path": span.get("path"),
        "span_id": span.get("span_id"),
        "span_sha256": span.get("span_sha256"),
        "lines": f"{span.get('start_line', span.get('line_start', 1))}-{span.get('end_line', span.get('line_end', 1))}",
        "ast_path": span.get("ast_path"),
        "normalized_fingerprint": span.get("normalized_fingerprint"),
        "matched_spans": [span.get("span_id")],
    }


def missing_code_evidence(capability: str) -> dict[str, Any]:
    return {
        "missing_code": {
            "intent_ref": capability,
            "reason": "capability ownership patterns produced no indexed code paths",
        },
        "matched_spans": [],
    }


def severity_for_code_path(path: str, risks_by_path: dict[str, list[dict[str, Any]]]) -> str:
    risks = risks_by_path.get(path) or []
    text = " ".join(str(r.get("risk") or r.get("inferred_risk") or r.get("category") or "") for r in risks)
    if any(token in text for token in ["credential", "secret", "network_access", "execute_process", "filesystem_delete"]):
        return "P1"
    if path.startswith("vac-rs/crates/runtime/") or path.startswith("vac-rs/crates/integrations/"):
        return "P1"
    return "P2"


def capability_patterns(manifest: dict[str, Any]) -> list[str]:
    out: list[str] = []
    ownership = manifest.get("ownership") if isinstance(manifest.get("ownership"), dict) else {}
    for target in ownership.get("targets") or []:
        if not isinstance(target, dict):
            continue
        include = target.get("include")
        if isinstance(include, list):
            out.extend(str(item) for item in include if item)
        elif isinstance(include, str):
            out.append(include)
    return sorted(set(out))


def sourceish(path: str) -> bool:
    return path.startswith(("vac-rs/", "scripts/", "vac-cli/", "docs/workflow-control-plane/", ".github/workflows/"))


def call_path_impact(path: str, relations_by_path: dict[str, list[dict[str, Any]]]) -> dict[str, Any]:
    relations = relations_by_path.get(path) or []
    relation_kinds = sorted(set(str(r.get("kind") or r.get("relation") or r.get("type") or "unknown") for r in relations))
    return {
        "relation_count": len(relations),
        "relation_kinds": relation_kinds[:12],
        "impact_radius": "high" if len(relations) > 250 else "medium" if len(relations) > 40 else "low",
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    index_manifest = read_json(ROOT / ".vac/index/index_manifest.json", {})
    counts = index_manifest.get("counts") or {}
    spans = read_jsonl(ROOT / ".vac/index/spans.jsonl")
    files = read_jsonl(ROOT / ".vac/index/files.jsonl")
    relations = read_jsonl(ROOT / ".vac/index/relations.jsonl")
    risks = read_jsonl(ROOT / ".vac/index/risks.jsonl")
    symbols = read_jsonl(ROOT / ".vac/index/symbols.jsonl")
    if not spans:
        raise SystemExit("cannot generate assessment without .vac/index/spans.jsonl")

    generated = now()
    index_hash = index_manifest.get("snapshot_hash") or canonical_hash(index_manifest)
    spans_by_path: dict[str, list[dict[str, Any]]] = {}
    for span in spans:
        p = str(span.get("path") or "")
        if p:
            spans_by_path.setdefault(p, []).append(span)
    relations_by_path: dict[str, list[dict[str, Any]]] = {}
    for rel in relations:
        p = str(rel.get("path") or rel.get("source_path") or rel.get("from_path") or "")
        if p:
            relations_by_path.setdefault(p, []).append(rel)
    risks_by_path: dict[str, list[dict[str, Any]]] = {}
    for risk in risks:
        p = str(risk.get("path") or risk.get("file") or "")
        if p:
            risks_by_path.setdefault(p, []).append(risk)

    cap_records: list[dict[str, Any]] = []
    for cap_path in sorted((ROOT / ".vac/capabilities").glob("*.yaml")):
        manifest = load_yaml(cap_path)
        cap_id = str(manifest.get("id") or cap_path.stem)
        patterns = capability_patterns(manifest)
        matched = sorted({str(row.get("path")) for row in files for pattern in patterns if row.get("path") and path_matches(pattern, str(row.get("path")))})
        cap_records.append({
            "capability": cap_id,
            "manifest": cap_path.relative_to(ROOT).as_posix(),
            "patterns": patterns,
            "code_paths": matched,
            "intent_spec": f".vac/specs/{cap_path.stem}.yaml",
        })

    owner_by_path: dict[str, list[str]] = {}
    for cap in cap_records:
        for p in cap["code_paths"]:
            owner_by_path.setdefault(p, []).append(cap["capability"])

    findings: list[dict[str, Any]] = []
    mappings: list[dict[str, Any]] = []

    # Intent -> code: every capability must either resolve to indexed paths or produce an explicit missing-code finding.
    for cap in cap_records:
        paths = cap["code_paths"]
        mappings.append({
            "join_kind": "intent_to_code",
            "capability": cap["capability"],
            "intent_spec": cap["intent_spec"],
            "patterns": cap["patterns"],
            "code_paths": paths[:80],
            "evidence_spans": [first_span_by_path(spans_by_path, p)["span_id"] for p in paths[:8] if first_span_by_path(spans_by_path, p)],
            "coverage_status": "matched" if paths else "intent_without_code",
        })
        if not paths:
            findings.append({
                "finding_id": "gap.intent_without_code." + cap["capability"].replace(".", "_"),
                "category": "intent_without_code",
                "severity": "P2",
                "intent_ref": cap["capability"],
                "recommendation": "Capability has no deterministic code join. Assign ownership targets or demote it to planned until code exists.",
                "evidence": missing_code_evidence(cap["capability"]),
                "rubric": {"deterministic_fact": True, "call_path_impact": "none", "blocks_release": False},
            })

    # Code -> intent: source-ish files must be owned by at least one capability. Emit current-span evidence.
    for row in files:
        path = str(row.get("path") or "")
        if not path or not sourceish(path):
            continue
        owners = owner_by_path.get(path, [])
        if owners:
            continue
        span = first_span_by_path(spans_by_path, path)
        if not span:
            continue
        severity = severity_for_code_path(path, risks_by_path)
        findings.append({
            "finding_id": "gap.code_without_intent." + hashlib.sha1(path.encode()).hexdigest()[:12],
            "category": "code_without_intent",
            "severity": severity,
            "intent_ref": "intent.workspace.vac.runtime_v1_5.assessment_grounded",
            "recommendation": "Register this source path under a capability ownership target or quarantine it from agent writes.",
            "evidence": evidence_from_span(span),
            "rubric": {
                "deterministic_fact": True,
                "semantic_explanation_constrained_to_span": True,
                "call_path_impact": call_path_impact(path, relations_by_path),
                "blocks_release": False,
            },
        })

    # Baseline -> code: surface high-relation/risk hotspots as deterministic assessment findings.
    hotspot_candidates = sorted(
        (p for p in spans_by_path if sourceish(p)),
        key=lambda p: (len(risks_by_path.get(p, [])), len(relations_by_path.get(p, []))),
        reverse=True,
    )[:25]
    for path in hotspot_candidates:
        if path in {f.get("evidence", {}).get("path") for f in findings}:
            continue
        span = first_span_by_path(spans_by_path, path)
        if not span:
            continue
        impact = call_path_impact(path, relations_by_path)
        risk_count = len(risks_by_path.get(path, []))
        if risk_count == 0 and impact["impact_radius"] == "low":
            continue
        findings.append({
            "finding_id": "gap.baseline_to_code." + hashlib.sha1(path.encode()).hexdigest()[:12],
            "category": "baseline_to_code",
            "severity": "P1" if risk_count or impact["impact_radius"] == "high" else "P2",
            "intent_ref": "baseline.engineering.security_maintenance",
            "recommendation": "High-risk/high-relation source requires capability-linked validation, evidence, and TV cargo coverage before readiness can rise.",
            "evidence": evidence_from_span(span),
            "rubric": {
                "deterministic_fact": True,
                "risk_count": risk_count,
                "call_path_impact": impact,
                "blocks_release": False,
            },
        })

    # Deduplicate by finding_id and sort deterministically.
    by_id = {f["finding_id"]: f for f in findings}
    findings = [by_id[k] for k in sorted(by_id)]

    baseline = {
        "schema_version": 1,
        "kind": "assessment_report",
        "id": "assessment.baseline.current",
        "generated_at": generated,
        "deterministic_generated_at": generated,
        "evidence_recorded_at": recorded_at(),
        "status": "sv_static_baselined_tv_pending",
        "index": index_manifest.get("id", "index.workspace.current"),
        "index_snapshot_hash": index_hash,
        "summary": {
            "files_indexed": int(counts.get("files", len(files))),
            "symbols_indexed": int(counts.get("symbols", len(symbols))),
            "relations_indexed": int(counts.get("relations", len(relations))),
            "risk_findings": int(counts.get("risks", len(risks))),
            "spans_indexed": int(counts.get("spans", len(spans))),
            "read_plans_indexed": int(counts.get("read_plans", 0)),
            "capabilities": len(cap_records),
            "intent_to_code_mappings": len(cap_records),
            "code_to_intent_paths": len(owner_by_path),
            "assessment_findings": len(findings),
        },
        "coverage": {
            "rust_ast_mode": (index_manifest.get("coverage") or {}).get("rust_ast_mode"),
            "semantic_span_level": "function_impl_module_file",
            "join_model": "intent_to_code + code_to_intent + baseline_to_code",
            "tv_cargo_gates": "NotEvaluated",
            "l2_broker": "NotImplemented",
            "assessment_authority": "sv_static_heuristic_scaffold",
            "severity_basis": "deterministic rubric over heuristic index; not product AST scoring",
        },
    }
    baseline["snapshot_hash"] = canonical_hash({k: v for k, v in baseline.items() if k != "snapshot_hash"})

    gap = {
        "schema_version": 1,
        "kind": "gap_report",
        "id": "gap.init.current",
        "generated_at": generated,
        "deterministic_generated_at": generated,
        "evidence_recorded_at": recorded_at(),
        "index": index_manifest.get("id", "index.workspace.current"),
        "index_snapshot_hash": index_hash,
        "intent": "intent.workspace.vac",
        "baseline_debt_policy": "Cargo/clippy/test/L2 broker remain TV-Pending by operator instruction; SV findings are current-span grounded. Static heuristic severity must not be marketed as AST-grounded product scoring.",
        "summary": {
            "findings_total": len(findings),
            "blocking_findings": sum(1 for f in findings if f["severity"] in {"P0", "critical"}),
            "p1_findings": sum(1 for f in findings if f["severity"] == "P1"),
            "p2_findings": sum(1 for f in findings if f["severity"] == "P2"),
            "fresh_against_index": True,
            "join_model": "intent_to_code/code_to_intent/baseline_to_code",
            "assessment_authority": "sv_static_heuristic_scaffold",
            "severity_basis": "heuristic_index_with_deterministic_rubric",
        },
        "mappings": mappings,
        "findings": findings,
    }
    gap["snapshot_hash"] = canonical_hash({k: v for k, v in gap.items() if k != "snapshot_hash"})

    write_json(OUT / "baseline.json", baseline)
    write_json(OUT / "gap_report.json", gap)
    print("VAC assessment report generated")
    print("baseline_counts=" + json.dumps(baseline["summary"], sort_keys=True))
    print("findings=" + str(len(findings)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
