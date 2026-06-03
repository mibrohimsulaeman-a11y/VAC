# Plan 30G — Skills/plugins + external-agent owner-bus migration evidence

Date: 2026-05-25
Slice: Plan 30G
Status: **partial — skills complete; local plugin provider added; external-agent background import and remote plugin parity remain**

## Scope

Plan 30G covers the remaining skills/plugins and external-agent configuration controls that must move through `vac-local-runtime-owner::RuntimeCommandBus` before Plan 31 can safely retire protocol DTOs.

Required surfaces:

- `skills/list` for current-cwd and arbitrary-cwd requests.
- Plugin surface: list/read/install/uninstall/set-enabled.
- External-agent config detect/import.

## Migrated command surfaces

Implemented owner-bus command shapes in `vac-rs/local-runtime-owner/src/command_bus.rs` and public exports in `vac-rs/local-runtime-owner/src/lib.rs`:

- `RuntimeSkillsListCommand`
- `RuntimePluginSurfaceCommand`
- `RuntimeExternalAgentConfigDetectCommand`
- `RuntimeExternalAgentConfigImportCommand`

The direct retained TUI path in `vac-rs/tui/src/app_server_session.rs` now attempts these owner-bus commands before using the compatibility backend path.

## Skills/list status

Current-cwd `skills/list` is owner-bus backed.

The owner-bus implementation uses retained `ThreadManager`, retained `EnvironmentManager`, and the current loaded `Config` to build `vac_core::skills::SkillsLoadInput`, resolve effective plugin skill roots from the retained plugin manager, and convert core skill metadata/errors into protocol-compatible skills-list DTOs for the existing TUI UI path.

Arbitrary-cwd `skills/list` is now owner-bus backed. The owner command bus resolves a scoped `Config` per requested cwd with `ConfigBuilder::fallback_cwd`, computes effective plugin skill roots from that scoped config, preserves absolute per-cwd extra-root validation, and returns `RuntimeSkillsListResponse` without using the app-server request path. If config resolution fails for a requested cwd, the owner bus returns the narrower typed error `RuntimeCommandBusError::SkillsConfigResolutionFailed { cwd, message }`; the TUI compatibility request path is retained only for that non-default error path.

## Plugin surface status

Plugin surface is not complete.

The owner bus has an explicit `RuntimePluginSurfaceCommand` shape and rejects unsupported operations with `RuntimeCommandBusError::PluginSurfaceOwnerProviderRequired(operation)`.

Remaining app-server-only provider dependencies found in `vac-rs/app-server/src/vac_message_processor/plugins.rs`:

- `plugin_list_response` depends on app-server `load_latest_config`, auth-manager refresh, workspace plugin enablement policy, app-server outgoing callbacks, remote catalog fetch, marketplace-load error DTO mapping, and app-server protocol DTO mapping.
- `plugin_read_response` depends on app-server config fallback resolution, remote plugin read, plugin app summary loading, product-restriction filtering, and app-server DTO mapping.
- `plugin_install_response` / `plugin_uninstall_response` depend on plugin manager mutation plus post-install MCP OAuth/app-auth flows, config reload, effective-plugin changed callbacks, and app-server notification behavior.
- Plugin enablement currently routes through generic app-server config write (`ConfigValueWrite`) from the TUI background request path, not a standalone lower-level owner provider.

Narrowed blocker: plugin list/read/install/uninstall/set-enabled requires a dedicated owner-native provider module that owns plugin DTO mapping, workspace plugin policy, remote catalog access, config reload/effective-plugin callbacks, and post-install MCP/app-auth side effects outside the app-server message processor. The owner bus now reports `PluginSurfaceOwnerProviderRequired(operation)`, which is a per-operation technical blocker rather than a broad app-server-provider placeholder.

## External-agent config status

External-agent config detect/import is partially owner-backed.

The owner bus now has migration-item DTOs, an owner-side `ExternalAgentConfigService`, and normal detect/import routing through `RuntimeCommandBus`. The TUI startup/import path probes the owner bus first and retains compatibility fallback for explicit background import requirements or unexpected owner-side errors.

Remaining app-server-only provider dependencies found in:

- `vac-rs/app-server/src/external_agent_config_api.rs`
- `vac-rs/app-server/src/config/external_agent_config.rs`
- `vac-rs/app-server/src/message_processor.rs`

Narrowed blocker:

- External-agent detect now uses the owner provider and returns migration details for config, skills, agents.md, plugins, MCP servers, hooks, subagents, commands, and sessions.
- External-agent import now routes through the owner provider for synchronous import work, but pending plugin imports and session import orchestration can still require background completion. The owner bus reports `ExternalAgentConfigBackgroundImportRequired` for work that cannot yet be completed under the current owner command/event contract. Treating import as fully complete without background completion semantics would be a false green.

## Remaining app-server fallbacks

Compatibility fallbacks remain for:

- `skills/list` only when owner per-cwd config resolution itself fails with `SkillsConfigResolutionFailed`; normal current-cwd and arbitrary-cwd requests are owner-bus backed.
- plugin list/read/install/uninstall/set-enabled paths through existing app-server request/config-write handlers until `PluginSurfaceOwnerProviderRequired(operation)` is resolved with an owner-native provider.
- external-agent config import only when the owner provider reports `ExternalAgentConfigBackgroundImportRequired` or an unexpected owner-side error requires compatibility fallback.
- non-retained sessions where retained managers/config are unavailable.

These fallbacks are not Plan 31-ready final owner paths.

## Tests

Focused tests added in `vac-rs/local-runtime-owner/src/lib.rs`:

- `command_bus_p30g_skills_supports_arbitrary_cwd_without_fallback`
- `command_bus_p30g_plugin_remains_blocked_and_external_agent_detect_is_owner_backed`

These tests prevent fake-success completion claims for unsupported Plan 30G surfaces.

## Validation

Requested validation commands for this slice:

```sh
cd /home/emp/Documents/VAC/vastar-agentic-cli/vac-rs
df -h . /tmp
ps -eo pid,ppid,stat,etime,cmd | rg 'cargo|rustc|sccache' || true
sccache --show-stats || true
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-local-runtime-owner
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
```

Docs validation:

```sh
git diff --check -- docs/workflow-control-plane/plans/30-prompt-and-active-controls-cutover.md docs/workflow-control-plane/plans/30-evidence docs/workflow-control-plane/plans/24-local-runtime-owner-replacement.md
```

## Plan 30G conclusion

Plan 30G is **partial**, not complete. The arbitrary-cwd skills blocker is resolved, external-agent detect/import is owner-provider backed for normal paths, and W5C adds owner-native local plugin list/read/install/uninstall/set-enabled routing. Remote plugin marketplace parity and external-agent background import completion remain narrower technical blockers.

Completed:

- Current-cwd and arbitrary-cwd skills list are routed through `RuntimeCommandBus`.
- Plugin and external-agent config command shapes are present in the owner bus.
- TUI direct retained path probes owner bus for skills and external-agent config.
- External-agent detect/import uses owner-side migration DTOs/provider conversion for normal paths.
- Unsupported plugin and background-import paths fail closed with typed errors rather than silent success.

Not complete:

- Remote plugin marketplace read/install/uninstall parity without app-server fallback.
- External-agent background plugin/session import completion semantics and owner-owned completion/event contract.

Plan 31 remains blocked until these provider gaps are resolved or explicitly re-scoped with owner-accessible lower-level providers.
## W3F decision — provider migration not safe in this slice

Date: 2026-05-25
Decision: **non-terminal blocker; Plan 30 remains partial**

W3F did not lift the remaining providers in this slice. The current workspace still has a large unrelated dirty tree across `.vac/**`, `docs/**`, `vac-rs/core/src/control_plane/**`, CLI/config/exec/TUI/protocol files, and the likely provider target files are already dirty from other work. Implementing plugin/external-agent provider extraction on top of that would risk mixing unrelated control-plane/runtime changes with Plan 30G provider semantics.

The blocker is **not accepted as terminal completion** for Plan 30. It is accepted only as a safe stop condition for this cleanup lane. Plan 31 remains blocked until one of the following happens:

1. **Provider migration path:** extract owner-accessible lower-level providers for arbitrary-cwd skills resolution, plugin list/read/install/uninstall/set-enabled, and external-agent detect/import; then remove the required app-server compatibility fallbacks with targeted validation.
2. **Policy re-scope path:** explicitly declare plugin and external-agent surfaces out of Plan 30/31 retirement scope, with owner-approved user-facing risk notes and no false claim that Plan 30G is complete.

Current fallback state remains unchanged:

- current-cwd and arbitrary-cwd `skills/list` are owner-bus backed;
- local plugin list/read/install/uninstall/set-enabled operations are owner-bus backed in the retained/direct TUI path;
- remote plugin marketplace operations remain a narrow compatibility fallback;
- external-agent detect/import is owner-provider backed for normal paths, with `ExternalAgentConfigBackgroundImportRequired` still possible for pending plugin/session background work;
- app-server compatibility fallback remains only for remote plugin operations, external-agent background completion, unexpected owner errors, and non-retained sessions while those narrowed blockers are unresolved.

Next safe implementation slice: isolate or clean the dirty provider-target files, then create a focused provider-extraction plan/patch before any Plan 31 DTO retirement.

## W5C update — local plugin provider path

Date: 2026-05-26
Decision: **local plugin provider path implemented; Plan 30 remains partial**

W5C adds `vac-rs/local-runtime-owner/src/plugin_surface.rs` and owner-side runtime plugin DTOs/conversions, then routes local plugin list/read/install/uninstall/set-enabled through `RuntimeCommandBus` for retained/direct TUI sessions.

Validated owner-backed paths:

- `plugin/list` uses retained `ThreadManager`, `AuthManager`, current/scoped config, feature gate checks, marketplace load errors, featured plugin ids, and owner DTO conversion.
- `plugin/read` uses local marketplace path + plugin name and owner DTO conversion for plugin detail, skills, apps, and MCP server names.
- `plugin/install` uses local marketplace path + plugin name, clears plugin/skills caches, maps auth policy, and returns protocol-compatible install response.
- `plugin/uninstall` uses local plugin id and clears plugin/skills caches.
- plugin enablement intercepts `ConfigValueWrite` for `plugins.<id>` upserts and calls owner-side config enablement.

Remaining non-terminal blockers:

- Remote plugin marketplace read/install/uninstall still falls back to app-server compatibility; the TUI owner prefilter returns `Ok(None)` for those remote/non-local cases rather than fake `Value::Null` success.
- External-agent imports that need background plugin/session completion still report `ExternalAgentConfigBackgroundImportRequired` and require an owner-owned completion/event contract before Plan 31 can start.
