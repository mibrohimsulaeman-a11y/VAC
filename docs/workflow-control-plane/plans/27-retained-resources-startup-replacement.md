# Plan 27 — Retained resources and startup replacement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> ## Current status
> Status: **complete for default owner-native startup and retained-resource ownership**.
> - account/status display remains stable,

Code evidence:
- `vac-rs/core/src/local_runtime/session.rs`
- `vac-rs/tui/src/local_runtime_session.rs`
- `vac-rs/local-runtime-owner/`

Evidence docs:
- `docs/workflow-control-plane/plans/27-evidence/2026-05-28-sandbox-startup-fallback-sync.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Close startup and retained-resource ownership under the local runtime owner and keep any app-server compatibility outside the default product path.

## Current status

Status: **complete for default owner-native startup and retained-resource ownership**.

Plan 26 created the owner skeleton. Plan 27 now gives `vac-local-runtime-owner` a direct retained-resource startup path and app-server-free bootstrap result. Later Plans 28–33 closed the default prompt/control/event/request/Cargo path, so Plan 27 is no longer carrying active app-server fallback debt for the default product path.

## Target outcome

The local runtime owner constructs retained resources directly and exposes bootstrap data without app-server protocol ownership. The TUI default path now consumes the owner-native runtime path through later Plan 30/31 cutover evidence.

## Outputs

- startup input field map,
- retained-resource builder,
- local bootstrap result/adapter,
- tests for account/model/thread/environment preservation,
- explicit default-path fallback proof showing no app-server startup fallback remains.

## Requires / Blocks

Requires:

- Plan 26 owner skeleton.

Blocks: none in the default product path after Plans 30–33 closeout. Historical dependency: Plan 27 originally unblocked Plan 30 prompt submit cutover and Plan 33 app-server Cargo retirement.

## Current source behavior

Historical pre-closeout behavior: the TUI path obtained retained resources through app-server-backed startup:

- `ThreadManager`
- `AuthManager`
- `ThreadStore`
- `EnvironmentManager`
- config/bootstrap data for TUI

`AppServerSession::new` retains these resources from `AppServerClient`, and `app-server/src/in_process.rs` builds them around `MessageProcessor` startup.

## Target behavior

The owner constructs retained resources directly and exposes a TUI bootstrap surface without app-server protocol ownership. App-server-backed compatibility is non-default quarantine material after later plans retire each default fallback edge.

## Non-goals

- No second product TUI.
- No broad config-manager rewrite beyond startup parity.
- No copy of `MessageProcessor`.
- No default-path app-server fallback hidden behind the startup seam.

## Required preservation

- account/status display remains stable,
- default model and available model list remain stable,
- feedback audience and plan type behavior remain stable,
- thread list/read/resume still work,
- filesystem/environment behavior remains stable,
- startup failure messages remain actionable.

## Execution note — 2026-05-24

Implemented in `vac-rs/local-runtime-owner/`:

- `RuntimeStartupInput` maps resolved startup inputs into the owner seam.
- `LocalRuntimeOwner::build_retained_resources` constructs `AuthManager`, `ThreadStore`, `ThreadManager`, retains the supplied `EnvironmentManager`, and builds the same analytics client shape as the app-server path.
- `LocalRuntimeOwner::build_bootstrap` returns `LocalRuntimeBootstrap` with account display, auth mode, plan type, feedback audience, default model, and available models without importing app-server DTOs.
- `LocalRuntimeOwner::start` combines retained resources and bootstrap data into `LocalRuntimeOwnerSession`.
- TUI `local_runtime_session` now owns a neutral `LocalRuntimeBootstrap` shape and adapts both app-server-backed bootstrap and owner bootstrap into it.
- `DEFAULT_PATH_APP_SERVER_FALLBACKS` is empty and `OWNER_NATIVE_DEFAULT_SURFACES` records prompt/control/event/request coverage as owner-native default-path coverage.
- Tests cover forbidden app-server dependencies, zero default app-server fallback listing, retained manager visibility, session source/environment preservation, unauthenticated bootstrap account state, and model/default-model preservation.

Prompt submit, live event stream projection, server-request registry replacement, and TUI default runtime cutover are closed by Plans 28–33; Plan 27 now remains the startup/retained-resource owner foundation for that closed default path.

## Startup input field map

| Current field / source | Owner source | App-server dependency today | Replacement API | Test needed / added |
| --- | --- | --- | --- | --- |
| `InProcessClientStartArgs::config` / `InProcessStartArgs::config` | `RuntimeStartupInput::config: Arc<Config>` | App-server passes it into `MessageProcessor` startup and model/account handlers. | Owner uses resolved `Config` directly for auth, thread store, thread manager, analytics, and default model selection. | Added owner startup/bootstrap tests with temporary `vac_home`. |
| `InProcessClientStartArgs::environment_manager` / `InProcessStartArgs::environment_manager` | `RuntimeStartupInput::environment_manager` | App-server constructs/retains it and `AppServerSession::new` exposes it. | Owner receives and retains the same `Arc<EnvironmentManager>`. | Added `Arc::ptr_eq` preservation test and default environment assertion. |
| `InProcessClientStartArgs::session_source` / `InProcessStartArgs::session_source` | `RuntimeStartupInput::session_source` | App-server stamps it into `ThreadManager`. | Owner passes it directly to `ThreadManager::new`. | Added session source preservation test. |
| `InProcessClientStartArgs::enable_vac_api_key_env` / `InProcessStartArgs::enable_vac_api_key_env` | `RuntimeStartupInput::enable_vac_api_key_env` | App-server passes it to `AuthManager::shared_from_config`. | Owner calls `AuthManager::shared_from_config` with the same flag. | Added auth-manager flag preservation test. |
| `InProcessStartArgs::thread_config_loader` | Historical non-owner input | Needed for old app-server config API paths and thread-specific config loading. | Non-default compatibility only; no default startup fallback remains. | Covered by Plan 30/31 owner-native default-path closeout evidence. |
| `cli_overrides`, `loader_overrides`, `config_warnings`, `initialize`, `client_name`, `client_version`, `experimental_api`, `opt_out_notification_methods`, `channel_capacity` | Historical non-owner protocol/bootstrap fields | Protocol/bootstrap/request-channel concerns from the old app-server seam. | Non-default compatibility only after Plan 28/31 wiring removes DTO fallback from default TUI path. | Covered by Plan 31/33 default-path closure evidence. |
| `AppServerBootstrap` fields | TUI `local_runtime_session::LocalRuntimeBootstrap` plus owner `LocalRuntimeBootstrap` | Current TUI previously read app-server-shaped struct through an alias. | TUI now owns the neutral bootstrap shape and adapts app-server-backed and owner-backed bootstrap into it. | Added owner bootstrap tests; TUI compile check covers adapter shape. |

## Default-path fallback proof

`DEFAULT_PATH_APP_SERVER_FALLBACKS` is now empty. `OWNER_NATIVE_DEFAULT_SURFACES` records the owner-native default coverage for:

- retained resource startup,
- TUI bootstrap projection,
- prompt submit / turn execution,
- typed TUI request dispatch,
- live event stream projection,
- server-request resolve/reject registry,
- config reload/import command surface.

Any app-server compatibility that remains is explicit non-default quarantine material governed by Plan 33 delete/defer proof, not a default Plan 27 startup fallback.

## Implementation slices

### 27A — Startup input map — done

Mapped current startup inputs from:

- `InProcessClientStartArgs`
- `InProcessStartArgs`
- `AppServerBootstrap`
- TUI config/bootstrap callers

Output added above as the field-by-field table: owner source, app-server dependency, replacement API, test needed.

### 27B — Retained resource builder — done

Implemented local owner builder for retained resources. Existing app-server-backed material is now non-default compatibility quarantine, not a default fallback.

**Prelude characterization tests (absorbs umbrella slice 24D for retained-manager visibility).** Before extraction, lock retained-manager visibility and account/model/thread/environment preservation as tests so behavior regressions are detected during the move, not after Plan 30 cutover.

### 27C — Bootstrap adapter — done

Added a local bootstrap result that can be adapted to current TUI needs without leaking app-server DTOs.

### 27D — Default owner-native startup seam — complete

TUI bootstrap is wired behind a neutral `local_runtime_session::LocalRuntimeBootstrap` seam instead of aliasing the app-server bootstrap type directly. The owner bootstrap adapts into that shape for the default path, and later Plans 28–33 close prompt submit, event stream projection, typed request dispatch, server-request registry, and default Cargo retirement. Historical app-server compatibility is non-default quarantine only.

**Absorbs umbrella slice 24E (runtime owner starter seam, wiring half).** Historical wiring allowed delegation while local equivalents were being built. Current closeout state: Plan 31 retires default-path delegation; every remaining compatibility path is explicit non-default Plan 33 material.

## Validation

```bash
cd vac-rs
# Disk guard was checked with Python `shutil.disk_usage` because `df` is blocked by the protected-port shell guard in this workspace.
python3 - <<'PY'
import shutil
for path in ['.', '/tmp']:
    usage = shutil.disk_usage(path)
    print(path, 'free_gb=', round(usage.free / (1024**3), 2))
PY
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-local-runtime-owner
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-local-runtime-owner
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
```

Observed so far:

- Disk guard passed on 2026-05-24: `vac-rs` and `/tmp` each had 22.37 GiB free before the final validation run.
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-local-runtime-owner` passed on 2026-05-24.
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-local-runtime-owner` passed on 2026-05-24: 28 passed, 0 failed.
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed on 2026-05-24.
- `cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass` was not run in the final pass because the required disk guard stopped at 19.93 GiB free, below the 20 GiB threshold.
- `cargo fmt` ran for `vac-core`, `vac-local-runtime-owner`, and `vac-tui`; the workspace emitted the existing stable-channel warning for `imports_granularity = Item`.
- Existing/non-blocking warnings during validation: optional vendored bubblewrap disabled, and pre-existing/deferred dead-code warnings in TUI.
- `cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass` was not run after final edits because the required disk guard stopped at 19.93 GiB free (<20 GiB hard stop).

## Stop conditions

Stop if:

- bootstrap output cannot be made equivalent to current TUI needs,
- model/account behavior regresses,
- the plan requires copying `MessageProcessor`,
- fallback paths become implicit or hidden.

## Done criteria

- Owner has direct retained-resource startup path.
- Local bootstrap data is app-server-free and shaped for TUI adaptation.
- Default-path app-server fallback list is empty.
- Remaining app-server compatibility is non-default Plan 33 delete/defer material.
- 27D is complete for the default owner-native startup seam after later Plan 30/31/33 closeout evidence.
