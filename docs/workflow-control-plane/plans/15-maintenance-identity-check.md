# Plan 15 — Identity check maintenance workflow


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **complete for identity-check readiness and root status sync**.
> - Workflow/capability status is no longer split: `maintenance.identity-check`, `vac.identity.check`, and root surface status are all ready after Plan 19/11 status synchronization. Missing credentials or unavailable operator identity must still report blocked/skipped evidence rather than pass.
> ### 2026-05-28 status sync closeout
> | 5 | P2 | Planner | Drift | Historical workflow/capability status split | ✅ resolved — workflow, capability, and surface route readiness are now synchronized to ready; unavailable live identity remains blocked/skipped evidence, not a non-ready capability label. |

Code evidence:
- `vac-rs/core/src/control_plane/identity_check.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete for identity-check readiness and root status sync**.

Target outcome: identity maintenance workflow validates account/auth identity state and reports actionable pass/fail/blocked evidence.

Outputs: workflow manifest, runner integration, visible result states, validation tests, and docs.

Requires / Blocks: requires workflow runner, policy integration, and registry diagnostics.

Stop conditions: stop if identity checks require real credentials without a blocked/skip state or leak sensitive account data.

Done criteria: workflow can run safely, reports structured evidence, and handles unavailable identity as blocked/skipped rather than pass.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Prevent old identity terms or old product assumptions from reappearing outside donor source.

## Workflow

```text
.vac/workflows/maintenance.identity-check.yaml
```

## Current implementation slice

- `vac.identity.check` is the dedicated maintenance capability for product identity scanning.
- `maintenance.identity-check.yaml` is read-only and does not require approval before it runs.
- The workflow runner scans the product roots (`vac-rs/`, `vac-cli/`, `.vac/`, and `docs/product/`) for legacy identity terms, while intentionally quarantining the legacy reference docs that document the boundary itself.
- The TUI workflow browser and `vac doctor workflow` render the scan summary and line-level diagnostics so a failure is visible without digging into the source tree.
- Product docs now avoid forbidden legacy identity wording in the Plan 15A baseline so the maintenance identity scan can report a clean current repo.
- The identity scanner now models forbidden terms and exemptions with metadata so diagnostics include severity, reason, and replacement hint.
- The release gate now orders identity check first and lets the read-only identity step run before approval-gated release steps.
- The maintenance identity workflow is now marked ready after the current repo baseline reports zero findings and the release gate runs identity before approval-gated steps.
- Workflow/capability status is no longer split: `maintenance.identity-check`, `vac.identity.check`, and root surface status are all ready after Plan 19/11 status synchronization. Missing credentials or unavailable operator identity must still report blocked/skipped evidence rather than pass.

## Implementation

1. Add typed identity check capability.
2. Exclude `.git` and `donor/`.
3. Scan product docs/source/package metadata.
4. Report forbidden terms as diagnostics.
5. Render pass/fail in workflow browser/progress.

## Validation

- Workflow fails when forbidden term is introduced in product source.
- Workflow ignores donor source.
- TUI shows failure path and line summary.

### 2026-05-28 status sync closeout

- `vac.identity.check` is ready in `.vac/capabilities/identity-check.yaml`.
- `maintenance.identity-check` is ready in `.vac/workflows/maintenance.identity-check.yaml`.
- Root surface route readiness is ready for the identity and identity-check routes.
- Live credential absence remains a blocked/skipped evidence state, not a reason to keep the capability partial.

## Done

Product identity stays VAC-native automatically.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | SE | Handler | `IdentityCheck` handler asimetris — report `None` → success (beda dari NoDuplicateTui/Architecture/Ownership yang fail-closed) | Fail-closed bila report `None` |
| 2 | P1 | Reviewer | Scanner | Literal substring case-sensitive — bypass mudah dengan variasi case/punctuation | Case-insensitive + alias patterns |
| 3 | P1 | SE | Scope | Scan scope sempit (skip `docs/workflow-control-plane`, `README.md`, `AGENTS.md`) | Include docs + manifests |
| 4 | P2 | SE | UX | Exemption tidak ditampilkan di report | Render exemption list |
| 5 | P2 | Planner | Drift | Historical workflow/capability status split | ✅ resolved — workflow, capability, and surface route readiness are now synchronized to ready; unavailable live identity remains blocked/skipped evidence, not a non-ready capability label. |
| 6 | P2 | Reviewer | Test | Negative test bypass case tidak ada | Tambah |
| 7 | P2 | Architect | Coverage | Identity check tidak include README/AGENTS.md | Include |
| 8 | P3 | Planner | Docs | Plan wording drift ("identity check first" tapi release-gate order beda) | Update |
| 9 | P3 | Reviewer | Test | Comprehensive scanner test | Add |
