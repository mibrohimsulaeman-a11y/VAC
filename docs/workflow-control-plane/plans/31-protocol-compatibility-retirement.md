# Plan 31 — Protocol compatibility retirement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE_DEFAULT_PATH**

Source status lines:
> ## Current status
> Status: **complete for the default TUI DTO owner-native path** — 31A inventory, 31B mapping, 31C defer-marker, 31C owner-native default cutover, and the final owner-native runtime-protocol shim are recorded. Active TUI source no longer imports app-server protocol/client crates on the default path; legacy app-server session transport is isolated behind `legacy-app-server-compat` as non-default delete/defer evidence.

Code evidence:
- `vac-rs/runtime-protocol`
- `vac-rs/core/src/local_runtime`
- `vac-rs/tui/src/session_protocol.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/31-evidence/2026-05-26-app-server-protocol-dep-graph.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-authmode-relocation.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-compat-defer-marker.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-compatibility-inventory.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-dto-owner-mapping.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-owner-native-default-cutover.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-owner-native-runtime-protocol-complete.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-runtime-protocol-dto-closure.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-threaditem-relocation.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-28-sandbox-mapping-superseded.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Retire app-server protocol compatibility from the active TUI local runtime path.

This plan cleans the compatibility quarantine after the owner path is functionally equivalent.

## Current status

Status: **complete for the default TUI DTO owner-native path** — 31A inventory, 31B mapping, 31C defer-marker, 31C owner-native default cutover, and the final owner-native runtime-protocol shim are recorded. Active TUI source no longer imports app-server protocol/client crates on the default path; legacy app-server session transport is isolated behind `legacy-app-server-compat` as non-default delete/defer evidence.

This plan starts only after Plan 30 proves behavior. It is cleanup of active protocol ownership, not a behavior migration shortcut.

## Target outcome

Active TUI local runtime code no longer owns or imports app-server protocol DTOs. Any remaining compatibility is historical, test-only, or explicitly non-default/deferred.

## Outputs

- compatibility inventory,
- DTO owner mapping table,
- replacement owner DTOs/conversion wrappers,
- conversion tests,
- grep evidence for remaining `vac_app_server*` references.

## Requires / Blocks

Requires:

- Plan 30 prompt/control cutover.

Blocks:

- Plan 32 `.vac` no-app-server gates,
- Plan 33 Cargo dependency removal.

## Current seam

`vac-rs/tui/src/session_protocol.rs` currently mixes:

- owner-migrated DTO re-exports,
- app-server compatibility aliases,
- conversion helpers,
- `ThreadSourceKind` compatibility mapping,
- app-server `ClientRequest`, `ServerRequest`, and notification DTOs used by the old adapter.

## Target state

- TUI active runtime code does not import `vac_app_server*`.
- `session_protocol.rs` contains only local owner/product DTOs or deliberate compatibility for non-default deferred paths.
- Every conversion has a test or explicit evidence.

## Non-goals

- No name-only migration.
- No Cargo dependency removal until grep and compile evidence are clean.
- No behavior changes without tests.

## Implementation slices

### 31A — Compatibility inventory

Run targeted grep for:

```text
vac_app_server
vac_app_server_client
vac_app_server_protocol
ClientRequest
ServerRequest
ServerNotification
TuiSessionEvent
AppServerSession
```

Classify each as active, historical, test-only, or deferred non-default.


### 2026-05-27 sandbox slice — 31A compatibility inventory

Recorded current TUI/app-server compatibility footprint in `31-evidence/2026-05-27-sandbox-compatibility-inventory.md`. This slice does not remove dependencies; it makes the active compatibility bridge and Cargo edges explicit input for 31C/33B.

### 31B — DTO owner mapping

For each active DTO, map to:

- existing `vac_protocol` / `vac_core::local_runtime` type,
- new local owner type,
- explicit conversion wrapper,
- blocker/defer.


### 2026-05-27 sandbox slice — 31B DTO owner mapping

Recorded current DTO/adapter ownership in `31-evidence/2026-05-27-sandbox-dto-owner-mapping.md`. The mapping narrows the remaining active compatibility seam to `session_protocol.rs` compatibility aliases/conversions and the deferred `app_server_session.rs` adapter. This slice does not remove imports; it defines the replacement/defer boundary for 31C.

### 31C — Replace active imports

Replace active app-server protocol imports in TUI runtime path with owner DTOs or conversion wrappers.

### 2026-05-27 sandbox slice — 31C explicit compatibility defer marker

Recorded `31-evidence/2026-05-27-sandbox-compat-defer-marker.md`. The remaining app-server DTO/session seam now carries `VAC_RUNTIME_OWNER_COMPAT_DEFER: plan31-app-server-compat-quarantine`, and `vac doctor runtime-owner-gates` condenses marked files into `app_server_compat_defer_present`. This is explicit delete/defer evidence, not completion of owner-native DTO replacement.

### 31D — Delete compatibility dead code

Remove unused aliases/conversions only after compile and tests prove they are unused.

### 2026-05-27 sandbox slice — 31C owner-native default cutover

Recorded `31-evidence/2026-05-27-sandbox-owner-native-default-cutover.md` and `31-evidence/2026-05-27-sandbox-owner-native-runtime-protocol-complete.md`. The active TUI session facade now routes remaining app-server-shaped DTO names through `vac-runtime-protocol`, a product/runtime protocol crate, instead of importing app-server crates directly. `vac-rs/tui/Cargo.toml` no longer enables app-server crates by default; the legacy adapter is gated by `legacy-app-server-compat`.


### 2026-05-27 sandbox slice — 31D runtime-protocol DTO ownership closure

Recorded `31-evidence/2026-05-27-sandbox-runtime-protocol-dto-closure.md`. The default TUI DTO facade now imports `vac-runtime-protocol` directly for the remaining session wire shapes. `vac-rs/tui/Cargo.toml` no longer lists `vac-app-server` or `vac-app-server-protocol`; the only remaining TUI app-server Cargo reference is the optional/non-default legacy client transport used by `legacy-app-server-compat`. This closes Plan 31 for the default owner-native path while preserving a quarantined rollback seam.

## 2026-05-27 sandbox closeout — default owner-native DTO path

Plan 32G originally introduced runtime-owner warnings for active app-server imports. The current Plan 31D closeout resolves those warnings for the default TUI DTO facade by moving remaining session wire DTOs into `vac-runtime-protocol`.

Current meaning:

- `session_protocol.rs` and `session_protocol_compat.rs` do not import `vac_app_server*` crates on the default path.
- Remaining `AppServer*` identifier names are compatibility names for existing TUI call sites; their type ownership is now `vac-runtime-protocol` or local owner-native request/session handles.
- Legacy app-server transport remains non-default behind `legacy-app-server-compat` and is delete/defer evidence, not part of the default local product path.

## Validation

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
git grep -n "vac_app_server_protocol\|vac_app_server_client\|vac_app_server" -- tui/src || true
```

Remaining matches must be historical comments, deleted code, or explicitly documented non-default defers.

## Stop conditions

Stop if:

- a DTO cannot be mapped without semantic ambiguity,
- conversion tests are missing for non-trivial mappings,
- removal would hide an active fallback path,
- grep evidence still shows active app-server protocol ownership.

## Done criteria

- Active TUI local runtime path has no direct app-server protocol/client imports in default-owned source files.
- Remaining wire DTO compatibility in the default path is owned by `vac-runtime-protocol`, not app-server protocol/client crates.
- Legacy app-server session transport is non-default quarantine and can be deleted later without blocking the default product path.


## 2026-05-28 sandbox sync

- Plan 30 is now complete for the default owner-native TUI product path; Plan 31 remains complete for DTO ownership, with only non-default legacy cleanup deferred to Plan 33.
