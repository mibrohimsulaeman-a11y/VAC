# VAC-Init Batch 2-5 Validation Gate

This gate validates the second implementation checkpoint for the VAC-Init Control Plane foundation.

## Commands

```bash
df -h /mnt/data /tmp
rustfmt --edition 2024 --check \
  vac-rs/core/src/control_plane/vac_init_manifest_contract.rs \
  vac-rs/core/src/control_plane/vac_init_registry_validator.rs \
  vac-rs/core/src/control_plane/vac_init_lifecycle.rs \
  vac-rs/core/src/control_plane/vac_init_ownership_scanner.rs \
  vac-rs/core/src/control_plane/mod.rs

rustc --edition 2024 --test vac-rs/core/src/control_plane/vac_init_manifest_contract.rs
rustc --edition 2024 --test vac-rs/core/src/control_plane/vac_init_registry_validator.rs
rustc --edition 2024 --test vac-rs/core/src/control_plane/vac_init_lifecycle.rs
rustc --edition 2024 --test vac-rs/core/src/control_plane/vac_init_ownership_scanner.rs

bash scripts/check-vac-init-manifest-structs-contract.sh
bash scripts/check-vac-init-registry-validator-contract.sh
bash scripts/check-vac-init-lifecycle-contract.sh
bash scripts/check-vac-init-ownership-scanner-contract.sh
bash scripts/check-vac-init-batch2-5-contract.sh
python3 YAML parse .vac
source artifact hygiene
```

## Pass criteria

- All four Rust modules compile as isolated test harnesses.
- All test suites pass.
- All `.vac` manifests parse as YAML.
- All new capabilities/workflows/surface routes are registered.
- No `target/`, `.git/`, compiled object files, or build cache are packaged into the artifact.

## Known sandbox limitation

Full workspace build/link is not a required gate for this batch. The batch uses direct `rustc --test`, `rustfmt`, YAML parse, static contract scripts, and source hygiene.
