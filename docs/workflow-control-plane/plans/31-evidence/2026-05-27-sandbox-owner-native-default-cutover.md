# Plan 31C — Sandbox owner-native default cutover evidence

Status: implemented for the default TUI source boundary.

## What changed

- `vac-rs/tui/src/session_protocol.rs` no longer names `vac_app_server_protocol` directly. It imports the remaining wire DTO names through the explicitly quarantined `session_protocol_compat.rs` seam.
- `vac-rs/tui/src/app_server_session.rs` no longer names `vac_app_server_client` or `vac_app_server_protocol` directly. It imports them through `legacy_app_server_compat`, making the old bridge a feature-scoped compatibility adapter rather than the active DTO owner.
- `vac-rs/tui/src/legacy_app_server_compat.rs` is the only TUI module allowed to name app-server crates.
- `vac-rs/tui/src/session_protocol_compat.rs` imports `vac-runtime-protocol`, so the default DTO shim is owner-native and no longer app-server-protocol-owned.

## Default-path rule

The default TUI Cargo feature set is now empty. Only `vac-app-server-client` remains optional through `legacy-app-server-compat` for historical adapter work; app-server protocol ownership moved to `vac-runtime-protocol`. The default DTO owner is `vac-runtime-protocol`.

## Evidence commands

```bash
rg -n "vac_app_server|vac-app-server" vac-rs/tui/src vac-rs/tui/Cargo.toml
python3 /mnt/data/vac-output/plan31_33_34_static_checks.py
```

## Remaining non-default seam

The legacy app-server session adapter is intentionally not deleted in this slice. It remains available only as an explicitly named non-default legacy feature while Plan 33 records Cargo delete/defer proof for the wider workspace.
