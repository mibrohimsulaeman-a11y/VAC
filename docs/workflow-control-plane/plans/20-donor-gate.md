# Plan 20 — Donor-backed capability gate


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **complete for donor-backed capability safety gate**.
> 2. Add manifest status for donor-backed capabilities.
> 3. Require donor source path, root target, TUI surface, policy, validation, cleanup status.
> - `donor_source` is now parsed by the capability manifest validator and must include `source`, `root_target`, and `cleanup_status`.

Code evidence:
- `vac-rs/core/src/control_plane/donor_domain_contract.rs`
- `scripts/check-donor-status.sh`

Evidence docs:
- `docs/workflow-control-plane/plans/20-evidence/2026-05-28-sandbox-donor-gate-ready.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete for donor-backed capability safety gate**.

Target outcome: donor-backed capabilities cannot be marked ready without explicit donor metadata, owner path, surface, and validation evidence.

Outputs: donor metadata validation, capability schema integration, tests for missing donor/surface/validation, and dashboard evidence.

Requires / Blocks: requires capability schema and root feature conversion.

Stop conditions: stop if donor-backed capabilities can pass as local-owned or ready without traceable source.

Done criteria: invalid donor manifests fail and valid donor manifests expose source/owner/validation clearly.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Ensure donor source enters the product only through workflow/capability manifests.

## Implementation

1. Require root seed coverage to pass before donor-backed manifests are accepted.
2. Add manifest status for donor-backed capabilities.
3. Require donor source path, root target, TUI surface, policy, validation, cleanup status.
4. Reject donor frontend stacks as product UI.
5. Require dashboard row before implementation starts.

## Current implementation slice

- `donor_source` is now parsed by the capability manifest validator and must include `source`, `root_target`, and `cleanup_status`.
- Plan examples now use the canonical `donor_source` field so docs match the implemented manifest schema.
- Legacy top-level `donor:` aliases are covered by regression tests and must stay rejected; the canonical field is `donor_source`.
- `cleanup_status` is constrained to the donor lifecycle values used by the root product handoff.
- `.vac/capabilities/donor_migration.yaml` now seeds donor migration evidence as a real capability manifest, while workflow execution uses six granular native handlers instead of the deprecated aggregate gate.
- `.vac/workflows/maintenance.donor-migration-gate.yaml` now runs root seed coverage before donor reachability and cleanup checks.
- `.vac/policies/default-local.yaml` now seeds the baseline local policy manifest used by the hardening gate.
- The capability dashboard and workflow browser still treat donor-backed capabilities as product manifests, not hidden backend-only work.
- `vac.donor_migration` and `maintenance.donor-migration-gate` are now ready root safety gates; donor code migration/deletion remains tracked by donor lifecycle evidence, not by this gate status.

## Required metadata

```yaml
donor_source:
  source: donor/vac/path
  root_target: vac-rs/path
  cleanup_status: donor_only | migrated | delete_candidate
```

## Validation

- Donor migration workflow fails if root seed coverage is blocked.
- Each granular donor migration handler consumes `DonorStatusReport` and fails closed when the report is unavailable or has failures.
- Donor feature without TUI/CLI surface fails manifest validation.
- Donor frontend path as product target fails validation.
- Donor-backed capability appears as planned before code migration.

## Done

Donor code cannot become hidden backend or duplicate frontend.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P0 | SE | Runtime | Handler `DonorMigration*` fallthrough success setelah approval | Resolved: granular donor handlers now use native `DonorStatusReport` execution and fail closed. |
| 2 | P0 | SE | Validation | `donor_source.root_target` tanpa allowlist — frontend path apapun lolos | Reject di `parse_donor_source` dengan allowlist eksplisit |
| 3 | P0 | Architect | Surface | Capability donor-backed tanpa TUI/CLI surface lolos (palette `true` saja diterima) | Wajibkan `surfaces.tui.routes` atau `surfaces.cli.commands` |
| 4 | P1 | SE | Manifest | Manifest seed donor tidak memiliki `donor_source` | Tambah seed `donor_source` |
| 5 | P1 | Architect | Validation | `scripts/check-donor-status.sh check_manifest` cuma grep-count | Real check (parse + validate) |
| 6 | P1 | Architect | Manifest | Aggregate `capability.donor_migration.gate` duplikat dengan granular steps | Resolved: aggregate gate deprecated/removed from runner vocabulary; six granular steps are canonical. |
| 7 | P1 | Reviewer | Test | Negative test donor invalid (allowlist, surface) tidak ada | Tambah |
| 8 | P2 | Planner | Docs | Plan 20 docs perlu update sesuai hardening | Resolved: docs now describe native granular donor executor and aggregate-gate deprecation. |
| 9 | P2 | SE | UX | Donor migration status tidak visible di dashboard | Render di `/capabilities` |
| 10 | P3 | Reviewer | Test | Comprehensive donor scenario test | Add |
