#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ -f docs/monolith-quality/O5_3_UTILS_CONSOLIDATION_REPORT.md ]]
[[ -f .vac/registry/o5-o6-completion-state.yaml ]]
grep -q 'O5.3' docs/monolith-quality/O5_3_UTILS_CONSOLIDATION_REPORT.md
grep -q 'Planned_NotEvaluated' .vac/registry/o5-o6-completion-state.yaml
printf 'O5.3 utils consolidation report: PASS (NotEvaluated)\n'
