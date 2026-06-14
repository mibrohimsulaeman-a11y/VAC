#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
run() { local label="$1"; shift; printf '\n== %s ==\n' "$label"; timeout 120 "$@"; }
run deterministic-index python3 scripts/generate-deterministic-index-sv.py .
run compile-registry python3 scripts/compile-vac-registry-sv.py .
run assessment-report python3 scripts/generate-assessment-report-sv.py .
run generate-spec-sync python3 scripts/generate-spec-sync-report-sv.py .
run refresh-evidence-logs python3 scripts/refresh-evidence-logs-sv.py .
run assessment-freshness python3 scripts/check-assessment-freshness.py .
run evidence-log-freshness python3 scripts/check-evidence-log-freshness.py .
run runtime-db-schema python3 scripts/check-v19-runtime-db-schema.py .
run fixture-coverage python3 scripts/check-vac-v19-fixture-coverage.py .
run tui-lifecycle-static python3 scripts/check-tui-lifecycle-e2e-static.py .
run tui-e2e-coverage python3 scripts/check-vac-tui-e2e-coverage.py .
run storage-classes python3 scripts/check-v19-storage-classes.py .
run ci-workdir python3 scripts/check-ci-workdir-v19.py .
run clippy-debt-strategy python3 scripts/check-clippy-debt-strategy.py .
run sv-static python3 scripts/sv_static_validate.py .
run sv-deep python3 scripts/vac-sv-deep-validate.py .
if [[ "${VAC_GENERATE_ROOT_CHECKPOINT_MANIFEST:-0}" == "1" ]]; then
  run generate-checkpoint-manifest python3 scripts/generate-checkpoint-manifest.py . vac-runtime-v19-storage-cleanup-source-clean
else
  printf '\n== generate-checkpoint-manifest ==\n'
  printf 'source_workspace_mode=skip_root_checkpoint_manifest\n'
fi
run checkpoint-integrity python3 scripts/check-checkpoint-integrity.py .
run runtime-state7 python3 scripts/vac-runtime-state7-merged-audit-sv.py .
printf '\nVAC v1.9 final SV gate: PASS\n'
printf 'cargo_tv=NotEvaluated\n'
printf 'l2_broker=NotImplemented\n'
