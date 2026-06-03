#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require_file() {
  if [[ ! -f "$1" ]]; then
    echo "FAIL: missing $1" >&2
    exit 1
  fi
}
require_grep() {
  local pattern="$1"
  local path="$2"
  if ! grep -qE "$pattern" "$path"; then
    echo "FAIL: missing pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_file docs/monolith-quality/O6_1_DEPANIC_TRIAGE.md
require_file docs/monolith-quality/O6_2_UNSAFE_TRIAGE.md
require_file docs/monolith-quality/O6_2_SAFETY_ANNOTATION_COVERAGE.md
require_file .vac/registry/o6-quality-triage.yaml

require_grep 'unwrap_runtime:' .vac/registry/o6-quality-triage.yaml
require_grep 'expect_runtime:' .vac/registry/o6-quality-triage.yaml
require_grep 'panic_runtime:' .vac/registry/o6-quality-triage.yaml
require_grep 'previous_grep_upper_bound_retired: true' .vac/registry/o6-quality-triage.yaml
require_grep 'unsafe_upper_bound:' .vac/registry/o6-quality-triage.yaml
require_grep 'cargo_clippy_status: NotEvaluated' .vac/registry/o6-quality-triage.yaml
require_grep 'source_runtime:' .vac/registry/o6-quality-triage.yaml
require_grep 'stale_generic_comments: 0' .vac/registry/o6-quality-triage.yaml
require_grep 'retired_metric:' .vac/registry/o6-quality-triage.yaml

if ! bash scripts/check-vac-o6-2-safety-coverage.sh >/dev/null; then
  exit 1
fi
printf 'O6 quality triage: PASS corrected de-panic surface and SAFETY report present\n'
