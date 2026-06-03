# VAC-Init Scanner and Policy Inference Hardening

Status: implemented in sandbox production RC batch.

## Scope

E1-E3 hardens source classification, generated/vendor/build exclusions, risk confidence thresholds, Rust risk pattern detection, and fail-closed policy inference.

## Code-backed contract

```text
vac-rs/core/src/control_plane/vac_init_scanner_policy_hardening.rs
```

## Gate

```bash
bash scripts/check-vac-init-scanner-policy-hardening-contract.sh
```

The gate uses targeted `rustc --test` and YAML/static checks. Full workspace build is not a required sandbox gate.
