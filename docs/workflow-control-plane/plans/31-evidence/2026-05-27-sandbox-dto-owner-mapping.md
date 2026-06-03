# Plan 31 evidence — DTO owner mapping snapshot

Date: 2026-05-27

## Scope

Recorded the current app-server protocol DTO owner map for the active/deferred TUI local runtime path. This is a mapping/evidence slice only; it does not remove dependencies or claim Plan 31 completion.

## Mapping summary

| Area | Current app-server symbol family | Intended owner / replacement posture | Status |
|---|---|---|---|
| `vac-rs/tui/src/session_protocol.rs` | `Additional*Permissions`, `Guardian*`, `Hook*`, `ModelVerification`, `Network*`, `ThreadSortKey` compatibility aliases | Replace with owner/product DTOs or narrow conversion wrappers before Cargo removal | active compatibility |
| `vac-rs/tui/src/session_protocol.rs` | `ThreadGoal`, `TurnPlanStepStatus`, `AutoReviewDecisionSource` conversion helpers | Keep only while conversion wrappers are needed; each remaining mapping needs test/evidence | active compatibility |
| `vac-rs/tui/src/session_protocol.rs` | `pub(crate) use vac_app_server_protocol::{...}` re-export block | Split owner-native DTOs from app-server compatibility before Plan 33 | active compatibility |
| `vac-rs/tui/src/app_server_session.rs` | `vac_app_server_client`, `CommandMigration`, `HookMigration`, `McpServerMigration`, `SessionMigration`, `SubagentMigration` | Explicit non-default/deferred app-server session adapter until Plan 30G/31 replacement closes | deferred active adapter |
| `vac-rs/tui/src/app_server_session.rs` | `AppServerSession` and `impl LocalRuntimeSession for AppServerSession` | Remove from default local path or quarantine as non-default compatibility before Cargo retirement | blocker for Plan 33 |
| `vac-rs/local-runtime-owner/src` | app-server source imports | Owner-native path should remain clean | currently clean in active scan |
| `vac-rs/exec/src/runtime_adapter.rs` | app-server source imports | Owner-native runtime adapter should remain clean | currently clean in active scan |

## Counts from sandbox audit

```text
Cargo manifests with app-server dependency/name hits: 17
Active source scan hits:
- vac-rs/local-runtime-owner/src: 0
- vac-rs/tui/src/local_runtime_session.rs: 0 direct app-server import hits in the scan pattern
- vac-rs/tui/src/session_protocol.rs: 25
- vac-rs/tui/src/app_server_session.rs: 58
- vac-rs/exec/src/runtime_adapter.rs: 0
```

## Interpretation

Plan 31B is now documented: the compatibility DTO seam is concentrated in `session_protocol.rs` and the deferred `app_server_session.rs` adapter. Replacement/deletion is still blocked because removing those symbols would affect compatibility routing and because Plan 33 still has nonzero Cargo app-server consumers.

## Validation command

```bash
rg -n "vac_app_server|vac-app-server|AppServerSession|ClientRequest|ServerRequest|ServerNotification|TuiSessionEvent" vac-rs/tui/src vac-rs/exec/src vac-rs/local-runtime-owner/src vac-rs/cli/src vac-rs/core/src -S
```
