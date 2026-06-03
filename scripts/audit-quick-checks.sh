#!/usr/bin/env bash
# Run the standard scheduled-audit quick checks once and emit aggregate JSON.
#
# Usage:
#   ./scripts/audit-quick-checks.sh [repo-root]
#
# The script intentionally executes all checks even if one fails, then exits
# non-zero if any check failed. Human-readable command output goes to stderr;
# the final JSON summary is written to stdout.

set -u -o pipefail

repo_root="${1:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
cd "$repo_root" || exit 2

vac_bin="./vac-rs/target/debug/vac"
if [ ! -x "$vac_bin" ]; then
  echo "ERROR: $vac_bin not found or not executable; build vac-cli before running quick checks." >&2
  printf '{"ok":false,"error":"missing vac binary","checks":[]}\n'
  exit 2
fi

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))'
}

results=()
failures=0

run_check() {
  local name="$1"
  shift
  local tmp status tail_json command_json
  tmp="$(mktemp)"
  echo "=== $name ===" >&2
  echo "+ $*" >&2
  "$@" >"$tmp" 2>&1
  status=$?
  cat "$tmp" >&2
  if [ "$status" -ne 0 ]; then
    failures=$((failures + 1))
  fi
  command_json="$(printf '%s' "$*" | json_escape)"
  tail_json="$(tail -n 40 "$tmp" | json_escape)"
  results+=("{\"name\":\"$name\",\"status\":$status,\"command\":$command_json,\"output_tail\":$tail_json}")
  rm -f "$tmp"
}

run_check registry "$vac_bin" doctor registry .
run_check surfaces "$vac_bin" doctor surfaces .
run_check workflow "$vac_bin" doctor workflow .
run_check policy "$vac_bin" doctor policy .
run_check donor "$vac_bin" doctor donor .
run_check donor_status scripts/check-donor-status.sh all

if [ "$failures" -eq 0 ]; then
  ok=true
else
  ok=false
fi

{
  printf '{"ok":%s,"failures":%s,"checks":[' "$ok" "$failures"
  for i in "${!results[@]}"; do
    if [ "$i" -gt 0 ]; then
      printf ','
    fi
    printf '%s' "${results[$i]}"
  done
  printf ']}\n'
}

if [ "$failures" -eq 0 ]; then
  exit 0
fi
exit 1
