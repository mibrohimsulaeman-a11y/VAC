#!/usr/bin/env python3
from __future__ import annotations

import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
allowed_prefixes = (
    "docs/workflow-control-plane/plans/",
    "docs/workflow-control-plane/TUI_RUNTIME_REBRAND_IMPLEMENTATION_NOTES.md",
    ".vac/registry/ledger/architecture-migration.yaml",
    "vac-rs/crates/foundation/vac-brand/src/lib.rs",
    "vac-rs/crates/foundation/vac-paths/src/lib.rs",
)
allowed_files = {"vac-rs/Cargo.lock", "scripts/check-brand-allowlist.py"}
legacy_terms = [
    "stak" + "pak",
    "stack" + "pak",
    "stak" + "ai",
    "stak" + "apk",
    ".stak" + "pak",
]
pattern = re.compile("|".join(re.escape(term) for term in legacy_terms), re.IGNORECASE)
violations: list[str] = []

for path in root.rglob("*"):
    if not path.is_file():
        continue
    rel = path.relative_to(root).as_posix()
    if rel in allowed_files or any(rel.startswith(prefix) for prefix in allowed_prefixes):
        continue
    if any(part in {".git", "target", "node_modules", "__pycache__", "scripts"} for part in path.parts):
        continue
    if path.suffix.lower() in {".png", ".gif", ".jpeg", ".jpg", ".lock", ".zip", ".pyc"}:
        continue
    text = path.read_text(errors="ignore")
    for lineno, line in enumerate(text.splitlines(), 1):
        if pattern.search(line):
            violations.append(f"{rel}:{lineno}: {line.strip()}")

if violations:
    print("Brand allowlist violations:")
    print("\n".join(violations[:200]))
    if len(violations) > 200:
        print(f"... {len(violations) - 200} more")
    sys.exit(1)
print("PASS brand allowlist gate")
