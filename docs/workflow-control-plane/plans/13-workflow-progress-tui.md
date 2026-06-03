# Plan 13 — Workflow progress and lifecycle in TUI


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented baseline; active contract is visible workflow lifecycle**.

Code evidence:
- `vac-rs/tui/src/workflow_progress.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented baseline; active contract is visible workflow lifecycle**.

Target outcome: TUI shows workflow run lifecycle, progress, results, and failures from runner events.

Outputs: progress state model, TUI rendering, lifecycle tests/snapshots, and validation evidence.

Requires / Blocks: requires safe workflow runner; blocks maintenance/release workflow operator confidence.

Stop conditions: stop if workflow execution is backend-only or failures are not visible in TUI.

Done criteria: pending/running/succeeded/failed/blocked states render correctly and update during runs.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Make workflow execution observable.

## Lifecycle states

```text
pending
running
waiting_approval
success
failed
cancelled
```

## Implementation

1. Add workflow progress model in TUI.
2. Render each step with lifecycle state.
3. Show failure reason and recovery hint.
4. Show approval wait state.
5. Link workflow events to activity log.

## Validation

- Running maintenance workflow shows progress.
- Failed step shows exact error.
- Cancelled workflow shows terminal state.

## Done

Workflows never execute invisibly.

## Current implementation slice

- The control-plane runner now exposes a typed execution progress report with lifecycle states, progress counters, approval wait, failure, and cancellation.
- The workflow browser now projects that report through a dedicated TUI progress model, rendering each step with `pending`, `running`, `waiting_approval`, `success`, `failed`, or `cancelled` state and showing the typed workflow activity log trace.
- `vac doctor workflow` already renders the same execution progress report as text, so the operator-facing observable surface exists while the dedicated progress panel continues to evolve.
- The TUI progress model now has explicit regression coverage for cancelled terminal state, including cancellation reason and unchanged pending steps.
- The TUI progress model now has explicit regression coverage for the success terminal state, ensuring all-succeeded steps render the success lifecycle and a `finished` event in the activity log.
- The TUI progress model now has stronger terminal activity-log regression coverage for cancelled and failed states, including cancellation events, failure events, detail, and recovery hint rendering.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | SE | UX | `waiting_approval` counter event-based, tidak update setelah resolve | Recompute pada resume |
| 2 | P2 | Reviewer | Test | Resumed-after-approval flow tidak ditest | Tambah test |
| 3 | P2 | Architect | Manifest | UI flags tidak otoritatif (manual maintenance per step) | Wire ke renderer dari manifest |
| 4 | P2 | SE | UX | Rendering datar tanpa styling severity | Tambah styling per severity |
| 5 | P3 | SE | UX | Progress percent tidak smooth | Polish animation |
