#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
FIXTURE = ROOT / "tests/fixtures/confirmed-intent/executable-invariants.json"
TRACE_DOC = ROOT / "docs/audit/VAC_CONFIRMED_INTENT_TRACEABILITY.md"
errors: list[str] = []


def read(path: Path, label: str) -> str:
    if not path.is_file():
        errors.append(f"missing {label}: {path.relative_to(ROOT)}")
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def require(name: str, ok: bool) -> None:
    if not ok:
        errors.append(name)


def load_fixture() -> dict[str, Any]:
    text = read(FIXTURE, "executable invariant fixture")
    if not text:
        return {}
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        errors.append(f"invalid executable fixture JSON: {exc}")
        return {}
    if not isinstance(value, dict):
        errors.append("executable fixture root must be an object")
        return {}
    return value


def case_combined_text(case: dict[str, Any]) -> str:
    chunks: list[str] = []
    for rel in case.get("files", []):
        chunks.append(read(ROOT / str(rel), f"case file for {case.get('case_id')}: {rel}"))
    return "\n".join(chunks)



def redaction_fixture_is_valid(fixture: dict[str, Any]) -> bool:
    outputs = fixture.get("redacted_outputs", [])
    required = fixture.get("redacted_required", [])
    forbidden = fixture.get("raw_forbidden", fixture.get("raw_secrets", []))
    if not isinstance(outputs, list) or not all(isinstance(item, str) for item in outputs):
        return False
    if not isinstance(required, list) or not all(isinstance(item, str) for item in required):
        return False
    if not isinstance(forbidden, list) or not all(isinstance(item, str) for item in forbidden):
        return False
    combined = "\n".join(outputs)
    return all(token in combined for token in required) and all(token not in combined for token in forbidden)

def case_is_valid(case: dict[str, Any], combined: str) -> bool:
    required = case.get("required_tokens", [])
    forbidden = case.get("forbidden_tokens", [])
    if not isinstance(required, list) or not all(isinstance(item, str) for item in required):
        return False
    if not isinstance(forbidden, list) or not all(isinstance(item, str) for item in forbidden):
        return False
    return all(token in combined for token in required) and all(token not in combined for token in forbidden)


payload = load_fixture()
trace = read(TRACE_DOC, "confirmed intent traceability doc")
require("fixture_id", payload.get("fixture_id") == "confirmed_intent.executable_invariants.v1")
require("fixture_status", payload.get("status") == "SV-Pass")
cases = payload.get("cases", [])
require("cases_nonempty", isinstance(cases, list) and len(cases) >= 3)
seen: set[str] = set()

for raw_case in cases if isinstance(cases, list) else []:
    if not isinstance(raw_case, dict):
        errors.append("case must be an object")
        continue
    case_id = str(raw_case.get("case_id", ""))
    invariant = str(raw_case.get("invariant", ""))
    fixture_id = str(raw_case.get("fixture_id", ""))
    trace_status = str(raw_case.get("trace_status", ""))
    require(f"case_has_id:{case_id or '<missing>'}", bool(case_id))
    require(f"case_id_unique:{case_id}", case_id not in seen)
    seen.add(case_id)
    require(f"{case_id}:has_invariant", bool(invariant))
    require(f"{case_id}:has_fixture_id", fixture_id.startswith("executable:"))
    require(f"{case_id}:trace_status_sv_pass", trace_status == "SV-Pass")

    combined = case_combined_text(raw_case)
    for token in raw_case.get("required_tokens", []):
        require(f"{case_id}:required_token:{token}", isinstance(token, str) and token in combined)
    for token in raw_case.get("forbidden_tokens", []):
        require(f"{case_id}:forbidden_token_absent:{token}", isinstance(token, str) and token not in combined)

    require(f"{case_id}:trace_mentions_invariant", invariant in trace)
    require(f"{case_id}:trace_mentions_fixture", fixture_id in trace)
    require(f"{case_id}:trace_mentions_status", trace_status in trace)

    forbidden = raw_case.get("forbidden_tokens", [])
    if isinstance(forbidden, list) and forbidden and isinstance(forbidden[0], str):
        mutated = combined + "\n" + forbidden[0] + "\n"
        require(f"{case_id}:negative_forbidden_token_rejected", not case_is_valid(raw_case, mutated))

    redaction_fixture = raw_case.get("redaction_fixture")
    if redaction_fixture is not None:
        require(f"{case_id}:redaction_fixture_object", isinstance(redaction_fixture, dict))
        if isinstance(redaction_fixture, dict):
            require(
                f"{case_id}:redaction_fixture_valid",
                redaction_fixture_is_valid(redaction_fixture),
            )
            raw_forbidden = redaction_fixture.get(
                "raw_forbidden", redaction_fixture.get("raw_secrets", [])
            )
            outputs = redaction_fixture.get("redacted_outputs", [])
            if (
                isinstance(raw_forbidden, list)
                and raw_forbidden
                and isinstance(raw_forbidden[0], str)
                and isinstance(outputs, list)
                and outputs
                and isinstance(outputs[0], str)
            ):
                mutated_fixture = dict(redaction_fixture)
                mutated_fixture["redacted_outputs"] = [outputs[0] + raw_forbidden[0], *outputs[1:]]
                require(
                    f"{case_id}:negative_raw_secret_rejected",
                    not redaction_fixture_is_valid(mutated_fixture),
                )

required_cases = {
    "broker_no_l2_overclaim",
    "autopilot_cli_no_release_overclaim",
    "messaging_delivery_failure_explicit",
    "messaging_tokens_redacted",
    "remote_credential_material_redacted",
}
for case_id in sorted(required_cases - seen):
    errors.append(f"missing required executable case: {case_id}")

if errors:
    print("VAC confirmed intent executable fixtures: FAIL")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("VAC confirmed intent executable fixtures: PASS")
print("confirmed_intent_executable_fixtures=SV-Pass")
print("executable_cases=" + str(len(cases)))
print("negative_forbidden_token_checks=true")
