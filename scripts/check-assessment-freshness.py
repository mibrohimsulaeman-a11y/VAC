#!/usr/bin/env python3
"""VAC assessment freshness doctor (SV-safe, no cargo required).

Validates the v1.9 requirement that assessment findings are grounded in the
current deterministic index. This closes the gap where gap_report.json could
cite historical span ids/hash values while the index had already changed.
"""
from __future__ import annotations

import json
import pathlib
import sys
from typing import Any

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
ERRORS: list[str] = []


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text())
    except Exception as exc:
        ERRORS.append(f"invalid json {path.relative_to(ROOT)}: {exc}")
        return None


def jsonl_rows(path: pathlib.Path) -> list[dict[str, Any]]:
    if not path.exists():
        ERRORS.append(f"missing index file {path.relative_to(ROOT)}")
        return []
    rows = []
    for lineno, line in enumerate(path.read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
            if isinstance(row, dict):
                rows.append(row)
            else:
                ERRORS.append(f"non-object jsonl row {path.relative_to(ROOT)}:{lineno}")
        except Exception as exc:
            ERRORS.append(f"invalid jsonl {path.relative_to(ROOT)}:{lineno}: {exc}")
    return rows


def current_counts() -> dict[str, int]:
    manifest = load_json(ROOT / ".vac/index/index_manifest.json") or {}
    outputs = manifest.get("outputs") or {}
    keys = ["files", "symbols", "relations", "risks", "spans", "read_plans"]
    counts: dict[str, int] = {}
    for key in keys:
        rel = outputs.get(key) or f".vac/index/{key}.jsonl"
        counts[key] = len(jsonl_rows(ROOT / rel))
        expected = (manifest.get("counts") or {}).get(key)
        if expected is not None and expected != counts[key]:
            ERRORS.append(f"index manifest count stale: {key} manifest={expected} actual={counts[key]}")
    return counts


def span_index() -> dict[str, dict[str, Any]]:
    rows = jsonl_rows(ROOT / ".vac/index/spans.jsonl")
    index = {}
    for row in rows:
        span_id = row.get("span_id")
        if isinstance(span_id, str):
            index[span_id] = row
    return index


def normalize_sha(value: Any) -> str | None:
    if not isinstance(value, str) or not value:
        return None
    return value if value.startswith("sha256:") else f"sha256:{value}"


def check_finding_span(finding: dict[str, Any], spans: dict[str, dict[str, Any]]) -> None:
    fid = str(finding.get("finding_id") or "<unknown>")
    evidence = finding.get("evidence") or {}
    if not isinstance(evidence, dict):
        ERRORS.append(f"{fid} evidence must be object")
        return

    span_ids: list[str] = []
    raw_span = evidence.get("span_id")
    if isinstance(raw_span, str) and raw_span:
        span_ids.append(raw_span)
    for item in evidence.get("matched_spans") or []:
        if isinstance(item, str) and item:
            span_ids.append(item)
        elif isinstance(item, dict) and isinstance(item.get("span_id"), str):
            span_ids.append(item["span_id"])

    missing_code = evidence.get("missing_code") or evidence.get("intent_without_code")
    if not span_ids:
        if isinstance(missing_code, dict) and missing_code.get("intent_ref") and evidence.get("matched_spans") == []:
            return
        ERRORS.append(f"{fid} has no current span_id/matched_spans and no explicit missing-code intent proof")
        return

    for span_id in sorted(set(span_ids)):
        current = spans.get(span_id)
        if current is None:
            path = evidence.get("path")
            current_for_path = None
            if isinstance(path, str):
                candidates = [row for row in spans.values() if row.get("path") == path]
                if candidates:
                    current_for_path = candidates[0]
            if current_for_path:
                ERRORS.append(
                    f"{fid} stale span_id missing from current index: {span_id}; current for path is {current_for_path.get('span_id')} {current_for_path.get('span_sha256')}"
                )
            else:
                ERRORS.append(f"{fid} span_id missing from current index: {span_id}")
            continue
        expected = normalize_sha(evidence.get("span_sha256"))
        if expected is not None and expected != current.get("span_sha256"):
            ERRORS.append(f"{fid} span_sha256 mismatch for {span_id}: report={expected} current={current.get('span_sha256')}")


def check_baseline(counts: dict[str, int]) -> None:
    baseline = load_json(ROOT / ".vac/assessment/baseline.json") or {}
    summary = baseline.get("summary") or {}
    checks = {
        "files_indexed": counts.get("files"),
        "risk_findings": counts.get("risks"),
        "spans_indexed": counts.get("spans"),
        "symbols_indexed": counts.get("symbols"),
        "relations_indexed": counts.get("relations"),
        "read_plans_indexed": counts.get("read_plans"),
    }
    for key, expected in checks.items():
        if expected is None:
            continue
        if summary.get(key) != expected:
            ERRORS.append(f"baseline summary stale: {key}={summary.get(key)} current={expected}")
    if baseline.get("index_snapshot_hash") != (load_json(ROOT / ".vac/index/index_manifest.json") or {}).get("snapshot_hash"):
        ERRORS.append("baseline index_snapshot_hash does not match current index manifest snapshot_hash")


def main() -> int:
    counts = current_counts()
    spans = span_index()
    gap = load_json(ROOT / ".vac/assessment/gap_report.json") or {}
    if gap.get("kind") != "gap_report":
        ERRORS.append("gap_report kind mismatch")
    for finding in gap.get("findings") or []:
        if isinstance(finding, dict):
            check_finding_span(finding, spans)
        else:
            ERRORS.append("gap_report finding is not object")
    check_baseline(counts)
    if ERRORS:
        print("VAC assessment freshness: FAIL")
        print(f"assessment_freshness_checked={len(gap.get('findings') or [])}")
        for err in ERRORS:
            print(f"- {err}")
        return 1
    print("VAC assessment freshness: PASS")
    print(f"assessment_freshness_checked={len(gap.get('findings') or [])}")
    print("assessment_baseline_counts=current_index_counts")
    print("assessment_findings_span_hashes=current_index")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
