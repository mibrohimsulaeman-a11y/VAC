# VAC-Init Fixture Matrix and Security Regression

Status: implemented in sandbox production RC batch.

## Scope

G1-G3 adds fixture matrix coverage, registers selected cargo gate commands, and defines security regression cases for shell/path/approval/evidence/memory/ownership attacks.

## Code-backed contract

```text
vac-rs/core/src/control_plane/vac_init_fixture_security_regression.rs
```

## Gate

```bash
bash scripts/check-vac-init-fixtures-security-regression-contract.sh
```

The gate uses targeted `rustc --test` and YAML/static checks. Full workspace build is not a required sandbox gate.
