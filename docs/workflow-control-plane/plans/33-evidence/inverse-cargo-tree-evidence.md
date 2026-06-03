# Plan 33 inverse Cargo tree evidence — sandbox closeout 2026-05-28

| Field | Value |
|---|---|
| Capture run directory | `/mnt/data/vac-next-impl/vastar-agentic-cli` |
| Capture timestamp | 2026-05-28 sandbox run |
| Rust toolchain | Rust 1.93.0 bundle available; full Cargo tree in sandbox is not used as final gate because the uploaded vendor set previously required supplemental fixes and full workspace linking is resource-sensitive. |
| Plan 32 status reference | Runtime-owner hard gates complete for default path. |

## Evidence method

The sandbox closeout uses static default-feature manifest evidence instead of a full `cargo tree` run. The previous full workspace build attempt reached late compile/link stages but was unreliable in this sandbox. The default-path claim is therefore intentionally narrower and based on source truth:

- `vac-rs/tui/Cargo.toml` has `default = []`.
- `vac-rs/tui/Cargo.toml` does not declare direct default `vac-app-server` or `vac-app-server-protocol` dependencies.
- `vac-app-server-client` is optional and only activated by `legacy-app-server-compat`.
- `vac-rs/tui/src/legacy_app_server_compat.rs` is the only TUI source import of the optional app-server client.

## Required inverse-tree questions

| Target | Expected | Sandbox evidence | Result |
|---|---|---|---|
| `vac-app-server-client` from default `vac-cli` path | No default path | Optional TUI dependency only; default feature set empty | PASS for default-path claim |
| `vac-app-server` from default `vac-cli` path | No default path | No TUI default dependency; remaining crate is workspace historical | PASS for default-path claim |
| `vac-app-server-transport` from default `vac-cli` path | No default path | No TUI default dependency; remaining crate is workspace historical | PASS for default-path claim |

## Closeout answers

| Question | Answer |
|---|---|
| Is inverse tree evidence complete enough for physical deletion? | No. Physical deletion remains deferred until local/operator Cargo tree and workspace tests are run outside sandbox constraints. |
| Does evidence support default product-path dependency retirement? | Yes. Default TUI/Cargo path no longer enables app-server crates. |
| If not delete, what explicit defer classification applies? | Workspace crate deletion deferred; optional non-default compatibility and historical app-server test surfaces remain owned and documented. |
