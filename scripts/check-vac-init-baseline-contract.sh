#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
}

require_grep() {
  local pattern="$1"
  local path="$2"
  if ! grep -qE "$pattern" "$path"; then
    echo "missing pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_file docs/vac-init/VAC_INIT_EXECUTION_PLAN.md
require_file docs/vac-init/VAC_INIT_IMPLEMENTATION_MAP.md
require_file docs/validation/VAC_INIT_BASELINE_AUDIT.md
require_file docs/validation/VAC_INIT_SCHEMA_ENVELOPE_GATE.md

require_grep 'Batch 0' docs/vac-init/VAC_INIT_EXECUTION_PLAN.md
require_grep 'Batch 1' docs/vac-init/VAC_INIT_EXECUTION_PLAN.md
require_grep 'schema_envelope.rs' docs/vac-init/VAC_INIT_IMPLEMENTATION_MAP.md
require_grep 'kind_registry.rs' docs/vac-init/VAC_INIT_IMPLEMENTATION_MAP.md
require_grep 'Full workspace build/link is intentionally not used' docs/validation/VAC_INIT_BASELINE_AUDIT.md
require_grep 'target/' docs/vac-init/VAC_INIT_EXECUTION_PLAN.md
require_grep 'Artifact' docs/vac-init/VAC_INIT_EXECUTION_PLAN.md

printf 'vac-init baseline contract: PASS\n'
