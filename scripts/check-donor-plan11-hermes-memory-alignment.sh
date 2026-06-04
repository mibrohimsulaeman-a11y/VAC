#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
[[ ! -d docs/donor-migration ]] || fail "docs/donor-migration must stay pruned from living docs"
[[ -f .vac/registry/donor-inventory.yaml ]] || fail "missing donor inventory registry"
[[ -f vac-rs/crates/control-plane/control-plane/src/control_plane/donor_domain_contract.rs ]] || fail "missing code-backed donor domain contract"
grep -q 'PLAN11_HERMES_MEMORY_CANONICAL_CAPABILITIES' vac-rs/crates/control-plane/control-plane/src/control_plane/donor_domain_contract.rs || fail "Plan 11 memory contract missing"
printf 'donor Plan 11 Hermes memory alignment: PASS (registry/code-backed; living donor docs pruned)
'
