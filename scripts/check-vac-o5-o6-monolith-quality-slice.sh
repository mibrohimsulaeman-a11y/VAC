#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }

require_file .vac/registry/o5-godfile-staging-state.yaml
require_file .vac/registry/o6-panic-surface.yaml
require_file .vac/registry/o5-donor-delete-gate.yaml
require_file vac-rs/crates/foundation/runtime-protocol/src/lib.rs
require_file vac-rs/crates/foundation/runtime-protocol/src/bin/export.rs
require_file vac-rs/crates/foundation/runtime-protocol/src/bin/write_schema_fixtures.rs

for dir in vac-rs/app-server vac-rs/app-server-client vac-rs/app-server-protocol vac-rs/app-server-transport; do
  [[ ! -e "$dir" ]] || fail "retired app-server crate still exists: $dir"
done

if find vac-rs -name legacy_include.rs -print -quit | grep -q .; then
  find vac-rs -name legacy_include.rs -print >&2
  fail "legacy_include.rs still present"
fi

require_grep 'vac-runtime-protocol-export' vac-rs/crates/foundation/runtime-protocol/Cargo.toml
require_grep 'vac-runtime-protocol-write-schema-fixtures' vac-rs/crates/foundation/runtime-protocol/Cargo.toml

if grep -q 'vac-app-server' vac-rs/Cargo.toml vac-rs/crates/surfaces/tui/Cargo.toml; then
  grep -n 'vac-app-server' vac-rs/Cargo.toml vac-rs/crates/surfaces/tui/Cargo.toml >&2
  fail "workspace/TUI manifests still contain app-server dependencies"
fi

require_grep 'provider_network_retained: true' .vac/registry/o5-o6-monolith-quality-state.yaml
require_grep 'all_audit_remainder_2026_05_30:' .vac/registry/o5-o6-monolith-quality-state.yaml
if [[ "${VAC_O6_1_RESCAN:-1}" = "1" ]]; then
  bash scripts/check-vac-o6-1-depanic-surface.sh >/dev/null || exit 1
else
  require_grep 'cfg_test_lines_removed:' .vac/registry/o6-panic-surface.yaml
  require_grep 'scanner_output_sha256:' .vac/registry/o6-panic-surface.yaml
  printf 'O6.1 de-panic surface: SKIP_RESCAN aggregate mode; dedicated gate must run separately\n'
fi
require_grep 'decision: full_donor_source_tree_deleted_sv_done_tv_pending' .vac/registry/o5-donor-delete-gate.yaml
require_grep 'TV-Pending' .vac/registry/o5-o6-monolith-quality-state.yaml

printf 'vac O5/O6 monolith-quality slice: PASS all audit remainder state verified; cargo TV-Pending\n'
