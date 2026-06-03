#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
ledger=.vac/registry/static-validation-ledger.yaml
[[ -f "$ledger" ]] || fail "missing static validation ledger"
grep -q 'status: pass' "$ledger" || fail "static validation ledger is not pass"
grep -q 'cargo_status: TV-Pending' "$ledger" || fail "ledger must preserve cargo TV-Pending in static-only mode"
for required in   scripts/check-vac-o5-2-semantic-split-hash.sh   scripts/check-vac-o5-2-godfile-staging-all.sh   scripts/check-vac-f5-protocol-duplication.sh   scripts/check-vac-o5-5-donor-delete-gate.sh   scripts/check-vac-o6-2-safety-coverage.sh   scripts/check-vac-o5o6-current-auditfix-static.sh   scripts/check-vac-tui-visual-parity-static.sh; do
  grep -q -- "- $required" "$ledger" || fail "ledger missing required gate: $required"
done
grep -q 'all_audit_remainder_2026_05_30:' .vac/registry/o5-o6-completion-state.yaml || fail "completion state missing all-audit section"
grep -q 'TV-Pending' .vac/registry/o5-o6-completion-state.yaml || fail "completion state missing TV-Pending caveat"
printf 'O5/O6 all audit remainder: PASS source/static; cargo TV-Pending
'
