# Plan 11 — Slash, palette, CLI surface convergence


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **complete for shared surface route readiness; active contract is one surface registry**.
> 4. Add route status: ready, partial, planned, unavailable, cli-only.
> ## Completion status
> - [x] Surface route catalog loads slash, palette, CLI, TUI, and statusline routes from manifests.

Code evidence:
- `vac-rs/tui/src/surface_route_catalog.rs`
- `vac-rs/core/src/control_plane/surface_readiness.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/11-evidence/2026-05-28-sandbox-surface-route-readiness.md`
- `docs/workflow-control-plane/plans/11-evidence/2026-05-28-sandbox-surface-route-status-sync.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete for shared surface route readiness; active contract is one surface registry**.

Target outcome: slash commands, command palette, and CLI surface metadata converge on shared control-plane declarations where appropriate.

Outputs: shared surface mapping, slash/palette integration, tests for duplicate/drift prevention, and validation evidence.

Requires / Blocks: requires surface schema and registry loader; blocks duplicate-command cleanup and consistent UX.

Stop conditions: stop if a new independent command registry is introduced or user-visible command labels drift.

Done criteria: surfaces resolve shared declarations and tests catch duplicate or missing registrations.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Use surface manifests to prevent command drift across slash commands, command palette actions, and CLI routes.

## Implementation

1. Map slash help entries to `.vac/surfaces/slash.yaml`.
2. Map palette entries to `.vac/surfaces/palette.yaml`.
3. Map CLI command catalog to `.vac/surfaces/cli.yaml` where practical.
4. Add route status: ready, partial, planned, unavailable, cli-only.
5. Prevent visible routes without capability metadata.
6. Guard slash, palette, and CLI coverage with tests so user-visible command surfaces cannot drift silently.

## Validation

- Every visible slash command has capability metadata.
- Every visible built-in slash command is declared in `.vac/surfaces/slash.yaml`.
- Every top-level CLI command is declared in `.vac/surfaces/cli.yaml`.
- Every `vac doctor` subcommand is declared in `.vac/surfaces/cli.yaml`.
- Every palette command action is declared in `.vac/surfaces/palette.yaml`.
- `vac doctor registry .` passes against the root manifests.
- `vac doctor surfaces .` passes and reports route-kind coverage.

## Completion status

Completed in Plan 11 closeout pass.

### Delivered

- [x] Slash popup/listing is guarded against manifest drift.
- [x] Slash surface now declares **55 routes** in `.vac/surfaces/slash.yaml`.
- [x] CLI surface now declares **17 routes** in `.vac/surfaces/cli.yaml`, including top-level commands and `vac doctor` subcommands.
- [x] Palette surface coverage is guarded by `palette_surface` tests.
- [x] Surface route catalog loads slash, palette, CLI, TUI, and statusline routes from manifests.
- [x] `/capabilities` renders a surface route summary for slash, palette, and CLI routes loaded from the current workspace.
- [x] `vac doctor registry .` validates the registry after the surface convergence changes.
- [x] `vac doctor surfaces .` validates route-kind coverage after the surface convergence changes.
- [x] Visible routes in `.vac/surfaces/{cli,slash,palette,tui}.yaml` are synchronized to `ready` once they are manifest-backed and drift-free.
- [x] Surface doctor output includes a side-effect-free `surface route readiness` summary so route status regressions do not become silent UI drift.

### Current implementation slice

- Slash popup rows carry route metadata from `.vac/surfaces/slash.yaml` when a manifest entry exists.
- Route status is surfaced as a right-side tag in the popup, and visible routes require a capability owner in the manifest.
- The 2026-05-28 sandbox slice promotes all drift-free visible root routes from `partial/planned` to `ready` and adds `surface_readiness.rs` to fail surface doctor when a visible route regresses to non-ready.
- The seeded slash surface covers every visible built-in slash command currently exposed by the TUI.
- The seeded palette surface covers the top-level command actions `open_capabilities`, `open_workflow`, `open_status`, `open_activity`, `open_debug_config`, and `open_model`.
- The seeded CLI surface covers `vac`, top-level CLI commands, and all current `vac doctor` subcommands.
- Plan 11 now matches the Plan 05 hardening pattern: schema + route manifests + guard tests + doctor validation + closeout docs are aligned.

### Validation commands

```bash
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui all_visible_builtin_commands_are_declared_in_slash_surface_manifest -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-cli top_level_cli_commands_are_declared_in_cli_surface_manifest -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-cli doctor_subcommands_are_declared_in_cli_surface_manifest -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui palette_surface -- --nocapture
cargo run --manifest-path vac-rs/Cargo.toml -p vac-surface-cli -- doctor registry .
cargo run --manifest-path vac-rs/Cargo.toml -p vac-surface-cli -- doctor surfaces .
```

### Commit trail

- `5b7c504` — Plan 11 code closeout: slash, CLI, and palette surface convergence guards are in place.
- `docs(plan11): mark surface convergence guarded` — documentation closeout for the guarded surface convergence state.


### 2026-05-28 sandbox surface route readiness closeout

- `.vac/surfaces/cli.yaml`, `.vac/surfaces/slash.yaml`, `.vac/surfaces/palette.yaml`, and `.vac/surfaces/tui.yaml` now mark every visible manifest-backed route as `ready`.
- `vac-rs/core/src/control_plane/surface_readiness.rs` records a side-effect-free route readiness scan. It does not execute UI handlers; it verifies that the shared surface registry no longer advertises visible `partial` or `planned` routes after capability/root readiness closed.
- `vac doctor surfaces` now renders `surface route readiness: ready=true ...` and treats non-ready visible routes as a failure. This closes the old Plan 11 route-status follow-up without claiming physical runtime execution for every optional command.

## Done

There is one guarded product command surface model, not many drifting lists. Slash, palette, and CLI routes are declared in manifests, validated by focused tests, and checked by registry/surface doctors.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | Architect | Convergence | Typed slash dispatch bypass manifest (hardcoded) | Route via manifest |
| 2 | P1 | Architect | Manifest | Alias `/clean`/`/quit` tidak first-class | Add alias model |
| 3 | P2 | Architect | Drift | Owner inconsistent across slash/palette/CLI | Sync owner |
| 4 | P2 | Architect | Validation | Doctor surfaces tidak validasi konvergensi | Add convergence check |
| 5 | P2 | SE | UX | CLI nested cmd coverage partial vs slash | Cover all routes |
| 6 | P2 | Architect | Drift | `/architecture`/`/ownership` route attributed ke `vac.workflow` | Re-attribute |
| 7 | P2 | Reviewer | Test | Test surface convergence tidak ada | Add |
| 8 | P3 | Planner | Docs | Plan 11 docs perlu update convergence model | Update |

## Maintenance follow-up 2026-05-22

Refactor `chatwidget/` (split jadi `slash_dispatch.rs` + `status_surfaces.rs`) memisahkan typed slash dispatch dari `ChatWidget` callsite — fondasi untuk evolusi finding #1 (typed dispatch tidak boleh hardcoded di widget body). `cargo build -p vac-surface-tui --lib` 2026-05-22 clean (1m32s, 0 errors); 5/5 `capability_dashboard_root_features` tests PASS. Manifest-driven slash popup metadata dari `.vac/surfaces/slash.yaml` tetap intact pasca split.


## 2026-05-28 sandbox route status sync

Visible route statuses across `.vac/surfaces/{cli,palette,slash,tui}.yaml` are now synchronized to `ready` for the default product surface registry. Evidence: [11-evidence/2026-05-28-sandbox-surface-route-status-sync.md](11-evidence/2026-05-28-sandbox-surface-route-status-sync.md).
