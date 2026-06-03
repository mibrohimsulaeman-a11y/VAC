# PRD — Scheduler, Hooks, Monitor, and Autopilot Triggers

## Overview

VAC may eventually run workflows from schedules, hooks, monitors, file changes, or bounded autopilot triggers.

This PRD covers:

```text
vac.scheduler
vac.cron
vac.hooks
vac.monitor
vac.file_watcher
vac.autopilot_triggers
```

## Product stance

This is P2/P3. It must not ship before local TUI/exec, Local Runtime Contract, policy, approval, and session engine are stable.

VAC must not become daemon-heavy by accident.

## Product goal

Enable repeatable and policy-bound automation after the local operator product is trustworthy.

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.scheduler` | Queue and run jobs/workflows. | P2 |
| `vac.cron` | Time-based workflow triggers. | P2/P3 |
| `vac.hooks` | Repo/lifecycle hook triggers. | P2/P3 |
| `vac.monitor` | Monitor repo/process/task state. | P2/P3 |
| `vac.file_watcher` | Trigger checks when files change. | P2/P3 |
| `vac.autopilot_triggers` | Start bounded autonomous work from triggers. | P3 |

## Scheduler requirements

Scheduler jobs must include:

- job id,
- workflow/task id,
- trigger source,
- policy context,
- approval behavior,
- next run/last run,
- status,
- failure reason.

## Trigger requirements

Trigger types:

```text
manual
cron
file_change
repo_hook
monitor_event
external_connector_event
```

All non-manual triggers require policy classification.

## Autopilot safety

Autopilot triggers must be bounded by:

- allowed workflow list,
- max files changed,
- max retries,
- approval requirements,
- quiet hours or run windows when applicable,
- explicit disable switch.

## TUI surfaces

```text
/scheduler
/hooks
/monitor
/status
```

TUI should show:

- scheduled jobs,
- enabled/disabled triggers,
- next run,
- last result,
- pending approvals,
- policy blocks.

## Acceptance criteria

### MVP for this phase

- Scheduler capability can be declared as planned/disabled.
- Manual workflow execution is stable before automatic triggers.
- Trigger policy model exists before trigger execution.

### Safety

- No background mutation without explicit policy and approval model.
- Autopilot trigger cannot run unbounded retries.
- File watcher cannot silently edit files.

### UX

- User can see whether automation is enabled.
- User can disable triggers.
- User can inspect last run and failure reason.
