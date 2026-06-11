#!/usr/bin/env python3
"""Static CI sanity check for VAC v1.9 nested Rust workspace."""
from __future__ import annotations
import pathlib, sys

ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
errors: list[str] = []

ci = ROOT / ".github/workflows/ci.yml"
release = ROOT / ".github/workflows/build-and-release.yml"
for path in [ci, release]:
    if not path.is_file():
        errors.append(f"missing workflow: {path.relative_to(ROOT)}")

if ci.is_file():
    text = ci.read_text()
    if "working-directory: vac-rs" not in text and "--manifest-path vac-rs/Cargo.toml" not in text:
        errors.append("ci.yml Rust job must run cargo from vac-rs or pass --manifest-path vac-rs/Cargo.toml")
    if "hashFiles('vac-rs/Cargo.lock')" not in text:
        errors.append("ci.yml cargo cache must key on vac-rs/Cargo.lock")

if release.is_file():
    text = release.read_text()
    required = [
        "--manifest-path vac-rs/Cargo.toml",
        "-w /build/vac-rs",
        "vac-rs/target/",
        "vac-rs/Cargo.toml",
    ]
    for token in required:
        if token not in text:
            errors.append(f"build-and-release.yml missing v1.9 nested-workspace token: {token}")

if errors:
    print("VAC CI workdir v1.9: FAIL")
    for error in errors:
        print("-", error)
    raise SystemExit(1)
print("VAC CI workdir v1.9: PASS")
print("rust_workspace=vac-rs")
