#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail=0

bash scripts/check-no-app-server-local-path-contract.sh || fail=1

if grep -q 'app_server_retirement:' .vac/registry/o5-o6-monolith-quality-state.yaml \
  && grep -q 'provider_network_retained: true' .vac/registry/o5-o6-monolith-quality-state.yaml; then
  echo "PASS: O5/O6 state records in-process-local app-server retirement and provider retention"
else
  echo "FAIL: O5/O6 state does not record app-server retirement/provider retention" >&2
  fail=1
fi

if grep -q 'local_semantics: in_process_without_separate_app_server_process' .vac/registry/o5-o6-monolith-quality-state.yaml \
  && grep -q 'separate_local_runtime_scope: archived_not_requested' .vac/registry/o5-o6-monolith-quality-state.yaml; then
  echo "PASS: state documents local=in-process and archives separate-runtime scope"
else
  echo "FAIL: state missing local/separate-runtime scope separation" >&2
  fail=1
fi

exit "$fail"
