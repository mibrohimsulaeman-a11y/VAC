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

if grep -q 'Epic A App-Server Retirement' docs/monolith-quality/O5O6_EPIC_A_APP_SERVER_RETIREMENT_REPORT.md \
  && grep -qi 'offline penuh' docs/monolith-quality/O5O6_EPIC_A_APP_SERVER_RETIREMENT_REPORT.md; then
  echo "PASS: report documents local=in-process and archives offline-full scope"
else
  echo "FAIL: report missing local/offline scope separation" >&2
  fail=1
fi

exit "$fail"
