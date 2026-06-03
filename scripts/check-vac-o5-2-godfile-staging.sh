#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
# Compatibility entry point retained for older workflow/capability IDs.
bash scripts/check-vac-o5-2-godfile-staging-all.sh
