#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
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

require_absent() {
  local pattern="$1"
  local path="$2"
  if grep -qE "$pattern" "$path"; then
    echo "FAIL: forbidden pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_file .vac/registry/plans/plan.dogfood.assistant-model.vac-session.yaml
require_file docs/dogfood/2026-05-30-assistant-model-vac-dogfood.md
require_file .vac/registry/evidence/evidence.2026-05-30-dogfood-assistant-model.yaml
require_file .vac/registry/trajectory/dogfood-assistant-model.yaml
require_file .vac/capabilities/dogfood-session.yaml
require_file .vac/workflows/maintenance.dogfood-assistant-model.yaml
require_file scripts/check-no-hardcoded-readiness-scoreboard.sh

require_grep 'kind: plan' .vac/registry/plans/plan.dogfood.assistant-model.vac-session.yaml
require_grep 'status: completed' .vac/registry/plans/plan.dogfood.assistant-model.vac-session.yaml
require_grep 'kind: evidence' .vac/registry/evidence/evidence.2026-05-30-dogfood-assistant-model.yaml
require_grep 'raw_chain_of_thought: true' .vac/registry/trajectory/dogfood-assistant-model.yaml
require_grep 'mktemp -d' scripts/check-no-hardcoded-readiness-scoreboard.sh
require_absent '>/tmp/vac-hardcoded' scripts/check-no-hardcoded-readiness-scoreboard.sh
require_absent 'cat /tmp/vac-hardcoded' scripts/check-no-hardcoded-readiness-scoreboard.sh

# Safe rationale only. The only allowed raw_chain_of_thought text is the
# explicit exclusion marker required by the vac why contract.
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-dogfood-session.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

if grep -RIn 'chain of thought\|private reasoning\|scratchpad' docs/dogfood .vac/registry/evidence .vac/registry/trajectory \
  | grep -v 'raw_chain_of_thought: true' >"$TMPROOT/unsafe-rationale.txt"; then
  cat "$TMPROOT/unsafe-rationale.txt" >&2
  exit 1
fi

bash scripts/check-no-hardcoded-readiness-scoreboard.sh >/dev/null

printf 'vac dogfood session: PASS\n'
