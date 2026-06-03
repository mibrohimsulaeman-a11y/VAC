# VAC-Init Durable Stores

Status: implemented in sandbox production RC batch.

## Scope

D1-D3 durable stores define atomic writes, schema envelopes, path normalization, approval replay protection, and store locations for init/source/ownership/risk/approval/trajectory/memory records.

## Code-backed contract

```text
vac-rs/core/src/control_plane/vac_init_durable_stores.rs
```

## Gate

```bash
bash scripts/check-vac-init-durable-stores-contract.sh
```

The gate uses targeted `rustc --test` and YAML/static checks. Full workspace build is not a required sandbox gate.
