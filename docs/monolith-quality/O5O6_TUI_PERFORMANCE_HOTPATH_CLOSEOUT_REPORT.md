# O5/O6 TUI Performance Hot-Path Closeout

## Root cause

The latest TUI performance audit found one critical hot-path: `operator_console::pre_draw_tick()` refreshed every second and the refresh path could execute autopilot scheduler work. Even after run-state dedupe, that still mixed rendering with execution and could wake the UI and scheduler continuously while `/runtime` stayed open.

Secondary findings included repeated markdown parsing/wrapping cost for transcript redraws and stale capability wording that still described the scheduler as monitor-only.

## Implementation

- `operator_console.rs`
  - Replaced unconditional `refresh()` with `refresh_view(force)`.
  - `pre_draw_tick()` now calls `refresh_view(false)` and returns `true` only when persisted status signatures changed.
  - Runtime tick checks file metadata for `status.yaml`, `run-state.yaml`, `actions.log.yaml`, and `runs.log.yaml`.
  - Draw path no longer calls `refresh_vac_init_autopilot_scheduler_status`.

- `operator_ui.rs`
  - `AutopilotSchedulerState::from_workspace` is now read-only and only parses `.vac/registry/autopilot/status.yaml`.

- `markdown_render.rs`
  - Added a small bounded in-process memo cache keyed by markdown source, width, and cwd.

- `scripts/check-vac-control-plane-audit-findings-static.sh`
  - Rejects scheduler execution from render paths.
  - Requires dirty signatures and memoized markdown rendering.

## Validation

Source/static gates are expected to pass in this sandbox. Cargo and live profiler checks remain TV-Pending because `rustc/cargo` and an interactive terminal profiler are unavailable.

## Remaining TV-Pending checks

- `cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-tui -p vac-core`
- `cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui operator_console markdown_render wrapping`
- Live `/runtime` idle profile proving no workflow spawn during draw ticks.
- Live markdown/wrapping benchmark on a long transcript.
