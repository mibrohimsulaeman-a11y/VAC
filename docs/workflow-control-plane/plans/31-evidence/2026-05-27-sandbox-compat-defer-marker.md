# Plan 31C Sandbox Evidence — Compatibility defer marker

Date: 2026-05-27

## Slice

This slice does not delete the app-server DTO seam yet. It narrows the active Plan 31 blocker by making the remaining seam machine-readable as an explicit quarantine/defer boundary.

## Code changes

- `vac-rs/tui/src/session_protocol.rs` now carries `VAC_RUNTIME_OWNER_COMPAT_DEFER: plan31-app-server-compat-quarantine` at the DTO facade boundary.
- `vac-rs/tui/src/app_server_session.rs` now carries the same marker at the deferred compatibility adapter boundary.
- `vac doctor runtime-owner-gates` now collapses marked files into `app_server_compat_defer_present` diagnostics instead of producing noisy per-line import warnings.
- `.vac/capabilities/tui_session_runtime.yaml` and `.vac/capabilities/runtime_approval_bridge.yaml` record the same quarantine marker in their `compatibility_transport` entries.

## Meaning

This is replacement/defer proof, not no-app-server green proof. Plan 31C remains incomplete until active DTOs are owner-native or the compatibility file is moved fully behind a non-default feature/path.

## Remaining blocker

- `vac-rs/tui/src/session_protocol.rs` still exports broad app-server protocol DTO compatibility.
- `vac-rs/tui/src/app_server_session.rs` still backs the current TUI runtime adapter.
- Plan 33 cannot remove Cargo edges until the default local product path stops compiling these references.
