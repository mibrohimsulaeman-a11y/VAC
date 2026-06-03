# VAC TUI Operator UI Hardening 4-10 Implementation

Status: ready  
Baseline: `vac-source-tui-operator-ui-hardening3.zip`  
Output target: `vac-source-tui-operator-ui-hardening4-10.zip`

This document records the implementation contract for hardening slices 4-10. It is intentionally source-driven and `.vac`-driven so the sandbox can validate the work without a full workspace link.

## Hardening 4 — Capability Dashboard Runtime Fidelity

The dashboard now has typed records instead of string-only rows:

- `CapabilityManifestStatus`
- `CapabilityDashboardRecord`
- `DiagnosticSeverity`
- `ControlPlaneDiagnostic`
- `CapabilityDashboardState::from_manifest_records(...)`

The runtime dashboard adapter in `vac-rs/tui/src/capability_dashboard.rs` maps loaded control-plane manifests into typed dashboard records and maps registry diagnostics into typed diagnostic records. The rendered dashboard still keeps a non-blank fallback when there are no YAML/control-plane errors.

Gate:

```bash
bash scripts/check-tui-capability-dashboard-runtime-contract.sh
```

## Hardening 5 — Agent Streaming Runtime Wiring Contract

The active agent screen keeps the core behavior from Batch 1-3 and adds explicit metadata validation:

- conversation timeline,
- user prompt,
- agent turn metadata,
- thinking/status line,
- context usage bar,
- composer line,
- tool timeline limited to the last five tool entries.

The model still includes the six stable tool states:

```text
queued, running, streaming, passed, failed, cancelled
```

Gate:

```bash
bash scripts/check-tui-agent-streaming-runtime-contract.sh
```

## Hardening 6 — Approval Popup Safety Contract

The approval popup now uses explicit safety/risk state:

- `ApprovalRisk`
- `ApprovalSafetyContext`
- `ApprovalPopupState::policy_gate_summary()`

The UI remains a renderer for policy decisions. It does not evaluate or bypass policy. Destructive shell operations are labeled `DESTRUCTIVE` and the popup keeps cwd/context/risk/policy/hotkeys visible.

Gate:

```bash
bash scripts/check-tui-approval-popup-safety-contract.sh
```

## Hardening 7 — Autopilot Scheduler Monitor-Only Contract

Autopilot runtime jobs remain monitor-only by default. The state now has typed monitor actions:

- `AutopilotActionSpec`
- `AutopilotSchedulerState::default_monitor_actions()`

The panel states that actions are policy-gated and destructive tasks require approval gates.

Gate:

```bash
bash scripts/check-autopilot-scheduler-monitor-only-contract.sh
```

## Hardening 8 — Idle / First Launch Visual Fidelity Matrix

The deterministic renderer now covers a fixed viewport matrix:

```text
120x36
140x40
180x48
```

For every scenario:

```text
first-launch
idle
agent-working
approval-popup
runtime-jobs
capability-dashboard
```

This produces 18 plain snapshots and 18 ANSI snapshots in the validation docs.

Gate:

```bash
bash scripts/check-tui-operator-visual-fidelity-matrix.sh
```

## Hardening 9 — `.vac` Spec-Driven Consolidation

Hardening 4-10 is registered as `.vac` capabilities and workflows, with the TUI surface manifest referencing the new capability ids.

New capability manifests:

```text
.vac/capabilities/tui-capability-dashboard-runtime.yaml
.vac/capabilities/tui-agent-streaming-runtime.yaml
.vac/capabilities/tui-approval-popup-safety.yaml
.vac/capabilities/autopilot-scheduler-monitor-only.yaml
.vac/capabilities/tui-operator-visual-fidelity-matrix.yaml
.vac/capabilities/tui-spec-driven-consolidation.yaml
.vac/capabilities/tui-source-artifact-hygiene.yaml
```

New workflow manifests:

```text
.vac/workflows/maintenance.tui-capability-dashboard-runtime.yaml
.vac/workflows/maintenance.tui-agent-streaming-runtime.yaml
.vac/workflows/maintenance.tui-approval-popup-safety.yaml
.vac/workflows/maintenance.autopilot-scheduler-monitor-only.yaml
.vac/workflows/maintenance.tui-operator-visual-fidelity-matrix.yaml
.vac/workflows/maintenance.tui-spec-driven-consolidation.yaml
.vac/workflows/maintenance.tui-source-artifact-hygiene.yaml
.vac/workflows/maintenance.tui-hardening-4-10.yaml
```

Gate:

```bash
bash scripts/check-tui-spec-driven-consolidation.sh
```

## Hardening 10 — Source Hygiene & Artifact Discipline

The source artifact gate rejects:

- `target/`,
- `.git/`,
- `.rlib`, `.rmeta`, `.o`, `.d`,
- unexpected large non-doc source files.

Gate:

```bash
bash scripts/check-tui-source-artifact-hygiene.sh
```

## Aggregate Gate

```bash
bash scripts/check-tui-hardening-4-10-contract.sh
```

This gate runs the Batch 1-3 regression lock and all Hardening 4-10 gates.
