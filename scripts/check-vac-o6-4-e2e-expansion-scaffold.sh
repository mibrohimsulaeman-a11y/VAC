#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ -f .vac/registry/o6-e2e-expansion-state.yaml ]]
[[ -f docs/monolith-quality/O6_4_E2E_EXPANSION_SCAFFOLD.md ]]
grep -q 'cargo_e2e_status: NotEvaluated' .vac/registry/o6-e2e-expansion-state.yaml
printf 'O6.4 E2E expansion scaffold: PASS (NotEvaluated)\n'
