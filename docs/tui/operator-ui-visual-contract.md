# VAC TUI Operator UI Visual Contract

Status: batch2 implemented as source-backed snapshot contract.

This document records the visual target used by the `.vac` operator-console TUI renderer. It is intentionally source-driven: the renderer owns structured state models, deterministic text snapshots, and static scripts so the UI can be validated without a full workspace build or a live terminal session.

## Scope

Batch 2 tightens the seven requested operator screens:

1. `/status` remains account/model/provider/token focused and does not render promotional rate-limit/credit copy.
2. First launch renders a terminal operator shell with title, tabs, startup snapshot, composer, and bottom statusline.
3. Capability Dashboard renders a 3-column control-plane view: workspace feed, registry/table, diagnostics.
4. Agent streaming renders conversation metadata, thinking/status line, context bar, and only the last five tool events.
5. Approval popup renders warning-style bordered content with destructive classification and policy-gated hotkeys.
6. Runtime jobs renders the monitor-only autopilot scheduler panel.
7. Idle renders `VIL-native ready`, recent tasks, update notice, composer, and statusline.

## Source of truth

- Renderer state/model: `vac-rs/tui/src/operator_ui.rs`
- Snapshot renderer script: `scripts/render-tui-operator-snapshots.sh`
- Visual contract gate: `scripts/check-tui-operator-ui-visual-contract.sh`
- Static operator contract gate: `scripts/check-tui-operator-ui-contract.sh`
- Autopilot contract gate: `scripts/check-autopilot-scheduler-contract.sh`

## Snapshot sizes

The deterministic snapshots are rendered at fixed terminal dimensions:

- `first-launch-120x36.txt`
- `idle-120x36.txt`
- `agent-working-120x36.txt`
- `approval-popup-120x36.txt`
- `runtime-jobs-120x36.txt`
- `capability-dashboard-180x48.txt`

These files are generated into `docs/validation/tui-operator-ui-snapshots/` and serve as the non-live visual proof for sandbox validation.

## Non-goals

The snapshot renderer does not claim pixel-perfect ANSI color fidelity. It preserves screen structure, density, viewport chrome, content ordering, labels, and policy/diagnostic text so later live TUI testing can focus on ratatui style polish instead of re-discovering layout gaps.
