# Plan 30G — Provider blocker evidence (W5B)

Date: 2026-05-26
Slice: W5B — external-agent provider extraction and remaining provider blockers
Status: **superseded by W5C for local plugin paths / still partial** — external-agent detect/import owner provider is present for normal paths; W5C adds owner-native local plugin paths, while remote plugin parity and external-agent background completion semantics remain blockers.

## Scope

This evidence records the W5B provider migration state after extracting the external-agent config provider into `vac-local-runtime-owner` and wiring the TUI retained/direct path through `RuntimeCommandBus`.

Intended W5B files:

- `vac-rs/local-runtime-owner/src/command_bus.rs`
- `vac-rs/local-runtime-owner/src/external_agent_config.rs`
- `vac-rs/local-runtime-owner/src/lib.rs`
- `vac-rs/local-runtime-owner/Cargo.toml`
- `vac-rs/tui/src/app_server_session.rs`
- `vac-rs/Cargo.lock`
- Plan 30 status/evidence docs

## Completed in W5B

- Added owner-side external-agent migration DTOs and conversion helpers in `RuntimeCommandBus`.
- Added `ExternalAgentConfigService` under `vac-local-runtime-owner` so detect/import no longer depends on app-server internals for normal paths.
- Routed TUI retained/direct external-agent detect/import through `RuntimeCommandBus`.
- Preserved compatibility fallback for explicit background-import requirements or unexpected owner-side errors.
- Updated the local-runtime-owner test to prove plugin remains blocked while external-agent detect is owner-backed.

## Remaining blocker 1 — plugin surface provider parity

W5C supersedes the broad plugin blocker for retained/direct local plugin list/read/install/uninstall/set-enabled. Those paths now route through `RuntimeCommandBus` and owner-side plugin DTO conversion. The remaining plugin blocker is narrower: remote marketplace plugin read/install/uninstall parity still requires app-server compatibility fallback.

The current owner command shape is still too narrow for app-server parity across list/read/install/uninstall/set-enabled. Required unresolved pieces include:

1. `plugin/list` parity:
   - feature-gate and workspace policy checks;
   - `AuthManager` access;
   - effective-plugin changed callback;
   - remote catalog fetch and featured plugin ids;
   - marketplace-load error DTOs.
2. `plugin/read` parity:
   - local marketplace path vs remote marketplace name inputs;
   - app metadata/auth summary mapping;
   - local/remote plugin detail DTOs.
3. `plugin/install` parity:
   - local marketplace path vs remote marketplace name inputs;
   - post-install config reload;
   - effective-plugin callback;
   - MCP OAuth startup side effects;
   - app-auth side effect/summary DTOs.
4. `plugin/uninstall` parity:
   - local plugin id vs remote plugin id branching;
   - remote installed cache refresh after mutation;
   - effective-plugin callback.
5. `SetEnabled` parity:
   - owner-native input/output contract and product decision for enablement updates.

Implementing plugin fake-success behind the current underspecified command shape would be a false green.

## Remaining blocker 2 — external-agent background import completion

External-agent detect/import is now owner-provider backed for normal paths, but import is not final Plan 31-ready completion for every case. Remaining gaps:

1. Pending plugin imports can still require background work. The owner bus reports `RuntimeCommandBusError::ExternalAgentConfigBackgroundImportRequired` when import cannot be completed synchronously under the current owner command contract.
2. Session migration items are detected and represented in migration details, but session import orchestration still needs an owner-owned completion/ledger/event contract before app-server compatibility can be removed.
3. App-server currently emits `externalAgentConfig/import/completed` after synchronous or background completion. The owner path needs an equivalent completion signal before this surface can be considered fully migrated.

## Validation so far

Passed after the W5B external-agent provider patch:

```sh
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-local-runtime-owner
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-local-runtime-owner command_bus_p30g_plugin_remains_blocked_and_external_agent_detect_is_owner_backed --lib
```

Full `cargo +1.93.0 test -p vac-local-runtime-owner` initially failed because the previous test still expected `ExternalAgentConfigMigrationDetailsRequired` and used an empty `vac_home`, which triggered plugin cache absolute-path validation before reaching the old expectation. The test has been updated to use a temporary `vac_home` and prove owner-backed external-agent detect succeeds for an empty source.

Full owner test rerun remains required before commit once other active Cargo lanes are quiet.

## Decision

Plan 30G remains **partial / not complete**. W5B narrowed external-agent from DTO/provider-unavailable to owner-backed normal detect/import plus explicit background-completion blocker. W5C narrows plugin provider parity to remote marketplace parity after implementing retained/direct local plugin list/read/install/uninstall/set-enabled through the owner command bus. Plan 31 remains blocked until remote plugin parity and external-agent background completion semantics are completed or explicitly re-scoped.
