# Session Engine contract

## Purpose

The Session Engine owns durable local work state for VAC.

It makes task execution resumable, inspectable, checkpointed, and auditable.

The Session Engine is not the agent loop itself. It is the durable state layer used by the Local Runtime Contract, semantic coding loop, workflow runner, approval coordinator, and TUI projections.

## Responsibilities

The Session Engine owns:

- session creation,
- task lifecycle records,
- transcript durability,
- approval records,
- tool call records,
- validation records,
- compaction boundaries,
- checkpoints,
- resume candidates,
- stale cleanup,
- session evidence summary.

It must not own:

- model provider transport,
- TUI rendering,
- policy decisions,
- tool execution internals,
- connector credentials.

## Session lifecycle

```text
created
active
waiting_approval
completed
failed
cancelled
archived
```

Allowed transitions:

```text
created -> active
active -> waiting_approval
waiting_approval -> active
active -> completed
active -> failed
active -> cancelled
failed -> archived
completed -> archived
cancelled -> archived
```

## Task lifecycle

```text
pending
running
waiting_approval
completed
failed
cancelled
```

A session may contain multiple tasks, but MVP can start with one active task per session.

## Submit-message lifecycle

A submitted prompt should produce a durable lifecycle:

```text
prompt_received
session_loaded
context_prepared
task_started
plan_ready
policy_checked
approval_wait_if_needed
execution_started
validation_started
validation_finished
evidence_ready
task_finished
```

Each phase should be visible either in TUI activity or in session evidence.

## Transcript model

The transcript is an append-only record of operator-visible work.

Transcript item kinds:

```text
user_message
assistant_message
tool_call
tool_result
approval_request
approval_decision
validation_result
system_notice
checkpoint_notice
compaction_notice
evidence_summary
```

Required fields:

```yaml
transcript_item:
  id: item_123
  session_id: session_123
  task_id: task_123
  kind: tool_result
  created_at: timestamp
  visibility: operator
  redaction_status: clean
  payload: {}
```

## Compaction boundary

Context compaction must be explicit.

A compaction record should include:

- source item range,
- summary id,
- reason,
- token/context savings when available,
- whether raw source remains available locally,
- redaction status.

TUI should show compaction as a visible event, not silent disappearance.

## Checkpoints

A checkpoint is a recoverable state before or after risky work.

Checkpoint kinds:

```text
before_mutation
after_mutation
before_validation
manual
```

Checkpoint record:

```yaml
checkpoint:
  id: checkpoint_123
  session_id: session_123
  task_id: task_123
  kind: before_mutation
  changed_files_snapshot: []
  created_at: timestamp
  restore_supported: true
```

Checkpoint storage must avoid copying secrets into unsafe locations.

## Resume candidates

Resume should not be a blind list of old sessions.

Candidate fields:

- session id,
- task summary,
- cwd,
- last status,
- last activity time,
- pending approval count,
- changed files count,
- validation state,
- resume safety,
- reason for ranking.

Ranking should prefer:

1. active sessions in current cwd,
2. failed sessions with safe recovery,
3. waiting approval sessions,
4. recently updated sessions.

## Approval hydration

When resuming a session, pending approvals must be restored into the Local Runtime Contract and TUI approval surface.

A pending approval must not be lost because the TUI restarted.

## Slash and command routing

Session Engine may record slash commands and command palette actions, but routing decisions belong to surface/workflow registries.

Transcript item:

```yaml
kind: slash_command
payload:
  command: /workflow
  resolved_surface: workflow_browser
```

## Storage contract

Storage implementation is not mandated, but it must provide:

- create session,
- append transcript item,
- append runtime event summary,
- store checkpoint metadata,
- list resume candidates,
- load session snapshot,
- mark stale/archived,
- cleanup old temp artifacts safely.

## TUI projections

TUI should be able to show:

- current session id,
- task summary,
- active step,
- changed files count,
- pending approvals,
- last validation state,
- checkpoint availability,
- resume/recovery action.

## Failure model

Session failures must distinguish:

```text
runtime_failed
validation_failed
approval_rejected
policy_denied
storage_failed
checkpoint_failed
resume_failed
```

Every failure needs:

- operator-facing message,
- recovery hint,
- retry safety,
- affected session/task id.

## Acceptance criteria

MVP acceptance:

```text
session created when local task starts
transcript records user prompt and assistant response
tool/approval/validation events can be recorded
pending approval survives TUI surface refresh
final evidence summary is associated with session
resume candidates can be listed
```

Safety acceptance:

```text
secret-like values are redacted before unsafe display/export
checkpoint storage does not copy forbidden files
approval rejection is durable
failed sessions keep recovery hints
```

UX acceptance:

```text
user can see current session state
user can resume recent work
user can inspect why a task failed
user can see whether restore/checkpoint exists
```
