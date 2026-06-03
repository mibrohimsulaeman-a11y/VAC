# VAC-Init Registry Validator Gate

The registry validator gate covers envelope loading, kind allowlist, duplicate ID detection, and minimum manifest readiness diagnostics.

Required script:

```bash
bash scripts/check-vac-init-registry-validator-contract.sh
```

The script directly compiles `vac_init_registry_validator.rs` with `rustc --test` and scans `.vac` for duplicate manifest IDs using Python YAML parsing.
