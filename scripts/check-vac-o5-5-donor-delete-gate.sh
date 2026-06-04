#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
for dir in donor vac-rs/app-server vac-rs/app-server-client vac-rs/app-server-protocol vac-rs/app-server-transport; do
  [[ ! -e "$dir" ]] || fail "retired donor/app-server path still exists: $dir"
done
[[ -f vac-rs/crates/foundation/runtime-protocol/src/bin/export.rs ]] || fail "missing runtime-protocol schema export bin"
[[ -f vac-rs/crates/foundation/runtime-protocol/src/bin/write_schema_fixtures.rs ]] || fail "missing runtime-protocol schema write fixtures bin"
grep -q 'decision: full_donor_source_tree_deleted_sv_done_tv_pending' .vac/registry/o5-donor-delete-gate.yaml || fail "donor gate state not updated"
printf 'O5.5 donor delete gate: PASS donor source tree and app-server scope deleted; cargo TV-Pending\n'
