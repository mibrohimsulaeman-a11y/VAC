# Plan 32 — `.vac` runtime-owner gates


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> ## Current status
> Status: **complete for default runtime-owner hard gates** — scaffold subcommand landed; L25 threshold proposal recorded in `32-evidence/2026-05-26-threshold-proposal.md`. L-32YAML authored the three optional workflow/policy manifests recorded in `32-evidence/2026-05-27-optional-manifest-ops.md`. Plan 32F elevated `missing_field` to a hard error, and the 2026-05-27 sandbox threshold batch added warning/error surfaces for app-server dependency presence, PTY false-green prevention, MessageProcessor copy prevention, and unsupported-control default defers after Plan 30/31/33 default-path prerequisites closed.
> Draft manifest content and validate against current schemas. Keep status non-ready until validation commands exist.
> - Updated the runtime-owner-gates regression test to assert non-zero exit, `status: fail`, and `level: error` for a missing required field.

Code evidence:
- `vac-rs/cli/src/doctor/runtime_owner_gates.rs`
- `vac-rs/cli/tests/runtime_owner_gates.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/32-evidence/2026-05-26-threshold-proposal.md`
- `docs/workflow-control-plane/plans/32-evidence/2026-05-27-optional-manifest-ops.md`
- `docs/workflow-control-plane/plans/32-evidence/2026-05-27-sandbox-runtime-owner-threshold-batch.md`
- `docs/workflow-control-plane/plans/32-evidence/2026-05-28-sandbox-hard-gate-promotion.md`
- `docs/workflow-control-plane/plans/32-evidence/2026-05-28-sandbox-threshold-doc-refresh.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Declare and enforce runtime-owner replacement gates in `.vac` after the code path is mature enough to validate.

`.vac` remains declarative. It must not become runtime business logic.

## Current status

Status: **complete for default runtime-owner hard gates** — scaffold subcommand landed; L25 threshold proposal recorded in `32-evidence/2026-05-26-threshold-proposal.md`. L-32YAML authored the three optional workflow/policy manifests recorded in `32-evidence/2026-05-27-optional-manifest-ops.md`. Plan 32F elevated `missing_field` to a hard error, and the 2026-05-27 sandbox threshold batch added warning/error surfaces for app-server dependency presence, PTY false-green prevention, MessageProcessor copy prevention, and unsupported-control default defers after Plan 30/31/33 default-path prerequisites closed.

This plan should start only after code can satisfy the gates. It must not create gates that are permanently red or fake-green. The initial `vac doctor runtime-owner-gates <root>` command is informational and reports warning-level diagnostics for skeleton manifest/source-domain issues without making those warnings CI-breaking yet. L25 proposes the concrete warning-to-error schedule; L-32YAML adds the optional declarative manifests as `planned` evidence surfaces without changing runtime code or claiming enforcement.

## Target outcome

`.vac` declares runtime-owner ownership and maintenance gates that can detect app-server regressions, PTY false-green, and unsupported control-path defers.

## Outputs

- runtime-owner capability manifests,
- TUI runtime/approval bridge manifests,
- no-app-server-local-path workflow,
- runtime-owner-gate workflow,
- policy preventing `MessageProcessor` copy and false-green deletion,
- registry/schema tests.

## Requires / Blocks

Requires:

- Plan 31 protocol compatibility retirement,
- control-plane manifest/schema support from Plans 02–07.

Blocks:

- Plan 33 final delete/defer proof.

## Prerequisites

- Plans 25–31 have produced a working local runtime owner path.
- Capability/workflow/policy schemas from earlier control-plane plans are ready.
- The no-app-server local path can be checked with real evidence.

## Expected manifests

```text
.vac/capabilities/local_runtime_owner.yaml
.vac/capabilities/tui_session_runtime.yaml
.vac/capabilities/runtime_approval_bridge.yaml
.vac/workflows/maintenance.runtime-owner-gate.yaml
.vac/workflows/maintenance.no-app-server-local-path.yaml
.vac/policies/runtime-owner-replacement.yaml
```

Exact filenames may change if registry conventions require it, but the capability/policy meaning must remain explicit.

## Gate semantics

- local runtime owner capability has a real Rust owner,
- TUI session runtime points at the owner path,
- approval bridge is explicitly covered,
- inverse Cargo tree must be clean before deletion is green,
- `vac_app_server*` imports in active TUI runtime path fail the gate,
- PTY gate cannot pass without a real TTY,
- `MessageProcessor` copy is forbidden,
- unsupported controls must be blockers or explicit non-default defers.


## Threshold proposal — 2026-05-26 L25

Evidence: `docs/workflow-control-plane/plans/32-evidence/2026-05-26-threshold-proposal.md`.

The original threshold proposal began with staged warnings. After the 2026-05-28 default-path closeout, default runtime-owner regressions are hard errors. Historical warning-level rationale remains in evidence files only; the active matrix below is the current contract.

| Finding code | Current level | Proposed hard-error point | Preconditions / linked plans | Acceptance criteria |
| --- | --- | --- | --- | --- |
| `missing_field` | hard error | Already promoted in Plan 32F | Required manifest fields are stable | Missing required Plan 32 manifest fields make the doctor fail. |
| `missing_source_domain` | hard error | Promoted after Plan 31/33 default-path closeout | Ownership paths are canonical for the default product path | Declared owner/docs/crate/module/path targets must exist and match intended Rust source domains. |
| `app_server_dependency_present` | hard error | Promoted after default TUI Cargo path retirement | Remaining app-server workspace crates are non-default/historical defer material | Active local runtime/TUI owner paths must not reintroduce direct app-server dependencies. |
| `pty_false_green` | hard error / blocked evidence | Promoted after release policy accepted `BLOCKED-OPERATOR` as non-pass | Synthetic/no-operator PTY evidence cannot be green | A PTY gate without real TTY evidence must fail or report `BLOCKED-OPERATOR`, never pass. |
| `message_processor_copy` | hard error | Active from first source-domain scan | Copying `MessageProcessor` into owner paths is a regression | Active owner/TUI paths must not recreate the legacy app-server message processor. |
| `unsupported_control_default_defer` | hard error | Promoted after Plan 30 owner-native parity registry | Unsupported controls are owner-native, blocked, or explicit non-default defer | Unsupported controls cannot be silently enabled on the default path. |


## Non-goals

- No runtime command execution inside `.vac` beyond maintenance validation workflows.
- No `.vac` gate before code can satisfy it.
- No broad `.vac/**` rewrite.

## Implementation slices

### 32A — Manifest design

Draft manifest content and validate against current schemas. Keep status non-ready until validation commands exist.

### 32B — Runtime owner capability

Add `local_runtime_owner` capability manifest with owner crate/module, validation commands, and known surfaces.

### 32C — TUI session/runtime approval capabilities

Add TUI runtime and approval bridge manifests so ownership is visible in dashboard/workflow surfaces.

### 32D — Maintenance workflows

Ready manifests now exist for `maintenance.no-app-server-local-path` and `maintenance.runtime-owner-gate`; see `32-evidence/2026-05-27-optional-manifest-ops.md`. They are enforceable for the default product path after Plan 31/33 closeout.

### 32E — Policy wiring

Ready policy manifest `vac.runtime-owner-replacement` now exists; see `32-evidence/2026-05-27-optional-manifest-ops.md`. It records conservative local gate policy only and is paired with runtime-owner source scanning for false-green deletion and `MessageProcessor` copying.

### 2026-05-27 sandbox slice — 32F missing-field threshold

- Elevated `missing_field` from warning to error in `vac doctor runtime-owner-gates`, so required Plan 32 manifest fields now fail the gate instead of reporting a green exit with warnings.
- Updated the runtime-owner-gates regression test to assert non-zero exit, `status: fail`, and `level: error` for a missing required field.
- Promoted `missing_source_domain` to hard error after Plan 31/33 default-path prerequisites closed.

### 2026-05-27 sandbox batch — 32G app-server/ownership target gate tightening

- Added warning-level `app_server_import_present` scanning for active runtime-owner/TUI source paths that still reference `vac_app_server*` or `AppServerSession`; this records Plan 31 compatibility-retirement evidence without making the current deferred path fake-red before Plan 31/33 close.
- Tightened nested ownership target schema enforcement: `ownership.targets[]` entries with a `crate_name` but no `module` now emit hard `missing_field` errors, matching the Plan 32F required-field threshold principle.
- Updated runtime-owner-gates regression coverage for active app-server source imports and missing ownership target modules.
- Improved text rendering mode wording so operators can distinguish hard errors (`missing_field`, `message_processor_copy`) from staged warnings (`app-server`, `pty`, `unsupported-control`).

## Validation

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-core
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

Also run the control-plane registry/manifest tests identified by Plans 02–07.


### 2026-05-27 sandbox slice — 32G runtime-owner threshold batch

- Added runtime-owner gate coverage for `app_server_dependency_present`, `pty_false_green`, `message_processor_copy`, and `unsupported_control_default_defer` so the gate now reports Plan 30/31/33 blockers as explicit diagnostics instead of green owner proof.
- App-server dependency/import, PTY false-green, and unsupported-control findings are hard errors for the default product path after Plan 30/31/33 closeout.

### 2026-05-27 sandbox slice — 32H docs threshold sync

Updated the threshold matrix so the Plan 32 table matches the implemented runtime-owner gate codes from the sandbox threshold batch. App-server dependency/import, PTY false-green, MessageProcessor copy, and unsupported-control default-defer checks are no longer described as `not implemented`; their hard-error posture is active for default-path regressions after Plan 30/31/33 closeout.
- Kept `message_processor_copy` as a hard error and extended `missing_field` hardening to malformed `ownership.targets[]` entries that omit `crate_name`.
- Evidence: `32-evidence/2026-05-27-sandbox-runtime-owner-threshold-batch.md`.

### 2026-05-26 L4 scaffold validation evidence

- `rustfmt +1.93.0 --check cli/src/doctor/runtime_owner_gates.rs cli/tests/runtime_owner_gates.rs` passed after scoped formatting.
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 nextest run -p vac-surface-cli runtime_owner_gates` passed: 4 tests run, 4 passed, 27 skipped.
- `./target/debug/vac doctor runtime-owner-gates ..` exited 0 and reported `status: green`, `manifests_checked=3`, `warnings=0`, `errors=0`.
- Full `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 nextest run -p vac-surface-cli` is blocked by pre-existing/out-of-scope doctor fixture failures in existing doctor tests (`ownership_doctor_renders_ownership_matrix`, release/workflow/architecture doctor tests). L4 did not expand scope to edit those fixtures or other doctor subcommand modules.

## Stop conditions

Stop if:

- gate implementation would require runtime business logic inside `.vac`,
- manifest schemas cannot express the ownership safely,
- no-app-server gate cannot distinguish active vs historical references,
- PTY `blocked_operator` would be treated as pass by default.

## Done criteria

- `.vac` declares runtime-owner capability and gates.
- Gates fail on app-server local-path regressions.
- Gates do not fake PTY success.
- Runtime logic remains in Rust.


## 2026-05-28 sandbox closeout

- Hard-gate promotion evidence: [32-evidence/2026-05-28-sandbox-hard-gate-promotion.md](32-evidence/2026-05-28-sandbox-hard-gate-promotion.md).
- `missing_source_domain`, default app-server regressions, false-green PTY evidence, and unsupported default controls are now hard errors.


## 2026-05-28 sandbox threshold documentation refresh

See [32-evidence/2026-05-28-sandbox-threshold-doc-refresh.md](32-evidence/2026-05-28-sandbox-threshold-doc-refresh.md). The active Plan 32 text now treats default runtime-owner regressions as hard errors rather than staged warnings.
