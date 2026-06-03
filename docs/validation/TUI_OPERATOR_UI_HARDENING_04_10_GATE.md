# TUI Operator UI Hardening 4-10 Gate

Status: ready

The Hardening 4-10 gate validates that Batch 1-3 foundations are still intact and that the deeper runtime/safety/spec/hygiene contracts are registered.

## Required checks

```bash
bash scripts/check-tui-hardening-regression-lock.sh
bash scripts/check-tui-capability-dashboard-runtime-contract.sh
bash scripts/check-tui-agent-streaming-runtime-contract.sh
bash scripts/check-tui-approval-popup-safety-contract.sh
bash scripts/check-autopilot-scheduler-monitor-only-contract.sh
bash scripts/check-tui-operator-visual-fidelity-matrix.sh
bash scripts/check-tui-spec-driven-consolidation.sh
bash scripts/check-tui-source-artifact-hygiene.sh
```

## Snapshot matrix

The visual matrix must generate 18 plain snapshots and 18 ANSI snapshots:

```text
6 scenarios × 3 viewport sizes = 18 snapshots
```

Viewport sizes:

```text
120x36
140x40
180x48
```

Scenarios:

```text
first-launch
idle
agent-working
approval-popup
runtime-jobs
capability-dashboard
```

## Runtime safety requirements

- Dashboard diagnostics must never render as a blank panel.
- Agent tool timeline must render only the last five tools.
- Approval popup must show `DESTRUCTIVE` when the request is destructive.
- Approval popup must not bypass policy.
- Autopilot scheduler must remain monitor-only and policy-gated.
- All hardening areas must be represented in `.vac` capabilities and workflows.
- Source artifact must not include `target/` or compiled objects.
