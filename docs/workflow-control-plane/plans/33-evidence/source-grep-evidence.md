# Plan 33 source grep evidence — sandbox closeout 2026-05-28

| Field | Value |
|---|---|
| Capture run directory | `/mnt/data/vac-next-impl/vastar-agentic-cli` |
| Capture timestamp | 2026-05-28 sandbox run |
| Operator | ChatGPT sandbox agent |
| Plan 32 status reference | `docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md` complete for default hard gates |

## Commands

| Command | Result | Interpretation |
|---|---|---|
| `rg -n 'vac_app_server|vac_app_server_client|vac_app_server_protocol|vac_app_server_transport|vac-app-server' vac-rs/tui/Cargo.toml vac-rs/tui/src` | Nonzero matches limited to optional legacy compatibility and comments | Default path is clean; optional `legacy-app-server-compat` remains explicit delete/defer evidence. |
| `rg -n 'vac_app_server|vac_app_server_client|vac_app_server_protocol|vac_app_server_transport|vac-app-server' vac-rs --glob 'Cargo.toml' --glob '*.rs'` | Remaining matches are workspace crates/tests, runtime-protocol compatibility fixtures, runtime-owner gate tests, and optional TUI legacy compatibility | Wider workspace deletion remains deferred; no false-green deletion claim. |

## Allowed remaining matches

| Path | Classification | Owner | Notes |
|---|---|---|---|
| `vac-rs/tui/Cargo.toml` | optional non-default compatibility | Plan 33 defer | `vac-app-server-client` appears only under `legacy-app-server-compat`; default feature set remains empty. |
| `vac-rs/tui/src/legacy_app_server_compat.rs` | optional non-default compatibility | Plan 33 defer | Kept behind feature gate, not default product runtime. |
| `vac-rs/tui/src/session_protocol.rs` | comment/docs marker | Plan 31/33 | States active protocol facade is independent from `vac-app-server*`. |
| `vac-rs/runtime-protocol/*` | schema compatibility fixture/export | Plan 31/33 defer | Compatibility naming retained for schema parity evidence, not active default TUI path. |
| `vac-rs/app-server*` | historical workspace crates/tests | Plan 33 defer | Workspace-wide physical deletion remains deferred. |
| `vac-rs/cli/src/doctor/runtime_owner_gates.rs` and tests | guard logic | Plan 32 | Required to keep app-server regressions detected. |

## Closeout answers

| Question | Answer |
|---|---|
| Does active default TUI path import app-server crates? | No. The only TUI import is isolated in `legacy_app_server_compat.rs` behind a non-default feature. |
| Does `vac-rs/tui/Cargo.toml` still depend on app-server crates by default? | No. `default = []`; the remaining `vac-app-server-client` edge is optional/non-default. |
| Are all remaining workspace consumers classified? | Yes, as optional compatibility, schema fixture/export, guard logic, or historical app-server workspace crates. |
| Is this evidence sufficient for Plan 33 default-path closeout? | Yes. It supports default-path retirement and explicit workspace crate deletion defer. |
