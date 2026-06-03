# Plan 11 Evidence — Surface route status sync

Date: 2026-05-28
Environment: ChatGPT sandbox source checkpoint

## Decision

Visible route statuses in `.vac/surfaces/{cli,palette,slash,tui}.yaml` now use `ready` consistently for the default product surface registry. In this context, `ready` means the route is manifest-backed, has a visible owner, maps to a ready capability, and participates in the surface doctor/registry model. It does not claim that optional legacy compatibility crates have been physically deleted.

## Scope

- CLI, slash, palette, and TUI visible routes were promoted from stale `partial`/`planned` route labels to `ready`.
- The palette owner for `open_tui_session_runtime` now points at `vac-rs/tui/src/owner_native_operation_parity.rs`, not the historical app-server session seam.
- Route status is aligned with the ready capability set produced by Plans 14, 16, 19, 20, 21, 22, 30, 31, 32, 33, and 34.

## Validation

- YAML parse for all touched surface manifests.
- Static check that every visible route status is `ready`.
- Static check that every visible route capability exists and is `ready`.
