# Plan 00E — Old runtime reachability and delete gate


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE_WITH_DEFERRED_PHYSICAL_DELETE**

Source status lines:
> Status: **complete after Plan 33 default local product path retirement; physical workspace crate deletion remains explicitly deferred**.
> ## Status

Code evidence:
- `docs/migration/00E_REACHABILITY_AUDIT.md`

Evidence docs:
- `docs/workflow-control-plane/plans/00E-evidence/2026-05-28-sandbox-delete-gate-supersession.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete after Plan 33 default local product path retirement; physical workspace crate deletion remains explicitly deferred**.

Target outcome: old runtime/app-server delete decisions are based on reachability evidence, not assumptions. If crates remain reachable, deletion is deferred with explicit owner/path evidence.

Outputs: cargo metadata/tree evidence, closeout docs, deferred classification, and handoff to runtime-owner replacement plans.

Requires / Blocks: requires 00F handoff evidence for current classification; blocks any claim that app-server crates can be deleted before Plan 33 inverse trees are clean.

Stop conditions: stop if deletion is attempted while `vac-cli` still reaches `vac-app-server*`, or if docs imply false-green removal.

Done criteria: reachability evidence is recorded, defers are explicit, and future delete/defer proof is assigned to Plan 33.


## Goal

After Local Runtime Contract rewiring, audit old runtime/server/API crates and delete or defer only what is genuinely unreachable from the local product path.

## Scope

Candidate families:

```text
app-server client/server/transport
backend client
cloud requirements
exec server
old API paths no longer needed for local TUI/exec
old protocol DTO-only crates
```

## Implementation

1. Run `cargo metadata`.
2. Run inverse dependency trees from `vac-cli` for old runtime crates.
3. Remove dependencies from `vac-exec`, `vac-tui`, `vac-core` only after local replacements are wired.
4. Delete crates only when unreachable from normal/build/dev edges or explicitly deferred behind non-default future capability.
5. Update workspace members.
6. Re-run build and reachability audits.

## Validation

```bash
cd vac-rs
cargo +1.93.0 metadata --no-deps --format-version 1
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
```

## Done

- Local product path no longer depends on old runtime/server client stack.
- Deleted crates are removed from workspace, or the delete gate records an
  evidence-backed defer decision.
- Deferred crates are classified as future capability / runtime-owner debt, not
  hidden dependency.
- `cargo check -p vac-surface-cli` passes when code changes are made; documentation-only
  closeout slices may use metadata, inverse-tree, and diff checks instead.

## Status

Closed as a two-stage gate:

- **2026-05-23 Phase 00 decision:** physical app-server crate deletion was deferred because the runtime-owner replacement was not yet complete.
- **2026-05-28 Plan 33 supersession:** the default local product Cargo path is retired from active app-server ownership. `vac-tui` default features no longer require direct `vac-app-server` or `vac-app-server-protocol` dependencies; remaining app-server workspace crates are explicit non-default compatibility/delete-defer material.
- **Current operator rule:** do not read the 2026-05-23 inverse-tree evidence as current default-path state. Use Plan 33 closeout evidence for active default-path claims and this plan only as the historical delete-gate handoff.
- **Physical deletion:** full removal of `vac-app-server*` workspace crates remains a future operator-gated slice. It must prove no non-default compatibility, tests, schema generation, or historical evidence paths still depend on those crates.
- `vac-cloud-requirements` is not part of the current workspace tree and is therefore outside the active delete gate.

00E is therefore complete as a historical reachability gate and Plan 33 owns the current delete/defer proof. The default product path may be treated as app-server-retired; physical workspace deletion remains explicitly deferred, not hidden debt.
