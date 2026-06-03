# VAC TUI rewire spec

## Goal

Make root TUI submit prompts and render activity through Local Runtime Contract.

## Target flow

```text
bottom input Enter
  -> RuntimeCommand::StartTask
runtime events
  -> activity/progress/approval/validation/evidence projections
approve/reject
  -> RuntimeCommand::Approve or Reject
```

## Required event mapping

| RuntimeEvent | TUI projection |
|---|---|
| `TaskStarted` | activity row, statusline task active |
| `AssistantDelta` | conversation/activity text |
| `ToolCallStarted` | activity/progress row |
| `ApprovalRequested` | approval surface/overlay |
| `ValidationStarted` | validation progress |
| `ValidationFinished` | validation result |
| `TaskCompleted` | evidence summary/completed state |
| `TaskFailed` | failure state with recovery hint |

## UX gate

Use real PTY dogfood, not only unit tests.

## Validation

Use `docs/validation/TUI_PTY_DOGFOOD_GATE.md`.
