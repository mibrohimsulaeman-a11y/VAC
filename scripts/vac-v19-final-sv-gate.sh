#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
run() { local label="$1"; shift; printf '\n== %s ==\n' "$label"; timeout 120 "$@"; }
run_long() { local label="$1"; shift; printf '\n== %s ==\n' "$label"; timeout "${VAC_GATE_LONG_STEP_TIMEOUT_SECONDS:-7200}" "$@"; }
if [[ "${VAC_CARGO_TV_CONSUME_PROOF:-0}" == "1" ]]; then
  run cargo-tv python3 scripts/check-cargo-tv.py . --summary-only
else
  run_long cargo-tv python3 scripts/check-cargo-tv.py .
fi
run deterministic-index python3 scripts/generate-deterministic-index-sv.py .
run rust-ast-index-coverage python3 scripts/check-rust-ast-index-coverage.py .
run compile-registry python3 scripts/compile-vac-registry-sv.py .
run assessment-report python3 scripts/generate-assessment-report-sv.py .
run generate-spec-sync python3 scripts/generate-spec-sync-report-sv.py .
run refresh-evidence-logs python3 scripts/refresh-evidence-logs-sv.py .
run assessment-freshness python3 scripts/check-assessment-freshness.py .
run evidence-log-freshness python3 scripts/check-evidence-log-freshness.py .
run runtime-db-schema python3 scripts/check-v19-runtime-db-schema.py .
run runtime-journal-writer python3 scripts/check-v19-runtime-journal-writer.py .
run fixture-coverage python3 scripts/check-vac-v19-fixture-coverage.py .
run tui-lifecycle-static python3 scripts/check-tui-lifecycle-e2e-static.py .
run tui-e2e-coverage python3 scripts/check-vac-tui-e2e-coverage.py .
run external-provider-remote-process-io python3 scripts/check-external-provider-remote-process-io-e2e.py .
run confirmed-intent-coverage python3 scripts/check-confirmed-intent-coverage.py .
run confirmed-intent-negative-fixtures python3 scripts/check-confirmed-intent-negative-fixtures.py .
run confirmed-intent-status python3 scripts/refresh-confirmed-intent-status.py .
run rust-ast-index-status python3 scripts/refresh-rust-ast-index-status.py .
run storage-classes python3 scripts/check-v19-storage-classes.py .
run ci-workdir python3 scripts/check-ci-workdir-v19.py .
run clippy-debt-strategy python3 scripts/check-clippy-debt-strategy.py .
run assessment-closure python3 scripts/check-vac-assessment-closure.py .
run sv-static python3 scripts/sv_static_validate.py .
run sv-deep python3 scripts/vac-sv-deep-validate.py .
if [[ "${VAC_GENERATE_ROOT_CHECKPOINT_MANIFEST:-0}" == "1" ]]; then
  run generate-checkpoint-manifest python3 scripts/generate-checkpoint-manifest.py . vac-runtime-v19-storage-cleanup-source-clean
else
  printf '\n== generate-checkpoint-manifest ==\n'
  printf 'source_workspace_mode=skip_root_checkpoint_manifest\n'
fi
run checkpoint-integrity python3 scripts/check-checkpoint-integrity.py .
run runtime-state4-adversarial python3 scripts/vac-runtime-state4-adversarial-sv.py .
run runtime-state5-operational python3 scripts/vac-runtime-state5-operational-sv.py .
run runtime-state6-semantics python3 scripts/vac-runtime-state6-semantics-sv.py .
run runtime-state7 python3 scripts/vac-runtime-state7-merged-audit-sv.py .
printf '\nVAC v1.9 final SV gate: PASS\n'
python3 scripts/check-cargo-tv.py . --summary-only
printf 'l2_broker=NotImplemented\n'
