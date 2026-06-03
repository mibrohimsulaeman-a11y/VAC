# Plan 00F Legacy Transport Reachability Ledger — 2026-05-24

## Status

Decision: defer Cargo edge removal.

The TUI still has visible compatibility seams. Removing Cargo edges in this slice would be false-green because the product path still relies on the compatibility adapter while the local runtime owner migration continues in Plans 24–33.

## Current direct Cargo edges

`vac-rs/tui/Cargo.toml` still declares:

- `vac-app-server`
- `vac-app-server-client`
- `vac-app-server-protocol`

These are intentionally classified by `.vac/capabilities/tui.yaml` as `compatibility_transport` and remain under Plan 00F / 00E retirement control.

## Current direct source references

Observed references are concentrated in the compatibility boundary:

- `vac-rs/tui/src/app_server_session.rs` owns the app-server-backed adapter implementation.
- `vac-rs/tui/src/session_protocol.rs` owns the app-server-protocol compatibility facade.
- `vac-rs/tui/src/legacy_core.rs` documents why broad TUI call sites should not import the historical bridge directly.

## Retirement checklist

Do not mark the architecture warning as solved until all checklist items are true:

1. TUI startup, prompt submit, review/approval, hook notification, thread list, and resume-picker flows run through non-app-server local runtime owner APIs.
2. `session_protocol.rs` no longer exports app-server protocol wrapper aliases to regular TUI consumers.
3. `app_server_session.rs` is either deleted or retained only as a test fixture with explicit compatibility ownership.
4. Architecture doctor reports no legacy transport warning.
5. Donor gate remains green.

## Validation snapshot

Latest local validation before this ledger:

- `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture` passed with 20 tests.
- `cargo build --manifest-path vac-rs/Cargo.toml -p vac-surface-cli` passed.
- Architecture doctor exited 0 with expected Plan 00F warnings.
- Donor gate passed.

## Next safe Plan 00F slice

Move one remaining app-server protocol wrapper from `session_protocol.rs` to a shared owner type, add or adjust conversion at the adapter boundary, and validate with focused TUI tests plus a narrow build. Do not remove Cargo dependencies in that same slice.
