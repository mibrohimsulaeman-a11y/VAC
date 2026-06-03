# Local Runtime Contract

## Purpose

The Local Runtime Contract is the backend contract for the initial local VAC product path.

It is the shared event/data model used by:

```text
root `vac` TUI path
vac exec
semantic coding loop
workflow runner
approval coordinator
session engine
tool dispatcher
evidence surface
```

The contract exists so local VAC execution is not modeled as a server/client protocol.

## Architecture rule

```text
Local UX talks to Local Runtime Contract.
Local Runtime Contract drives product events.
Product events drive TUI and exec output.
```

The contract must use VAC product terminology:

```text
session
task
command
event
approval
validation
evidence
```

It must not leak server-thread or transport-specific terminology into the local product path.

## Product boundary

The Local Runtime Contract is for local execution only.

In scope:

- `vac` TUI prompt submission,
- `vac exec` prompt submission,
- approval request/resolve,
- activity/progress stream,
- validation events,
- task completion/failure,
- evidence summary,
- session resume handoff.

Out of scope for MVP:

- remote sessions,
- cloud task delegation,
- external protocol serving,
- plugin marketplaces,
- daemon scheduling,
- multi-agent swarm orchestration.

Those can return later only as explicit capabilities.

## Core types

### RuntimeSession

A `RuntimeSession` represents a local operator session.

```yaml
runtime_session:
  id: session_123
  created_at: 2026-05-03T00:00:00Z
  cwd: /repo
  entrypoint: tui
  autonomy_mode: assist
  status: active
```

Required fields:

| Field | Meaning |
|---|---|
| `id` | Stable local session id. |
| `created_at` | Session creation timestamp. |
| `cwd` | Working directory for local execution. |
| `entrypoint` | `tui`, `exec`, or `workflow`. |
| `autonomy_mode` | `suggest`, `assist`, or `autopilot`. |
| `status` | `active`, `waiting_approval`, `completed`, `failed`, `cancelled`. |

### RuntimeTask

A `RuntimeTask` represents one unit of work inside a session.

```yaml
runtime_task:
  id: task_123
  session_id: session_123
  kind: semantic_coding
  prompt: Fix failing tests in auth module.
  status: running
```

Task kinds:

```text
semantic_coding
workflow
review
apply
maintenance
```

Task statuses:

```text
pending
running
waiting_approval
completed
failed
cancelled
```

### RuntimeCommand

`RuntimeCommand` is sent from TUI/CLI to runtime.

Initial commands:

```text
StartTask
CancelTask
Approve
Reject
ResumeSession
RunWorkflow
```

Suggested Rust shape:

```rust
pub enum RuntimeCommand {
    StartTask(StartTask),
    CancelTask { task_id: TaskId },
    Approve { approval_id: ApprovalId },
    Reject { approval_id: ApprovalId },
    ResumeSession { session_id: SessionId },
    RunWorkflow { workflow_id: WorkflowId, input: WorkflowInput },
}
```

### RuntimeEvent

`RuntimeEvent` is emitted by runtime and consumed by TUI/exec.

Initial events:

```text
SessionStarted
TaskStarted
AssistantDelta
ToolCallStarted
ToolCallFinished
ApprovalRequested
ApprovalResolved
ValidationStarted
ValidationFinished
TaskCompleted
TaskFailed
TaskCancelled
```

Near-term extensions:

```text
PlanReady
PatchPreviewReady
PatchApplied
EvidenceReady
WorkflowStepStarted
WorkflowStepFinished
SessionCheckpointCreated
SessionCompacted
```

### ApprovalRequest

`ApprovalRequest` is the only approval request shape for local execution.

```yaml
approval_request:
  id: approval_123
  task_id: task_123
  action: write_files
  risk: safe_edit
  reason: VAC needs to edit 2 files.
  resources:
    files:
      - src/auth.rs
      - tests/auth.rs
  preview:
    kind: diff
  validation_after:
    - cargo test -p auth
```

Required fields:

| Field | Meaning |
|---|---|
| `id` | Stable approval id. |
| `task_id` | Task requesting approval. |
| `action` | Requested action type. |
| `risk` | Policy risk class. |
| `reason` | Operator-facing reason. |
| `resources` | Files/tools/network/config affected. |
| `preview` | Diff/plan/command preview when available. |
| `validation_after` | Validation expected after approval. |

## Event semantics

### Events are append-only facts

Runtime events describe what happened. TUI projections should derive state from events and snapshots, not hidden runtime globals.

### Events are product-level

Bad event naming:

```text
ThreadStartResponseReceived
RpcNotificationPayload
```

Good event naming:

```text
TaskStarted
ApprovalRequested
ValidationFinished
EvidenceReady
```

### Events must be safe to show

Every event payload must be either safe for TUI display or explicitly carry redaction metadata.

## TUI projection requirements

TUI must be able to render these states from runtime events:

```text
empty
starting task
planning
running tool
waiting approval
validating
completed
failed
cancelled
```

Minimum default TUI status:

```text
Task: Fix auth tests
Step: validation
Risk: safe edit
Approval: none pending
Changed: 2 files
```

## `vac exec` requirements

`vac exec` consumes the same runtime event stream as TUI.

It should print:

- task start,
- assistant text deltas,
- tool calls when visible,
- approval failure if interactive approval is unavailable,
- validation result,
- final evidence summary,
- final exit status.

## Error contract

Runtime errors must include:

```yaml
error:
  code: validation_failed
  message: cargo test -p auth failed
  recovery_hint: inspect failing assertion and retry within scope
  retry_safe: true
  owner: vac.validation
```

Required fields:

| Field | Meaning |
|---|---|
| `code` | Stable machine-readable code. |
| `message` | Operator-facing message. |
| `recovery_hint` | Actionable next step. |
| `retry_safe` | Whether automatic retry is allowed. |
| `owner` | Capability/runtime area responsible. |

## Persistence boundary

The Local Runtime Contract does not require all events to be persisted in MVP, but it must not prevent persistence.

Session engine should be able to persist:

- session start,
- task start,
- prompts,
- tool calls,
- approvals,
- validations,
- checkpoints,
- evidence summary,
- final state.

## Implementation path

Start minimal and wire immediately.

Recommended first location:

```text
vac-rs/control-plane/src/local_runtime/
  mod.rs
  command.rs
  event.rs
  session.rs
  approval.rs
```

Extraction to a crate is allowed only after both TUI and exec are wired.

## Acceptance criteria

Architecture acceptance:

```text
vac exec submits local task without app-server client
root `vac` TUI path submits local task without app-server client
TUI receives RuntimeEvent stream
approval request/resolve uses ApprovalRequest
validation emits RuntimeEvent
final evidence emits RuntimeEvent
```

Build/dependency acceptance:

```text
cargo check -p vac-surface-cli passes
server/client runtime crates are not required for local prompt submission
reachability audit shows old runtime crates can be deleted or deferred
```

UX acceptance:

```text
user types prompt
activity appears
approval appears when needed
validation appears
completion/failure appears
terminal exits cleanly
```
