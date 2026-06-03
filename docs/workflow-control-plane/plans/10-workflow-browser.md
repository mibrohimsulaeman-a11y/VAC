# Plan 10 — TUI workflow browser


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with follow-up notes; active contract is registry-backed workflow browsing**.
> Target outcome: TUI workflow browser lists workflow manifests, status, steps, policy requirements, and diagnostics from registry data.
> 3. Render workflow id, title, status, inputs, steps, policy gates, validation.
> 7. Layout uses section headers, dividers, and a footer that calls out persistent-pane status.

Code evidence:
- `vac-rs/tui/src/workflow_browser.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with follow-up notes; active contract is registry-backed workflow browsing**.

Target outcome: TUI workflow browser lists workflow manifests, status, steps, policy requirements, and diagnostics from registry data.

Outputs: browser route/state, registry-backed list/detail views, required states, tests/snapshots, and validation evidence.

Requires / Blocks: requires workflow schema, loader, diagnostics, and TUI navigation.

Stop conditions: stop if workflows are shown from static fixtures instead of registry data or invalid workflows disappear silently.

Done criteria: browser displays valid workflows and actionable diagnostics for invalid ones.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Make workflows visible before execution exists.

## User flow

```text
vac
/workflow
```

## Implementation

1. Add workflow browser panel/route.
2. Load workflow registry snapshot.
3. Render workflow id, title, status, inputs, steps, policy gates, validation.
4. Render schema errors visibly.
5. Surface execution controls (`/workflow run <id>`) and approval hand-off (`/approvals respond <request_id>`) without auto-executing.
6. Provide capability filter affordance (`/workflow filter <capability>`) for large registries.
7. Layout uses section headers, dividers, and a footer that calls out persistent-pane status.

## Audit follow-up (2026-05-21)

| Finding | Status | Notes |
|---------|--------|-------|
| #1 Persistent pane | Foundational | `WorkflowBrowserState` introduced; full `bottom_pane` wiring tracked as next slice. |
| #2 Approval wiring | Resolved (surface) | Browser renders approval action lines with concrete request IDs and `/approvals respond` hints. |
| #3 Progress model restructure | Resolved | Steps now carry derived `capability_id`; filter and approval helpers exposed. |
| #4 Docs sync | Resolved | This section. |
| #5 Execution control | Resolved (surface) | Browser renders per-workflow `controls:` line with `/workflow run` and `/workflow inspect`. |
| #6 Capability filter | Resolved (model + render) | Filter helpers and hint line in the browser; state-aware renderer skips non-matching workflows. |
| #7 Negative tests | Resolved | New tests cover controls/filter rendering, capability filter, and progress helpers. |
| #8 Layout polish | Resolved | Header, dividers between workflows, and footer rendered. |

## Required states

```text
empty: no workflows found
loading: loading workflows
success: workflows loaded
failure: manifest/schema error
```

## Validation

- Maintenance workflows appear.
- Workflow with missing field shows error.
- Product TUI does not go blank if workflow registry fails.

### 2026-05-28 surface readiness sync

- `/workflow` remains the canonical TUI workflow browser route.
- The matching TUI surface route is now `ready` in `.vac/surfaces/tui.yaml`.
- The slash and palette routes that expose workflow navigation are also route-ready through the shared Plan 11 surface readiness scan.

## Done

VAC visibly has a workflow control plane from the operator perspective.

## Current implementation slice

- `/workflow` is now a built-in slash command in the root TUI.
- The browser projects the seeded `.vac/workflows/` registry snapshot.
- Registry load failures render diagnostics in the panel instead of leaving it blank.
- Loaded workflow manifests render id, title, status, inputs, steps, UI, policy, and validation details.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | SE | UX | `/workflow` = history cell dump, bukan persistent pane | Implement persistent pane |
| 2 | P1 | Architect | Approval | Approval action tidak wired di browser (view-only) | Wire approval action |
| 3 | P2 | Architect | Model | Execution trace dipotong jadi `WorkflowProgressModel` | Restructure model |
| 4 | P2 | Planner | Docs | Plan masih bilang "Do not run workflows" | Update doc |
| 5 | P2 | SE | UX | Tidak ada execution control dari browser | Add control |
| 6 | P2 | SE | UX | Capability filter tidak tersedia | Add filter |
| 7 | P3 | Reviewer | Test | Negative test minim | Tambah |
| 8 | P3 | Architect | Layout | Browser layout statis (no resize) | Polish |
