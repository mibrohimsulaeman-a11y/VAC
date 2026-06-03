# O5/O6 TUI Widget Styled Output Path Report

## Scope

This slice continues from `vac-o5o6-tui-widget-parity-final-source.zip` and addresses the follow-up audit finding that widget geometry existed but the output path still flattened ratatui `Buffer` cells to plain strings before live display.

## Root cause

`operator_widget_render.rs` painted operator screens with real ratatui widgets into an off-screen `Buffer`, but `buffer_to_lines` only copied `cell.symbol()` into a `String`. That preserved rounded geometry characters, but it dropped every foreground color, background color, and modifier from the cell. Runtime jobs and capability dashboard then passed those strings through semantic reclassification, which could not reconstruct per-cell style, pill backgrounds, or gauge fill.

Ratatui's model is explicitly cell-based: a `Buffer` stores `Cell` values, and each cell carries symbol plus style information including foreground/background and modifiers. Dropping `Cell::style()` therefore turns a styled terminal surface into monokrom text and can hide gauges whose visible fill is style/background-driven.

## Changes

- `operator_widget_render::buffer_to_lines` now groups consecutive cells with identical `Style` into styled `Span` runs.
- The conversion still trims only trailing empty background cells so committed text snapshots remain bounded while internal gauge/card/pill spaces keep their style.
- Added `buffer_conversion_preserves_cell_style_and_gauge_fill` to lock the style-preservation behavior at source/test level.
- Added production `*_visual_lines` helpers in `operator_ui.rs` for first launch, idle, agent streaming, approval, runtime jobs, and capability dashboard.
- Switched live call sites for first launch, idle, runtime jobs, and capability dashboard shell to consume styled `Line<'static>` values directly.
- Kept legacy string snapshot helpers for source/static text snapshots and compatibility tests only.
- Updated static gates to require `cell.style()` conversion, styled spans, visual-line call sites, and the new regression test.
- Repinned O5.2 semantic split hashes for changed split shards.
- Repinned O6.1 scanner metadata; `total_runtime_panic_surface` remains `251`, while `cfg_test_lines_removed` moved to `134233` because of the added `#[cfg(test)]` regression test.

## Validation

Executed in this sandbox and recorded in `docs/monolith-quality/logs/o5o6-tui-widget-style-output-path-final-validation-clean.log`:

- `bash -n scripts/check-vac-tui-visual-parity-static.sh scripts/check-tui-operator-snapshot-contract.sh scripts/check-vac-o5o6-all-audit-remainder.sh scripts/check-vac-o5-o6-monolith-quality-slice.sh` — PASS.
- `scripts/check-vac-tui-visual-parity-static.sh` — PASS.
- `scripts/check-tui-operator-snapshot-contract.sh` — PASS source/static; cargo-backed widget test NotEvaluated because cargo/rustc are unavailable.
- `scripts/check-vac-runtime-engine-wiring-static.sh` — PASS.
- `scripts/check-tui-source-artifact-hygiene.sh` — PASS.
- `scripts/check-legal-release-blockers.sh` — PASS.
- `scripts/check-vac-init-registry-strictness-contract.sh` — PASS; rustc unit gate NotEvaluated.
- `scripts/check-vac-init-registry-validator-contract.sh` — PASS; rustc unit gate NotEvaluated.
- `scripts/check-vac-o5-2-semantic-split-hash.sh` — PASS.
- `scripts/check-vac-o5-2-godfile-staging-all.sh` — PASS.
- `scripts/check-vac-o6-1-depanic-surface.sh` — PASS (`total=251`, `files=1253`, `cfg_test_lines_removed=134233`).
- `scripts/check-vac-o6-2-safety-coverage.sh` — PASS.
- `scripts/check-vac-o6-quality-triage.sh` — PASS.
- `scripts/check-vac-o5-5-donor-delete-gate.sh` — PASS.
- `scripts/check-vac-o5-o6-completion-state.sh` — PASS.
- `scripts/check-vac-o5-o6-monolith-quality-slice.sh` — PASS.
- `scripts/check-vac-o5o6-all-audit-remainder.sh` — PASS.

## Truth status

- SV-Done: style-preserving Buffer-to-Line conversion, live styled visual-line call sites, static visual/snapshot gates, O5.2 split hash repin, O6.1 registry repin, and aggregate source/static gates.
- TV-Pending: cargo-backed `vac-tui` tests, targeted/full cargo check/clippy/test, and live terminal screenshot/pixel parity.

## Remaining parity caveat

This slice fixes the color/background loss at the history-surface boundary. It does not claim full alternate-screen chrome or pixel parity. Full parity still requires a cargo-backed terminal run and screenshot gate that renders the operator view in a real TUI backend.
