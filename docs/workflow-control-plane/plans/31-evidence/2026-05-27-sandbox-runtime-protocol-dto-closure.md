# Plan 31D Evidence — Runtime Protocol DTO Closure

Date: 2026-05-27
Environment: ChatGPT sandbox

## Result

The default TUI session DTO facade now imports remaining wire DTOs from `vac-runtime-protocol`. The TUI default path no longer imports or depends on `vac-app-server-protocol`.

## Implementation evidence

- `vac-rs/tui/src/session_protocol_compat.rs` re-exports its explicit DTO list from `vac_runtime_protocol`.
- `vac-rs/tui/Cargo.toml` includes `vac-runtime-protocol = { workspace = true }`.
- `vac-rs/tui/Cargo.toml` no longer declares direct `vac-app-server` or `vac-app-server-protocol` dependencies.
- `legacy-app-server-compat` keeps only the optional `vac-app-server-client` transport seam for non-default rollback/defer evidence.

## Remaining compatibility

Some public TUI identifiers still use `AppServer*` names for behavior-preserving compatibility with existing call sites. Those names now resolve to owner-native/default-path DTOs or request handles and must not be interpreted as app-server crate ownership.
