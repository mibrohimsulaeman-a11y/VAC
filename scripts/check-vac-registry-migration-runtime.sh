#!/usr/bin/env bash
# REQUIRES_TOOLCHAIN: standalone rustc --test compile
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
RUSTC_BIN="${RUSTC:-rustc}"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-migration-runtime.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

require_file() { [[ -f "$1" ]] || { echo "FAIL: missing $1" >&2; exit 1; }; }

require_file vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_migration_runtime.rs
require_file vac-rs/crates/surfaces/cli/src/registry_cli.rs
require_file .vac/registry/migrations/migration.v1-hardening-h.yaml

"$RUSTC_BIN" --edition 2024 --test vac-rs/crates/control-plane/control-plane/src/control_plane/vac_init_migration_runtime.rs -o "$TMPROOT/vac_init_migration_runtime_test"
"$TMPROOT/vac_init_migration_runtime_test" --nocapture

grep -q 'RegistryCommand' vac-rs/crates/surfaces/cli/src/registry_cli.rs
grep -q 'Migrate' vac-rs/crates/surfaces/cli/src/registry_cli.rs
grep -q -- '--dry-run' vac-rs/crates/surfaces/cli/src/registry_cli.rs
grep -q -- '--apply' vac-rs/crates/surfaces/cli/src/registry_cli.rs
grep -q 'Migrations(MigrationDoctorCommand)' vac-rs/crates/surfaces/cli/src/doctor_cli.rs
grep -q 'kind: registry_status' .vac/registry/product.yaml
grep -q 'legacy_kind: product' .vac/registry/product.yaml
grep -q 'kind: registry_status' .vac/registry/status.yaml
grep -q 'legacy_kind: status' .vac/registry/status.yaml
grep -q 'kind: registry_status' .vac/registry/donor-inventory.yaml
grep -q 'legacy_kind: donor_inventory' .vac/registry/donor-inventory.yaml

printf 'vac registry migration runtime gate: PASS\n'
