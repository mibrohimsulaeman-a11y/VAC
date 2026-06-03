# Operator Console Visual Fidelity Gate

The VAC TUI visual target is a dark terminal operator console inspired by the uploaded screenshots: compact titlebar, tabbed shell, visible composer, bottom statusline, capability dashboard, agent streaming timeline, guarded approval popup, runtime jobs, and VIL-native idle state.

## What is proven in sandbox

The sandbox gate is deterministic snapshot validation, not a live interactive PTY capture. It proves:

- fixed viewport output for `120x36` and `180x48` surfaces;
- last-five tool timeline behavior;
- destructive approval labels and hotkeys;
- non-blank capability diagnostics;
- monitor-only autopilot scheduler surface;
- semantic ANSI styling with zero layout drift after ANSI stripping;
- live adapter wiring for `/runtime` and `/capabilities`.

## PTY gate

A real PTY proof should be run outside constrained sandbox using `scripts/check-tui-pty-gate.sh` with `VAC_TUI_ENABLE_LIVE_PTY=1`. The contract script remains passable in sandbox and documents the blocker honestly: live terminal screenshots are not claimed unless a real interactive PTY capture is generated.
