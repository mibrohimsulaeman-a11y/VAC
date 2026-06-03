#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "standard cargo hygiene static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }

require_file vac-rs/deny.toml
require_file vac-rs/.cargo/audit.toml
require_file .vac/workflows/maintenance.build-check.yaml
require_file docs/monolith-quality/O5O6_STANDARD_CARGO_HYGIENE_GATE.md

for id in fmt clippy deny audit; do
  require_grep "maintenance.build-check.validation.${id}" .vac/workflows/maintenance.build-check.yaml
done
require_grep '^    - fmt$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - --all$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - --check$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - clippy$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - --workspace$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - --all-targets$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - -D$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - warnings$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - deny$' .vac/workflows/maintenance.build-check.yaml
require_grep '^    - audit$' .vac/workflows/maintenance.build-check.yaml
require_grep 'Registered_NotEvaluated' docs/monolith-quality/O5O6_STANDARD_CARGO_HYGIENE_GATE.md
printf 'standard cargo hygiene static gate: PASS (cargo execution TV-Pending)\n'
