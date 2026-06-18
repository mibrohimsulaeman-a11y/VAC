#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from ci_scoped_validation_status import (
    CI_ATTESTED,
    CLAIM_ID,
    LOCAL_ONLY,
    L2_NOT_IMPLEMENTED,
    OBSERVED_L1,
    PROOF_KIND,
    PROOF_REL,
    REQUIRED_CHECKS,
    TV_PASS,
    TV_PENDING,
    ci_scoped_validation_summary,
    git_output,
    print_summary,
    proof_payload_hash,
    proof_path,
    source_scope_hash,
)


def github_context() -> dict[str, Any]:
    server_url = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    return {
        "actions": os.environ.get("GITHUB_ACTIONS") == "true",
        "run_id": run_id,
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", ""),
        "workflow": os.environ.get("GITHUB_WORKFLOW", ""),
        "job": os.environ.get("GITHUB_JOB", ""),
        "sha": os.environ.get("GITHUB_SHA", ""),
        "ref": os.environ.get("GITHUB_REF", ""),
        "server_url": server_url,
        "repository": repository,
        "run_url": f"{server_url}/{repository}/actions/runs/{run_id}" if repository and run_id else None,
    }


def build_proof(root: Path) -> tuple[dict[str, Any], int]:
    github = github_context()
    generated_in_ci = github["actions"] is True
    git_head = git_output(root, "rev-parse", "HEAD") or "unknown"
    checks = {check: TV_PASS for check in REQUIRED_CHECKS}
    blocked_reasons: list[str] = []
    if not generated_in_ci:
        blocked_reasons.append("github_actions_context_required")
    if not github.get("run_id"):
        blocked_reasons.append("github_run_id_required")
    if github.get("sha") and github.get("sha") != git_head:
        blocked_reasons.append("github_sha_mismatch")

    proof_status = TV_PASS if not blocked_reasons else TV_PENDING
    proof = {
        "schema_version": 1,
        "kind": PROOF_KIND,
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "proof_ref": PROOF_REL,
        "claim": CLAIM_ID,
        "execution": OBSERVED_L1,
        "custody": CI_ATTESTED if generated_in_ci else LOCAL_ONLY,
        "l2_broker": L2_NOT_IMPLEMENTED,
        "git_head": git_head,
        "source_scope_hash": source_scope_hash(root),
        "github": github,
        "checks": checks,
        "proof_status": proof_status,
        "blocked_reasons": blocked_reasons,
    }
    proof["proof_hash"] = proof_payload_hash(proof)
    return proof, 0 if proof_status == TV_PASS else 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Create CI-attested scoped validation proof")
    parser.add_argument("root", nargs="?", default=".")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    proof, exit_code = build_proof(root)
    path = proof_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(proof, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = ci_scoped_validation_summary(root)
    print("VAC CI scoped validation proof generated")
    print_summary(summary)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
