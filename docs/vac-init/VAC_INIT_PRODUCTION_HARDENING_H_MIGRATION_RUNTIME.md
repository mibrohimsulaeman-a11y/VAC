# VAC-Init Migration Runtime and Compatibility Cleanup

Status: implemented in sandbox production RC batch.

## Scope

H1-H2 defines migration records, rollback actions, dry-run/apply mode validation, structured verification commands, and active compatibility kind cleanup.

## Code-backed contract

```text
vac-rs/core/src/control_plane/vac_init_migration_runtime.rs
```

## Gate

```bash
bash scripts/check-vac-init-migration-runtime-contract.sh
```

The gate uses targeted `rustc --test` and YAML/static checks. Full workspace build is not a required sandbox gate.
