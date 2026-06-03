#!/usr/bin/env bash
# O5.1c/O5.5 successor gate: app-server protocol consumers migrated, then
# legacy app-server compatibility crates retired from the product source tree.
# Uses set -u only; grep returning 1 on zero matches must NOT abort the gate.
set -u
cd "$(dirname "$0")/.."
fail=0

residual=$(grep -rn 'vac_app_server_protocol::' vac-rs --include='*.rs' 2>/dev/null | wc -l | tr -d ' ')
if [ "$residual" != "0" ]; then
  echo "FAIL gate1: $residual residual vac_app_server_protocol path imports"; fail=1
else
  echo "PASS gate1: 0 residual vac_app_server_protocol path imports"
fi

barealias=$(grep -rn 'use vac_app_server_protocol' vac-rs --include='*.rs' 2>/dev/null | wc -l | tr -d ' ')
if [ "$barealias" != "0" ]; then
  echo "FAIL gate2: $barealias residual vac_app_server_protocol alias import(s)"; fail=1
else
  echo "PASS gate2: 0 residual vac_app_server_protocol alias imports"
fi

for dir in vac-rs/app-server vac-rs/app-server-client vac-rs/app-server-protocol vac-rs/app-server-transport; do
  if [ -e "$dir" ]; then
    echo "FAIL gate3: retired app-server crate still exists: $dir"; fail=1
  else
    echo "PASS gate3: retired app-server crate absent: $dir"
  fi
done

if [ -f vac-rs/runtime-protocol/src/bin/export.rs ] && [ -f vac-rs/runtime-protocol/src/bin/write_schema_fixtures.rs ]; then
  echo "PASS gate4: runtime-protocol schema bins own former protocol export/write fixtures"
else
  echo "FAIL gate4: runtime-protocol schema bins missing"; fail=1
fi

if grep -q 'vac-app-server' vac-rs/Cargo.toml vac-rs/tui/Cargo.toml 2>/dev/null; then
  echo "FAIL gate5: workspace/TUI manifests still mention vac-app-server"; fail=1
else
  echo "PASS gate5: workspace/TUI manifests have no vac-app-server dependency"
fi

echo "TV-PENDING: workspace build/lint/test = NotEvaluated (no stable compiler/vendor gate in this artifact)"
exit $fail
