# Plan 00A — Build unblock before control plane


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented / historical unblock**.

Code evidence:
- `vac-rs/core/src/control_plane/build_check.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented / historical unblock**.

Target outcome: the repository reaches a buildable baseline sufficient for Phase 00 local runtime work and later control-plane implementation.

Outputs: minimal build fixes, validation commands, and evidence that control-plane work is not blocked by unrelated compile failures.

Requires / Blocks: requires clean intent and preservation of unrelated dirty work; blocks 00B and later Phase 00 plans when build is broken.

Stop conditions: stop if build fixes require broad refactors, unrelated source cleanup, or hiding failing tests.

Done criteria: targeted build validation passes and only intended unblock files are changed.


## Goal

Make the root local product buildable enough for reliable implementation.

This plan exists because workflow-control-plane implementation must not start while default local build is blocked by unrelated native/vendor/server-oriented paths.

## Scope

- `cargo check -p vac-surface-cli`,
- sandbox/native dependency build behavior,
- temporary compile compatibility for old runtime crates while they are being removed,
- no new product features.

## Implementation

1. Ensure default local development does not require missing vendored native source when a runtime readiness warning is sufficient.
2. Keep sandbox capability represented as ready/degraded/unavailable at runtime or doctor surface.
3. Do not restore large caches/vendor trees unless explicitly required for release packaging.
4. Keep websocket/API compatibility patches only as temporary unblockers until local path no longer depends on them.
5. Run focused build checks.

## Validation

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 metadata --no-deps --format-version 1
```

## Done

- Root local build is no longer blocked by missing default vendored native source.
- Remaining failures, if any, are product-local and actionable.
- No donor implementation started.
- No new duplicate TUI/runtime path introduced.
