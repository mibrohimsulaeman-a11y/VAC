#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

from l2_broker_status import IMPLEMENTED, SV_PASS, l2_broker_summary, print_summary


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify L2 broker implementation/proof boundary")
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--require-implemented", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    summary = l2_broker_summary(root)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print("VAC L2 broker status: " + str(summary["status"]))
        print_summary(summary)
    if summary.get("claim_gate") != SV_PASS:
        return 1
    if args.require_implemented and summary.get("status") != IMPLEMENTED:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
