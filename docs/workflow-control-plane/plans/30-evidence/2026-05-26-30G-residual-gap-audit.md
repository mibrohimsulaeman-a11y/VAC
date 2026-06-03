# Plan 30G residual provider gap audit — L12 read-only scout

Date: 2026-05-26
Lane: L12
Scope: Plan 30G provider gap re-audit
Status: **diagnostic only / no source edits**

## Executive summary

30G is still **blocked / partial**, not safe to close.

The current repo has an owner-side external-agent config service in `vac-rs/local-runtime-owner/src/external_agent_config.rs` and normal TUI detect/import now tries `RuntimeCommandBus` first. That is a real narrowing from the older provider-unavailable state.

Residual blockers are narrower:

1. **Plugin surface is still not owner-backed.** TUI plugin and marketplace operations still dispatch typed app-server requests from `vac-rs/tui/src/app/background_requests.rs`; the owner command bus still fails closed with `PluginSurfaceOwnerProviderRequired(operation)`.
2. **External-agent import still has a background-completion semantic gap.** The owner service can do synchronous migration work and return pending plugin-import work, but `RuntimeCommandBus` cannot yet represent/drive the app-server background completion semantics for plugin/session import work.
3. **External-agent compatibility fallbacks still exist in the TUI retained/direct path.** Detect/import attempts owner bus first, then falls back to app-server paths for explicit background/dto blockers or unexpected owner errors.

No `vac-rs/cli/` external-agent or plugin surface call site was found in this audit beyond doctor/compatibility checks and comments; the residual runtime call sites are in `vac-rs/tui/`.

## Read-only audit method

Commands used were read-only discovery via `rg`, `find`, `cat`, and `git status` inside the repo root `/home/emp/Documents/VAC/vastar-agentic-cli`. No Cargo/type-check was run because the gaps are visible from source routing and existing evidence; avoiding Cargo also avoids touching build artifacts in a dirty shared workspace.

Important workspace note: before this file was created, the worktree already had unrelated modified/untracked files in `.vac/**`, `docs/**`, `vac-rs/**`, and scheduled-audit outputs. This lane must stage only this evidence file.

## Current state of `local-runtime-owner/src/external_agent_config.rs`

File: `vac-rs/local-runtime-owner/src/external_agent_config.rs`

What it now covers:

- Owner-side service exists:
  - `ExternalAgentConfigDetectOptions` at `external_agent_config.rs:43`
  - migration item enum and detail structs at `external_agent_config.rs:49`, `:62`, `:68`, `:73`, `:83`, `:89`, `:97`
  - `ExternalAgentConfigService` at `external_agent_config.rs:105`
  - `ExternalAgentConfigService::new` at `external_agent_config.rs:111`
  - `detect` at `external_agent_config.rs:119`
  - `import` at `external_agent_config.rs:156`
  - `import_plugins` at `external_agent_config.rs:681`
- Detection covers the external-agent scopes currently represented by the item enum and detail structs:
  - config/settings: `external_agent_config.rs:283`
  - MCP server config: `external_agent_config.rs:325`
  - hooks: `external_agent_config.rs:357`
  - skills: `external_agent_config.rs:387`
  - commands: `external_agent_config.rs:412`
  - subagents: `external_agent_config.rs:440`
  - `AGENTS.md`: `external_agent_config.rs:473`
  - sessions: `external_agent_config.rs:541`
  - plugin migration item details: `external_agent_config.rs:620`
- Import covers synchronous migration work for the same item families:
  - `Config`: `external_agent_config.rs:163`
  - `Skills`: `external_agent_config.rs:171`
  - `AgentsMd`: `external_agent_config.rs:179`
  - `Plugins`: `external_agent_config.rs:187`
  - `McpServerConfig`: `external_agent_config.rs:211`
  - `Subagents`: `external_agent_config.rs:219`
  - `Hooks`: `external_agent_config.rs:227`
  - `Commands`: `external_agent_config.rs:235`
  - `Sessions`: currently no synchronous import arm body at `external_agent_config.rs:243`
- The owner command bus has DTOs and conversion helpers for the external-agent item/detail surface:
  - command DTOs: `vac-rs/local-runtime-owner/src/command_bus.rs:105`, `:112`, `:118`, `:126`, `:139`, `:149`, `:155`, `:162`
  - read command routing: `command_bus.rs:468`, service detect call at `command_bus.rs:655`
  - write command routing: `command_bus.rs:561`, service import call at `command_bus.rs:674`
  - item/detail conversion helpers: `command_bus.rs:1076`, `:1089`, `:1102`, `:1136`, `:1170`, `:1214`, `:1260`, `:1268`

Still missing for full external-agent surface migration:

- **Background completion semantics.** `ExternalAgentConfigService::import` returns `Vec<PendingPluginImport>` (`external_agent_config.rs:159`) and pushes pending plugin imports for plugin migration (`external_agent_config.rs:200`). `RuntimeCommandBus::external_agent_config_import` turns any non-empty pending list into `RuntimeCommandBusError::ExternalAgentConfigBackgroundImportRequired` (`command_bus.rs:683`). That keeps the app-server fallback alive for imports that need background completion.
- **Session import semantics are not visibly owner-complete.** The `Sessions` import arm is empty (`external_agent_config.rs:243`). Detection can report sessions (`external_agent_config.rs:541`), but import does not appear to migrate sessions synchronously in the owner provider.
- **Metrics are intentionally stubbed/no-op.** `emit_migration_metric` exists at `external_agent_config.rs:1567`, but the comment says the owner provider intentionally leaves metrics to the JSON-RPC boundary (`external_agent_config.rs:1573`). This is probably acceptable for provider extraction, but it is not app-server behavior parity if completion telemetry is expected from the owner path.
- **Plugin import depends on plugin provider semantics that are not yet owned by the command bus.** `import_plugins` exists (`external_agent_config.rs:681`), but the broader plugin surface still has no owner provider (see below). This is why external-agent plugin migration can produce pending work that the owner bus refuses to claim as completed.

## External-agent call sites still routed/falling back through app-server

### TUI startup prompt caller

- `vac-rs/tui/src/external_agent_config_migration_startup.rs:264` calls `.external_agent_config_detect(ExternalAgentConfigDetectParams { ... })` through the `LocalRuntimeSession` trait. The concrete `AppServerSession` implementation tries owner bus first, but the type/trait surface is still the session/app-server facade, not a pure owner DTO surface.
- `vac-rs/tui/src/external_agent_config_migration_startup.rs:319` calls `app_server.external_agent_config_import(items).await`. Same note: concrete implementation tries owner bus first, but fallback remains in `AppServerSession`.

Classification: **Behavioral mismatch** for import; the owner cannot yet represent app-server background completion. Detect is mostly DTO/facade cleanup after import semantics are resolved.

### App-server request-handle direct app-server path

These are the direct typed app-server RPC methods still present on the request handle:

- `vac-rs/tui/src/app_server_session.rs:354` — `request_typed(ClientRequest::ExternalAgentConfigDetect { ... })`
- `vac-rs/tui/src/app_server_session.rs:363` — `request_typed(ClientRequest::ExternalAgentConfigImport { ... })`

Classification: **Behavioral mismatch**. They are now compatibility paths, but cannot be removed until the owner can complete all import semantics. Estimated mechanical removal after semantic fix: ~1-2 hours plus targeted tests.

### AppServerSession owner-first fallback logic

- `vac-rs/tui/src/app_server_session.rs:652` — direct request-handle detect path for non-retained/remote-like mode.
- `vac-rs/tui/src/app_server_session.rs:658` — retained/direct detect tries `RuntimeReadCommand::ExternalAgentConfigDetect`.
- `vac-rs/tui/src/app_server_session.rs:680-681` — detect fallback on `ExternalAgentConfigBackgroundImportRequired` / `ExternalAgentConfigMigrationDetailsRequired`.
- `vac-rs/tui/src/app_server_session.rs:689` — detect falls back to request-handle `.external_agent_config_detect(params)`.
- `vac-rs/tui/src/app_server_session.rs:701` — retained/direct import tries `RuntimeWriteCommand::ExternalAgentConfigImport`.
- `vac-rs/tui/src/app_server_session.rs:721-722` — import fallback on `ExternalAgentConfigBackgroundImportRequired` / `ExternalAgentConfigMigrationDetailsRequired`.
- `vac-rs/tui/src/app_server_session.rs:731` — import falls back to request-handle `.external_agent_config_import(migration_items)`.

Classification:

- `:658` / `:701` are good owner-first routing.
- `:680-689` and `:721-731` are **Behavioral mismatch** fallbacks, not DTO-only gaps. The owner bus intentionally rejects cases it cannot complete.
- `ExternalAgentConfigMigrationDetailsRequired` appears stale or defensive after current DTO helpers were added; if confirmed dead by tests, removing that fallback branch is **DTO mapping cleanup**, ~1 hour. Do not remove the app-server import fallback until background completion is owner-owned.

## Plugin surface gap list

### Owner command bus state

The owner bus has placeholder command shapes but no provider implementation:

- `vac-rs/local-runtime-owner/src/command_bus.rs:167` — `RuntimePluginSurfaceCommand`
- `command_bus.rs:175` — `RuntimePluginSurfaceOperation`
- `command_bus.rs:197` — `RuntimeReadCommand::PluginSurface(...)`
- `command_bus.rs:265` — `RuntimeWriteCommand::PluginSurface(...)`
- `command_bus.rs:423` — `PluginSurfaceOwnerProviderRequired(RuntimePluginSurfaceOperation)` error
- `command_bus.rs:464-465` — read plugin surface fails closed with `PluginSurfaceOwnerProviderRequired`
- `command_bus.rs:557-558` — write plugin surface fails closed with `PluginSurfaceOwnerProviderRequired`

Classification: **Provider trait gap**. The missing piece is not just DTO mapping; the owner needs a plugin/marketplace provider abstraction and implementation equivalent to the app-server plugin marketplace semantics.

### TUI plugin/marketplace paths that bypass owner bus

Event dispatch and background request entrypoints:

- `vac-rs/tui/src/app/event_dispatch.rs:381-382` — `FetchPluginsList` -> `self.fetch_plugins_list(app_server, cwd)`
- `event_dispatch.rs:437` — marketplace add -> `self.fetch_marketplace_add(app_server, cwd, source)`
- `event_dispatch.rs:443` — marketplace upgrade -> `self.fetch_marketplace_upgrade(app_server, cwd, marketplace_name)`
- `event_dispatch.rs:457` — refresh plugin list after marketplace add
- `event_dispatch.rs:476` — refresh plugin list after marketplace upgrade
- `event_dispatch.rs:484` — marketplace remove -> `self.fetch_marketplace_remove(...)`
- `event_dispatch.rs:511` — refresh plugin list after marketplace remove
- `event_dispatch.rs:514` — `FetchPluginDetail`
- `event_dispatch.rs:520-526` — plugin install dispatch
- `event_dispatch.rs:534-539` — plugin uninstall dispatch
- `event_dispatch.rs:572` — refresh list after plugin install
- `event_dispatch.rs:1205` — refresh list after plugin uninstall
- `vac-rs/tui/src/app/app_server_events.rs:109` — external-agent import-completed notification refreshes plugin list through app-server client

Background request functions and typed app-server RPCs:

- `vac-rs/tui/src/app/background_requests.rs:101` — async task calls `fetch_plugins_list(request_handle, cwd.clone())`
- `background_requests.rs:150` — async marketplace add
- `background_requests.rs:173` — async marketplace remove
- `background_requests.rs:195` — async marketplace upgrade
- `background_requests.rs:219` — async plugin install
- `background_requests.rs:244` — async plugin uninstall
- `background_requests.rs:616` — `request_typed(ClientRequest::PluginList { ... })`
- `background_requests.rs:656` — `request_typed(ClientRequest::PluginRead { ... })`
- `background_requests.rs:670` — `request_typed(ClientRequest::MarketplaceAdd { ... })`
- `background_requests.rs:715` — `request_typed(ClientRequest::MarketplaceRemove { ... })`
- `background_requests.rs:729` — `request_typed(ClientRequest::MarketplaceUpgrade { ... })`
- `background_requests.rs:743` — `request_typed(ClientRequest::PluginInstall { ... })`
- `background_requests.rs:761` — `request_typed(ClientRequest::PluginUninstall { ... })`

Chat widget event producers:

- `vac-rs/tui/src/chatwidget/plugins.rs:247` — sends `FetchPluginsList`
- `chatwidget/plugins.rs:1678` — sends `FetchPluginUninstall`
- `chatwidget/plugins.rs:1707` — sends `FetchPluginInstall`
- `chatwidget/plugins.rs:1818` — sends `FetchPluginDetail`

Classification for all plugin/marketplace paths: **Provider trait gap**. The TUI has UI/event plumbing; app-server owns the actual plugin marketplace provider behavior today. The owner bus DTO placeholder alone is insufficient.

## Gap classification table

| Gap | Evidence | Classification | Why |
| --- | --- | --- | --- |
| Plugin list/read/install/uninstall plus marketplace add/remove/upgrade still use typed app-server RPCs | `background_requests.rs:616`, `:656`, `:670`, `:715`, `:729`, `:743`, `:761` | Provider trait gap | Owner command bus has placeholder operations but returns `PluginSurfaceOwnerProviderRequired`; no owner plugin provider abstraction/implementation exists. |
| Owner command bus plugin surface fails closed | `command_bus.rs:423`, `:464-465`, `:557-558` | Provider trait gap | Explicit fail-closed error proves DTO shape exists but provider behavior is absent. |
| External-agent import fallback for pending plugin/session background work | `external_agent_config.rs:159`, `:200`, `command_bus.rs:683`, `app_server_session.rs:721-731` | Behavioral mismatch | App-server can continue background completion and emit completion notification; owner command bus cannot yet represent/drive that lifecycle. |
| External-agent sessions detection exists but import arm is empty | `external_agent_config.rs:541`, `external_agent_config.rs:243` | Behavioral mismatch | Owner provider can report sessions but does not visibly migrate them synchronously; closure needs either owner-owned session import or explicit re-scope. |
| External-agent app-server request-handle compatibility methods remain | `app_server_session.rs:354`, `:363`, fallback use at `:689`, `:731` | Behavioral mismatch now; DTO cleanup later | The direct RPC methods are compatibility fallbacks until import semantics are owner-complete. After that, removal is mechanical. |
| `ExternalAgentConfigMigrationDetailsRequired` fallback remains | `app_server_session.rs:680-681`, `:721-722`; error defined at `command_bus.rs:427` | DTO mapping missing / stale fallback candidate | Current DTO helpers appear to cover details, so this branch may be defensive/stale. Confirm with targeted tests before deleting. |

No gap in this audit appears **Blocked-by-upstream (waiting for vac_protocol change)**. The blockers are provider ownership and owner runtime semantics, not an observed protocol schema dependency.

## Recommended next safe slice

Recommended slice: **P-30G-plugin-list-owner-provider-read-only**.

Goal: close at least one plugin gap without touching Lane L7-owned `vac-rs/tui/src/session_protocol.rs` aliases.

Proposed boundaries:

1. Add an owner-side plugin provider abstraction/implementation in `vac-local-runtime-owner` for **read-only plugin list** only.
2. Route `RuntimeReadCommand::PluginSurface(List)` through that provider instead of `PluginSurfaceOwnerProviderRequired`.
3. In TUI `background_requests.rs`, switch only `fetch_plugins_list` to owner-first routing with compatibility fallback, similar to the external-agent detect/import pattern.
4. Do not touch `session_protocol.rs`; reuse existing session protocol DTOs only at the TUI adapter boundary if needed.
5. Validate with a narrow owner-bus unit test plus one TUI background request/unit test. Avoid full Plan 31 alias work.

Why this slice is safer than external-agent background import first:

- It closes a visible plugin provider gap without needing app-server completion notifications.
- It is read-only behavior first, so failure modes are easier to fallback safely.
- It avoids Lane L7-owned protocol alias churn.
- Once plugin list is owner-backed, plugin read/install/uninstall and marketplace mutation can be migrated incrementally.

Alternative if the human wants external-agent-only next: remove/verify the stale `ExternalAgentConfigMigrationDetailsRequired` fallback only after targeted tests prove details DTO parity. That is smaller, but it does not close the major 30G blocker because background completion would remain.

## Bottom line

30G closure is blocked by **plugin provider parity** and **external-agent background import completion semantics**. External-agent normal detect/import is now owner-backed for the non-background path, but the app-server compatibility edge remains necessary for plugin marketplace behavior and completion notification semantics.
