# Plan 01 — Create `.vac` control plane skeleton


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented / historical baseline; normalize before re-execution**.
> .vac/registry/status.yaml
> ## Implementation status
> **Status: Implemented — rechecked 2026-05-21.**

Code evidence:
- `.vac/registry/status.yaml`
- `.vac/capabilities`
- `.vac/workflows`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented / historical baseline; normalize before re-execution**.

Target outcome: the `.vac` repository skeleton exists with predictable directories, seed files, and validation hooks that later control-plane plans can rely on.

Outputs: `.vac` directory layout, initial registry seed files, plan/index docs, and validation evidence for repo layout creation.

Requires / Blocks: requires Phase 00 gate to be understood; blocks schema and registry plans that assume `.vac` paths exist.

Stop conditions: stop if creating the skeleton would overwrite existing `.vac` content, conflict with current schema conventions, or require broad workspace cleanup.

Done criteria: skeleton paths exist, validation commands pass, and historical notes are not treated as active requirements unless restated here.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Create the visible declarative repository pattern before feature migration begins.

## Target layout

```text
.vac/
  capabilities/
  workflows/
  policies/
  surfaces/
  registry/
```

## Implementation

1. Create all directories.
2. Add a short `.vac/README.md` explaining that this directory is product control plane, not runtime code.
3. Add placeholder registry files only where they are immediately valid.
4. Do not add donor-specific manifests yet.

## Initial files

```text
.vac/README.md
.vac/registry/product.yaml
.vac/registry/domains.yaml
.vac/registry/status.yaml
```

## Validation

- `test -d .vac/capabilities`
- `test -d .vac/workflows`
- `test -d .vac/policies`
- `test -d .vac/surfaces`
- `test -d .vac/registry`

## Done

The root layout visibly advertises VAC as a workflow-native product.

## Implementation status

**Status: Implemented — rechecked 2026-05-21.**

Verified in repo:

- `.vac/` root exists with `capabilities/`, `workflows/`, `policies/`, `surfaces/`, and `registry/` directories.
- Initial registry files exist: `.vac/registry/product.yaml`, `.vac/registry/domains.yaml`, and `.vac/registry/status.yaml`.
- `.vac/README.md` now documents the control-plane purpose, manifest families, naming conventions, schema-version semantics, and executable doctor gates.
- Subdirectory READMEs exist for capabilities, workflows, policies, surfaces, and registry; no stale "reserved for later" placeholder phrasing was found in those READMEs during the recheck.

Audit follow-up note:

- Findings #2 and #3 are covered by the updated `.vac` README/subdirectory documentation.
- Findings #4 and #5 are hygiene/CI follow-ups and are not required for the Plan 01 skeleton itself to be considered implemented.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P2 | Architect | Schema | `schema_version` semantics ambigu (global vs per-kind) | Dokumentasi + enforcement explicit |
| 2 | P3 | Planner | Docs | README subdirektori stale ("reserved for later plans") padahal sudah berisi manifest production | Update README sesuai state aktual |
| 3 | P3 | Planner | Docs | Naming convention manifest tidak terdokumentasi | Tulis canonical naming guide |
| 4 | P3 | Reviewer | Test | Tidak ada negative test untuk struktur skeleton | Tambah test skeleton drift |
| 5 | P3 | SE | Hygiene | Belum ada CI lint untuk struktur file `.vac/` | Tambah lint job |
