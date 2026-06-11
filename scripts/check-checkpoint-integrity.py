#!/usr/bin/env python3
"""Verify exact VAC checkpoint/source-tree integrity after extraction.

This is intentionally independent of Cargo. It checks the artifact properties
that caused the re-audit failure: stale compiled source hashes, stale index
counts, and readiness authority disagreement.
"""
from __future__ import annotations

import hashlib
import json
import pathlib
import sys
import subprocess
from typing import Any


# v1.9 pair mode: allow the same command name to validate split artifacts.
# Example: python3 scripts/check-checkpoint-integrity.py source-clean.zip state-export.zip
if len(sys.argv) == 3 and sys.argv[1].endswith('.zip') and sys.argv[2].endswith('.zip'):
    pair_checker = pathlib.Path(__file__).resolve().parent / 'check-v19-pair-integrity.py'
    proc = subprocess.run([sys.executable, str(pair_checker), sys.argv[1], sys.argv[2]], text=True)
    raise SystemExit(proc.returncode)

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors: list[str] = []
warnings: list[str] = []


def sha(path: pathlib.Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text())
    except Exception as exc:
        errors.append(f"invalid json {path.relative_to(ROOT)}: {exc}")
        return None


def walk_source_refs(value: Any):
    if isinstance(value, dict):
        if isinstance(value.get("path"), str) and isinstance(value.get("sha256"), str):
            yield value["path"], value["sha256"]
        for item in value.values():
            yield from walk_source_refs(item)
    elif isinstance(value, list):
        for item in value:
            yield from walk_source_refs(item)


def check_compiled_source_hashes() -> tuple[int, int]:
    checked = 0
    mismatches = 0
    compiled_roots = [ROOT / ".vac/cache/compiled", ROOT / ".vac/registry/compiled"]
    for compiled_root in compiled_roots:
        if not compiled_root.exists():
            continue
        for path in sorted(compiled_root.rglob("*.json")):
            data = read_json(path)
            if data is None:
                continue
            seen = set()
            for rel, expected in walk_source_refs(data):
                if not rel.startswith(".vac/") or not rel.endswith((".yaml", ".yml")):
                    continue
                key = (rel, expected)
                if key in seen:
                    continue
                seen.add(key)
                src = ROOT / rel
                checked += 1
                if not src.exists():
                    errors.append(f"compiled source reference missing: {path.relative_to(ROOT)} -> {rel}")
                    mismatches += 1
                    continue
                actual = sha(src)
                if actual != expected:
                    errors.append(f"compiled source hash mismatch: {path.relative_to(ROOT)} -> {rel} expected={expected[:19]} actual={actual[:19]}")
                    mismatches += 1
    return checked, mismatches

def jsonl_count(path: pathlib.Path) -> int:
    if not path.exists():
        errors.append(f"missing index output {path.relative_to(ROOT)}")
        return -1
    return sum(1 for line in path.read_text().splitlines() if line.strip())


def check_index_counts() -> dict[str, int]:
    manifest_path = ROOT / ".vac/index/index_manifest.json"
    manifest = read_json(manifest_path) or {}
    counts = manifest.get("counts") or {}
    outputs = manifest.get("outputs") or {}
    actual = {}
    for key in ["files", "symbols", "relations", "risks", "spans", "read_plans"]:
        out = outputs.get(key, f".vac/index/{key}.jsonl")
        actual[key] = jsonl_count(ROOT / out)
        if counts.get(key) != actual[key]:
            errors.append(f"index count mismatch: {key} manifest={counts.get(key)} actual={actual[key]}")
    return actual


def check_checkpoint_manifest_counts(actual_index: dict[str, int]) -> None:
    path = ROOT / "CHECKPOINT_MANIFEST.json"
    if not path.exists():
        # Source-workspace mode is valid for local TV gates: root handoff/checkpoint
        # metadata is a packaging artifact, not VAC v1.9 source authority. Split
        # source/state ZIP integrity is still enforced by the two-zip pair mode above.
        if (ROOT / ".vac/vac.toml").exists() and (ROOT / "vac-rs/Cargo.toml").exists():
            warnings.append("CHECKPOINT_MANIFEST.json absent; source-workspace mode skips package manifest count check")
            return
        errors.append("missing CHECKPOINT_MANIFEST.json")
        return
    data = read_json(path) or {}
    index = data.get("index_counts") or data.get("index") or {}
    if not index:
        errors.append("CHECKPOINT_MANIFEST.json missing index_counts")
        return
    for key, actual in actual_index.items():
        if index.get(key) != actual:
            errors.append(f"checkpoint manifest stale: {key} manifest={index.get(key)} actual={actual}")


def check_readiness_authority() -> None:
    status = read_json(ROOT / ".vac/registry/status.json") or {}
    caps_current = read_json(ROOT / ".vac/cache/compiled/capabilities/current.json") or read_json(ROOT / ".vac/registry/compiled/capabilities/current.json") or {}
    caps_aggregate = read_json(ROOT / ".vac/cache/compiled/capabilities.json") or read_json(ROOT / ".vac/registry/compiled/capabilities.json") or {}
    summary_status = status.get("readiness_summary")
    summary_current = caps_current.get("summary")
    summary_aggregate = caps_aggregate.get("summary")
    if summary_status != summary_current:
        errors.append(f"readiness mismatch: status.readiness_summary={summary_status} current.summary={summary_current}")
    if summary_aggregate != summary_current:
        errors.append(f"readiness mismatch: capabilities.json.summary={summary_aggregate} current.summary={summary_current}")


def check_duplicate_bins() -> None:
    cargo = ROOT / "vac-rs/crates/surfaces/vac-cli/Cargo.toml"
    if not cargo.exists():
        return
    text = cargo.read_text()
    if text.count('[[bin]]') != 1:
        errors.append("vac-cli Cargo.toml must contain exactly one [[bin]] stanza")



def check_assessment_freshness() -> None:
    manifest = read_json(ROOT / ".vac/index/index_manifest.json") or {}
    outputs = manifest.get("outputs") or {}
    spans_path = ROOT / (outputs.get("spans") or ".vac/index/spans.jsonl")
    spans: dict[str, dict[str, Any]] = {}
    if not spans_path.exists():
        errors.append(f"missing index spans for assessment freshness: {spans_path.relative_to(ROOT)}")
        return
    for lineno, line in enumerate(spans_path.read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except Exception as exc:
            errors.append(f"invalid spans jsonl {spans_path.relative_to(ROOT)}:{lineno}: {exc}")
            continue
        span_id = row.get("span_id") if isinstance(row, dict) else None
        if isinstance(span_id, str):
            spans[span_id] = row

    gap = read_json(ROOT / ".vac/assessment/gap_report.json") or {}
    if gap.get("kind") != "gap_report":
        errors.append("assessment gap_report kind mismatch")
    for finding in gap.get("findings") or []:
        if not isinstance(finding, dict):
            errors.append("assessment finding must be object")
            continue
        fid = finding.get("finding_id") or "<unknown>"
        evidence = finding.get("evidence") or {}
        if not isinstance(evidence, dict):
            errors.append(f"assessment finding {fid} evidence must be object")
            continue
        span_ids: list[str] = []
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
            errors.append(f"assessment finding lacks current span evidence: {fid}")
            continue
        for span_id in sorted(set(span_ids)):
            current = spans.get(span_id)
            if current is None:
                errors.append(f"assessment stale span_id: {fid} -> {span_id}")
                continue
            expected = evidence.get("span_sha256")
            if isinstance(expected, str):
                if not expected.startswith("sha256:"):
                    expected = "sha256:" + expected
                if expected != current.get("span_sha256"):
                    errors.append(f"assessment stale span hash: {fid} -> {span_id}")

    baseline = read_json(ROOT / ".vac/assessment/baseline.json") or {}
    summary = baseline.get("summary") or {}
    count_map = {
        "files_indexed": "files",
        "symbols_indexed": "symbols",
        "relations_indexed": "relations",
        "risk_findings": "risks",
        "spans_indexed": "spans",
        "read_plans_indexed": "read_plans",
    }
    for baseline_key, index_key in count_map.items():
        expected = (manifest.get("counts") or {}).get(index_key)
        if expected is not None and summary.get(baseline_key) != expected:
            errors.append(f"assessment baseline stale: {baseline_key}={summary.get(baseline_key)} current={expected}")
    if baseline.get("index_snapshot_hash") != manifest.get("snapshot_hash"):
        errors.append("assessment baseline snapshot_hash does not match current index manifest")


def check_evidence_log_freshness() -> None:
    checker = ROOT / "scripts/check-evidence-log-freshness.py"
    if not checker.exists():
        errors.append("missing evidence log freshness checker")
        return
    p = subprocess.run([sys.executable, str(checker), str(ROOT)], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if p.returncode != 0:
        errors.append("evidence log freshness failed: " + p.stdout.replace("\n", "; ")[:800])

def main() -> int:
    checked, mismatches = check_compiled_source_hashes()
    actual_index = check_index_counts()
    check_checkpoint_manifest_counts(actual_index)
    check_readiness_authority()
    check_duplicate_bins()
    check_assessment_freshness()
    check_evidence_log_freshness()
    if errors:
        print("VAC checkpoint integrity: FAIL")
        print(f"compiled_source_hashes_checked={checked}")
        print(f"compiled_source_hash_mismatches={mismatches}")
        for error in errors:
            print(f"- {error}")
        return 1
    print("VAC checkpoint integrity: PASS")
    print(f"compiled_source_hashes_checked={checked}")
    print("compiled_source_hash_mismatches=0")
    print("checkpoint_index_counts=actual_index_counts" if not warnings else "checkpoint_index_counts=source_workspace_mode_not_applicable")
    for warning in warnings:
        print(f"warning: {warning}")
    print("readiness_authority=canonical_capabilities_current")
    print("compiled_snapshot_location=.vac/cache/compiled")
    print("assessment_freshness=current_index")
    print("evidence_log_freshness=current_snapshot")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
