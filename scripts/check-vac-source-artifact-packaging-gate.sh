#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-.}"
ZIP_PATH="${VAC_SOURCE_ARTIFACT_ZIP:-}"
fail() { echo "source artifact packaging gate: $*" >&2; exit 1; }
if [[ -n "$ZIP_PATH" && -f "$ZIP_PATH" ]]; then
  unzip -l "$ZIP_PATH" | awk '{print $4}' | grep -E '(^|/)(target|\.git|node_modules)/' && fail "zip includes excluded build/cache path"
  unzip -tq "$ZIP_PATH" >/dev/null || fail "zip integrity failed"
else
  echo "source artifact packaging gate: no ZIP supplied; validating source metadata only"
fi
for required in LICENSE NOTICE THIRD_PARTY_NOTICES.md; do
  [[ -f "$ROOT/$required" ]] || fail "missing required release file: $required"
done
[[ -d "$ROOT/.vac/registry/evidence" ]] || fail "missing evidence registry"
[[ -f "$ROOT/.vac/registry/compile-debt-ledger.yaml" ]] || fail "missing compile debt ledger"
echo "source artifact packaging gate: PASS"
