#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
run() {
  local label="$1"; shift
  printf '\n== %s ==\n' "$label"
  timeout 90 "$@"
}
run_long() {
  local label="$1"; shift
  printf '\n== %s ==\n' "$label"
  timeout "${VAC_GATE_LONG_STEP_TIMEOUT_SECONDS:-7200}" "$@"
}
if [[ "${VAC_CARGO_TV_CONSUME_PROOF:-0}" == "1" ]]; then
  run cargo-tv python3 scripts/check-cargo-tv.py . --summary-only
else
  run_long cargo-tv python3 scripts/check-cargo-tv.py .
fi
run compile-registry python3 scripts/compile-vac-registry-sv.py .
run deterministic-index python3 scripts/generate-deterministic-index-sv.py .
run assessment-report python3 scripts/generate-assessment-report-sv.py .
run sv-static python3 scripts/sv_static_validate.py .
run sv-deep python3 scripts/vac-sv-deep-validate.py "$ROOT"
run tui-mock-data python3 scripts/check-tui-hardcoded-mock-data.py "$ROOT"
run tui-canonical-renderer python3 scripts/check-tui-canonical-renderer.py "$ROOT"
run brand-allowlist python3 scripts/check-brand-allowlist.py "$ROOT"
run docs-current-state python3 scripts/check-docs-current-state.py
run runtime-agent-e2e python3 scripts/vac-runtime-agent-e2e-sv.py
run runtime-v15-e2e python3 scripts/vac-runtime-v15-e2e.py .
run runtime-audit-closure python3 scripts/vac-runtime-audit-closure-sv.py .
run runtime-realpath-e2e python3 scripts/vac-runtime-realpath-e2e.py .
run external-provider-remote-process-io python3 scripts/check-external-provider-remote-process-io-e2e.py .
run runtime-state4-adversarial python3 scripts/vac-runtime-state4-adversarial-sv.py .
run runtime-state5-operational python3 scripts/vac-runtime-state5-operational-sv.py .
run runtime-state6-semantics python3 scripts/vac-runtime-state6-semantics-sv.py .
run runtime-state7-merged-audit python3 scripts/vac-runtime-state7-merged-audit-sv.py .
run final-idempotence python3 scripts/vac-final-idempotence-sv.py .
run tui-e2e-coverage python3 scripts/check-vac-tui-e2e-coverage.py .
run confirmed-intent-coverage python3 scripts/check-confirmed-intent-coverage.py .
run confirmed-intent-negative-fixtures python3 scripts/check-confirmed-intent-negative-fixtures.py .
run confirmed-intent-status python3 scripts/refresh-confirmed-intent-status.py .
run rust-ast-index-status python3 scripts/refresh-rust-ast-index-status.py .
run refresh-evidence-logs python3 scripts/refresh-evidence-logs-sv.py .
run evidence-log-freshness python3 scripts/check-evidence-log-freshness.py .
run generate-checkpoint-manifest python3 scripts/generate-checkpoint-manifest.py . vac-runtime-v15-state7-merged-audit-closure-checkpoint
run assessment-freshness python3 scripts/check-assessment-freshness.py .
run checkpoint-integrity python3 scripts/check-checkpoint-integrity.py .
run py-compile python3 -m py_compile \
  scripts/compile-vac-registry-sv.py \
  scripts/generate-deterministic-index-sv.py \
  scripts/generate-assessment-report-sv.py \
  scripts/check-assessment-freshness.py \
  scripts/cargo_tv_status.py \
  scripts/check-cargo-tv.py \
  scripts/external_provider_remote_process_io_status.py \
  scripts/check-external-provider-remote-process-io-e2e.py \
  scripts/ci-external-provider-remote-process-io-proof.py \
  scripts/generate-checkpoint-manifest.py \
  scripts/sv_static_validate.py \
  scripts/vac-sv-deep-validate.py \
  scripts/check-tui-hardcoded-mock-data.py \
  scripts/check-tui-canonical-renderer.py \
  scripts/check-brand-allowlist.py \
  scripts/check-docs-current-state.py \
  scripts/check-checkpoint-integrity.py \
  scripts/vac-runtime-agent-e2e-sv.py \
  scripts/vac-runtime-v15-e2e.py \
  scripts/vac-runtime-audit-closure-sv.py \
  scripts/vac-runtime-realpath-e2e.py \
  scripts/vac-runtime-state4-adversarial-sv.py \
  scripts/vac-runtime-state5-operational-sv.py \
  scripts/vac-runtime-state6-semantics-sv.py \
  scripts/vac-runtime-state7-merged-audit-sv.py \
  scripts/vac-final-idempotence-sv.py \
  scripts/check-evidence-log-freshness.py
printf '\nVAC re-audit final SV gate: PASS\n'
python3 scripts/check-cargo-tv.py . --summary-only
printf 'l2_broker=NotImplemented\n'
