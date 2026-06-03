# O5/O6 TUI Widget Snapshot Closeout Report

## Scope

This slice continues from `vac-o5o6-tui-widget-parity-checkpoint-003-source.zip` and closes the remaining source/static false-fail around operator TUI snapshots.

## Root cause

`operator_widget_render.rs` moved the operator surfaces from deterministic plain text to ratatui-backed geometry (`Block`, `Tabs`, `Gauge`, `Table`, `Layout`, `BorderType::Rounded`, and `Buffer`). The old snapshot contract still tried to compile `operator_ui_snapshot_contract.rs` directly with `rustc`. That path is no longer valid because the widget renderer depends on the vac-tui Cargo dependency graph, especially `ratatui`.

## Changes

- `scripts/check-tui-operator-snapshot-contract.sh` now validates the widget-backed source and committed snapshots without requiring rustc in a source-only sandbox.
- `scripts/render-tui-operator-snapshots.sh` no longer pretends direct rustc regeneration is valid. Without cargo it retains committed snapshots and reports TV-Pending.
- `vac-rs/tui/tests/operator_ui_snapshot_contract.rs` now includes `operator_widget_render` and checks bounded rounded widget geometry instead of exact fixed plain-text height/width.
- `vac-rs/tui/src/operator_ui.rs` now delegates `render_operator_snapshot` directly to `operator_widget_render`, removing the unreachable legacy pseudo-layout branch.
- `vac-rs/tui/src/operator_widget_render.rs` now uses the live idle placeholder for idle snapshots, avoiding stale update-banner copy.
- `docs/validation/tui-operator-ui-snapshots/*.txt` were refreshed to rounded widget geometry and scrubbed of stale pseudo-layout strings.

## Validation

Executed in this sandbox and recorded in `docs/monolith-quality/logs/o5o6-tui-widget-snapshot-closeout-final-validation.log`:

- `scripts/check-tui-operator-snapshot-contract.sh` — PASS source/static, cargo-backed widget test NotEvaluated because cargo/rustc are not available.
- `scripts/check-vac-tui-visual-parity-static.sh` — PASS source/static, live screenshot/pixel gate TV-Pending.
- `scripts/check-vac-runtime-engine-wiring-static.sh` — PASS source/static, cargo build/test TV-Pending.
- `scripts/check-vac-o6-1-depanic-surface.sh` — PASS after registry re-pin (`files=1253`, `total_runtime_panic_surface=251`, `cfg_test_lines_removed=134215`).
- `scripts/check-vac-o6-2-safety-coverage.sh` — PASS.
- `scripts/check-vac-o6-quality-triage.sh` — PASS.
- `scripts/check-vac-o5-5-donor-delete-gate.sh` — PASS.
- `scripts/check-vac-o5-o6-completion-state.sh` — PASS with cargo state retained as TV-Pending.
- `scripts/check-vac-o5-o6-monolith-quality-slice.sh` — PASS.
- `scripts/check-vac-o5o6-all-audit-remainder.sh` — PASS source/static with optimized nested O6.1 scan behavior.
- `scripts/check-tui-source-artifact-hygiene.sh` — PASS.
- `scripts/check-legal-release-blockers.sh` — PASS.
- `scripts/check-vac-init-registry-strictness-contract.sh` — PASS; rustc unit gate NotEvaluated because rustc is absent.
- `scripts/check-vac-init-registry-validator-contract.sh` — PASS; rustc unit gate NotEvaluated because rustc is absent.
- Stale snapshot grep — PASS; no `gpt-4o`, `327 files indexed`, `VAC 0.4.3 available`, `metrics  [`, `layout  left:`, `right / Diagnostics`, or `Diagnostics fallback` remains under `docs/validation/tui-operator-ui-snapshots`.

## Truth status

- SV-Done: widget snapshot static contract, visual parity static contract, stale snapshot cleanup, O6.1 registry re-pin, aggregate gate timeout hardening, and VAC registry preflight.
- TV-Pending: `cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui operator_ui_snapshot_contract`, targeted/full cargo check/clippy/test, and live TUI screenshot/pixel parity.

## Aggregate gate performance closeout

`check-vac-o5o6-all-audit-remainder.sh` previously re-ran the O6.1 panic-surface scanner through nested sub-gates, which could exceed the transient sandbox command window even after the dedicated O6.1 reproducibility gate had already passed. The aggregate now runs the dedicated O6.1 gate once, then invokes `check-vac-o5-o6-monolith-quality-slice.sh` with `VAC_O6_1_RESCAN=0`. The monolith gate still performs the full scanner by default when run standalone.
