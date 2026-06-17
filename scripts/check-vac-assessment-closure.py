#!/usr/bin/env python3
from __future__ import annotations
import hashlib, json, re, sys
from pathlib import Path
ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors: list[str] = []
def read(rel: str) -> str:
    path = ROOT / rel
    if not path.is_file():
        errors.append(f"missing file: {rel}")
        return ""
    return path.read_text(encoding="utf-8", errors="replace")
def require(cond: bool, msg: str) -> None:
    if not cond:
        errors.append(msg)
def load_json(rel: str) -> dict:
    text = read(rel)
    if not text:
        return {}
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        errors.append(f"invalid JSON {rel}: {exc}")
        return {}
    if not isinstance(value, dict):
        errors.append(f"JSON root must be object: {rel}")
        return {}
    return value
app = read("vac-rs/crates/surfaces/vac-cli/src/config/app.rs")
remote_service = read("vac-rs/crates/integrations/vac-remote-service/src/lib.rs")
require('unwrap_or("claude-opus-4-6")' not in app, "A-001: app config still has hardcoded fallback literal")
require("catalog_default_model(false)" in app, "A-001: app config must call catalog_default_model")
require("CATALOG_DEFAULT_MODEL" in remote_service and "catalog_default_model" in remote_service, "A-001: catalog default model declaration missing")
required_specs = {
    "vac-policy-intent.yaml": "intent.vac.control_plane.policy",
    "vac-evidence-intent.yaml": "intent.vac.control_plane.evidence",
    "vac-index-intent.yaml": "intent.vac.control_plane.index",
    "vac-spec-sync-intent.yaml": "intent.vac.control_plane.spec_sync",
    "vac-registry-compiler-intent.yaml": "intent.vac.control_plane.registry_compiler",
}
for filename, intent_id in required_specs.items():
    text = read(f".vac/specs/confirmed/{filename}")
    require("kind: intent_spec" in text, f"A-002: {filename} missing intent_spec kind")
    require("status: confirmed" in text, f"A-002: {filename} missing confirmed status")
    require(f"id: {intent_id}" in text, f"A-002: {filename} missing expected id {intent_id}")
    require("expected_code:" in text and "out_of_scope:" in text, f"A-002: {filename} missing traceability fields")
clippy_policy = read(".vac/policies/clippy-debt.yaml")
ci = read(".github/workflows/ci.yml")
require("assessment_closure:" in clippy_policy and "C-001" in clippy_policy, "C-001: clippy policy missing assessment closure ledger")
require("cargo clippy --manifest-path vac-rs/Cargo.toml --workspace --all-targets -- -D warnings" in ci, "C-001: CI workspace clippy command missing")
cargo_toml = read("vac-rs/Cargo.toml")
for lint in ["unwrap_used", "expect_used", "string_slice"]:
    require(re.search(rf"^{lint}\s*=\s*\"allow\"", cargo_toml, re.M) is not None, f"C-001: {lint} must remain explicit debt allow until deny migration")
for crate in ["vac-policy", "vac-readiness", "vac-evidence", "vac-registry-compiler", "vac-spec-sync", "vac-memory"]:
    crate_dir = ROOT / "vac-rs/crates/control-plane" / crate
    tests = 0
    for source in crate_dir.rglob("*.rs"):
        text = source.read_text(encoding="utf-8", errors="replace")
        tests += text.count("#[test]") + text.count("#[tokio::test]")
    require(tests > 0, f"Q-001: {crate} still has zero unit tests")
large_files: list[str] = []
for source in (ROOT / "vac-rs/crates").rglob("*.rs"):
    rel = source.relative_to(ROOT).as_posix()
    if "/target/" in rel:
        continue
    loc = len(source.read_text(encoding="utf-8", errors="replace").splitlines())
    if loc > 3000:
        large_files.append(f"{rel}:{loc}")
require(not large_files, "Q-002: Rust files over 3000 LOC remain: " + ", ".join(large_files[:10]))
allowlist = load_json(".vac/audit/assessment-closure/boundary-discard-allowlist.json")
entries = allowlist.get("entries", [])
require(isinstance(entries, list) and bool(entries), "Q-003: discard allowlist must be non-empty")
allowed = {(str(e.get("path")), str(e.get("code_sha256")), str(e.get("reason"))) for e in entries if isinstance(e, dict)}
for root in [ROOT/"vac-rs/crates/runtime/vac-broker/src", ROOT/"vac-rs/crates/integrations/vac-remote-service/src", ROOT/"vac-rs/crates/integrations/vac-mcp-server/src"]:
    for source in root.rglob("*.rs"):
        rel = source.relative_to(ROOT).as_posix()
        for lineno, line in enumerate(source.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            stripped = line.strip()
            if "let _ =" in stripped or stripped.endswith(".ok();") or ".ok();" in stripped:
                digest = hashlib.sha256(stripped.encode()).hexdigest()
                matches = [reason for path, code_hash, reason in allowed if path == rel and code_hash == digest]
                require(bool(matches), f"Q-003: untriaged discard {rel}:{lineno}: {stripped}")
                if matches:
                    require(matches[0].strip() != "", f"Q-003: discard {rel}:{lineno} missing rationale")
providers_root = ROOT / "vac-rs/crates/providers"
provider_dirs = sorted(path.name for path in providers_root.iterdir() if path.is_dir() and path.name.startswith("vac-provider-"))
require(provider_dirs == ["vac-provider-core"], f"Q-004: unexpected provider crate dirs remain: {provider_dirs}")
providers_readme = read("vac-rs/crates/providers/README.md")
require("vac-provider-core is the only active provider implementation crate" in providers_readme, "Q-004: provider ownership README missing centralization statement")
if errors:
    print("VAC assessment closure: FAIL")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)
print("VAC assessment closure: PASS")
print("closed_findings=A-001,A-002,C-001,Q-001,Q-002,Q-003,Q-004")
print("god_module_threshold=max_3000_loc")
print("boundary_discards=triaged_allowlist")
print("clippy_debt=controlled_ci_clippy_tv")
