#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ -f docs/monolith-quality/O5_4_WORKSPACE_CONSOLIDATION_REPORT.md ]]
grep -q 'Workspace member scan' docs/monolith-quality/O5_4_WORKSPACE_CONSOLIDATION_REPORT.md
printf 'O5.4 workspace consolidation report: PASS (NotEvaluated)\n'
