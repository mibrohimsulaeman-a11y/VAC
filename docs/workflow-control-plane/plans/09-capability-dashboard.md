# Plan 09 — TUI capability dashboard


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with validation history; active contract is visible capability state**.
> Target outcome: TUI capability dashboard presents registry-backed capability status, validation state, and diagnostics without hard-coded drift.
> 3. Render id, title, status, owner, surfaces, policy, validation.
> | # | Sev | Persp | Area | Finding | Status | Resolution |

Code evidence:
- `vac-rs/tui/src/capability_dashboard.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with validation history; active contract is visible capability state**.

Target outcome: TUI capability dashboard presents registry-backed capability status, validation state, and diagnostics without hard-coded drift.

Outputs: TUI route/state, registry integration, empty/loading/success/failure states, tests or snapshots, and validation evidence.

Requires / Blocks: requires registry loader and diagnostics; blocks operator visibility for control-plane adoption.

Stop conditions: stop if dashboard state is backend-only, invisible in TUI, or duplicates registry truth manually.

Done criteria: dashboard renders real registry data and failure states are visible to users.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Make all declared capabilities visible in the root TUI.

## Current implementation slice

- The root TUI exposes `/capabilities` as a built-in slash command.
- The surface projects the seeded control-plane registry snapshot from `.vac/capabilities/`.
- The dashboard now also surfaces the ownership scan matrix so unowned root domains show up in the root TUI instead of only in CLI diagnostics.
- Registry load failures render diagnostics instead of a blank panel.

## User flow

```text
vac
/capabilities
```

## Implementation

1. Add a TUI route or panel for capability dashboard.
2. Load registry snapshot.
3. Render id, title, status, owner, surfaces, policy, validation.
4. Render diagnostics if registry load fails.
5. Do not require workflow runner yet.

## Required states

```text
empty: no capabilities found
loading: loading control plane
success: capabilities loaded
failure: manifest/schema error
```

## Validation

- `/capabilities` renders initial root capabilities.
- Broken manifest appears as error row.
- No donor capability appears before donor phase.

### 2026-05-22 build + test validation

`cargo build -p vac-surface-tui --lib` clean (1m32s, 5 warnings, 0 errors) → `cargo test -p vac-surface-tui --lib capability_dashboard_root_features` ✅ **5/5 PASS** in 0.01s setelah compile 3m31s:

- `capability_dashboard_root_features_renders_hidden_summary`
- `capability_dashboard_root_features_marks_missing_ownership_hidden`
- `capability_dashboard_root_features_renders_partial_reason`
- `capability_dashboard_root_features_renders_overclaimed_modules`
- `capability_dashboard_root_features_renders_source_hidden_domains`

Memvalidasi audit findings #1–#5 (2026-05-20) tetap fungsional pasca refactor `local_runtime_session.rs` (trait `LocalRuntimeStartedThread` ekstraksi + `AppServerStartedThread` impl split di `app_server_session.rs`) yang menyentuh session lifecycle path.

### 2026-05-28 surface readiness sync

- `/capabilities` remains the canonical TUI capability dashboard route.
- The matching TUI/slash/palette declarations are now route-ready through the shared Plan 11 surface readiness scan.
- This closes metadata drift around dashboard visibility; runtime dashboard rendering remains guarded by the existing Plan 09 tests and registry diagnostics.

## Done

A user can see what VAC knows it can do.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Status | Resolution |
|---|-----|-------|------|---------|--------|------------|
| 1 | P1 | Architect | UX | Hidden-domain dirender di `/capabilities` tapi registry gate skip (lihat Plan 7) | ✅ resolved | UI dashboard mengangkat hidden source domains via `RootFeatureConversionReport` (row `source:<crate>/<module> hidden status=missing reason="unowned source domain"`); registry gate sengaja skip silently di `root_feature_conversion_diagnostics` (cabang `source:` → `continue`) supaya tidak double-report. Tests: `registry_root_feature_conversion_skips_unowned_source_domain_without_targets`, `registry_root_feature_conversion_skips_unowned_source_domain_when_targets_exist`, `conversion_report_includes_hidden_source_domain_entries`, `capability_dashboard_root_features_renders_source_hidden_domains`. |
| 2 | P2 | Architect | UX | Split antara hidden capability vs source-domain bingungkan user | ✅ resolved | `RootFeatureConversionReport` menggabung canonical hidden + source-domain rows; render row `source:<crate>/<module> hidden status=missing reason="unowned source domain"` punya skema seragam dengan capability row biasa. |
| 3 | P2 | Architect | Attribution | Owner attribution di `/capabilities` drift | ✅ resolved | `surface_capability_drift_diagnostics` membandingkan capability owner vs surface route owner dan emit warning yang tampil di dashboard `Control plane registry:` section. |
| 4 | P2 | SE | UX | Loading state hilang (tampil blank saat fetch) | ✅ resolved | `RegistryLoadState::Loading` (test `loading_report_renders_loading_state`) plus `Empty` state. Dashboard sync via `load_control_plane_registry_report`, jadi tidak ada window blank; state diagnostics tetap terlihat di output. |
| 5 | P3 | Reviewer | Test | Test hidden-domain rendering tidak ada | ✅ resolved | `capability_dashboard_root_features_renders_source_hidden_domains` di `vac-rs/tui/src/capability_dashboard.rs` (memverifikasi `source domains:` + row `vac-core/unowned_domain hidden`). |
