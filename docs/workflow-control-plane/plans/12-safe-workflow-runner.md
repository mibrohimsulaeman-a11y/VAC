# Plan 12 — Minimal safe workflow runner


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented baseline; active contract is minimal safe execution only**.
> - Donor migration workflow steps are now mapped to typed native handlers backed by `DonorStatusReport` and the canonical `vac.donor_migration` capability; the aggregate `capability.donor_migration.gate` alias is deprecated and no longer recognized.
> | 6 | P1 | SE | Runtime | `DonorMigration*` handler fall through | Resolved: granular donor handlers now consume `DonorStatusReport` and fail closed when unavailable or failing. |

Code evidence:
- `vac-rs/core/src/control_plane/workflow_runner.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented baseline; active contract is minimal safe execution only**.

Target outcome: workflow runner executes a constrained typed step vocabulary with policy checks, progress reporting, and no arbitrary execution by default.

Outputs: typed runner, allowed step vocabulary, forbidden step handling, policy checks, tests, and progress events.

Requires / Blocks: requires workflow schema and policy schema; blocks workflow progress, maintenance workflows, release gate, and PTY workflows.

Stop conditions: stop if runner accepts untyped commands, bypasses policy, or cannot report failed/skipped states.

Done criteria: safe steps run, forbidden steps fail deterministically, and tests cover lifecycle states.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Run only typed, safe built-in workflow steps.

## Current typed safe step vocabulary

The source of truth lives in `vac-rs/core/src/control_plane/workflow_runner.rs` as `INITIAL_ALLOWED_STEP_USES` plus `WORKFLOW_STEP_VOCABULARY`. The current typed handlers are:

```text
capability.root_seed.coverage          -> root_seed.coverage
capability.build.cargo_check          -> build.cargo_check
capability.identity.check             -> identity.check
capability.tui.no_duplicate_runtime   -> tui.no_duplicate_runtime
capability.ownership.scan             -> ownership.scan
capability.architecture.invariants    -> architecture.invariants
capability.activity.emit              -> activity.emit
capability.approval.request           -> approval.request
capability.tui.pty_gate               -> tui.pty_gate
capability.donor_migration.inventory_check      -> donor_migration.inventory_check
capability.donor_migration.drift_check          -> donor_migration.drift_check
capability.donor_migration.manifest_check       -> donor_migration.manifest_check
capability.donor_migration.reachability_check   -> donor_migration.reachability_check
capability.donor_migration.evidence_check       -> donor_migration.evidence_check
capability.donor_migration.commit_phrase_check  -> donor_migration.commit_phrase_check
```

All entries remain typed/report-backed. None of these handlers may execute arbitrary shell commands directly.

## Forbidden initially

```text
arbitrary shell
unapproved file mutation
hidden network access
running donor code directly
```

## Implementation

1. Add workflow execution state machine.
2. Validate workflow before run.
3. Resolve `uses:` references to built-in capability handlers.
4. Emit lifecycle events.
5. Stop on policy violation.
6. Reuse typed diagnostic reports for root seed coverage, identity, TUI uniqueness, architecture invariants, and ownership scan.

## Validation

- `maintenance.build-check` dry-runs.
- Running unsupported step fails safely.
- Lifecycle events are produced.
- CLI and TUI render from the same core workflow run report.
- Typed diagnostic handlers fail safely from report state instead of shelling out.

## Done

Workflow execution exists but remains safe and typed.

## Current implementation slice

- The workflow browser now renders a runner preview for each loaded workflow manifest.
- The preview is intentionally conservative: only the initial safe step vocabulary is treated as runnable.
- The seeded maintenance workflows now use the initial safe step vocabulary and dry-run as supported by the preview gate.
- `vac doctor workflow` now prints the same typed runner report so operators can validate the safe-runner gate from the root CLI.
- The typed runner report now resolves each step to a built-in handler when possible, so both the CLI and TUI can show supported vs blocked step resolution without executing the workflow.
- The runner report now emits a dry-run event sequence per workflow, which gives operators a typed validation trace without executing any step handler.
- The dry-run report now also carries a typed state trace, so the runner preview can show the workflow lifecycle transitions as well as the emitted events.
- The runner report now also carries a typed execution progress trace with running, waiting approval, success, failure, and cancelled states, so the CLI and TUI can observe workflow lifecycle without shell execution.
- Workflow approval requests can be persisted through the SQLite-backed `FileApprovalStore` and re-opened across process lifetimes. The store keeps typed approval payloads in `approval_requests` rows and migrates the earlier JSON draft format on first open.

- The `/workflow` TUI browser now renders from the same core workflow run report used by `vac doctor workflow`, keeping root seed, architecture, ownership, policy, dry-run, and execution progress context aligned across surfaces.
- `capability.ownership.scan` now consumes the ownership scan report during typed execution and fails safely when ready capabilities lack ownership metadata.
- Donor migration workflow steps are now mapped to typed native handlers backed by `DonorStatusReport` and the canonical `vac.donor_migration` capability; the aggregate `capability.donor_migration.gate` alias is deprecated and no longer recognized.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P0 | SE | Runtime | **Fallthrough success** di `WorkflowExecutionMachine::advance` — `match WorkflowStepHandler` tidak exhaustive; handler tanpa eksekusi → generic success | Exhaustive match; setiap handler tanpa eksekusi nyata → `fail_current_step` dengan reason executor-not-implemented |
| 2 | P0 | SE | Runtime | `BuildCargoCheck` handler previously lacked a real executor | Resolved: handler invokes `build_check::run_build_check`, supports injected reports for tests, and fails closed on nonzero cargo output. |
| 3 | P1 | SE | Runtime | `TuiPtyGate` previously risked approval bypass in headless environments | Resolved by Plan 23 policy: `BLOCKED-OPERATOR` is explicit non-pass evidence; no fake PTY pass. |
| 4 | P1 | SE | Runtime | Identity report `None` → silent success (asimetri vs NoDuplicateTui/Architecture/Ownership) | Symmetric fail-closed |
| 5 | P1 | SE | Runtime | `ActivityEmit` fall through ke `StepSucceeded` | Implement emit |
| 6 | P1 | SE | Runtime | `DonorMigration*` handler fall through | Resolved: granular donor handlers now consume `DonorStatusReport` and fail closed when unavailable or failing. |
| 7 | P2 | SE | Code | Duplicated failure code di multiple handler | Centralize |
| 8 | P2 | Reviewer | Test | Negative test untuk handler unimplemented tidak ada | Tambah |
| 9 | P2 | SE | UX | Preview/dry-run misleading ("supported") padahal tidak execute | Reflect actual handler state |
