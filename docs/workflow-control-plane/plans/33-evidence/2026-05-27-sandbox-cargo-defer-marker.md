# Plan 33 Sandbox Evidence — Cargo delete/defer marker

Date: 2026-05-27

## Slice

This slice records a defer-proof improvement for app-server Cargo retirement. It does not remove app-server crates from `vac-rs/tui/Cargo.toml`.

## Evidence

The remaining TUI app-server source seam is now explicitly marked as:

```text
VAC_RUNTIME_OWNER_COMPAT_DEFER: plan31-app-server-compat-quarantine
```

The runtime-owner gate reports the marked files as `app_server_compat_defer_present`, while Cargo manifest edges still report as `app_server_dependency_present` until deletion is safe.

## Decision

Plan 33 remains blocked for dependency deletion. The correct next step is symbol-by-symbol DTO replacement or non-default feature quarantine; deleting `vac-app-server*` dependencies now would be name-only cleanup and would break the current runtime path.
