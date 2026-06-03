#!/usr/bin/env python3
"""Run rustc-free VAC sandbox gates with explicit toolchain markers.

This replaces the old shell grep-substring skipper.  A gate is skipped only when
it declares `# REQUIRES_TOOLCHAIN:` near the top.  All other gates run with a
bounded timeout and their failure output is tailed for evidence.
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GATE_TIMEOUT_SECONDS = int(os.environ.get("VAC_GATE_TIMEOUT_SECONDS", "60"))


def marker_reason(path: Path) -> tuple[str, str] | None:
    try:
        for line in path.read_text(encoding="utf-8", errors="ignore").splitlines()[:16]:
            if line.startswith("# REQUIRES_TOOLCHAIN:"):
                return ("TV-Pending, requires toolchain", line.split(":", 1)[1].strip() or "toolchain gate")
            if line.startswith("# SUITE_SKIP:"):
                return ("ExplicitSuiteSkip", line.split(":", 1)[1].strip() or "validated separately")
    except OSError:
        return None
    return None


def tail(text: str, limit: int = 6) -> list[str]:
    lines = text.splitlines()
    return lines[-limit:]


def main() -> int:
    scripts = sorted((ROOT / "scripts").glob("check-vac-*.sh"))
    total = passed = failed = skipped = 0
    out_dir = Path(os.environ.get("VAC_GATE_OUTPUT_DIR", "/tmp"))
    out_dir.mkdir(parents=True, exist_ok=True)
    for script in scripts:
        base = script.name
        if base == "check-vac-sandbox-suite.sh":
            continue
        total += 1
        marker = marker_reason(script)
        if marker:
            label, reason = marker
            print(f"SKIP  ({label}: {reason}): {base}", flush=True)
            skipped += 1
            continue
        print(f"RUN   {base}", flush=True)
        out_path = out_dir / f"vac-gate-{base}.out"
        with out_path.open("w", encoding="utf-8") as handle:
            proc = subprocess.Popen(
                ["bash", str(script)],
                cwd=ROOT,
                text=True,
                stdout=handle,
                stderr=subprocess.STDOUT,
            )
            try:
                rc = proc.wait(timeout=GATE_TIMEOUT_SECONDS)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
                print(f"FAIL  {base} (timeout after {GATE_TIMEOUT_SECONDS}s)", flush=True)
                for line in tail(out_path.read_text(encoding="utf-8", errors="ignore")):
                    print(f"      {line}", flush=True)
                failed += 1
                continue
        output = out_path.read_text(encoding="utf-8", errors="ignore")
        if rc == 0:
            print(f"PASS  {base}", flush=True)
            passed += 1
        else:
            print(f"FAIL  {base}", flush=True)
            for line in tail(output):
                print(f"      {line}", flush=True)
            failed += 1
    print("----", flush=True)
    print(f"suite_total={total} passed={passed} failed={failed} skipped_tv_pending={skipped}", flush=True)
    return failed


if __name__ == "__main__":
    raise SystemExit(main())
