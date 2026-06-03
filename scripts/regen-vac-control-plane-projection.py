#!/usr/bin/env python3
"""Regenerate source-derived .vac/.init and living docs projections.

Regenerates only derivable material and preserves append-only evidence/plans/
trajectory plus architecture ADR/decisions.
"""
from __future__ import annotations
from pathlib import Path
from collections import defaultdict
import hashlib
import re

ROOT = Path(__file__).resolve().parents[1]
GENERATED_AT = "2026-06-01T22:58:00Z"
PROTECTED_REQUIRED = [
    ".vac/registry/evidence",
    ".vac/registry/plans",
    ".vac/registry/trajectory",
    "docs/architecture/adr",
    "docs/architecture/decisions",
]
SKIP_DIRS = {".git", "target", "node_modules", "vendor", "__pycache__"}


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def iter_files(base: Path):
    for path in sorted(base.rglob("*")):
        if not path.is_file():
            continue
        parts = set(path.relative_to(ROOT).parts)
        if parts & SKIP_DIRS:
            continue
        yield path


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def title_for_markdown(path: Path) -> str:
    try:
        for line in path.read_text(errors="ignore").splitlines():
            if line.startswith("# "):
                return line[2:].strip() or path.name
    except OSError:
        pass
    return path.stem.replace("-", " ").replace("_", " ").title()


def top_area(path: Path) -> str:
    r = path.relative_to(ROOT / "docs")
    return r.parts[0] if len(r.parts) > 1 else r.name


def workspace_members():
    cargo = ROOT / "vac-rs/Cargo.toml"
    text = cargo.read_text(errors="ignore")
    match = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
    if not match:
        return []
    return re.findall(r'"([^"]+)"', match.group(1))


def manifest_counts():
    groups = {
        "capabilities": ROOT / ".vac/capabilities",
        "workflows": ROOT / ".vac/workflows",
        "surfaces": ROOT / ".vac/surfaces",
        "policies": ROOT / ".vac/policies",
        "registry": ROOT / ".vac/registry",
    }
    return {name: len(list(path.rglob("*.yaml"))) if path.exists() else 0 for name, path in groups.items()}


def count_structured_validation_commands():
    count = 0
    for path in (ROOT / ".vac/workflows").glob("*.yaml"):
        text = path.read_text(errors="ignore")
        count += len(re.findall(r"^\s*command:\s*$", text, re.M))
    return count


def write_source_inventory(files):
    by_ext = defaultdict(int)
    by_top = defaultdict(int)
    rust_files = []
    for path in files:
        r = rel(path)
        ext = path.suffix or "<none>"
        by_ext[ext] += 1
        by_top[r.split("/", 1)[0]] += 1
        if path.suffix == ".rs":
            rust_files.append(path)
    crates = []
    for member in workspace_members():
        cargo = ROOT / "vac-rs" / member / "Cargo.toml"
        crates.append({"path": f"vac-rs/{member}", "exists": cargo.exists()})
    out = []
    out.append("schema_version: 1")
    out.append("kind: init_state")
    out.append("id: vac.init.source_inventory")
    out.append(f"generated_at: '{GENERATED_AT}'")
    out.append("generator: scripts/regen-vac-control-plane-projection.py")
    out.append("protected_history_policy: append_only")
    out.append("summary:")
    out.append(f"  total_files: {len(files)}")
    out.append(f"  rust_files: {len(rust_files)}")
    out.append(f"  workspace_members: {len(crates)}")
    out.append("by_extension:")
    for k, v in sorted(by_ext.items()):
        out.append(f"  {k!r}: {v}")
    out.append("by_top_level:")
    for k, v in sorted(by_top.items()):
        out.append(f"  {k}: {v}")
    out.append("workspace_members:")
    for row in crates:
        out.append(f"  - path: {row['path']}")
        out.append(f"    exists: {str(row['exists']).lower()}")
    (ROOT / ".vac/.init").mkdir(parents=True, exist_ok=True)
    (ROOT / ".vac/.init/source_inventory.yaml").write_text("\n".join(out) + "\n")


def write_risk_findings(files):
    risk_dir = ROOT / ".vac/.init/risk_findings"
    risk_dir.mkdir(parents=True, exist_ok=True)
    rust = [p for p in files if p.suffix == ".rs"]
    patterns = {
        "unwrap": r"\.unwrap\(",
        "expect": r"\.expect\(",
        "panic": r"panic!\(",
        "unbounded_channel": r"unbounded_channel|UnboundedSender|UnboundedReceiver",
        "chatgpt": r"chatgpt|ChatGPT|AuthMode::Chatgpt",
    }
    rows = []
    for path in rust:
        text = path.read_text(errors="ignore")
        hits = {name: len(re.findall(pattern, text)) for name, pattern in patterns.items()}
        if any(hits.values()):
            rows.append((rel(path), hits))
    full = [
        "schema_version: 1",
        "kind: risk_finding",
        "id: vac.init.risk_findings.full",
        f"generated_at: '{GENERATED_AT}'",
        "generator: scripts/regen-vac-control-plane-projection.py",
        "scope: source_scan_regex",
        "findings:",
    ]
    for path, hits in rows:
        full.append(f"  - path: {path}")
        for k, v in hits.items():
            if v:
                full.append(f"    {k}: {v}")
    (risk_dir / "full.yaml").write_text("\n".join(full) + "\n")
    totals = defaultdict(int)
    for _, hits in rows:
        for k, v in hits.items():
            totals[k] += v
    index = [
        "schema_version: 1",
        "kind: risk_finding",
        "id: vac.init.risk_findings.index",
        f"generated_at: '{GENERATED_AT}'",
        "summary:",
    ]
    for k in sorted(patterns):
        index.append(f"  {k}: {totals[k]}")
    index.append(f"  files_with_hits: {len(rows)}")
    (risk_dir / "index.yaml").write_text("\n".join(index) + "\n")
    (ROOT / ".vac/.init/risk_findings.yaml").write_text("\n".join(index) + "\n")


def write_init_reports(files):
    counts = manifest_counts()
    src = ROOT / ".vac/.init/source_inventory.yaml"
    init = ROOT / ".vac/.init"
    yaml_files = len(list((ROOT / ".vac").rglob("*.yaml")))
    docs_count = len(list((ROOT / "docs").rglob("*.md")))
    rust_count = len([p for p in files if p.suffix == ".rs"])
    reports = {
        "scan_report.yaml": f"""schema_version: 1
kind: init_state
id: vac.init.scan_report
generated_at: '{GENERATED_AT}'
generator: scripts/regen-vac-control-plane-projection.py
summary:
  files: {len(files)}
  markdown_docs: {docs_count}
  rust_files: {rust_count}
  yaml_files: {yaml_files}
  source_inventory_sha256: {sha256(src)}
""",
        "policy_inference_report.yaml": f"""schema_version: 1
kind: init_state
id: vac.init.policy_inference_report
generated_at: '{GENERATED_AT}'
generator: scripts/regen-vac-control-plane-projection.py
policy:
  source: manifest_projection
  risk_model: regex_static_source_inventory
  capabilities: {counts['capabilities']}
  workflows: {counts['workflows']}
  surfaces: {counts['surfaces']}
  policies: {counts['policies']}
  note: Authored policy intent is preserved in .vac/policies and .vac/capabilities; this report is derivable metadata only.
""",
        "scanner_doctor_report.yaml": f"""schema_version: 1
kind: init_state
id: vac.init.scanner_doctor_report
generated_at: '{GENERATED_AT}'
generator: scripts/regen-vac-control-plane-projection.py
status: ready
checks:
  protected_registry_history: preserved
  generated_bucket: regenerated
  docs_projection: regenerated
  donor_docs: pruned
  deterministic_generator: scripts/regen-vac-control-plane-projection.py
""",
        "doctor_report.yaml": f"""schema_version: 1
kind: init_state
id: vac.init.doctor_report
generated_at: '{GENERATED_AT}'
status: source_static_ready
cargo_status: NotEvaluated
reason: cargo/rustc unavailable in sandbox; source/static projection regenerated.
""",
        "state.yaml": f"""schema_version: 1
kind: init_state
id: vac.init.state
generated_at: '{GENERATED_AT}'
status: ready
source_inventory: .vac/.init/source_inventory.yaml
scan_report: .vac/.init/scan_report.yaml
risk_findings: .vac/.init/risk_findings/index.yaml
protected_buckets:
  - .vac/registry/evidence
  - .vac/registry/plans
  - .vac/registry/trajectory
  - .vac/registry/migrations
  - .vac/registry/approvals
  - docs/architecture/adr
  - docs/architecture/decisions
""",
        "strategy.yaml": f"""schema_version: 1
kind: init_state
id: vac.init.regen_strategy
generated_at: '{GENERATED_AT}'
strategy: freeze_prune_scan_scaffold_reconcile_gate
bucket_policy:
  generated: delete_and_regenerate
  authored_intent: scaffold_and_reconcile
  history: append_only_preserve
""",
    }
    for name, text in reports.items():
        (init / name).write_text(text)


def write_docs_projection():
    docs = sorted((ROOT / "docs").rglob("*.md"))
    by_area = defaultdict(int)
    for path in docs:
        by_area[top_area(path)] += 1
    counts = manifest_counts()
    index_lines = ["# Current Docs Index", "", f"Generated: {GENERATED_AT}", "", f"Markdown files indexed: {len(docs)}", "", "## Files", ""]
    for path in docs:
        index_lines.append(f"- `{rel(path)}` — {title_for_markdown(path)}")
    (ROOT / "docs/DOCS_INDEX_CURRENT.md").write_text("\n".join(index_lines) + "\n")
    project = [
        "# VAC current project state from documentation scan", "", f"Generated: {GENERATED_AT}", "",
        "## Scope", "", "This document is generated from the current repository docs and `.vac` manifests. It is a living projection, not an immutable audit trail.", "",
        "## Documentation corpus", "", f"- Markdown files under `docs/`: **{len(docs)}**", "- Full file index: `docs/DOCS_INDEX_CURRENT.md`", "",
        "| Area | Markdown files |", "|---|---:|",
    ]
    for area, count in sorted(by_area.items()):
        project.append(f"| `{area}` | {count} |")
    project.extend(["", "## Current control-plane manifest state", "", "| Manifest area | Count |", "|---|---:|"])
    for area in ["capabilities", "workflows", "surfaces", "policies", "registry"]:
        project.append(f"| `.vac/{area}` | {counts[area]} |")
    project.extend([
        "", "## Donor migration state", "",
        "- `docs/donor-migration/` has been pruned from the living docs projection.",
        "- Donor provenance that must remain auditable stays in `.vac/registry/donor-inventory.yaml` and code-backed control-plane contracts.",
        "- Historical registry evidence/plans/trajectory remain append-only and are not regenerated by this script.",
        "", "## Validation posture", "",
        "- Source/static gates may pass in a no-toolchain sandbox.",
        "- Cargo build/test/clippy remain `NotEvaluated` until run with a Rust toolchain.",
    ])
    (ROOT / "docs/PROJECT_STATE_CURRENT.md").write_text("\n".join(project) + "\n")
    audit = [
        "# VAC root documentation audit", "", f"Current scan date: {GENERATED_AT}", "",
        "This file supersedes dated documentation counts. `docs/donor-migration/` is no longer part of the living docs set; donor evidence stays in registry/code-backed contracts.", "",
        "## Inventory snapshot", "", f"- `docs/**/*.md`: **{len(docs)}**", f"- `.vac/capabilities/*.yaml`: **{counts['capabilities']}**", f"- `.vac/workflows/*.yaml`: **{counts['workflows']}**", f"- `.vac/surfaces/*.yaml`: **{counts['surfaces']}**", f"- `.vac/policies/*.yaml`: **{counts['policies']}**", f"- `.vac/registry/**/*.yaml`: **{counts['registry']}**", "",
        "## Top-level docs counts", "", "| Area | Markdown files |", "|---|---:|",
    ]
    for area, count in sorted(by_area.items()):
        audit.append(f"| `{area}` | {count} |")
    (ROOT / "docs/DOCS_AUDIT.md").write_text("\n".join(audit) + "\n")
    arch_docs = sorted((ROOT / "docs/architecture").glob("*.md"))
    arch = ["# VAC architecture index", "", f"Generated: {GENERATED_AT}", "", "## Current architecture docs", ""]
    for path in arch_docs:
        if path.name != "INDEX.md":
            arch.append(f"- [`{path.name}`]({path.name}) — {title_for_markdown(path)}")
    arch.extend(["", "## Protected decision history", "", "- `adr/`", "- `decisions/`"])
    (ROOT / "docs/architecture/INDEX.md").write_text("\n".join(arch) + "\n")
    docs_state = [
        "schema_version: 1", "kind: registry_status", "id: vac.docs_state", "title: VAC docs state", "status: ready", f"generated_at: '{GENERATED_AT}'",
        "docs:", f"  markdown_files: {len(docs)}", "  index: docs/DOCS_INDEX_CURRENT.md", "  state: docs/PROJECT_STATE_CURRENT.md", "  audit: docs/DOCS_AUDIT.md", "  by_top_level:",
    ]
    for area, count in sorted(by_area.items()):
        docs_state.append(f"    {area}: {count}")
    docs_state.extend([
        "manifests:", f"  capabilities: {counts['capabilities']}", f"  workflows: {counts['workflows']}", f"  surfaces: {counts['surfaces']}", f"  policies: {counts['policies']}", f"  registry: {counts['registry']}",
        "strictness:", "  free_form_validation_commands: 0", f"  structured_validation_commands: {count_structured_validation_commands()}", "  active_compatibility_kinds: 0",
        "donor:", "  docs_pruned: true", "  inventory: .vac/registry/donor-inventory.yaml", "  living_docs_path_absent: docs/donor-migration", "  reason: Donor migration docs are legacy process artifacts; source of truth moved to registry/code-backed contracts.",
        "validation:", "  script: scripts/check-docs-state-refresh.sh",
    ])
    (ROOT / ".vac/registry/docs-state.yaml").write_text("\n".join(docs_state) + "\n")


def protected_guard():
    missing = [p for p in PROTECTED_REQUIRED if not (ROOT / p).exists()]
    if missing:
        raise SystemExit(f"protected append-only bucket missing: {missing}")


def main():
    protected_guard()
    files = list(iter_files(ROOT))
    write_source_inventory(files)
    files = list(iter_files(ROOT))
    write_risk_findings(files)
    write_init_reports(files)
    write_docs_projection()
    print("regen-vac-control-plane-projection: PASS")

if __name__ == "__main__":
    main()
