# Operator Console Live Adapter

Batch 3 introduces a thin live adapter between the deterministic operator-console render model and ratatui.

## Contract

- `operator_ui` owns layout and state-derived text.
- `operator_style` owns dependency-free semantic classification and ANSI snapshot rendering.
- `operator_ui_styles` maps the same semantic roles into ratatui `Line<'static>` values.
- The adapter must preserve line count and textual content; it may only change style.

## Wired surfaces

- `/runtime` uses `render_autopilot_scheduler_lines()` and then `style_operator_lines_from_strings()`.
- `/capabilities` uses `render_capability_dashboard_shell()` and then `style_operator_lines_from_strings()`.

This keeps the live TUI on the same source-of-truth as the snapshot gate rather than maintaining a separate screenshot-specific renderer.
