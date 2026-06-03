#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail=0

retired_dirs=(
  vac-rs/app-server
  vac-rs/app-server-client
  vac-rs/app-server-protocol
  vac-rs/app-server-transport
)
for dir in "${retired_dirs[@]}"; do
  if [ -e "$dir" ]; then
    echo "FAIL: retired app-server crate path still exists: $dir" >&2
    fail=1
  else
    echo "PASS: retired app-server crate path absent: $dir"
  fi
done

workspace_refs=$(grep -nE '"app-server"|"app-server-client"|"app-server-protocol"|"app-server-transport"|vac-app-server|app_test_support' vac-rs/Cargo.toml | wc -l | tr -d ' ')
if [ "$workspace_refs" != "0" ]; then
  echo "FAIL: vac-rs/Cargo.toml still contains app-server workspace refs" >&2
  grep -nE '"app-server"|"app-server-client"|"app-server-protocol"|"app-server-transport"|vac-app-server|app_test_support' vac-rs/Cargo.toml >&2
  fail=1
else
  echo "PASS: workspace manifest has no app-server members/dependencies"
fi

tui_refs=$(grep -nE 'legacy-app-server-compat|vac-app-server|app_server_compat' vac-rs/tui/Cargo.toml | wc -l | tr -d ' ')
if [ "$tui_refs" != "0" ]; then
  echo "FAIL: TUI manifest still exposes app-server compatibility" >&2
  grep -nE 'legacy-app-server-compat|vac-app-server|app_server_compat' vac-rs/tui/Cargo.toml >&2
  fail=1
else
  echo "PASS: TUI manifest has no app-server compatibility feature/dependency"
fi

for legacy in \
  vac-rs/tui/src/legacy_app_server_compat.rs \
  vac-rs/tui/src/legacy_app_server_session.rs \
  vac-rs/tui/src/legacy_app_server_session \
  vac-rs/tui/src/app_server_session.rs; do
  if [ -e "$legacy" ]; then
    echo "FAIL: retired TUI compatibility path still exists: $legacy" >&2
    fail=1
  else
    echo "PASS: retired TUI compatibility path absent: $legacy"
  fi
done

for provider in 'vac-login = { workspace = true }' 'vac-model-provider = { workspace = true }' 'reqwest = { workspace = true'; do
  if grep -Fq "$provider" vac-rs/tui/Cargo.toml; then
    echo "PASS: provider/network capability retained: $provider"
  else
    echo "FAIL: provider/network capability missing from TUI manifest: $provider" >&2
    fail=1
  fi
done

if grep -Fq 'vac-provider-http = { path = "provider-http" }' vac-rs/Cargo.toml && [ -d vac-rs/provider-http ]; then
  echo "PASS: provider HTTP seam retained through vac-provider-http"
else
  echo "FAIL: provider HTTP seam missing after vac-chatgpt prune" >&2
  fail=1
fi

if [ -f vac-rs/runtime-protocol/src/bin/export.rs ] \
  && [ -f vac-rs/runtime-protocol/src/bin/write_schema_fixtures.rs ] \
  && grep -q 'vac-runtime-protocol-export' vac-rs/runtime-protocol/Cargo.toml \
  && grep -q 'vac-runtime-protocol-write-schema-fixtures' vac-rs/runtime-protocol/Cargo.toml; then
  echo "PASS: runtime-protocol owns relocated schema export binaries"
else
  echo "FAIL: runtime-protocol schema binaries are missing" >&2
  fail=1
fi

if [ -f vac-rs/tui/src/runtime_owner_session.rs ] \
  && grep -q 'mod runtime_owner_session;' vac-rs/tui/src/lib.rs \
  && grep -q 'crate::runtime_owner_session::' vac-rs/tui/src/local_runtime_session.rs; then
  echo "PASS: default TUI session boundary is runtime_owner_session"
else
  echo "FAIL: runtime_owner_session boundary not wired" >&2
  fail=1
fi

# Active source may retain generic app_server_events/app_server_requests naming, but it must not
# import the retired crates or expose the retired feature.
active_import_refs=$(grep -RInE 'use vac_app_server|use vac_app_server_client|vac_app_server::|vac_app_server_client::|legacy-app-server-compat' \
  vac-rs/tui/src vac-rs/local-runtime-owner/src vac-rs/cli/src vac-rs/core/src \
  --include='*.rs' | wc -l | tr -d ' ')
if [ "$active_import_refs" != "0" ]; then
  echo "FAIL: active runtime owner source still imports retired app-server crates/features" >&2
  grep -RInE 'use vac_app_server|use vac_app_server_client|vac_app_server::|vac_app_server_client::|legacy-app-server-compat' \
    vac-rs/tui/src vac-rs/local-runtime-owner/src vac-rs/cli/src vac-rs/core/src \
    --include='*.rs' >&2
  fail=1
else
  echo "PASS: active runtime owner source has no retired app-server imports/features"
fi

echo "TV-PENDING: cargo default build/tree verification = NotEvaluated until toolchain/vendor run"
exit "$fail"
