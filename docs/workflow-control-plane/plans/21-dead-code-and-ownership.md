# Plan 21 — Dead code and ownership enforcement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **complete for ownership/dead-code enforcement gate**.
> 5. Track donor cleanup status after every port.
> Every product domain has owner, surface, validation, or retirement status.

Code evidence:
- `vac-rs/core/src/control_plane/ownership_scan.rs`
- `vac-rs/core/src/control_plane/vac_init_ownership_scanner.rs`
- `vac-rs/core/src/control_plane/capability_manifest.rs`
- `vac-rs/cli/src/doctor_cli.rs`
- `vac-rs/tui/src/capability_dashboard.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/21-evidence/2026-05-28-sandbox-ownership-gate-ready.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete for ownership/dead-code enforcement gate**.

Target outcome: code ownership and dead-code gates prevent stale product surfaces, duplicate implementations, and unowned capabilities from accumulating.

Outputs: ownership checks, dead-code detection workflow, registry integration, tests, and documented exceptions.

Requires / Blocks: requires capability/surface manifests and root feature conversion.

Stop conditions: stop if enforcement deletes or rewrites code automatically without explicit owner evidence.

Done criteria: gates identify unowned/dead paths, allow documented exceptions, and fail safely with actionable diagnostics.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Prevent idle backend domains and orphan code.

## Implementation

1. Add ownership fields to capability manifests.
2. Create crate/module ownership matrix from manifests.
3. Flag root crates/modules with no capability owner unless test-only/retired.
4. Add maintenance workflow for ownership scan.
5. Track donor cleanup status after every port.

## Current implementation slice

- `vac-rs/core/src/control_plane/capability_manifest.rs` now accepts optional `ownership` metadata for a capability manifest.
- `vac-rs/core/src/control_plane/ownership_scan.rs` loads the control plane registry and renders an ownership matrix for capability manifests.
- `vac-rs/cli/src/doctor_cli.rs` now exposes `vac doctor ownership` for the ownership scan report.
- `vac-rs/tui/src/capability_dashboard.rs` now projects the ownership scan matrix into the root `/capabilities` TUI surface.
- `.vac/capabilities/*.yaml` now carry ownership metadata for the root capability seeds, so the matrix reflects real crate/module ownership instead of only the dedicated ownership capability.
- `.vac/workflows/maintenance.ownership-scan.yaml` wires the maintenance workflow to the new scan step.
- Capability manifests without ownership metadata are still visible as unowned entries in the scan report, so the matrix remains advisory for future ports and donor leftovers.
- Repo-wide unowned source-domain debt remains visible through the ownership scan matrix, while registry root-feature diagnostics stay scoped to root-seed overclaims so `vac doctor registry` remains the manifest consistency baseline instead of hard-blocking every legacy/quarantined crate at once.
- `vac.ownership` and `maintenance.ownership-scan` are now ready gates because the ownership scan is product-visible, safe-runner supported, and wired to release evidence.

## Validation

- Unknown root domain appears in dashboard as broken/unowned.
- Test-only modules can be explicitly classified.
- Retired code has deletion plan.

## Done

Every product domain has owner, surface, validation, or retirement status.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P0 | SE | Detection | Orphan crate (zero ownership) invisible — `build_unowned_source_domains` hanya scan crate yang sudah punya ownership target | Broaden inventory ke seluruh `vac-rs/*` |
| 2 | P0 | SE | Doctor | `vac doctor ownership` advisory-only (exit 0 walau ownership violation) | Wire to run report |
| 3 | P1 | SE | Manifest | Retired manifest tanpa deletion plan | Require deletion plan field |
| 4 | P1 | Architect | Detection | Test-only auto-ignore by name (heuristik) | Use explicit `test_only` flag |
| 5 | P1 | Architect | Ownership | Cartesian ownership (`crates × modules`) — multi-crate capability overclaim | Refine ownership target shape |
| 6 | P1 | SE | UX | Dashboard rendering raw text (tanpa struktur) | Structured render |
| 7 | P2 | Reviewer | Test | Negative test orphan crate tidak ada | Tambah |
| 8 | P2 | Architect | Manifest | Retired manifest hygiene tidak divalidasi | Validate retired metadata |
