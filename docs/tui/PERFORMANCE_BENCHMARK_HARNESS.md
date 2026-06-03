# VAC TUI performance benchmark harness

Status: source/static harness ready; runtime numbers are `TV-Pending` until cargo is available.

## Scope

This harness covers the audit findings for draw/render, markdown wrapping, resize/reflow, startup TTFF, event-stream backpressure, and operator-console idle behavior.

## Entry point

```bash
bash scripts/bench-vac-tui-performance.sh
```

If `cargo` is unavailable, the script writes a `NotEvaluated` status file instead of claiming benchmark results.

## Instrumentation

Set `VAC_TUI_PROFILE_STARTUP=1` to emit `vac_tui::startup_profile` tracing phases from `run_main` through the first ratatui handoff.

## Source/static fixes paired with this harness

- Markdown render cache capacity increased from 32 to 1024 and moved from `Vec` scan to `HashMap` + LRU queue.
- Operator console polling now backs off and stops when no runtime status files exist.
- Dead-end `/debug-m-drop` and `/debug-m-update` commands are hidden from the palette.
- `/workflow` prints run/inspect/filter actions directly in the persistent pane.
- Clipboard failures include OSC52/native clipboard/terminal guidance.
