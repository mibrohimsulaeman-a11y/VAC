# O5/O6 Autopilot Runtime Executor Closeout

Date: 2026-06-01

## Root cause

The previous `/runtime` surface had a real status writer and action hook, but the scheduler still behaved like a declarative file-state facade: workflow jobs were discovered as queued, actions only mutated YAML, tabs were presented as text, and refresh required opening the pane or pressing a key.

## Fix

- `vac_init_autopilot_scheduler.rs` now loads scheduled workflow manifests and executes them through `execute_workflow_manifest`.
- Scheduler status now records a real lifecycle: `running` during the tick, then `ok`, `failed`, `waiting_approval`, or `cancelled`.
- Workflow execution events are appended to `.vac/registry/autopilot/runs.log.yaml`.
- `retry` reruns the workflow runner; `cancel` persists a terminal cancellation; `inspect` and `attach` point at the persisted run log.
- `operator_console.rs` now supports Tab/arrow navigation across the five tabs and uses a live pre-draw refresh hook.
- Static gates reject regressions back to status-only scheduler behavior or non-navigable console chrome.

## Verification

Source/static gates passed in the sandbox:

- `scripts/check-vac-control-plane-audit-findings-static.sh`
- `scripts/check-vac-init-registry-validator-contract.sh`
- `scripts/check-vac-o6-1-depanic-surface.sh`
- `VAC_O6_1_RESCAN=0 scripts/check-vac-o5o6-all-audit-remainder.sh`

## TV-Pending

Cargo/rustc and live terminal smoke tests were not available in the sandbox. Required follow-up:

- `cargo check --manifest-path vac-rs/Cargo.toml -p vac-core -p vac-surface-tui -p vac-surface-cli`
- `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core vac_init_autopilot_scheduler`
- `cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui operator_console bottom_pane`
- Live `/runtime` smoke: open console, navigate tabs, observe refresh, run retry/cancel/inspect/attach, and verify status/run/action logs.
