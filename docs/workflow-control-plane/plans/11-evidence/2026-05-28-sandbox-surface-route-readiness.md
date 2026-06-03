# Plan 11 evidence — sandbox surface route readiness closeout

Date: 2026-05-28

## Decision

The shared root surface registry is ready for the default product path. Route readiness here means manifest-backed visible route metadata is complete and drift-free; it does not execute every UI action.

## Implemented

- Promoted visible routes in `.vac/surfaces/cli.yaml`, `.vac/surfaces/slash.yaml`, `.vac/surfaces/palette.yaml`, and `.vac/surfaces/tui.yaml` to `ready`.
- Added `vac-rs/core/src/control_plane/surface_readiness.rs` as a side-effect-free scanner for visible route readiness.
- Wired `SurfaceDoctorReport` to render route readiness and fail if a visible route regresses to non-ready.

## Validation

- `rustfmt --edition 2024 --check vac-rs/core/src/control_plane/surface_readiness.rs vac-rs/core/src/control_plane/surface_doctor.rs vac-rs/core/src/control_plane/mod.rs`
- `rustc --edition 2024 --test vac-rs/core/src/control_plane/surface_readiness.rs -o /tmp/vac-surface-readiness-test && /tmp/vac-surface-readiness-test`
- YAML parse for `.vac/surfaces/*.yaml`
- Static route readiness sweep: no `partial`, `planned`, `unavailable`, or `cli_only` statuses remain in visible root surface manifests.
