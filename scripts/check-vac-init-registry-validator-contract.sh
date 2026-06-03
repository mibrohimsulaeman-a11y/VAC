#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-registry.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
TEST_BIN="$TMPROOT/vac_init_registry_validator_test"
require_file() { [[ -f "$1" ]] || { echo "missing required file: $1" >&2; exit 1; }; }
require_grep() { grep -qE "$1" "$2" || { echo "missing pattern in $2: $1" >&2; exit 1; }; }
require_file vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_grep 'pub struct RegistryManifestRecord' vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_grep 'pub struct RegistryValidationReport' vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_grep 'validate_registry_records' vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_grep 'validate_registry_yaml_documents' vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_grep 'duplicate manifest id' vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_grep 'ready capability requires' vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs
require_grep 'pub mod vac_init_registry_validator;' vac-rs/control-plane/src/control_plane/mod.rs
require_file .vac/capabilities/vac-init-registry-validator.yaml
require_file .vac/workflows/maintenance.vac-init-registry-validator.yaml
require_grep 'vac\.init\.registry-validator' .vac/surfaces/cli.yaml
if command -v "$RUSTC_BIN" >/dev/null 2>&1; then
  "$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/vac_init_registry_validator.rs -o "$TEST_BIN"
  "$TEST_BIN" --nocapture
else
  echo "registry validator rustc unit gate: NotEvaluated (rustc not found: $RUSTC_BIN)" >&2
fi
python3 - <<'PY'
import pathlib, yaml
count=0
ids={}
# Generated init risk-finding shards can be multi-megabyte scanner artifacts.
# The strict registry gate already validates source-controlled manifests and
# scanner-owned risk payload shape is checked by scanner gates; skipping these
# payloads keeps the registry validator deterministic in sandbox/no-rustc runs.
paths = sorted(
    path
    for path in pathlib.Path('.vac').rglob('*.yaml')
    if not str(path).startswith('.vac/.init/risk_findings/')
    and not str(path).startswith('.vac/.init/source_inventory')
)
for path in paths:
    data=yaml.safe_load(path.read_text(encoding='utf-8'))
    if not isinstance(data, dict):
        raise SystemExit(f'{path}: root is not mapping')
    for field in ('schema_version','kind','id'):
        if field not in data:
            raise SystemExit(f'{path}: missing {field}')
    mid=str(data['id'])
    if mid in ids:
        raise SystemExit(f'duplicate id {mid}: {ids[mid]} and {path}')
    ids[mid]=path
    count+=1
print(f'.vac registry validator preflight OK: {count} manifests')
PY
printf 'vac-init registry validator static contract: PASS (rustc unit may be NotEvaluated)\n'
