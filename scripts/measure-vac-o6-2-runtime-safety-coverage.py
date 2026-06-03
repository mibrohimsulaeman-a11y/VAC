#!/usr/bin/env python3
"""Measure O6.2 runtime unsafe SAFETY coverage without test pollution.

The old shell-only gate counted files such as `*_tests.rs` and inline
`#[cfg(test)]` modules.  This scanner keeps the source-runtime denominator
separate from path/cfg-test exclusions and rejects the generic boilerplate
comments that caused a false 558/558 headline.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Iterable

UNSAFE_RE = re.compile(r"\bunsafe\s*(?:\{|fn\b|impl\b|extern\b|trait\b)")
CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\([^\]]*\btest\b[^\]]*\)\s*\]")

STALE_GENERIC_SAFETY = {
    "SAFETY: PTY/Windows handle boundary requires valid OS handles/pointers and lifetimes guarded by the surrounding wrapper.",
    "SAFETY: Sandbox/OS boundary keeps kernel ABI calls behind this wrapper; caller must pass validated descriptors and preserve surrounding invariants.",
    "SAFETY: Unsafe operation is retained behind the existing module boundary; caller must uphold the surrounding invariants until TV verification confirms the refactor.",
    "SAFETY: Shell/runtime boundary preserves existing checked invariants; unsafe operation is isolated until TV verification can validate it with cargo.",
    "SAFETY: FFI boundary is isolated here; caller must uphold ABI, pointer validity, and ownership invariants documented by this module.",
    "SAFETY: Platform FFI call is confined to this wrapper; surrounding code must uphold pointer validity and OS API preconditions.",
}


@dataclass(frozen=True)
class FileCoverage:
    total: int
    covered: int
    cfg_test_excluded: int


@dataclass(frozen=True)
class CoverageReport:
    source_runtime_total: int
    source_runtime_covered: int
    linux_host_runtime_total: int
    linux_host_runtime_covered: int
    path_test_excluded: int
    cfg_test_excluded: int
    stale_generic_comments: int
    missing: list[str]
    stale: list[str]
    by_file: dict[str, FileCoverage]


def is_runtime_source_path(path: Path) -> bool:
    rel = path.as_posix()
    if not rel.startswith("vac-rs/") or not rel.endswith(".rs"):
        return False
    parts = rel.split("/")
    if any(part in {"tests", "fixtures", "benches"} or part.startswith("test-") for part in parts):
        return False
    name = path.name
    if name.endswith("_test.rs") or name.endswith("_tests.rs"):
        return False
    if name.startswith("test_") or name == "tests.rs":
        return False
    return True


def is_linux_host_runtime_path(path: Path) -> bool:
    """Return source files likely compiled on the Linux sandbox host.

    This is a secondary metric only.  O6.2 source-runtime still tracks all
    platform runtime code, including Windows-specific crates, because release
    quality cannot ignore target-specific unsafe blocks.
    """
    if not is_runtime_source_path(path):
        return False
    parts = path.as_posix().split("/")
    if "windows-sandbox-rs" in parts:
        return False
    if "/win/" in path.as_posix():
        return False
    name = path.name.lower()
    if name.startswith("windows_") or name.endswith("_windows.rs") or "windows" in name:
        return False
    return True


def unsafe_line(line: str) -> bool:
    stripped = line.strip()
    return bool(stripped and not stripped.startswith("//") and UNSAFE_RE.search(line))


def cfg_test_spans(lines: list[str]) -> set[int]:
    """Naively mark items guarded by #[cfg(test)] as test-only.

    This is a static lint, not a Rust parser.  It intentionally errs on the side
    of marking the whole next braced item after a cfg(test) attribute.  That is
    sufficient for VAC O6.2 denominator hygiene and keeps the gate rustc-free.
    """
    skipped: set[int] = set()
    i = 0
    while i < len(lines):
        if not CFG_TEST_RE.search(lines[i]):
            i += 1
            continue
        skipped.add(i)
        j = i + 1
        while j < len(lines):
            stripped = lines[j].strip()
            if not stripped or stripped.startswith("#[") or stripped.startswith("///") or stripped.startswith("//"):
                skipped.add(j)
                j += 1
                continue
            break
        if j >= len(lines):
            i = j
            continue
        depth = 0
        started = False
        k = j
        while k < len(lines):
            skipped.add(k)
            for ch in lines[k]:
                if ch == "{":
                    depth += 1
                    started = True
                elif ch == "}" and started:
                    depth -= 1
            if started and depth <= 0:
                break
            if not started and ";" in lines[k]:
                break
            k += 1
        i = max(k + 1, j + 1)
    return skipped


def nearby_safety_comment(lines: list[str], idx: int, excluded: set[int]) -> tuple[bool, str | None]:
    # O6.2 genuine mode: each runtime unsafe site needs its own immediately
    # preceding SAFETY comment.  A broad three-line window let one comment cover
    # adjacent unsafe calls and was the source of the earlier false-green metric.
    j = idx - 1
    while j >= 0 and not lines[j].strip():
        j -= 1
    if j >= 0 and j not in excluded:
        stripped = lines[j].strip()
        if "SAFETY:" in stripped:
            return True, stripped.lstrip("/ ")
    return False, None


def scan(root: Path) -> CoverageReport:
    source_total = 0
    source_covered = 0
    linux_total = 0
    linux_covered = 0
    path_excluded = 0
    cfg_excluded = 0
    missing: list[str] = []
    stale: list[str] = []
    by_file: dict[str, FileCoverage] = {}

    for path in sorted(root.rglob("vac-rs/**/*.rs")):
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
        rel = path.as_posix()
        if not is_runtime_source_path(path):
            for line in lines:
                if unsafe_line(line):
                    path_excluded += 1
            continue

        excluded = cfg_test_spans(lines)
        file_total = 0
        file_covered = 0
        file_cfg_excluded = 0
        linux_path = is_linux_host_runtime_path(path)
        for idx, line in enumerate(lines):
            if not unsafe_line(line):
                continue
            if idx in excluded:
                cfg_excluded += 1
                file_cfg_excluded += 1
                continue

            source_total += 1
            file_total += 1
            if linux_path:
                linux_total += 1
            ok, comment = nearby_safety_comment(lines, idx, excluded)
            if ok:
                source_covered += 1
                file_covered += 1
                if linux_path:
                    linux_covered += 1
                if comment in STALE_GENERIC_SAFETY:
                    stale.append(f"{rel}:{idx + 1}: {comment}")
            else:
                missing.append(f"{rel}:{idx + 1}: missing SAFETY comment")
        if file_total or file_cfg_excluded:
            by_file[rel] = FileCoverage(file_total, file_covered, file_cfg_excluded)

    return CoverageReport(
        source_runtime_total=source_total,
        source_runtime_covered=source_covered,
        linux_host_runtime_total=linux_total,
        linux_host_runtime_covered=linux_covered,
        path_test_excluded=path_excluded,
        cfg_test_excluded=cfg_excluded,
        stale_generic_comments=len(stale),
        missing=missing,
        stale=stale,
        by_file=by_file,
    )


def load_registry_numbers(path: Path) -> dict[str, int]:
    numbers: dict[str, int] = {}
    stack: list[tuple[int, str]] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw.strip() or raw.lstrip().startswith("#") or ":" not in raw:
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        key, value = raw.strip().split(":", 1)
        while stack and stack[-1][0] >= indent:
            stack.pop()
        full_key = ".".join([item[1] for item in stack] + [key])
        value = value.strip().strip("'").strip('"')
        if re.fullmatch(r"-?\d+", value):
            numbers[full_key] = int(value)
        stack.append((indent, key))
    return numbers


def print_summary(report: CoverageReport) -> None:
    print(f"source_runtime_safety_coverage: {report.source_runtime_covered}/{report.source_runtime_total}")
    print(f"linux_host_runtime_safety_coverage: {report.linux_host_runtime_covered}/{report.linux_host_runtime_total}")
    print(f"excluded_path_test_or_fixture: {report.path_test_excluded}")
    print(f"excluded_cfg_test: {report.cfg_test_excluded}")
    print(f"stale_generic_safety_comments: {report.stale_generic_comments}")


def main(argv: Iterable[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--check-registry", type=Path)
    parser.add_argument("--allow-stale-generic", action="store_true")
    args = parser.parse_args(list(argv))

    report = scan(Path(args.root))
    if args.json:
        payload = asdict(report)
        payload["by_file"] = {key: asdict(value) for key, value in report.by_file.items()}
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        print_summary(report)

    if report.missing:
        print("Missing SAFETY comments:", file=sys.stderr)
        print("\n".join(report.missing[:200]), file=sys.stderr)
        return 1
    if report.stale and not args.allow_stale_generic:
        print("Stale generic SAFETY comments:", file=sys.stderr)
        print("\n".join(report.stale[:200]), file=sys.stderr)
        return 1
    if args.check_registry:
        numbers = load_registry_numbers(args.check_registry)
        expected = {
            "safety_coverage.source_runtime.covered": report.source_runtime_covered,
            "safety_coverage.source_runtime.total": report.source_runtime_total,
            "safety_coverage.linux_host_runtime.covered": report.linux_host_runtime_covered,
            "safety_coverage.linux_host_runtime.total": report.linux_host_runtime_total,
            "safety_coverage.exclusions.path_test_or_fixture": report.path_test_excluded,
            "safety_coverage.exclusions.cfg_test": report.cfg_test_excluded,
        }
        stale = [f"{key}: registry={numbers.get(key)!r} actual={value!r}" for key, value in expected.items() if numbers.get(key) != value]
        if stale:
            print("Registry coverage numbers are stale:", file=sys.stderr)
            print("\n".join(stale), file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
