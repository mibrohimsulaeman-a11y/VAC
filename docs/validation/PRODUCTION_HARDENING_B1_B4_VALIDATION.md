# Production Hardening B1-B4 Validation

## Required targeted gates

```bash
bash scripts/check-vac-init-cli-runtime-foundation-contract.sh
rustfmt --check \
  vac-rs/core/src/control_plane/vac_init_cli_runtime.rs \
  vac-rs/cli/src/init_cli.rs \
  vac-rs/cli/src/plan_cli.rs \
  vac-rs/cli/src/why_cli.rs \
  vac-rs/cli/src/doctor_cli.rs \
  vac-rs/cli/src/main.rs
```

## Expected results

- CLI source declares `init_cli`, `plan_cli`, and `why_cli`.
- Top-level clap surface includes `init`, `plan`, and `why`.
- Doctor taxonomy includes `evidence`, `memory`, and `init`.
- `.vac/surfaces/cli.yaml` declares `vac init`, `vac plan`, `vac plan validate`, and `vac why`.
- `.vac/.init/state.yaml` exists and uses `kind: init_state`.
- Safe trajectory index exists and excludes raw chain-of-thought.
- Full workspace build remains intentionally out of scope for this batch.
