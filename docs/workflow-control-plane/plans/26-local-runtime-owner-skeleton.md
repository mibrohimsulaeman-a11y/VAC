# Plan 26 — Local runtime owner skeleton


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> ## Current status
> Status: **implemented — owner skeleton crate added (2026-05-24)**.

Code evidence:
- `vac-rs/core/src/local_runtime/session.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/26-evidence/2026-05-28-sandbox-owner-skeleton-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Create the non-app-server lifecycle owner skeleton that will eventually implement the TUI `LocalRuntimeSession` path.

This plan creates structure and compile-time ownership seams only. It must not change runtime behavior by default.

## Current status

Status: **implemented — owner skeleton crate added (2026-05-24)**.

Plan 25 has identified the semantic-contract boundary and lifecycle gaps. Plan 26 selected the preferred dedicated crate placement and added a compile-only skeleton without changing TUI behavior.

## Target outcome

A compile-only non-app-server owner skeleton exists, with explicit placement and module boundaries, but the TUI still runs through the existing behavior path.

## Outputs

- selected owner placement recorded,
- crate/module skeleton,
- compile-only tests,
- workspace/Cargo edits only if the selected placement requires them,
- no TUI behavior change.

## Execution note — 2026-05-24

Implemented the preferred dedicated crate placement:

```text
vac-rs/local-runtime-owner/
```

Added the planned skeleton modules and empty public seam types:

- `LocalRuntimeOwner`
- `LocalRuntimeOwnerSession`
- `RuntimeRetainedResources`
- `RuntimeCommandBus`
- `RuntimeEventStream`
- `ServerRequestRegistry`
- `RuntimeShutdownHandle`

Cargo changes were limited to adding the workspace member and workspace dependency entry for `vac-local-runtime-owner`. The crate directly depends only on `vac-core` and includes compile-only tests that instantiate the skeleton, touch the `vac_core::local_runtime` semantic contract, and assert the crate manifest does not name forbidden app-server crates. No TUI code path was rewired and no app-server Cargo edge was removed.

## Requires / Blocks

Requires:

- Plan 25 semantic contract audit.

Blocks:

- Plan 27 retained-resource startup work,
- Plan 28 registry ownership work,
- Plan 29 event stream ownership work.

## Dependency on previous plans

- Plan 24 defines the umbrella architecture and safety gates.
- Plan 25 clarifies the semantic contract available in `vac_core::local_runtime`.

## Placement decision

Selected placement (2026-05-24): `vac-rs/local-runtime-owner/` as package `vac-local-runtime-owner`. The preflight found no existing conflicting crate/module; the new crate depends only on `vac-core` for the compile-only semantic seam and avoids all forbidden `vac-app-server*` crates.

Preferred placement:

```text
vac-rs/local-runtime-owner/
```

Package name:

```text
vac-local-runtime-owner
```

Allowed alternative: an existing crate/module only if dependency analysis proves it will not pull TUI/app-server concerns into `vac-core`.

## Skeleton modules

```text
src/lib.rs
src/session.rs
src/startup.rs
src/command_bus.rs
src/event_stream.rs
src/server_requests.rs
src/retained_resources.rs
src/shutdown.rs
```

## Allowed dependencies

- `vac-core`
- `vac-config`
- `vac-thread-store`
- `vac-login`
- `vac-state`
- `vac-protocol`
- `vac-model-provider`
- `vac-core-skills`
- `vac-core-plugins`
- `vac-exec-server` only while structurally required

## Forbidden final dependencies

- `vac-app-server`
- `vac-app-server-client`
- `vac-app-server-protocol`
- `vac-app-server-transport`

## Non-goals

- No TUI behavior change.
- No app-server Cargo edge removal.
- No `.vac` manifest changes unless schemas/tests are ready.
- No bulk copy from `app_server_session.rs`.
- No copy of app-server `MessageProcessor`.

## Implementation slices

### 26A — Placement preflight

- Inspect `vac-rs/Cargo.toml`, `vac-rs/tui/Cargo.toml`, and current dirty work.
- Confirm no conflicting crate/module exists.
- Record selected placement in this plan if different from preferred crate.

### 26B — Create skeleton

- Add the selected crate/module.
- Define empty public structs/traits only where needed:
  - `LocalRuntimeOwner`
  - `LocalRuntimeOwnerSession`
  - `RuntimeRetainedResources`
  - `RuntimeCommandBus`
  - `RuntimeEventStream`
  - `ServerRequestRegistry`
  - `RuntimeShutdownHandle`

### 26C — Compile-only tests

- Add minimal tests proving the skeleton compiles and does not depend on app-server crates.
- No behavior claim yet.
- **Absorbs umbrella slice 24E (runtime owner starter seam, compile half).** Historical skeleton work allowed a compile-only delegating seam. Current closeout state: Plan 31 retired default-path delegation; any legacy adapter is non-default compatibility material only.

## Validation

If using new crate:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-local-runtime-owner
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

If using an existing crate, replace the first command with that package.

## Stop conditions

Stop if:

- the selected placement would require final dependencies on `vac-app-server*`,
- adding the crate requires broad unrelated workspace edits,
- the skeleton starts changing runtime behavior,
- dependency analysis would pull TUI concerns into `vac-core`.

## Done criteria

- Owner skeleton exists.
- It compiles.
- It has no app-server dependency.
- TUI behavior is unchanged.
