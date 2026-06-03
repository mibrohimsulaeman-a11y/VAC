#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-init-schema.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
SCHEMA_TEST="$TMPROOT/schema_envelope_test"
KIND_TEST="$TMPROOT/kind_registry_test"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
}

require_grep() {
  local pattern="$1"
  local path="$2"
  if ! grep -qE "$pattern" "$path"; then
    echo "missing pattern in $path: $pattern" >&2
    exit 1
  fi
}

require_file vac-rs/control-plane/src/control_plane/schema_envelope.rs
require_file vac-rs/control-plane/src/control_plane/kind_registry.rs
require_file .vac/capabilities/vac-init-control-plane.yaml
require_file .vac/capabilities/vac-init-schema-envelope.yaml
require_file .vac/workflows/maintenance.vac-init-baseline-audit.yaml
require_file .vac/workflows/maintenance.vac-init-schema-envelope.yaml
require_file scripts/check-vac-init-baseline-contract.sh
require_file docs/vac-init/VAC_INIT_EXECUTION_PLAN.md
require_file docs/vac-init/VAC_INIT_IMPLEMENTATION_MAP.md
require_file docs/validation/VAC_INIT_BASELINE_AUDIT.md
require_file docs/validation/VAC_INIT_SCHEMA_ENVELOPE_GATE.md

require_grep 'pub struct SchemaEnvelope' vac-rs/control-plane/src/control_plane/schema_envelope.rs
require_grep 'pub struct SchemaEnvelopeFields' vac-rs/control-plane/src/control_plane/schema_envelope.rs
require_grep 'SUPPORTED_SCHEMA_VERSION: u32 = 1' vac-rs/control-plane/src/control_plane/schema_envelope.rs
require_grep 'parse_schema_envelope_from_yaml_str' vac-rs/control-plane/src/control_plane/schema_envelope.rs
require_grep 'validate_manifest_id' vac-rs/control-plane/src/control_plane/schema_envelope.rs
require_grep 'pub enum VacManifestKind' vac-rs/control-plane/src/control_plane/kind_registry.rs
require_grep 'KIND_REGISTRY' vac-rs/control-plane/src/control_plane/kind_registry.rs
require_grep 'CurrentWorkspaceCompatibility' vac-rs/control-plane/src/control_plane/kind_registry.rs
for kind in capability policy workflow workflow_step surface registry_status domains init_state evidence plan approval_request ownership_report memory_record risk_finding migration trajectory test_assertion product status donor_inventory; do
  require_grep "\"$kind\"" vac-rs/control-plane/src/control_plane/kind_registry.rs
done
require_grep 'pub mod schema_envelope;' vac-rs/control-plane/src/control_plane/mod.rs
require_grep 'pub mod kind_registry;' vac-rs/control-plane/src/control_plane/mod.rs
require_grep 'pub use schema_envelope::SchemaEnvelope;' vac-rs/control-plane/src/control_plane/mod.rs
require_grep 'pub use kind_registry::VacManifestKind;' vac-rs/control-plane/src/control_plane/mod.rs

require_grep '^schema_version: 1$' .vac/capabilities/vac-init-schema-envelope.yaml
require_grep '^kind: capability$' .vac/capabilities/vac-init-schema-envelope.yaml
require_grep '^id: vac\.init\.schema-envelope$' .vac/capabilities/vac-init-schema-envelope.yaml
require_grep 'bash scripts/check-vac-init-schema-envelope-contract.sh' .vac/capabilities/vac-init-schema-envelope.yaml
require_grep '^kind: workflow$' .vac/workflows/maintenance.vac-init-schema-envelope.yaml
require_grep '^id: maintenance\.vac-init-schema-envelope$' .vac/workflows/maintenance.vac-init-schema-envelope.yaml
require_grep 'vac\.init\.schema-envelope' .vac/surfaces/cli.yaml

"$RUSTC_BIN" --edition 2024 --cfg vac_standalone_schema_envelope --test vac-rs/control-plane/src/control_plane/schema_envelope.rs -o "$SCHEMA_TEST"
"$SCHEMA_TEST" --nocapture
"$RUSTC_BIN" --edition 2024 --test vac-rs/control-plane/src/control_plane/kind_registry.rs -o "$KIND_TEST"
"$KIND_TEST" --nocapture

python3 - <<'PY'
import pathlib
import re
import sys
import yaml

known = {
    'capability', 'policy', 'workflow', 'workflow_step', 'surface',
    'registry_status', 'domains', 'init_state', 'evidence', 'plan',
    'approval_request', 'ownership_report', 'memory_record', 'risk_finding',
    'migration', 'trajectory', 'test_assertion',
    # Current-workspace compatibility descriptors kept until registry migration.
    'product', 'status', 'donor_inventory',
}
identifier = re.compile(r'^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)+$')
count = 0
for path in sorted(pathlib.Path('.vac').rglob('*.yaml')):
    data = yaml.safe_load(path.read_text(encoding='utf-8'))
    if not isinstance(data, dict):
        raise SystemExit(f'{path}: root YAML must be a mapping')
    for field in ('schema_version', 'kind', 'id'):
        if field not in data:
            raise SystemExit(f'{path}: missing envelope field {field}')
    if data['schema_version'] != 1:
        raise SystemExit(f'{path}: unsupported schema_version {data["schema_version"]!r}')
    if data['kind'] not in known:
        raise SystemExit(f'{path}: unknown kind {data["kind"]!r}')
    manifest_id = str(data['id'])
    if manifest_id != 'vac' and not identifier.match(manifest_id):
        raise SystemExit(f'{path}: invalid manifest id {manifest_id!r}')
    count += 1
print(f'.vac envelope scan OK: {count} files')
PY

bash scripts/check-vac-init-baseline-contract.sh

if [[ -x scripts/check-tui-source-artifact-hygiene.sh ]]; then
  bash scripts/check-tui-source-artifact-hygiene.sh
fi

printf 'vac-init schema envelope contract: PASS\n'
