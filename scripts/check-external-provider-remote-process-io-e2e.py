#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

from external_provider_remote_process_io_status import (
    TV_FAIL,
    TV_PASS,
    TV_STALE,
    external_provider_remote_process_io_summary,
    print_summary,
)


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify scoped proof without running local Cargo")
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--require-proof", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    summary = external_provider_remote_process_io_summary(root)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print("VAC external provider remote process IO E2E proof: " + str(summary["status"]))
        print_summary(summary)

    status = summary["status"]
    if status in {TV_FAIL, TV_STALE}:
        return 1
    if args.require_proof and status != TV_PASS:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
