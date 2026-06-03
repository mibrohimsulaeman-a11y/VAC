# Plan 16 build readiness closeout — 2026-05-28

Status: complete for the default targeted build capability.

## What changed

- `vac.build` is now `ready` instead of `partial`.
- `vac-rs/core/src/control_plane/build_readiness.rs` records a code-backed readiness report that verifies:
  - the build capability manifest is ready,
  - `maintenance.build-check` is ready and uses `capability.build.cargo_check`,
  - the build workflow requires `execute_process` approval,
  - `vac doctor build` is declared as a validation route,
  - the default command is the serial targeted check: `cargo +1.93.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli`,
  - full workspace build evidence is explicitly operator-gated instead of silently claimed by sandbox validation.

## Validation evidence

- `rustfmt --edition 2024 --check vac-rs/core/src/control_plane/build_readiness.rs`: pass.
- Targeted Rust harness for build readiness: pass.
- YAML parse of `.vac/capabilities/build.yaml` and `.vac/workflows/maintenance.build-check.yaml`: pass.

## Full workspace build policy

Full workspace build remains a release/operator evidence item because it is resource-heavy and environment-sensitive. This is not a Plan 16 implementation blocker because the default build capability now has an approval-gated targeted build command with structured timeout, exit-status, and diagnostic reporting.
