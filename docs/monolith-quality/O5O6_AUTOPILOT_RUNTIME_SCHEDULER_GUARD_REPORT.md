# O5/O6 Autopilot Runtime Scheduler Guard Closeout

## Root cause

The prior executor closeout correctly moved `/runtime` from a status facade to a `workflow_runner`-backed executor. The remaining critical audit finding was refresh correctness: the operator console calls `pre_draw_tick` every second, and each refresh could re-execute already-terminal workflows. That made a real executor unsafe because `cargo check` or other allowed workflow steps could respawn repeatedly while the console stayed open.

## Fix

- Added `.vac/registry/autopilot/run-state.yaml` via `AUTOPILOT_RUN_STATE_PATH`.
- Added `AutopilotRunState` persistence with `last_run_at`, `next_run_at`, and terminal state.
- Added `workflow_is_due`, `schedule_interval_secs`, and `format_next_run` helpers.
- `refresh_autopilot_scheduler_status` now skips `ok`, `failed`, `waiting_approval`, and `cancelled` jobs until they are due again or explicitly retried.
- `retry` still forces a workflow rerun through `workflow_runner` and updates run-state.
- Real `.vac` registry workspaces now evaluate workflow approval policy and call `execute_workflow_manifest_with_approval_policy`.
- `cancel` remains honest: it persists a cooperative scheduler skip marker and does not claim to kill a detached process that the current synchronous runner does not expose.
- The static audit gate now rejects missing run-state dedupe, schedule interval logic, or approval-policy-free autopilot execution.

## Validation

PASS source/static:

- `scripts/check-vac-control-plane-audit-findings-static.sh`
- `scripts/check-vac-init-registry-strictness-contract.sh`
- `scripts/check-vac-init-registry-validator-contract.sh`
- `scripts/check-vac-o6-1-depanic-surface.sh`
- `scripts/check-vac-o6-quality-triage.sh`
- `scripts/check-vac-o6-2-safety-coverage.sh`
- `scripts/check-vac-o5-o6-completion-state.sh`
- `VAC_O6_1_RESCAN=0 scripts/check-vac-o5-o6-monolith-quality-slice.sh`
- `VAC_O6_1_RESCAN=0 scripts/check-vac-o5o6-all-audit-remainder.sh`
- `scripts/check-vac-source-artifact-packaging-gate.sh`
- `scripts/check-vac-tui-visual-parity-static.sh`
- `scripts/check-tui-operator-snapshot-contract.sh`

## TV-Pending

- `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core vac_init_autopilot_scheduler`
- `cargo check --manifest-path vac-rs/Cargo.toml -p vac-core -p vac-surface-tui -p vac-surface-cli`
- Live `/runtime` terminal smoke proving no repeated spawn across draw ticks before `next_run`.
- Live approval-policy smoke for a workflow step that requires approval.
- Live retry/cancel/inspect/attach smoke verifying `status.yaml`, `run-state.yaml`, `runs.log.yaml`, and `actions.log.yaml`.

## Honest caveats

This slice closes the critical per-tick respawn bug at source/static level. It does not claim cargo-backed or live terminal success because Rust toolchain and live TUI execution are unavailable in this sandbox. Current cancel semantics are cooperative future-run suppression; true process-kill requires an async job registry or cancellable workflow runner handle.
