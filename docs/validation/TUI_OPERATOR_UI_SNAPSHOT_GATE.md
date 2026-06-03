# TUI operator UI snapshot gate

## Purpose

This gate validates the screenshot-driven operator-console surfaces without requiring a full interactive TTY or full workspace build.

The gate is intentionally text-snapshot based because sandbox builds can time out during heavy linking. It checks the render model that feeds the terminal TUI and keeps the `.vac` surfaces inspectable before live PTY dogfooding.

## Covered surfaces

| Scenario | Snapshot | Viewport |
|---|---|---:|
| First launch / operator shell | `docs/validation/tui-operator-ui-snapshots/first-launch-120x36.txt` | 120x36 |
| Idle ready screen | `docs/validation/tui-operator-ui-snapshots/idle-120x36.txt` | 120x36 |
| Agent working / streaming | `docs/validation/tui-operator-ui-snapshots/agent-working-120x36.txt` | 120x36 |
| Approval popup | `docs/validation/tui-operator-ui-snapshots/approval-popup-120x36.txt` | 120x36 |
| Runtime jobs / autopilot scheduler | `docs/validation/tui-operator-ui-snapshots/runtime-jobs-120x36.txt` | 120x36 |
| Capability dashboard | `docs/validation/tui-operator-ui-snapshots/capability-dashboard-180x48.txt` | 180x48 |

## Contract

The renderer must preserve:

- fixed viewport dimensions for every snapshot;
- terminal-first title bar, tab bar, bottom composer, and statusline;
- first-launch startup snapshot rows for version/provider/model/fallback/profile/rulebook/runtime/isolation/autopilot/mcp/sessions/VIL detect;
- idle `VIL-native ready` state, hints, recent tasks, and update notice;
- agent streaming timeline with only the latest five tools visible;
- destructive approval popup with `DESTRUCTIVE`, policy reason, batch progress, and hotkeys;
- autopilot scheduler monitor-only status, queue/running counts, job table, inspect card, and actions;
- capability dashboard left/center/right layout with YAML/control-plane diagnostics instead of a blank panel.

## Validation command

```bash
bash scripts/check-tui-operator-snapshot-contract.sh
```

The script compiles only the isolated renderer harness via direct `rustc --edition 2024 --test`, generates the text snapshots, and greps the generated output for the operator-surface contract.

## Relationship to PTY dogfood

This gate does not replace live PTY dogfooding. It is the lower-cost render-model gate that should pass before `TUI_PTY_DOGFOOD_GATE.md` is attempted.

If an environment has no real TTY, this gate may pass while the PTY gate remains `BLOCKED-OPERATOR`.
