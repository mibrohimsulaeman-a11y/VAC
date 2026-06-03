# Plan 33 evidence — app-server consumer classification snapshot

Date: 2026-05-27

## Scope

Recorded a workspace consumer classification for app-server Cargo edges and active source references. This is evidence only; it does not remove Cargo dependencies and does not claim Plan 33 completion.

## Cargo consumers found

| Consumer class | Manifests | Classification |
|---|---:|---|
| App-server crates themselves | `app-server`, `app-server-client`, `app-server-protocol`, `app-server-transport`, `app-server/tests/common` | owned compatibility/runtime quarantine until delete/defer proof is green |
| Default/near-default product crates | `tui`, `core`, `config`, `connectors`, `core-plugins`, `tools`, `login`, `model-provider-info`, `otel`, `analytics`, `exec-server`, `external-agent-sessions` | still nonzero; blocks Cargo retirement claim |
| Local runtime owner/exec active scan | `local-runtime-owner/src`, `exec/src/runtime_adapter.rs` | no direct app-server import hits in this scan |

## Current blockers

- `vac-rs/tui/Cargo.toml` still has `vac-app-server`, `vac-app-server-client`, and `vac-app-server-protocol`.
- `vac-rs/tui/src/session_protocol.rs` still owns compatibility aliases/conversions from `vac_app_server_protocol`.
- `vac-rs/tui/src/app_server_session.rs` still implements the deferred app-server adapter and `LocalRuntimeSession` compatibility path.
- Plan 31 replacement must reduce active source imports before Plan 33B can safely remove TUI Cargo edges.

## Decision

Plan 33 remains `BLOCKED / final proof gate`. The correct next implementation slice is Plan 31C replacement/quarantine narrowing, not deleting app-server crates or editing Cargo edges first.

## Validation command

```bash
rg -n "vac_app_server|vac-app-server|AppServerSession|ClientRequest|ServerRequest|ServerNotification|TuiSessionEvent" vac-rs/tui/src vac-rs/exec/src vac-rs/local-runtime-owner/src vac-rs/cli/src vac-rs/core/src -S
```
