#!/usr/bin/env bash
# Donor retirement/provenance consistency gate.
set -eo pipefail
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"
MODE="${1:-all}"
fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK:   $*"; }
check_docs_pruned() { [[ ! -d docs/donor-migration ]] || fail "docs/donor-migration is retired and must stay pruned"; ok "donor living docs tree pruned"; }
check_registry() { [[ -f .vac/registry/donor-inventory.yaml ]] || fail "missing donor inventory"; grep -q 'legacy_kind: donor_inventory' .vac/registry/donor-inventory.yaml || fail "missing legacy_kind provenance"; ok "donor registry provenance present"; }
check_contracts() { [[ -f vac-rs/control-plane/src/control_plane/donor_domain_contract.rs ]] || fail "missing donor contract source"; for plan in 01 02 03 04 05 06 07 08 09 10 11; do grep -Eq "plan_id: \"?$plan\"?" vac-rs/control-plane/src/control_plane/donor_domain_contract.rs || fail "domain contract missing plan_id $plan"; done; ok "static donor domain contract registry covers plans 01-11"; }
check_manifest() { if grep -rl 'donor_source:' .vac/capabilities/*.yaml >/tmp/vac_donor_files.$$ 2>/dev/null; then count="$(wc -l </tmp/vac_donor_files.$$ | tr -d ' ')"; rm -f /tmp/vac_donor_files.$$; ok "donor-backed capability metadata retained for $count manifest(s)"; else ok "no donor-backed capability manifests remain in active local-agent path"; fi; }
check_reachability() { hits="$(grep -r 'vac_shell\|vac_tui_runtime' vac-rs/cli/Cargo.toml vac-rs/tui/Cargo.toml vac-rs/core/Cargo.toml 2>/dev/null | grep -v '^#' | grep -v 'test\|dev-dependencies' || true)"; [[ -z "$hits" ]] || fail "product Cargo.toml contains quarantined donor dependency: $hits"; ok "no product Cargo.toml directly depends on quarantined donor crates"; }
case "$MODE" in
  inventory|drift|evidence|commit-phrase) check_docs_pruned; check_registry ;;
  manifest) check_manifest ;;
  contracts) check_contracts ;;
  plan11-hermes) bash scripts/check-donor-plan11-hermes-memory-alignment.sh ;;
  reachability) check_reachability ;;
  all) check_docs_pruned; check_registry; check_manifest; check_contracts; bash scripts/check-donor-plan11-hermes-memory-alignment.sh; check_reachability ;;
  *) echo "Usage: $0 [inventory|drift|manifest|contracts|plan11-hermes|reachability|evidence|commit-phrase|all]"; exit 1 ;;
esac
printf '
DONOR MIGRATION GATE PASSED (retired living docs; registry/code-backed provenance retained).
'
