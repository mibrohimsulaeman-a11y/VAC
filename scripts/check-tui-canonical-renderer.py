#!/usr/bin/env python3
"""Static guard: one canonical VAC v1.9 operator renderer path."""
from __future__ import annotations
import sys
from pathlib import Path
root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
view = (root / "vac-rs/crates/surfaces/vac-tui/src/view.rs").read_text(encoding="utf-8", errors="replace")
mod = (root / "vac-rs/crates/surfaces/vac-tui/src/services/mod.rs").read_text(encoding="utf-8", errors="replace")
errors=[]
if "crate::services::vac_operator::render_vac_operator_console(f, state)" not in view:
    errors.append("view.rs does not call canonical vac_operator renderer first")
first = view.find("crate::services::vac_operator::render_vac_operator_console")
legacy = view.find("operator_console::render")
if first < 0 or (legacy >= 0 and first > legacy):
    errors.append("legacy operator_console render can take over before canonical renderer")
for token in ["pub mod vac_operator;", "legacy non-default renderer", "use services::vac_operator"]:
    if token not in mod:
        errors.append(f"services/mod.rs missing marker {token!r}")
if errors:
    print("VAC TUI canonical renderer gate: FAIL")
    for err in errors:
        print(f"- {err}")
    sys.exit(1)
print("VAC TUI canonical renderer gate: PASS")
