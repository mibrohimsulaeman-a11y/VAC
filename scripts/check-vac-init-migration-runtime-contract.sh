#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: optional cargo test for vac-core migration runtime unit tests
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [ -f "$1" ] || fail "missing required file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
SRC="vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_migration_runtime.rs"
require_file "$SRC"
require_file scripts/check-vac-init-migration-runtime-contract.sh
require_file .vac/capabilities/migration-runtime.yaml
require_file .vac/workflows/maintenance.migration-runtime.yaml
require_file .vac/registry/migrations/migration.v1-hardening-h.yaml
require_grep "ChangeType" "$SRC"
require_grep "VersionBump" "$SRC"
require_grep "build_migration_plan" "$SRC"
require_grep "build_rollback_plan" "$SRC"
require_grep "preview_migration" "$SRC"
require_grep "apply_migration_action_to_yaml_text" "$SRC"
require_grep "is_registry_doctor" "$SRC"
require_grep "VerificationMustRunRegistryDoctor" "$SRC"
require_grep "validate_migration_record_depth" vac-rs/crates/control-plane/control-plane/src/control_plane/mod.rs
python3 - <<'PY'
import pathlib, yaml, sys
for rel, legacy in [
    ('.vac/registry/product.yaml', 'product'),
    ('.vac/registry/status.yaml', 'status'),
    ('.vac/registry/donor-inventory.yaml', 'donor_inventory'),
]:
    data = yaml.safe_load(pathlib.Path(rel).read_text())
    assert data['kind'] == 'registry_status', f'{rel} not migrated to registry_status'
    assert data.get('legacy_kind') == legacy, f'{rel} missing legacy_kind {legacy}'
migration = yaml.safe_load(pathlib.Path('.vac/registry/migrations/migration.v1-hardening-h.yaml').read_text())
assert migration['kind'] == 'migration'
assert migration.get('changes'), 'migration requires changes'
assert migration.get('rollback'), 'migration requires rollback'
command = migration['verification']['command']
assert isinstance(command, dict), 'migration verification command must be structured'
for field in ('id', 'runner', 'args'):
    assert field in command, f'migration verification command missing {field}'
print('migration compatibility cleanup: PASS')
PY
if command -v cargo >/dev/null 2>&1; then
  (cd vac-rs && cargo test --offline -p vac-core vac_init_migration_runtime --lib)
  rc=$?
  [ "$rc" -eq 0 ] || exit "$rc"
  echo 'migration-runtime contract: PASS (cargo unit)'
else
  echo 'migration-runtime contract: PASS (static); cargo unit NotEvaluated (cargo not found)'
fi
