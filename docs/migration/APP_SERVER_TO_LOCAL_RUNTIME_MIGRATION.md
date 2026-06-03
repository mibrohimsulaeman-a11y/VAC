# App-server to Local Runtime migration

## Goal

Move the default local product path from old client/protocol abstractions to Local Runtime Contract.

## Current problem

Local TUI/exec paths still use old runtime client/protocol types for prompt submission, thread/task state, approvals, and events.

## Target replacement

```text
old request/response concepts -> RuntimeCommand
old protocol events -> RuntimeEvent
old thread/session DTOs -> RuntimeSession/RuntimeTask/session engine
old approval DTOs -> ApprovalRequest
```

## Mapping guide

| Current concept | Target concept |
|---|---|
| start thread/task request | `RuntimeCommand::StartTask` |
| resume thread/session | `RuntimeCommand::ResumeSession` |
| read thread/task | session engine snapshot |
| event stream item | `RuntimeEvent` |
| approval request | `ApprovalRequest` |
| task done/fail | `TaskCompleted` / `TaskFailed` |

## Migration order

```text
1. introduce Local Runtime Contract
2. rewire vac exec
3. rewire vac tui
4. remove old client/protocol deps from local path
5. run reachability audit
6. delete or defer old runtime crates
```

## Validation

Use `docs/validation/LOCAL_RUNTIME_GATE.md`.
