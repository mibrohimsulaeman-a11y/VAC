#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: aggregate invokes child gates with standalone rustc tests after static callsite checks
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

bash scripts/check-no-hardcoded-readiness-scoreboard.sh
bash scripts/check-vac-workflow-spec-compliance.sh
bash scripts/check-vac-doctor-release-real-reports.sh
bash scripts/check-vac-init-runtime-gate-callsite-integration.sh
bash scripts/check-vac-init-live-stores.sh

printf 'vac-init deterministic hardening p0-p4: PASS\n'
