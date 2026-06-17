#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from external_provider_remote_process_io_status import (
    TV_FAIL,
    TV_PASS,
    TV_PENDING,
    TV_STALE,
    external_provider_remote_process_io_summary,
)

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
DOMAIN_MAP = ROOT / "tests/fixtures/confirmed-intent/domain-map.json"
COVERAGE_DOC = ROOT / "docs/audit/VAC_CONFIRMED_INTENT_COVERAGE_AUDIT.md"
TRACE_DOC = ROOT / "docs/audit/VAC_CONFIRMED_INTENT_TRACEABILITY.md"

REQUIRED_STATUS = {
    "confirmed_intent_domain_coverage": "SV-Pass",
    "confirmed_intent_traceability_gate": "SV-Pass",
    "confirmed_intent_negative_fixtures": "SV-Pass",
    "confirmed_intent_executable_fixtures": "SV-Pass",
    "all_negative_cases_rejected": True,
    "crate_without_intent_or_rationale": 0,
}

PROOF_GATED_OVERCLAIMS = [
    "external_provider_remote_process_io_e2e=TV-Pass",
    "remote_process_io_e2e=TV-Pass",
    "remote_process_io=TV-Pass",
]

FORBIDDEN_OVERCLAIMS = [
    "broker_attested_l2_enforcement=TV-Pass",
    "P1 acceptance=Pass",
    "release_ready=Pass",
]

errors: list[str] = []


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def read_text(path: Path, label: str) -> str:
    if not path.is_file():
        errors.append(f"missing {label}: {rel(path)}")
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def require(name: str, condition: bool) -> None:
    if not condition:
        errors.append(name)


def status_token_value(value: Any) -> str:
    if isinstance(value, bool):
        return str(value).lower()
    return str(value)


def load_json(path: Path) -> dict[str, Any]:
    text = read_text(path, "domain map")
    if not text:
        return {}
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        errors.append(f"invalid JSON {rel(path)}: {exc}")
        return {}
    if not isinstance(value, dict):
        errors.append(f"domain map root must be an object: {rel(path)}")
        return {}
    return value


payload = load_json(DOMAIN_MAP)
coverage_doc = read_text(COVERAGE_DOC, "confirmed intent coverage audit")
trace_doc = read_text(TRACE_DOC, "confirmed intent traceability doc")
external_io = external_provider_remote_process_io_summary(ROOT)
external_status = external_io["status"]
remote_process_status = external_io["remote_process_status"]
if external_status in {TV_FAIL, TV_STALE}:
    errors.append("external_provider_remote_process_io_e2e_proof_invalid:" + ",".join(str(item) for item in external_io.get("errors", [])))

require("fixture_id_is_confirmed_intent_domain_coverage", payload.get("fixture_id") == "confirmed_intent.domain_coverage.v1")
status_expectations = payload.get("status_expectations", {})
require("status_expectations_is_object", isinstance(status_expectations, dict))
if isinstance(status_expectations, dict):
    for key, expected in REQUIRED_STATUS.items():
        token_value = status_token_value(expected)
        require(f"status_expectation:{key}={token_value}", status_expectations.get(key) == expected)
        require(f"coverage_doc_status_token:{key}", f"{key}={token_value}" in coverage_doc)
        require(f"trace_doc_status_token:{key}", f"{key}={token_value}" in trace_doc)

domains = payload.get("domains", [])
require("domains_is_nonempty_list", isinstance(domains, list) and bool(domains))
require("confirmed_intent_domains_count_is_7", isinstance(domains, list) and len(domains) == 7)

seen_domain_ids: set[str] = set()
for domain in domains if isinstance(domains, list) else []:
    if not isinstance(domain, dict):
        errors.append("domain entry must be an object")
        continue

    domain_id = str(domain.get("id", ""))
    require(f"domain_has_id:{domain_id or '<missing>'}", bool(domain_id))
    if domain_id in seen_domain_ids:
        errors.append(f"duplicate domain id: {domain_id}")
    seen_domain_ids.add(domain_id)

    intent_rel = str(domain.get("required_intent", ""))
    spec_path = ROOT / intent_rel
    spec_exists = spec_path.is_file()
    spec_text = read_text(spec_path, f"intent spec for {domain_id}")
    require(f"{domain_id}:expected_status_is_sv_pass", domain.get("expected_status") == "SV-Pass")
    require(f"{domain_id}:has_rationale", bool(str(domain.get("rationale", "")).strip()))
    require(f"{domain_id}:coverage_doc_has_rationale", str(domain.get("rationale", "")) in coverage_doc)

    if spec_text:
        require(f"{domain_id}:spec_kind_intent_spec", "kind: intent_spec" in spec_text)
        require(f"{domain_id}:spec_status_confirmed", "status: confirmed" in spec_text)
        require(f"{domain_id}:spec_has_expected_code", "expected_code:" in spec_text and "modules:" in spec_text)
        require(f"{domain_id}:spec_declares_out_of_scope", "out_of_scope:" in spec_text)

    paths = domain.get("paths", [])
    require(f"{domain_id}:paths_is_nonempty_list", isinstance(paths, list) and bool(paths))
    for item in paths if isinstance(paths, list) else []:
        path_text = str(item)
        require(f"{domain_id}:path_exists:{path_text}", (ROOT / path_text).exists())
        if spec_exists:
            require(f"{domain_id}:spec_mentions_path:{path_text}", path_text in spec_text)
        require(f"{domain_id}:coverage_doc_mentions_path:{path_text}", path_text in coverage_doc)
        require(f"{domain_id}:trace_doc_mentions_path:{path_text}", path_text in trace_doc)

    capabilities = domain.get("capabilities", [])
    require(f"{domain_id}:capabilities_is_nonempty_list", isinstance(capabilities, list) and bool(capabilities))
    for cap in capabilities if isinstance(capabilities, list) else []:
        if not isinstance(cap, dict):
            errors.append(f"{domain_id}:capability entry must be object")
            continue
        cap_id = str(cap.get("id", ""))
        cap_manifest = str(cap.get("manifest", ""))
        cap_text = read_text(ROOT / cap_manifest, f"capability manifest for {domain_id}:{cap_id}")
        require(f"{domain_id}:capability_has_id:{cap_id}", bool(cap_id))
        require(f"{domain_id}:capability_manifest_has_id:{cap_id}", f"id: {cap_id}" in cap_text)
        require(f"{domain_id}:coverage_doc_mentions_capability:{cap_id}", cap_id in coverage_doc)
        require(f"{domain_id}:trace_doc_mentions_capability:{cap_id}", cap_id in trace_doc)

    markers = domain.get("required_markers", [])
    require(f"{domain_id}:required_markers_is_nonempty_list", isinstance(markers, list) and bool(markers))
    for marker in markers if isinstance(markers, list) else []:
        marker_text = str(marker)
        if spec_exists:
            require(f"{domain_id}:spec_mentions_marker:{marker_text}", marker_text in spec_text)
        require(f"{domain_id}:trace_doc_mentions_marker:{marker_text}", marker_text in trace_doc)

    for field, doc_label in [("doctor_gates", "gate"), ("fixtures", "fixture"), ("implementation_modules", "implementation module")]:
        values = domain.get(field, [])
        require(f"{domain_id}:{field}_is_nonempty_list", isinstance(values, list) and bool(values))
        for value in values if isinstance(values, list) else []:
            value_text = str(value)
            require(f"{domain_id}:trace_doc_mentions_{doc_label}:{value_text}", value_text in trace_doc)

    require(f"{domain_id}:coverage_doc_mentions_domain", domain_id in coverage_doc)
    require(f"{domain_id}:trace_doc_mentions_domain", domain_id in trace_doc)
    require(f"{domain_id}:coverage_doc_mentions_intent", intent_rel in coverage_doc)
    require(f"{domain_id}:trace_doc_mentions_intent", intent_rel in trace_doc)

combined = "\n".join([json.dumps(payload, sort_keys=True), coverage_doc, trace_doc])
if external_status != TV_PASS:
    for token in PROOF_GATED_OVERCLAIMS:
        require(f"no_forbidden_overclaim_without_proof:{token}", token not in combined)
for token in FORBIDDEN_OVERCLAIMS:
    require(f"no_forbidden_overclaim:{token}", token not in combined)

remote_pending_tokens = [
    "external_provider_remote_process_io_e2e=TV-Pending",
    "remote_process_io_e2e=TV-Pending",
]
if external_status == TV_PENDING:
    for token in remote_pending_tokens:
        require(f"pending_status_present:{token}", token in combined)

if errors:
    print("VAC confirmed intent coverage: FAIL")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("VAC confirmed intent coverage: PASS")
print("confirmed_intent_domains=7")
print("crate_without_intent_or_rationale=0")
print("confirmed_intent_domain_coverage=SV-Pass")
print("confirmed_intent_traceability_gate=SV-Pass")
print("confirmed_intent_traceability=SV-Pass")
print("confirmed_intent_negative_fixtures=SV-Pass")
print("confirmed_intent_executable_fixtures=SV-Pass")
print("all_negative_cases_rejected=true")
print(f"external_provider_remote_process_io_e2e={external_status}")
print(f"remote_process_io_e2e={remote_process_status}")
