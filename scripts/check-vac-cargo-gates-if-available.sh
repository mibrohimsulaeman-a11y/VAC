#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

status="NotEvaluated"
reason="cargo_or_rustc_missing"
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
  if [[ -d vac-rs/vendor ]]; then
    cargo metadata --manifest-path vac-rs/Cargo.toml --offline --no-deps >/tmp/vac-cargo-metadata.json
    status="Pass"
    reason="cargo_metadata_offline_no_deps_passed"
  else
    status="NotEvaluated"
    reason="vac-rs/vendor_absent"
  fi
fi

printf 'cargo gate status: %s (%s)\n' "$status" "$reason"
if [[ "$status" == "Pass" ]]; then
  exit 0
fi

# NotEvaluated is an honest non-failure for sandbox artifacts. It must never be
# interpreted as build readiness.
exit 0
