#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
ledger=.vac/registry/static-validation-ledger.yaml
[[ -f "$ledger" ]] || fail "missing static validation ledger"
grep -q 'status: pass' "$ledger" || fail "static validation ledger is not pass"
grep -q 'cargo_status: TV-Pending' "$ledger" || fail "static-only completion state must preserve cargo TV-Pending"
grep -q 'overall_status: Implemented_NotEvaluated' .vac/registry/o5-o6-completion-state.yaml || fail "completion state not Implemented_NotEvaluated"
grep -q 'all_audit_remainder_2026_05_30:' .vac/registry/o5-o6-completion-state.yaml || fail "missing all-audit remainder completion section"
grep -q 'TV-Pending' .vac/registry/o5-o6-completion-state.yaml || fail "missing TV-Pending caveat"
printf 'O5/O6 completion state: PASS (Implemented_NotEvaluated; cargo TV-Pending)
'
