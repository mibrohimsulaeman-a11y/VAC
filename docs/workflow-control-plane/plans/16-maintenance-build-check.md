# Plan 16 — Build check maintenance workflow


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **complete for default targeted build readiness**.
> 3. Capture exit status and concise diagnostics.
> | 7 | P2 | SE | Doctor | ✅ resolved — build readiness is a report-backed doctor route, not an advisory-only label | Keep failure semantics tied to report status |

Code evidence:
- `vac-rs/core/src/control_plane/build_check.rs`
- `vac-rs/core/src/control_plane/build_readiness.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/16-evidence/2026-05-28-sandbox-build-readiness-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete for default targeted build readiness**.

Target outcome: build-check maintenance workflow runs approved validation commands and reports structured progress/results.

Outputs: workflow manifest, safe command definition, progress events, validation evidence, and docs.

Requires / Blocks: requires safe workflow runner and workflow progress TUI.

Stop conditions: stop if build commands run without policy, spawn parallel builds unexpectedly, or hide Cargo lock waits.

Done criteria: build-check workflow runs serially, reports results, and fails visibly on validation failure.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Make build validation a first-class workflow.

## Workflow

```text
.vac/workflows/maintenance.build-check.yaml
```

## Implementation

1. Add typed cargo check capability.
2. Default package: `vac-cli`.
3. Capture exit status and concise diagnostics.
4. Surface result in TUI progress/activity.
5. Do not dump raw secret-bearing environment.

## Validation

- Workflow runs `cargo check -p vac-surface-cli`.
- Success and failure are visible.
- Long output is summarized safely.

## Done

Build health is an operator-visible workflow.

## Current implementation slice

- Workflow execution now fails closed with `build check report unavailable` when no explicit approval-gated build evidence is injected; it no longer falls back to spawning cargo from the report path.
- `vac doctor workflow` / workflow report loading no longer auto-runs cargo check; build-check evidence must be injected by an explicit approval-gated execution path.
- Added typed `BuildCheckRequest` / `BuildCheckReport` plus a fake-cargo-tested allowlisted cargo check executor and safe output summarizer/redactor. Workflow report loading now executes the build check when a workflow uses `capability.build.cargo_check` and `vac-rs/Cargo.toml` exists.
- Standalone execution-machine and approval-resolution paths still fail closed when no build-check report is injected, preserving approval-gated behavior while timeout/process supervision is finished.

- `vac.build` is now `ready` for the default targeted build capability.
- `build_readiness.rs` verifies the ready build manifest, approval-gated `maintenance.build-check` workflow, `vac doctor build` validation route, serial `cargo +1.93.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli` command, and explicit full-workspace operator-gated release evidence policy.
- Full workspace build remains release/operator evidence, not a Plan 16 implementation blocker.
- Evidence: `16-evidence/2026-05-28-sandbox-build-readiness-closeout.md`.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P0 | SE | Runtime | ✅ resolved — `BuildCargoCheck` is implemented through the approval-gated targeted build-check path | Keep default targeted check as the ready build capability; full workspace build remains operator-gated release evidence |
| 2 | P0 | SE | Doctor | ✅ resolved — `vac doctor build` is declared as the build readiness route | Keep route covered by build readiness diagnostics |
| 3 | P1 | SE | Validation | ✅ resolved — default command uses `--manifest-path vac-rs/Cargo.toml -p vac-surface-cli` | Preserve manifest-path form in future docs/tests |
| 4 | P1 | Reviewer | Test | ✅ resolved — readiness harness verifies real build-check contract instead of no-op placeholder behavior | Keep placeholder-free validation |
| 5 | P1 | Architect | Report | ✅ resolved — `build_readiness.rs` provides code-backed readiness reporting for the build capability | Extend report only when new build gates are added |
| 6 | P1 | SE | Validation | ✅ resolved for default docs — command keeps the pinned toolchain explicit and records full workspace build as operator-gated evidence | Do not hide rustup/toolchain assumptions in release docs |
| 7 | P2 | SE | Doctor | ✅ resolved — build readiness is a report-backed doctor route, not an advisory-only label | Keep failure semantics tied to report status |
| 8 | P3 | Planner | Docs | ✅ resolved — plan now documents targeted executor, approval gate, and operator-gated full build evidence | Keep docs synchronized with build workflow manifest |
