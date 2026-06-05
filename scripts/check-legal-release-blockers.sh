#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require_file() {
  [[ -f "$1" ]] || { echo "FAIL: missing $1" >&2; exit 1; }
}

require_file THIRD_PARTY_NOTICES.md
require_file docs/legal/NOTICES.md
require_file .vac/registry/legal-release-blockers.yaml

grep -q 'Portions of this repository are derived from or historically based on OpenAI Codex CLI' THIRD_PARTY_NOTICES.md
if grep -q 'No third-party attributions are required' docs/legal/NOTICES.md; then
  echo "FAIL: docs/legal/NOTICES.md still claims no third-party attributions are required" >&2
  exit 1
fi
grep -q 'dependency_attribution: NotEvaluated' .vac/registry/legal-release-blockers.yaml
grep -q 'cargo_build_status: NotEvaluated' .vac/registry/legal-release-blockers.yaml
grep -q 'cargo_deny_licenses: NotEvaluated' .vac/registry/legal-release-blockers.yaml
grep -q 'cargo_about_generate: NotEvaluated' .vac/registry/legal-release-blockers.yaml

printf 'legal release blockers: PASS (build/legal gates registered as NotEvaluated)\n'
