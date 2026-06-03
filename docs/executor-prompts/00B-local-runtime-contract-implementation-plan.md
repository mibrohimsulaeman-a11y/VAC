# Implementation plan — 00B Local Runtime Contract

## Purpose

This is the execution plan for the local agent implementing Step 00B.

Goal:

```text
Introduce the minimal Local Runtime Contract and wire it into at least one real root product path.
```

This plan must be executed after Step 00A build unblock has passed.

Current known gate from Step 00A:

```text
cargo check -p vac-surface-cli: pass
vac-linux-sandbox vendor blocker: fixed
vac-api websocket blocker: fixed
app-server/API decouple: not done
runtime local-first: not done
```

## Must-read docs

Read these first:

```text
docs/architecture/decisions/ADR-0007-local-runtime-contract.md
docs/architecture/local-runtime-contract.md
docs/architecture/semantic-coding-loop.md
docs/architecture/session-engine.md
docs/workflow-control-plane/plans/00B-local-runtime-contract.md
docs/executor-prompts/00B-local-runtime-contract.md
docs/validation/LOCAL_RUNTIME_GATE.md
docs/donor-migration/DONOR_E2E_UX_PRODUCTION_RULE.md
```

## Hard constraints

Do not:

```text
start .vac registry implementation
start donor migration
delete app-server/API crates yet
create a second TUI path
wire donor shell stack
fake a no-op app-server adapter
rename old protocol types and call it done
commit or stash unless explicitly asked
delete caches/vendor folders unless explicitly asked
```

Preserve unrelated working-tree changes.

## Product rule

The Local Runtime Contract must use VAC product terms:

```text
session
task
command
event
approval
validation
evidence
```

It must not mimic old server/thread terminology as the new internal API.

Bad:

```text
ThreadStart
ThreadRead
ClientRequest
ClientResponse
```

Good:

```text
StartTask
TaskStarted
ApprovalRequested
ValidationFinished
TaskCompleted
```

## Implementation scope

Implement the minimum contract needed for later Step 00C and 00D:

```text
RuntimeSession
RuntimeTask
RuntimeCommand
RuntimeEvent
ApprovalRequest
ApprovalResolved
ValidationStarted
ValidationFinished
TaskFailure
RuntimeError
```

The contract must be referenced by real product-local code before Step 00B is considered complete.

Acceptable minimal wiring:

```text
vac-exec references RuntimeCommand/RuntimeEvent in a local adapter or planning seam
or
vac-tui references RuntimeEvent in a local projection seam
or
vac-core exposes local_runtime module with tests and one product crate imports it
```

Unacceptable:

```text
adding DTO structs that nothing imports
adding a standalone crate not used by vac-cli graph
adding only documentation
```

## Recommended file layout

Start inside `vac-core` unless source inspection shows a lower-risk existing location.

Recommended path:

```text
vac-rs/core/src/local_runtime/
  mod.rs
  ids.rs
  session.rs
  task.rs
  command.rs
  event.rs
  approval.rs
  validation.rs
  error.rs
```

Export from:

```text
vac-rs/core/src/lib.rs
```

Do not create `vac-local-runtime` crate yet unless there is a compelling compile/dependency reason.

## Type design

### IDs

Use small typed wrappers or transparent structs.

Required ids:

```text
SessionId
TaskId
ApprovalId
WorkflowId
ToolCallId
```

Design goal:

```text
clear display/debug behavior
cheap clone
serde only if existing crate policy supports it cleanly
no dependency bloat
```

### RuntimeSession

Fields:

```text
id
created_at or created_order if timestamp dependency is undesirable
cwd
entrypoint
autonomy_mode
status
```

Enums:

```text
RuntimeEntrypoint: Tui | Exec | Workflow
AutonomyMode: Suggest | Assist | Autopilot
RuntimeSessionStatus: Active | WaitingApproval | Completed | Failed | Cancelled
```

### RuntimeTask

Fields:

```text
id
session_id
kind
prompt
status
```

Enums:

```text
RuntimeTaskKind: SemanticCoding | Workflow | Review | Apply | Maintenance
RuntimeTaskStatus: Pending | Running | WaitingApproval | Completed | Failed | Cancelled
```

### RuntimeCommand

Initial variants:

```text
StartTask(StartTask)
CancelTask { task_id }
Approve { approval_id }
Reject { approval_id }
ResumeSession { session_id }
RunWorkflow { workflow_id, input }
```

`StartTask` fields:

```text
prompt
autonomy_mode
entrypoint
cwd optional or explicit depending existing runtime style
```

### RuntimeEvent

Initial variants:

```text
SessionStarted(SessionStarted)
TaskStarted(TaskStarted)
AssistantDelta(AssistantDelta)
ToolCallStarted(ToolCallStarted)
ToolCallFinished(ToolCallFinished)
ApprovalRequested(ApprovalRequest)
ApprovalResolved(ApprovalResolved)
ValidationStarted(ValidationStarted)
ValidationFinished(ValidationFinished)
TaskCompleted(TaskCompleted)
TaskFailed(TaskFailed)
TaskCancelled(TaskCancelled)
```

Optional near-term variants may be added if useful but avoid overbuilding:

```text
PlanReady
PatchPreviewReady
PatchApplied
EvidenceReady
WorkflowStepStarted
WorkflowStepFinished
```

### ApprovalRequest

Fields:

```text
id
task_id
action
risk
reason
resources
preview
validation_after
```

Enums:

```text
ApprovalAction: WriteFiles | ExecuteProcess | NetworkAccess | ConnectorCall | Restore | Other
RiskLevel: ReadOnly | SafeEdit | BroadEdit | Destructive | Execute | Network | Credential | Unknown
PreviewKind: None | Text | Diff | Command | FileList
```

### Validation events

`ValidationStarted` fields:

```text
task_id
command_display
```

`ValidationFinished` fields:

```text
task_id
command_display
status
summary
```

Statuses:

```text
Passed
Failed
Skipped
Cancelled
```

### RuntimeError / TaskFailed

Fields:

```text
code
message
recovery_hint
retry_safe
owner
```

## Product wiring requirement

After adding types, wire them into at least one of these seams.

Preferred low-risk wiring option:

```text
vac-exec imports vac_core::local_runtime and constructs RuntimeCommand::StartTask in the path that currently prepares local execution.
```

Alternative acceptable option:

```text
vac-tui imports vac_core::local_runtime::RuntimeEvent and maps at least TaskStarted/TaskFailed into an internal local projection test seam.
```

The wiring does not need to remove app-server-client yet. That is Step 00C/00D.

But Step 00B must create a real bridge point for Step 00C/00D.

## Tests to add

Add focused tests where practical:

```text
construct StartTask command
construct TaskStarted event
construct ApprovalRequest with safe edit risk
construct ValidationFinished passed/failed
ensure Display/Debug output is stable enough for operator text
```

Do not add broad snapshot tests unless needed.

## Validation commands

Run:

```bash
cd vac-rs
cargo +1.93.0 fmt -p vac-core -p vac-exec -p vac-surface-tui -p vac-surface-cli
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-core local_runtime --lib
cargo +1.93.0 metadata --no-deps --format-version 1
```

If `cargo fmt` prints the known `imports_granularity = Item` nightly warning but exits 0, report it as warning, not failure.

If a package test target name differs, run the closest focused test and report the exact command.

## Dependency audit commands

Run these for information only; Step 00B is not expected to remove old runtime stack yet:

```bash
cd vac-rs
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-api --edges normal,build
```

Expected after Step 00B:

```text
old runtime crates may still be reachable
this is acceptable only if Local Runtime Contract exists and has product-local wiring
```

## Acceptance criteria

Step 00B passes only if:

```text
Local Runtime Contract module/types exist
types use product-local terminology
at least one product-local crate/path imports or constructs the contract
cargo check -p vac-surface-cli passes
focused tests pass or are honestly reported if not possible
no donor shell path introduced
no .vac registry implementation started
old runtime stack not deleted prematurely
```

## UX acceptance

This step is architecture groundwork, but it must preserve UX direction.

Report the UX impact explicitly:

```text
This introduces the product event model that will let TUI and exec show task/activity/approval/validation/completion without depending on app-server protocol.
```

Do not claim:

```text
local-first runtime complete
TUI migrated
exec migrated
.vac registry ready
production-ready
```

Those are later gates.

## Final response format

Use this exact structure:

```text
1. Current status: pass/fail/partial
2. Files changed
3. Local Runtime Contract types added
4. Product-local wiring added
5. Tests run and exact results
6. Dependency audit result
7. UX impact
8. Remaining blockers
9. Whether Step 00C can start
```

If the product-local wiring is missing, say:

```text
Step 00B is partial, not complete.
```
