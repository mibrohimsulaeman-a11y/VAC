#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from external_provider_remote_process_io_status import (
    TV_FAIL,
    TV_STALE,
    external_provider_remote_process_io_summary,
)

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
DOMAIN_MAP = ROOT / "tests/fixtures/confirmed-intent/domain-map.json"
STATUS = ROOT / ".vac/registry/status.json"
COVERAGE_DOC = "docs/audit/VAC_CONFIRMED_INTENT_COVERAGE_AUDIT.md"
TRACE_DOC = "docs/audit/VAC_CONFIRMED_INTENT_TRACEABILITY.md"


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise SystemExit(f"missing required file: {path.relative_to(ROOT)}")
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SystemExit(f"expected JSON object: {path.relative_to(ROOT)}")
    return value


payload = load_json(DOMAIN_MAP)
status = load_json(STATUS)
expectations = payload.get("status_expectations", {})
domains = payload.get("domains", [])
if not isinstance(expectations, dict):
    raise SystemExit("domain-map status_expectations must be an object")
if not isinstance(domains, list):
    raise SystemExit("domain-map domains must be a list")

external_io = external_provider_remote_process_io_summary(ROOT)
external_status = external_io["status"]
if external_status in {TV_FAIL, TV_STALE}:
    raise SystemExit("invalid external provider remote process IO proof: " + ",".join(str(item) for item in external_io.get("errors", [])))

summary: dict[str, Any] = {
    "confirmed_intent_domain_coverage": expectations.get("confirmed_intent_domain_coverage"),
    "confirmed_intent_traceability_gate": expectations.get("confirmed_intent_traceability_gate"),
    "confirmed_intent_traceability": "SV-Pass",
    "confirmed_intent_negative_fixtures": expectations.get("confirmed_intent_negative_fixtures"),
    "confirmed_intent_executable_fixtures": expectations.get("confirmed_intent_executable_fixtures"),
    "all_negative_cases_rejected": expectations.get("all_negative_cases_rejected"),
    "crate_without_intent_or_rationale": expectations.get("crate_without_intent_or_rationale"),
    "external_provider_remote_process_io_e2e": external_status,
    "remote_process_io_e2e": external_io.get("remote_process_status"),
    "external_provider_remote_process_io_e2e_execution": external_io.get("execution"),
    "external_provider_remote_process_io_e2e_custody": external_io.get("custody"),
    "external_provider_remote_process_io_e2e_proof_ref": external_io.get("proof_ref"),
    "external_provider_remote_process_io_e2e_proof_hash": external_io.get("proof_hash"),
    "domain_count": len(domains),
    "domain_map": str(DOMAIN_MAP.relative_to(ROOT)),
    "coverage_audit": COVERAGE_DOC,
    "traceability_audit": TRACE_DOC,
    "domains": [domain.get("id") for domain in domains if isinstance(domain, dict)],
}

required = {
    "confirmed_intent_domain_coverage": "SV-Pass",
    "confirmed_intent_traceability_gate": "SV-Pass",
    "confirmed_intent_negative_fixtures": "SV-Pass",
    "confirmed_intent_executable_fixtures": "SV-Pass",
    "all_negative_cases_rejected": True,
    "crate_without_intent_or_rationale": 0,
}
for key, expected in required.items():
    if summary.get(key) != expected:
        raise SystemExit(f"unexpected confirmed intent status {key}={summary.get(key)!r}, expected {expected!r}")
if summary["domain_count"] != 7:
    raise SystemExit(f"unexpected confirmed intent domain count: {summary['domain_count']}")

updated: dict[str, Any] = {}
inserted = False
for key, value in status.items():
    if key == "confirmed_intent":
        continue
    updated[key] = value
    if key == "control_plane_version":
        updated["confirmed_intent"] = summary
        inserted = True
if not inserted:
    updated["confirmed_intent"] = summary

STATUS.write_text(json.dumps(updated, indent=2) + "\n", encoding="utf-8")

print("VAC confirmed intent status refreshed")
print("confirmed_intent_domain_coverage=SV-Pass")
print("confirmed_intent_traceability_gate=SV-Pass")
print("confirmed_intent_traceability=SV-Pass")
print("confirmed_intent_negative_fixtures=SV-Pass")
print("confirmed_intent_executable_fixtures=SV-Pass")
print("all_negative_cases_rejected=true")
print("crate_without_intent_or_rationale=0")
print(f"external_provider_remote_process_io_e2e={external_status}")
print(f"remote_process_io_e2e={external_io.get('remote_process_status')}")
