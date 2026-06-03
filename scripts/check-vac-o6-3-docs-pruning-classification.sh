#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ -f .vac/registry/o6-docs-pruning-state.yaml ]]
[[ -f docs/monolith-quality/O6_3_DOCS_PRUNING_CLASSIFICATION.md ]]
grep -q 'classification_only_no_physical_delete' .vac/registry/o6-docs-pruning-state.yaml
printf 'O6.3 docs pruning classification: PASS\n'
