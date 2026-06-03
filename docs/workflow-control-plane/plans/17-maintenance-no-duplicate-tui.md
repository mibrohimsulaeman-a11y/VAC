# Plan 17 — No duplicate TUI maintenance workflow


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with audit fixes; active contract is duplicate TUI prevention**.
> ## Resolution status (2026-05-21)
> | # | Status | Lokasi | UX dampak bagi user/operator |
> | 3 | P2 | Workflow `status: planned` padahal dipakai di `release-gate` | `.vac/workflows/maintenance.no-duplicate-tui.yaml` → `status: ready` |

Code evidence:
- `vac-rs/core/src/control_plane/no_duplicate_tui.rs`
- `vac-rs`
- `docs/product`
- `docs/product/CAPABILITY_MAP.md`
- `docs/product/domain-prds/tui-action-recorder-replay.md`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/17-evidence/2026-05-28-sandbox-index-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with audit fixes; active contract is duplicate TUI prevention**.

Target outcome: maintenance workflow detects duplicate TUI/surface implementations and prevents regressions toward multiple product TUIs.

Outputs: workflow manifest, detection logic, regression tests, validation evidence, and resolved audit notes.

Requires / Blocks: requires surface schema, registry loader, and maintenance workflow runner.

Stop conditions: stop if detection is string-only with high false positives and no allowlist/evidence strategy.

Done criteria: known duplicate patterns fail, legitimate single-TUI paths pass, and TUI/product command invariants are protected.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Prevent duplicate product frontend paths.

## Current implementation slice

- The root TUI capability manifest now lives at `.vac/capabilities/tui.yaml`.
- The maintenance workflow lives at `.vac/workflows/maintenance.no-duplicate-tui.yaml`.
- The workflow runner scans `vac-rs`, `vac-cli`, `.vac`, and `docs/product`.
- Allowlisted legacy docs remain `docs/product/CAPABILITY_MAP.md` and `docs/product/domain-prds/tui-action-recorder-replay.md`.
- The scanner is focused on exact donor/frontend indicators such as `vac_tui_runtime`, `vac_shell_runtime_loop`, `second TUI runtime`, `renamed terminal app`, and `old product assumption`.
- `/workflow` and `vac doctor workflow` render the same no-duplicate-TUI report through the root product path.

## Workflow

```text
.vac/workflows/maintenance.no-duplicate-tui.yaml
```

## Implementation

1. Add typed source scan capability.
2. Detect donor frontend imports in product source.
3. Detect additional TUI runtime crates/routes if introduced.
4. Ensure root TUI remains `vac-rs/tui`.
5. Render failures as actionable diagnostics.

## Validation

- Introducing donor TUI route fails check.
- Root TUI source passes.
- Donor directory is not considered product path.
- The current no-duplicate-TUI workflow is visible through the root TUI and CLI doctor surface.

## Done

The repo cannot silently grow another product TUI.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P1 | Reviewer | Scanner | Keyword-only (5 forbidden term, case-sensitive substring) — tidak deteksi crate/route baru | Structural scan (`Cargo.toml`/`[[bin]]`/dependency `ratatui`/`crossterm` di luar `vac-rs/tui`) |
| 2 | P1 | Architect | Coverage | Plan acceptance broader tidak ditest (struktural detection) | Add structural test |
| 3 | P2 | SE | UX | Failure reason tidak edukasi invariant | Educational message + link plan |
| 4 | P2 | Architect | Boundary | Plan 00F boundary tidak eksplisit (legacy TUI vs new TUI duplication) | Clarify boundary |
| 5 | P3 | Reviewer | Test | Structural scanner test tidak ada | Add |

## Resolution status (2026-05-21)

Semua finding audit 2026-05-20 sudah ditutup di `vac-rs/core/src/control_plane/no_duplicate_tui.rs`. Implementasi slice & UX dampak:

| # | Status | Lokasi | UX dampak bagi user/operator |
|---|--------|--------|------------------------------|
| 1 | ✅ FIXED | `structural_tui_cargo_finding` — deteksi `[[bin]]` + `ratatui`/`crossterm` di luar `vac-rs/tui/` | Tambah TUI crate baru (apapun namanya) langsung ditolak oleh `/workflow` & `vac doctor workflow` sebelum merge — bukan hanya keyword match. |
| 2 | ✅ FIXED | Test `no_duplicate_tui_detects_structural_alt_tui_crate` + `_detects_crossterm_based_duplicate_tui_binary` | Regressi structural detection ketangkep di CI bareng keyword test. |
| 3 | ✅ FIXED | `NoDuplicateTuiReport::failure_reason` — "TUI Uniqueness Invariant Violation" + path/line/term + link Plan 17 | Operator yang melihat pesan gagal langsung tahu invariant apa yang dilanggar, di mana, dan ke mana baca dokumennya. |
| 4 | ✅ FIXED | Komentar "Architectural Boundary (Plan 00F Alignment)" di `should_skip_path` | Reviewer baru paham kenapa `donor/` di-skip — bukan bug, tapi sengaja untuk Plan 00F runtime-retirement. |
| 5 | ✅ FIXED | Test `no_duplicate_tui_ignores_approved_roots_and_ignores_library_usages` | Confirms allowlist crate (workspace root, `vac-rs/tui`, library non-binary seperti `ansi-escape`) tidak ditandai false-positive. |

## Audit cycle 2026-05-21 — fixes implemented

Audit refresh 2026-05-21 (`VAC Plan 00–21 Hardening Audit — Findings`) menemukan 4 gap tersisa untuk Plan 17 (P1 + P2). Semua diimplementasikan di siklus ini:

| # | Sev | Gap (audit 2026-05-21) | Fix |
|---|-----|------------------------|-----|
| 1 | P1 | `NO_DUPLICATE_TUI_SCAN_ROOTS=["."]` self-trigger pada doc Plan 17 | Scan roots dipersempit ke `["vac-rs", ".vac"]`; `docs/` di-exclude by design (doc plan legit mengutip vocabulary forbidden) |
| 2 | P1 | Cargo detection substring (bukan TOML-parsed) | `structural_tui_cargo_finding` sekarang pakai `toml::from_str` ke struct `CargoManifest` — parses `[package]` / `[[bin]]` / `[dependencies]` secara terstruktur. Manifest workspace tanpa `[package]` di-skip otomatis (no false-positive di `vac-rs/Cargo.toml`); manifest malformed di-skip dengan parse-error (build gate yang tangkap syntax). |
| 3 | P2 | Workflow `status: planned` padahal dipakai di `release-gate` | `.vac/workflows/maintenance.no-duplicate-tui.yaml` → `status: ready` |
| 4 | P2 | Plan 00F boundary masih prose only | Diangkat jadi const code `PLAN_00F_QUARANTINED_DIRS: &[&str] = &[".git", "donor", "target"]` dengan doc comment Plan 00F alignment di atas declaration |

### Regression test baru

| Test | Memvalidasi |
|------|-------------|
| `no_duplicate_tui_does_not_self_trigger_on_plan_doc` | Doc Plan 17 yang menyebut vocabulary forbidden TIDAK trigger finding (`docs/` tidak di scan roots) |
| `no_duplicate_tui_ignores_malformed_cargo_manifest` | Cargo.toml malformed tidak crash + tidak false-positive (build gate pegang validasi syntax) |

### Validasi

```bash
cargo test -p vac-core --lib control_plane::no_duplicate_tui:: -- --nocapture
```

Expected: **6 tests PASS** (4 lama + 2 baru).

Workflow `maintenance.no-duplicate-tui` sekarang `ready` dan tetap di `release-gate` tanpa drift status `planned`-vs-`ready`.

## Validation 2026-05-22

`cargo build -p vac-surface-tui --lib` clean (1m32s, 5 warnings, 0 errors) pasca refactor `local_runtime_session.rs` (trait `LocalRuntimeStartedThread` ekstraksi) + `app_server_session.rs` (impl split). Single TUI invariant tetap intact: detector `structural_tui_cargo_finding` tidak false-positive pada refactor in-place di `vac-rs/tui/`. 5/5 `capability_dashboard_root_features` tests PASS.
