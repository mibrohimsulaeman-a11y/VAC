# Plan 30G — W5C plugin owner provider evidence

Date: 2026-05-26
Slice: W5C — plugin owner provider path
Status: **partial / not complete** — local plugin list/read/install/uninstall/set-enabled are owner-backed for retained/direct TUI sessions; remote plugin marketplace parity and external-agent background import completion remain blockers.

## Scope

W5C targets the Plan 30G plugin provider blocker without starting Plan 31 DTO retirement. The slice keeps app-server protocol DTOs at the TUI facade while moving required local plugin operations through `vac-local-runtime-owner::RuntimeCommandBus`.

Intended W5C files:

- `vac-rs/local-runtime-owner/Cargo.toml`
- `vac-rs/local-runtime-owner/src/lib.rs`
- `vac-rs/local-runtime-owner/src/command_bus.rs`
- `vac-rs/local-runtime-owner/src/plugin_surface.rs`
- `vac-rs/tui/src/app_server_session.rs`
- `vac-rs/tui/src/session_protocol.rs`
- `vac-rs/Cargo.lock`

## Implemented

- Added owner-native runtime plugin DTOs and conversions in `local-runtime-owner/src/plugin_surface.rs`.
- Extended `RuntimePluginSurfaceCommand` with retained owner resources and plugin inputs needed by list/read/install/uninstall/set-enabled.
- Added owner read responses for `PluginList` and `PluginRead`.
- Added owner write responses for `PluginInstall`, `PluginUninstall`, and `PluginSetEnabled`.
- Routed owner command bus plugin read/write operations to concrete owner functions instead of the previous broad `PluginSurfaceOwnerProviderRequired(operation)` blocker.
- Routed retained/direct TUI plugin list/read/install/uninstall and plugin enablement config writes through `RuntimeCommandBus`.
- Preserved app-server compatibility only for narrow remote/non-local plugin cases, unexpected owner errors, and non-retained sessions.

## Fallback boundaries

Normal local plugin paths no longer use app-server fallback in retained/direct sessions:

- `plugin/list`
- `plugin/read` with local marketplace path
- `plugin/install` with local marketplace path
- `plugin/uninstall` with local plugin id
- `ConfigValueWrite` upserts for `plugins.<id>.enabled`

Remote or non-local plugin read/install/uninstall still return `Ok(None)` from the TUI owner prefilter so the existing app-server compatibility backend handles them. This avoids the previous false-success risk where remote cases could deserialize a `Value::Null` response.

## Validation

Passed after W5C fixes:

```sh
cd /home/emp/Documents/VAC/vastar-agentic-cli/vac-rs
df -h . /tmp
cargo +1.93.0 fmt --check -p vac-local-runtime-owner -p vac-surface-tui
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-local-runtime-owner
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

Observed owner test result: 38 passed, 0 failed.

## Remaining blockers

Plan 30G is still not complete because:

1. Remote plugin marketplace read/install/uninstall parity is still a compatibility fallback.
2. External-agent import can still require background plugin/session completion and reports `ExternalAgentConfigBackgroundImportRequired` until an owner-owned completion/event contract exists.

Plan 31 remains blocked until these provider gaps are completed or explicitly re-scoped.
