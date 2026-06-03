#!/usr/bin/env bash
# O6.2 source-runtime SAFETY gate.
# This gate is intentionally rustc-free and does not infer TV/cargo status.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! python3 scripts/measure-vac-o6-2-runtime-safety-coverage.py --check-registry .vac/registry/o6-safety-coverage.yaml; then
  exit 1
fi

if ! grep -q 'coverage_status: SV-Done' .vac/registry/o6-safety-coverage.yaml; then
  echo 'missing coverage_status: SV-Done in .vac/registry/o6-safety-coverage.yaml' >&2
  exit 1
fi
if ! grep -q 'tv_status: TV-Pending' .vac/registry/o6-safety-coverage.yaml; then
  echo 'missing tv_status: TV-Pending in .vac/registry/o6-safety-coverage.yaml' >&2
  exit 1
fi
if ! grep -q 'source_runtime:' .vac/registry/o6-safety-coverage.yaml; then
  echo 'missing source_runtime coverage block' >&2
  exit 1
fi
if ! grep -q 'TV-Pending' .vac/registry/compile-debt-ledger.yaml; then
  echo 'compile-debt-ledger must keep cargo/geiger verification TV-Pending' >&2
  exit 1
fi

printf 'O6.2 SAFETY coverage: PASS source-runtime static gate\n'
