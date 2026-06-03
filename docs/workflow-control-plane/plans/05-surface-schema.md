# Plan 05 — Surface manifest schema


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with completion history; active contract is surface metadata consistency**.
> - `statusline`
> - `status`
> 3. Require visible status and route classification.

Code evidence:
- `vac-rs/core/src/control_plane/surface_manifest.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/05-evidence/2026-05-28-sandbox-surface-schema-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with completion history; active contract is surface metadata consistency**.

Target outcome: UI/CLI/slash/palette surfaces are declared with typed metadata so capabilities can expose user-facing entry points consistently.

Outputs: surface schema, manifest validation, surface files, and tests for invalid routes or missing required fields.

Requires / Blocks: requires capability schema; blocks dashboard/browser/slash convergence and root feature conversion.

Stop conditions: stop if surface declarations drift from actual TUI/CLI routes without a reconciliation plan.

Done criteria: surface manifests validate and downstream UI surfaces can resolve them without hard-coded duplicates.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Declare TUI, slash, palette, and CLI exposure in one place.

## Schema shape

Surface manifests are typed declarations with:

- `schema_version: 1`
- `kind: surface`
- stable `id` starting with `surface.`
- operator-facing `title`
- non-empty `routes`
- non-empty `capabilities`

Each route is classified as one of:

- `tui`
- `slash`
- `palette`
- `cli`
- `statusline`

Route entries carry the target field for their kind, plus metadata:

- `capability`
- `visible`
- `status`
- `owner` for visible routes
- `reason` for `cli_only` or `unavailable` routes

## Surface files

```text
.vac/surfaces/tui.yaml
.vac/surfaces/slash.yaml
.vac/surfaces/palette.yaml
.vac/surfaces/cli.yaml
```

## Implementation

1. Define surface schema.
2. Link each surface entry to a capability id.
3. Require visible status and route classification.
4. Forbid visible entries without capability owner.
5. Make slash help and palette eventually read from this metadata.

## Validation

- Visible surface with missing capability fails.
- CLI-only surface must say why.
- Planned capability can be visible only if clearly marked planned/partial.
- Visible route capability must appear in the surface capability list.
- `cli_only` is the serialized route status value.

## Completion status

Completed in Plan 05 hardening pass.

### Delivered

- [x] Surface schema supports `tui`, `slash`, `palette`, `cli`, and `statusline` route kinds.
- [x] Root surface manifests exist for TUI, slash, palette, and CLI surfaces.
- [x] `SurfaceRouteCatalog` loads slash, palette, CLI, TUI, and statusline routes from manifests.
- [x] Capability dashboard renders route kinds in order: TUI, slash, palette, CLI, statusline.
- [x] Slash popup/listing is manifest-driven; built-in commands without a slash route do not appear.
- [x] Built-in slash command coverage is guarded by tests against `.vac/surfaces/slash.yaml`.
- [x] Surface registry validates route capabilities against loaded capability manifests.
- [x] `vac doctor surfaces` reports surface manifest and route-kind coverage.

### Validation commands

```bash
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui manifest_filters_hard_coded_commands_without_routes -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui all_visible_builtin_commands_are_declared_in_slash_surface_manifest -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core surface_manifest -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core surface_registry -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core surface_doctor -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-cli surfaces_doctor -- --nocapture
cargo run --manifest-path vac-rs/Cargo.toml -p vac-surface-cli -- doctor registry .
cargo run --manifest-path vac-rs/Cargo.toml -p vac-surface-cli -- doctor surfaces .
```

### Commit trail

- `6398e64 feat(plan05): seed tui surface manifest`
- `f2b3288 feat(plan05): extend SurfaceRouteCatalog with tui+statusline routes`
- `6f84d41 feat(plan05): manifest-driven slash command listing`
- `b9ef715 chore(plan05): apply vac-tui formatting`
- `6bac301 feat(plan05): harden surface validation and doctor`
- `a3f7ee3 chore(plan05): apply core and cli formatting`

## Done

No command appears in the TUI/palette/slash help without capability metadata. Surface manifests are validated against the capability registry, and `vac doctor surfaces` exposes route coverage for operator/developer feedback.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | Architect | Drift | ✅ resolved — surface doctor/readiness cross-validates capability↔surface route drift | Keep reverse drift diagnostics enabled |
| 2 | P1 | Architect | Schema | ✅ resolved — visible routes are explicit surface entries; palette eligibility stays capability metadata | Do not treat palette bool as route proof alone |
| 3 | P2 | SE | Doctor | ✅ resolved — surface doctor reports drift, duplicate, route-kind, and readiness state | Keep doctor as source-of-truth |
| 4 | P2 | Reviewer | Schema | ✅ resolved — duplicate route diagnostics are surfaced instead of silently first-winning | Preserve duplicate checks |
| 5 | P2 | Architect | Drift | ✅ resolved — owner attribution drift is checked by surface diagnostics | Keep route owner aligned to capability owner |
| 6 | P3 | Reviewer | Test | ✅ resolved — reverse-drift/readiness tests cover current surface manifests | Add tests for new surface kinds |
| 7 | P3 | Architect | Attribution | ✅ resolved — architecture/ownership routes are attributed to their owning capabilities | Keep .vac surface manifests authoritative |
