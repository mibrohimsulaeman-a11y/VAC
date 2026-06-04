#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/production-rc.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path" >&2
    exit 1
  fi
}

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_production_rc.rs
require_file scripts/check-vac-init-live-scanner-policy.sh
require_file scripts/check-vac-tui-real-data-adapters.sh
require_file scripts/check-vac-evidence-why-live-index.sh
require_file scripts/check-vac-registry-migration-runtime.sh
require_file scripts/check-vac-minimal-e2e-dry-run.sh

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_production_rc.rs -o "$TMPROOT/vac_init_production_rc_test"
"$TMPROOT/vac_init_production_rc_test" --nocapture

bash scripts/check-no-hardcoded-readiness-scoreboard.sh
bash scripts/check-vac-workflow-spec-compliance.sh
bash scripts/check-vac-minimal-e2e-dry-run.sh

# Confirm P5-P9 gates are wired into manifests; component scripts are executed
# individually in the artifact validation log to avoid one long aggregate step.
for script in \
  scripts/check-vac-init-live-scanner-policy.sh \
  scripts/check-vac-tui-real-data-adapters.sh \
  scripts/check-vac-evidence-why-live-index.sh \
  scripts/check-vac-registry-migration-runtime.sh \
  scripts/check-vac-minimal-e2e-dry-run.sh; do
  if ! grep -R "$script" .vac/capabilities .vac/workflows >/dev/null; then
    echo "FAIL: $script is not wired into .vac manifests" >&2
    exit 1
  fi
done

printf 'vac-init production rc contract: PASS\n'
