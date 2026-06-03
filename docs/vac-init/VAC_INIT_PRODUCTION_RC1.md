# VAC-Init Production RC

Status: implemented in sandbox production RC batch.

## Scope

I1-I3 aggregates release doctor pass criteria, E2E dry-run safety, production RC scoring, artifact discipline, and known limitations.

## Code-backed contract

```text
vac-rs/core/src/control_plane/vac_init_production_rc.rs
```

## Gate

```bash
bash scripts/check-vac-init-production-rc-contract.sh
```

The gate uses targeted `rustc --test` and YAML/static checks. Full workspace build is not a required sandbox gate.
