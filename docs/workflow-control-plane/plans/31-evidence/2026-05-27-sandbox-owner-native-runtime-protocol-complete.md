# Plan 31 — Owner-native runtime protocol completion evidence

Status: complete for the default TUI DTO owner path.

## What changed

- Added `vac-rs/runtime-protocol` as the product/runtime protocol owner crate.
- `vac-rs/tui/src/session_protocol_compat.rs` now imports `vac_runtime_protocol`, not `vac_app_server_protocol`.
- `vac-rs/tui/src/session_protocol.rs` remains the single TUI facade and keeps an explicit symbol list instead of wildcarding app-server DTOs.
- `vac-rs/tui/src/legacy_app_server_session.rs` and `legacy_app_server_compat.rs` are non-default compatibility quarantine behind `legacy-app-server-compat`.

## Default-path proof

- `vac-rs/tui/Cargo.toml` has `default = []`.
- `vac-app-server-client` is optional-only.
- `vac-app-server-protocol` is no longer a direct `vac-tui` dependency.
- Direct `vac_app_server_protocol` and `vac_app_server_client` imports are absent from the default TUI facade.

## Validation used in sandbox

```bash
rustfmt --edition 2024 --check \
  vac-rs/tui/src/session_protocol.rs \
  vac-rs/tui/src/session_protocol_compat.rs \
  vac-rs/tui/src/app_server_session.rs \
  vac-rs/tui/src/legacy_app_server_session.rs
python3 /mnt/data/vac-output/plan31_33_34_static_checks.py
```

Full `cargo check -p vac-surface-tui` remains a heavier validation lane for a local machine because this sandbox has repeatedly reset during large Cargo linking/graph work.
