# PRD — Trace, Signal, Trajectory, and Why

## Overview

VAC needs explainability surfaces for autonomous execution.

This PRD covers:

```text
vac.signal
vac.trace
vac.trajectory
vac.why
vac.evidence_export
```

## Product goal

VAC should answer:

```text
What happened?
Why did it happen?
Which signal influenced the decision?
What changed?
What can be reviewed or exported?
```

## User problems

- Agent decisions are hard to audit.
- Long tool outputs hide important details.
- Users cannot see why a retry happened.
- Evidence summaries can be too shallow.
- Teams need exportable review artifacts.

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.signal` | Capture bounded useful signals from outputs/events. | P1/P2 |
| `vac.trace` | Record execution events safely with redaction. | P1/P2 |
| `vac.trajectory` | Analyze task path, decisions, and retries. | P2 |
| `vac.why` | User-facing explanation reports. | P1/P2 |
| `vac.evidence_export` | Export reviewable summaries/traces. | P2 |

## Signal model

A signal is a distilled fact useful for the task.

Examples:

- validation failed with specific error,
- file likely owns bug,
- policy denied broad edit,
- retry repeated same failure,
- connector rule influenced plan.

Signal record:

```yaml
signal:
  id: signal_123
  task_id: task_123
  kind: validation_failure
  source: tool_result_456
  summary: expected 401, got 500
  confidence: high
  redaction_status: clean
```

## Trace model

Trace records execution facts.

Trace event kinds:

```text
task_started
plan_ready
tool_started
tool_finished
approval_requested
approval_resolved
validation_finished
retry_started
evidence_ready
task_finished
```

Trace must not expose raw secrets.

## Trajectory model

Trajectory explains the path of a task.

It should include:

- plan versions,
- key decisions,
- retry reasons,
- policy blocks,
- changed scope,
- final outcome.

## Why reports

`/why` should answer user questions like:

```text
Why did VAC edit this file?
Why did VAC retry?
Why did VAC stop?
Why does VAC need approval?
```

A why report should cite evidence ids, not vague claims.

## Export requirements

Evidence export should support:

- task summary,
- changed files,
- approvals,
- validations,
- selected signals,
- redaction summary,
- connector attributions.

Export is policy-gated and redacted by default.

## TUI surfaces

```text
/why
/evidence
/activity
/status
```

TUI should show:

- important signals,
- trace availability,
- trajectory summary,
- export availability,
- redaction warnings.

## Acceptance criteria

### MVP

- Important validation/tool/policy signals are captured.
- Evidence summary can cite signals.
- `/why` can explain at least edit, approval, retry, and stop reasons.

### Safety

- Trace/export is redacted.
- Export requires policy check.
- Connector-derived rules are attributed.

### UX

- User can understand why VAC acted.
- User can review task trajectory after failure.
- User can export a safe summary when supported.
