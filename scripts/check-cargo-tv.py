#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import pathlib
import shlex
import subprocess
import sys
import time
from datetime import datetime, timezone
from typing import Any

from cargo_tv_status import (
    PROOF_REL,
    REQUIRED_CHECKS,
    TV_FAIL,
    TV_PASS,
    canonical_hash,
    cargo_tv_summary,
    cargo_workspace_hash,
    git_output,
    print_summary,
    proof_path,
    sha256_bytes,
)

COMMAND_GROUPS: list[dict[str, Any]] = [
    {
        "id": "cargo_metadata",
        "commands": [
            [
                "cargo",
                "metadata",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "--locked",
                "--format-version",
                "1",
            ]
        ],
        "timeout_seconds": 300,
    },
    {
        "id": "cargo_fmt",
        "commands": [
            [
                "cargo",
                "fmt",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "--all",
                "--",
                "--check",
            ]
        ],
        "timeout_seconds": 300,
    },
    {
        "id": "cargo_check",
        "commands": [
            [
                "cargo",
                "check",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "--workspace",
                "--all-targets",
                "--locked",
            ]
        ],
        "timeout_seconds": 1800,
    },
    {
        "id": "cargo_clippy",
        "commands": [
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ]
        ],
        "timeout_seconds": 2400,
    },
    {
        "id": "cargo_test",
        "commands": [
            [
                "cargo",
                "test",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "--workspace",
                "--all-targets",
                "--locked",
            ],
            [
                "cargo",
                "test",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "-p",
                "vac-foundation",
                "--features",
                "sqlite",
                "--locked",
            ],
            [
                "cargo",
                "test",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "-p",
                "vac-cli",
                "--features",
                "libsql-test",
                "--locked",
            ],
            [
                "cargo",
                "test",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "-p",
                "vac-messaging-gateway",
                "--features",
                "libsql-test",
                "--locked",
            ],
            [
                "cargo",
                "test",
                "--manifest-path",
                "vac-rs/Cargo.toml",
                "-p",
                "vac-provider-core",
                "--features",
                "network-tests",
                "--locked",
            ],
        ],
        "timeout_seconds": 2400,
    },
]


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace(
        "+00:00", "Z"
    )


def quote_cmd(cmd: list[str]) -> str:
    return " ".join(shlex.quote(part) for part in cmd)


def command_env() -> dict[str, str]:
    env = os.environ.copy()
    env.setdefault("CARGO_TERM_COLOR", "never")
    env.setdefault("RUST_BACKTRACE", "0")
    env.setdefault("LC_ALL", "C")
    return env


def run_command(root: pathlib.Path, cmd: list[str], timeout_seconds: int) -> dict[str, Any]:
    started_at = utc_now()
    started = time.monotonic()
    try:
        completed = subprocess.run(
            cmd,
            cwd=root,
            env=command_env(),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout_seconds,
            check=False,
        )
        output = completed.stdout or ""
        exit_code = completed.returncode
        timed_out = False
    except subprocess.TimeoutExpired as exc:
        raw_output = exc.stdout or ""
        output = raw_output if isinstance(raw_output, str) else raw_output.decode(
            "utf-8", errors="replace"
        )
        exit_code = 124
        timed_out = True
    elapsed = time.monotonic() - started
    return {
        "command": cmd,
        "command_display": quote_cmd(cmd),
        "exit_code": exit_code,
        "status": TV_PASS if exit_code == 0 else TV_FAIL,
        "timed_out": timed_out,
        "started_at": started_at,
        "ended_at": utc_now(),
        "elapsed_seconds": round(elapsed, 3),
        "output_sha256": sha256_bytes(output.encode("utf-8", errors="replace")),
        "output_tail": output[-4000:],
    }


def run_group(root: pathlib.Path, group: dict[str, Any]) -> dict[str, Any]:
    print(f"\n== {group['id']} ==", flush=True)
    command_results = []
    for cmd in group["commands"]:
        print("$ " + quote_cmd(cmd), flush=True)
        result = run_command(root, cmd, int(group["timeout_seconds"]))
        command_results.append(result)
        print(f"exit={result['exit_code']}", flush=True)
        if result["exit_code"] != 0:
            if result.get("output_tail"):
                print(result["output_tail"], file=sys.stderr)
            break
    status = TV_PASS if all(r["exit_code"] == 0 for r in command_results) else TV_FAIL
    return {
        "status": status,
        "commands": command_results,
    }


def write_proof(root: pathlib.Path, checks: dict[str, Any], before_hash: str, after_hash: str) -> dict[str, Any]:
    proof = {
        "schema_version": 1,
        "kind": "cargo_tv_current_run_proof",
        "generated_at": utc_now(),
        "proof_ref": PROOF_REL,
        "git_head": git_output(root, ["rev-parse", "HEAD"]),
        "git_status_vac_rs": git_output(root, ["status", "--short", "--", "vac-rs"]),
        "cargo_workspace_hash": after_hash,
        "cargo_workspace_hash_before": before_hash,
        "cargo_workspace_unchanged_during_run": before_hash == after_hash,
        "checks": checks,
    }
    proof["proof_status"] = (
        TV_PASS
        if before_hash == after_hash
        and all(checks.get(check_id, {}).get("status") == TV_PASS for check_id in REQUIRED_CHECKS)
        else TV_FAIL
    )
    proof["proof_hash"] = canonical_hash(proof)
    path = proof_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(proof, indent=2, sort_keys=True) + "\n")
    return proof


def run_cargo_tv(root: pathlib.Path) -> int:
    before_hash = cargo_workspace_hash(root)
    checks: dict[str, Any] = {}
    for group in COMMAND_GROUPS:
        result = run_group(root, group)
        checks[group["id"]] = result
        if result["status"] != TV_PASS:
            break
    for missing in REQUIRED_CHECKS:
        checks.setdefault(missing, {"status": TV_FAIL, "commands": []})
    after_hash = cargo_workspace_hash(root)
    write_proof(root, checks, before_hash, after_hash)
    summary = cargo_tv_summary(root, consume_proof=True)
    print_summary(summary)
    if summary.get("status") != TV_PASS:
        print("cargo_tv_errors=" + json.dumps(summary.get("errors", []), sort_keys=True))
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Run or summarize current-worktree Cargo TV proof.")
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--summary-only", action="store_true", help="do not run Cargo; validate and print existing proof")
    parser.add_argument("--json", action="store_true", help="print summary as JSON")
    args = parser.parse_args()

    root = pathlib.Path(args.root).resolve()
    if args.summary_only:
        summary = cargo_tv_summary(root, consume_proof=True)
        if args.json:
            print(json.dumps(summary, sort_keys=True))
        else:
            print_summary(summary)
            if summary.get("status") != TV_PASS:
                print("cargo_tv_errors=" + json.dumps(summary.get("errors", []), sort_keys=True))
        return 0 if summary.get("status") == TV_PASS else 1
    return run_cargo_tv(root)


if __name__ == "__main__":
    raise SystemExit(main())
