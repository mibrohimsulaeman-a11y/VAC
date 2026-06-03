# Plan 06 — Rust registry loader


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with production follow-up notes; active contract is typed registry loading**.
> | Original finding | Status | Evidence |
> ### Production-grade follow-up status
> | Priority | Area | Status | Evidence |

Code evidence:
- `vac-rs/core/src/control_plane/registry.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/06-evidence/2026-05-28-sandbox-registry-loader-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with production follow-up notes; active contract is typed registry loading**.

Target outcome: Rust registry loader reads `.vac` manifests, validates schemas, reports diagnostics, and exposes typed registry data without side effects.

Outputs: loader modules, parser/validation tests, diagnostics integration, and no-execution-on-load guarantees.

Requires / Blocks: requires Plans 02–05 schemas; blocks dashboard, workflow browser, safe runner, and runtime-owner gate manifests.

Stop conditions: stop if loading manifests executes workflow commands, hides diagnostics, or silently drops invalid manifests.

Done criteria: registry load is deterministic, invalid files report structured errors, and tests cover all manifest kinds.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Load `.vac` manifests into typed Rust snapshots without executing anything.

## Target modules

```text
vac-rs/core/src/control_plane/capability_registry.rs
vac-rs/core/src/control_plane/workflow_registry.rs
vac-rs/core/src/control_plane/policy_registry.rs
vac-rs/core/src/control_plane/surface_registry.rs
vac-rs/core/src/control_plane/registry.rs
```

If module layout differs, place code in the closest root configuration/runtime crate.

## Implementation

1. Find the repository root and `.vac` directory.
2. Load YAML files from each control plane subdirectory.
3. Validate schema version and kind.
4. Return typed snapshots for each registry family.
5. Provide a combined `ControlPlaneRegistry` snapshot for the full `.vac` load path.
6. Return structured diagnostics with file and field paths.
7. Do not execute commands or import donor code.

## Validation

```bash
cd vac-rs
cargo test -p vac-core capability_registry
cargo test -p vac-core workflow_registry
cargo test -p vac-core policy_registry
cargo test -p vac-core surface_registry
cargo check -p vac-surface-cli
```

## Done

Rust can load the full control plane and report precise errors.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P2 | Reviewer | UX | ✅ resolved — duplicate id diagnostics include id and previous path | Keep collision diagnostics actionable |
| 2 | P2 | Architect | Validation | ✅ resolved — combined registry checks cross-family id collisions | Keep full registry loader authoritative |
| 3 | P2 | Architect | Validation | ✅ resolved — combined registry validates workflow uses against known capabilities | Do not bypass combined loader |
| 4 | P3 | Reviewer | Test | ✅ resolved — collision and aggregate parse tests exist | Extend for new manifest families |
| 5 | P3 | SE | UX | ✅ resolved — registry loader aggregates family errors before reporting | Preserve multi-error UX |

---

## Implementation audit (2026-05-22)

Audit ini mereview hasil implementasi Plan 06 sebagai **code reviewer**, **software engineer**, dan **auditor production readiness**. Scope utama: `registry.rs`, `workflow_registry.rs`, `registry_diagnostics.rs`, `workflow_manifest.rs`, dan `registry_tests.rs`.

### Verdict

| Persona | Verdict | Ringkasan |
|---|---|---|
| Code Reviewer | Pass with follow-up | P1 Plan 06 sudah tertutup: aggregated loading, cross-family collision check, dan post-load workflow `uses` validation sudah ada. Follow-up tersisa terutama kualitas diagnostics dan API contract. |
| Software Engineer | Implemented for combined loader | `load_control_plane_registry_at_root` sekarang mengumpulkan family errors, cek collision lintas capability/workflow/policy/surface, lalu validasi step `uses` terhadap vocabulary atau capability id. |
| Auditor | Production-acceptable with P2/P3 hardening | Tidak ada P0/P1 Plan 06 yang masih open untuk combined registry path. Production hardening berikutnya: jangan biarkan helper `load_workflow_registry_with_known_capabilities` memberi sinyal palsu, dan tingkatkan evidence diagnostics. |

### Resolved items from 2026-05-20 audit

| Original finding | Status | Evidence |
|---|---|---|
| Cross-family ID collision tidak dicek | Resolved | `validate_cross_family_unique_ids` mengumpulkan ID dari capability, workflow, policy, surface dalam satu map. |
| Workflow `steps[].uses` tidak divalidasi pasca-load | Resolved for combined loader | `validate_workflow_step_uses` dijalankan setelah semua family registry berhasil load. |
| Fail-fast loading / whack-a-mole UX | Resolved | `load_manifest_registry_at_root` dan `load_control_plane_registry_at_root` memakai error vector + aggregate flattening. |
| Collision test tidak ada | Resolved | `full_registry_reports_cross_family_collisions_in_one_pass`. |
| Aggregated parse error test tidak ada | Resolved | `full_registry_reports_all_manifest_parse_errors`. |

### Production-grade follow-up status

Follow-up P2/P3 dari audit 2026-05-22 sudah ditutup untuk area yang aman masuk Plan 06 tanpa mengubah shape `registry_diagnostics.rs`:

| Priority | Area | Status | Evidence |
|---|---|---|---|
| P2 | Diagnostics UX | Resolved | Duplicate manifest diagnostics sekarang menyertakan `id` dan `previous_path` lewat error display/message content: `duplicate manifest id `{id}` (previously loaded from `{previous_path}`)`. |
| P2 | API contract | Resolved | `load_workflow_registry_with_known_capabilities` tidak lagi mengabaikan `known_capabilities`; helper memvalidasi setiap workflow manifest terhadap vocabulary + known capability set dan mengembalikan aggregate error bila perlu. |
| P2 | Drift risk | Resolved | Resolver `workflow_step_use_resolves` dipusatkan di `workflow_manifest.rs` dan dipakai oleh manifest validator serta combined registry post-load validation. |
| P3 | Coverage | Resolved | Test tambahan mengunci exact capability id (`vac.foo`), alias (`capability.foo` -> `vac.foo`), vocabulary alias, helper unknown-capability rejection, dan diagnostic previous-path detail pada collision. |
| P3 | Partial-failure diagnostics | ✅ resolved / documented | Jika capability registry gagal parse, declared-capability validation tetap tidak dipaksakan agar tidak menghasilkan false unknown-capability errors; operator tetap mendapat aggregate parse diagnostics dulu. |

### Validation status

- `cargo nextest run --manifest-path vac-rs/Cargo.toml -p vac-core --lib -E 'test(registry)'` — **pass**: 44 passed, 1802 skipped.
- Follow-up validation: `cargo nextest run --manifest-path vac-rs/Cargo.toml -p vac-core --lib -E 'test(registry) or test(workflow_manifest) or test(workflow_registry)'` — **pass**: 63 passed, 1787 skipped.
- `./vac-rs/target/debug/vac doctor registry .` memakai existing binary — **exit 0**; registry loaded 32 manifests; root seed coverage pass `capabilities=13/13 policies=6/6 surfaces=4/4 workflows=3/3`; warnings tetap diagnostics.
- Historical `cargo build --manifest-path vac-rs/Cargo.toml -p vac-surface-cli` blocker from `vac-tui` slash/status surfaces is no longer an active Plan 06 status claim. Current sandbox closeout uses targeted schema/registry/surface-readiness checks; full workspace builds remain operator-gated release evidence because sandbox linking has proven unstable.

### UX impact

Operator sekarang mendapat lebih banyak error registry dalam satu run, bukan satu-per-satu. Collision lintas family fail-closed, workflow step typo pada `uses` tidak lagi lolos diam-diam di combined registry path, dan duplicate-ID output lebih actionable karena menyebut ID serta path manifest sebelumnya. Caller API helper juga tidak lagi mendapat false sense of validation dari `load_workflow_registry_with_known_capabilities`.

### 2026-05-28 stale validation note

Plan 06 is closed for typed registry loading. The old `vac-tui` compile blocker note is retained only as historical audit context; it is not an open registry-loader implementation blocker. Surface route readiness is now checked by Plan 11 and `vac doctor surfaces`, not by ad-hoc manual surface claims.
