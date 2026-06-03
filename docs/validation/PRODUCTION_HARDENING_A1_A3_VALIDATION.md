# Production Hardening A1-A3 Validation

## Validation commands

```bash
python3 - <<'PY'
import pathlib, yaml
for path in pathlib.Path('.vac').rglob('*.yaml'):
    yaml.safe_load(path.read_text())
print('yaml parse: PASS')
PY

bash scripts/check-vac-init-registry-strictness-contract.sh
rustfmt --check vac-rs/core/src/control_plane/vac_init_registry_strictness.rs vac-rs/core/src/control_plane/vac_init_registry_validator.rs vac-rs/core/src/control_plane/mod.rs
RUSTC=<toolchain>/bin/rustc bash scripts/check-vac-init-registry-strictness-contract.sh
```

## Expected result

- `.vac` YAML parse succeeds.
- Compatibility kinds are absent from active `kind` fields.
- Free-form `validation.commands` count is zero.
- Ready capability missing-field count is zero.
- Strictness Rust tests pass.
- Source artifact excludes `target/`, `.git/`, dependency cache, and compiled outputs.

## Known boundary

This validation does not require a full workspace Cargo build. That remains an operator-gated release activity in later hardening phases.
