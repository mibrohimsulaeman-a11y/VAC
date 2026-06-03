# Plan 33 — App-server Cargo retirement and delete/defer proof


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE_WITH_DEFERRED_PHYSICAL_DELETE**

Source status lines:
> ## Current status
> Status: **complete for default local product Cargo path / workspace crate deletion deferred**.
> - `docs/workflow-control-plane/plans/33-evidence/2026-05-26-baseline-refresh.md` — L6 baseline refresh snapshot after runtime ownership state changes; records current grep snapshot, `vac-tui` tree app-server status, `vac doctor registry ..` output, and delta vs prior baseline.
> Record current grep/tree/status output. Confirm only intended files will be touched.

Code evidence:
- `docs/workflow-control-plane/plans/33-evidence/closeout-evidence-index.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-26-baseline-refresh.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-26-mid-30G-snapshot.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-delete-defer-snapshot.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-consumer-classification.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-cargo-defer-marker.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-default-cargo-cutover.md`
- `docs/workflow-control-plane/plans/33-evidence/baseline-2026-05-25.md`
- `docs/workflow-control-plane/plans/33-evidence/baseline-2026-05-25-post-30E/summary.md`
- `vac-rs/tui/Cargo.toml`

Evidence docs:
- `docs/workflow-control-plane/plans/33-evidence/2026-05-24-plan00f-legacy-transport-reachability.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-24-stage3-summary.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-26-baseline-refresh.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-26-mid-30G-snapshot.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-cargo-defer-marker.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-consumer-classification.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-default-cargo-cutover.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-default-path-retirement-complete.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-delete-defer-snapshot.md`
- `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-direct-tui-dependency-removal.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Remove app-server Cargo edges from the default local product path and produce final delete/defer evidence.

This is the cleanup/proof plan after local runtime owner replacement is complete.

## Current status

Status: **complete for default local product Cargo path / workspace crate deletion deferred**.

Latest baseline refresh: `docs/workflow-control-plane/plans/33-evidence/2026-05-26-baseline-refresh.md`.

Latest mid-30G snapshot: `docs/workflow-control-plane/plans/33-evidence/2026-05-26-mid-30G-snapshot.md`.

Latest sandbox delete/defer snapshot: `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-delete-defer-snapshot.md`.

Latest sandbox consumer classification: `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-consumer-classification.md`.

Latest sandbox Cargo defer-marker evidence: `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-cargo-defer-marker.md`.

Latest sandbox default Cargo cutover evidence: `docs/workflow-control-plane/plans/33-evidence/2026-05-27-sandbox-default-cargo-cutover.md`.

Latest sandbox closeout evidence index: `docs/workflow-control-plane/plans/33-evidence/closeout-evidence-index.md`.

Plan 33 outcome in this sandbox slice:

- `vac-tui` no longer enables app-server crates in the default feature set.
- `vac-tui` no longer lists direct `vac-app-server` or `vac-app-server-protocol` dependencies; remaining TUI app-server Cargo reference is only the optional/non-default `vac-app-server-client` legacy transport feature.
- Remaining app-server references are classified as optional/non-default compatibility or wider workspace historical/protocol consumers.
- Full deletion of app-server crates remains deferred because workspace-wide non-TUI consumers still exist; this is explicit delete/defer proof, not false deletion.

This plan must run last. It is not a refactor plan; it is the evidence-backed delete/defer closure for app-server reachability.

## Evidence index

- `docs/workflow-control-plane/plans/33-evidence/2026-05-26-mid-30G-snapshot.md` — L18 mid-30G snapshot after later mainline 30G/L1-continuation work; records active TUI source grep counts, current `vac-app-server` manifest dependents, `vac doctor registry ..` output, and 30G blocker progress.
- `docs/workflow-control-plane/plans/33-evidence/2026-05-26-baseline-refresh.md` — L6 baseline refresh snapshot after runtime ownership state changes; records current grep snapshot, `vac-tui` tree app-server status, `vac doctor registry ..` output, and delta vs prior baseline.
- `docs/workflow-control-plane/plans/33-evidence/baseline-2026-05-25.md` — prior baseline evidence file.
- `docs/workflow-control-plane/plans/33-evidence/baseline-2026-05-25-post-30E/summary.md` — prior post-30E evidence run summary.

## Target outcome

The default local product path from `vac-cli` no longer reaches app-server crates. Remaining app-server crates are either deleted or explicitly classified as non-default/deferred with owner evidence.

## Outputs

- TUI Cargo dependency removal,
- source grep evidence,
- inverse Cargo tree evidence,
- full validation matrix evidence,
- workspace-wide consumer classification,
- Phase 00 / 00E closeout doc updates,
- delete commit or defer classification.

## Prerequisites

- Plans 25–32 are green.
- TUI uses non-app-server local runtime owner by default.
- Active TUI source path has no `vac_app_server*` imports.
- `.vac` runtime-owner gates exist and pass.
- PTY gate is passed or `BLOCKED-OPERATOR` is recorded honestly.

## Scope

- Remove app-server default dependencies from `vac-rs/tui/Cargo.toml`; app-server compatibility may remain only as optional non-default feature evidence.
- Re-run inverse Cargo trees from `vac-cli`.
- Update Phase 00 / 00E docs with final evidence.
- Delete app-server crates only if workspace-wide consumers are gone.
- Otherwise classify app-server crates as explicit non-default/deferred capability crates.

## Non-goals

- No deletion before inverse-tree evidence.
- No broad workspace cleanup/prune.
- No unrelated dirty work staging.
- No code movement to make deletion look green.

## Required checks

### Source grep

```bash
git grep -n "vac_app_server\|vac_app_server_client\|vac_app_server_protocol\|vac_app_server_transport" -- vac-rs/tui/src || true
```

Allowed remaining matches: historical docs/comments only, or explicit non-default/deferred path documented in this plan and closeout docs.

### Cargo inverse tree

```bash
cd vac-rs
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-transport --edges normal,build
```

Expected: no path from `vac-cli` through default local product path.

## Implementation slices


### 2026-05-27 sandbox slice — delete/defer snapshot

Recorded `33-evidence/2026-05-27-sandbox-delete-defer-snapshot.md` after the runtime-owner threshold batch. The snapshot keeps Plan 33 blocked because active TUI/app-server compatibility references and watched Cargo app-server dependencies remain nonzero.

### 33A — Pre-removal evidence

Record current grep/tree/status output. Confirm only intended files will be touched.

### 33B — TUI dependency removal

Remove from `vac-rs/tui/Cargo.toml` if no active source imports remain:

- `vac-app-server`
- `vac-app-server-client`
- `vac-app-server-protocol`

### 2026-05-27 sandbox slice — defer marker proof before Cargo removal

Recorded `33-evidence/2026-05-27-sandbox-cargo-defer-marker.md`. The remaining active seam is now explicitly marked as Plan 31 compatibility quarantine. Default product Cargo dependency removal is complete; remaining optional legacy compatibility is explicit workspace deletion/defer evidence and must not appear on the default TUI path.

### 33C — Full validation matrix

Run TUI/core/CLI checks and nextest targets.

### 33D — Workspace-wide app-server consumer audit

Search all workspace Cargo manifests and source imports. Classify remaining consumers.


### 2026-05-27 sandbox slice — 33D consumer classification

Recorded `33-evidence/2026-05-27-sandbox-consumer-classification.md`. The classification keeps Plan 33 blocked and explicitly rejects Cargo dependency deletion while `tui`, `session_protocol.rs`, and `app_server_session.rs` still carry active compatibility ownership.

### 33E — Delete or defer

If no consumers remain, delete app-server crates and workspace dependencies.

If consumers remain, do not delete. Mark app-server crates as explicit non-default/deferred capability crates and record the owner/path.

### 33F — Closeout docs

Update:

- `docs/migration/PHASE00_CLOSEOUT_STATUS.md`
- `docs/migration/00E_REACHABILITY_AUDIT.md`
- `docs/workflow-control-plane/plans/00E-runtime-reachability-delete-gate.md`
- `docs/workflow-control-plane/plans/24-local-runtime-owner-replacement.md` if needed


### 2026-05-27 sandbox slice — direct TUI protocol/server dependency removal

Recorded `33-evidence/2026-05-27-sandbox-direct-tui-dependency-removal.md`. `vac-rs/tui/Cargo.toml` no longer declares direct `vac-app-server` or `vac-app-server-protocol` dependencies. The default feature set stays empty, and the only TUI-side app-server Cargo edge left is the optional `vac-app-server-client` transport under `legacy-app-server-compat`. Workspace-wide app-server crate deletion remains deferred because app-server crates and their tests still own non-default/historical compatibility surfaces.

## Validation

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-transport --edges normal,build
```

## Stop conditions

Stop if:

- any inverse tree still reaches app-server through the default `vac-cli` path,
- source grep shows active TUI app-server imports,
- validation fails after dependency removal,
- workspace-wide consumers are unclear,
- deletion would remove a still-owned non-default capability.

## Done criteria

- Default local product path no longer reaches app-server crates.
- TUI Cargo default feature path no longer depends on app-server crates; remaining app-server crates are optional/non-default or outside the default product path.
- App-server crates are deleted or explicitly classified as non-default/deferred.
- Phase 00 / 00E docs show truthful final evidence.
- No false-green deletion claim remains.


## 2026-05-28 sandbox sync

- Default product Cargo path retirement is complete; remaining `vac-app-server*` workspace crates are historical/non-default compatibility cleanup, not a default TUI path blocker.
