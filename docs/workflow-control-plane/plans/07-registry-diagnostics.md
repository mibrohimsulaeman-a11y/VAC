# Plan 07 — Registry diagnostics and error UX


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with audit history; active contract is user-visible registry errors**.

Code evidence:
- `vac-rs/core/src/control_plane/registry_diagnostics.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/07-evidence/2026-05-28-sandbox-registry-diagnostics-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with audit history; active contract is user-visible registry errors**.

Target outcome: registry/schema errors are actionable in CLI/TUI surfaces and include file path, field path, kind, and suggested fix where possible.

Outputs: diagnostic types, renderer integration, tests/snapshots, and visible failure states.

Requires / Blocks: requires registry loader; blocks dashboard/browser/release gates that depend on reliable errors.

Stop conditions: stop if errors become generic strings or invalid manifests are hidden from operators.

Done criteria: malformed manifests show structured diagnostics in automated tests and user-facing surfaces.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Make broken manifests visible and repairable from CLI/TUI.

## Current implementation slice

- `vac_core::control_plane::registry_diagnostics` exposes `RegistryDiagnosticSeverity`, `RegistryLoadState`, `RegistryDiagnostic`, and `RegistryLoadReport`.
- `vac doctor registry [PATH]` prints the report and exits non-zero on registry load failure.
- `RegistryLoadReport::render_tui_lines()` gives the TUI projection helper for the same report.
- The root TUI surfaces the report through `/debug-config` as a control-plane registry section.

## Implementation

1. Define diagnostic type: severity, file, field path, message, hint.
2. Add CLI-accessible debug output for registry load.
3. Add TUI projection for registry errors on `/debug-config`.
4. Keep diagnostics concise and non-secret.

## Visible states

```text
empty: no manifests found
loading: loading control plane
success: registry loaded
failure: manifest errors with path and field
```

## Validation

- Invalid YAML appears as a structured diagnostic.
- Missing required field appears with exact field path.
- TUI can render registry load failure instead of blank screen.

## Done

A bad workflow file cannot silently break the product.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | SE | Diag | ✅ resolved — hidden source domains are always surfaced from `unowned_source_domains()` | Keep hidden-domain diagnostics visible |
| 2 | P2 | Reviewer | Diag | ✅ resolved — duplicate diagnostics include identifiers and previous paths | Keep messages specific |
| 3 | P2 | Reviewer | Diag | ✅ resolved — partial conversion entries render info/warning diagnostics | Keep partial reasons current |
| 4 | P2 | Reviewer | Test | ✅ resolved — doctor/registry failure paths are covered by diagnostics tests | Keep non-zero failure behavior |
| 5 | P3 | Planner | Docs | ✅ resolved — hidden-domain semantics are documented in Plan 19/21/22 evidence | Keep ownership docs aligned |
