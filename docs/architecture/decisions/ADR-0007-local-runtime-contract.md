# ADR-0007 — VAC local product path uses Local Runtime Contract

## Status

Accepted for product architecture.

## Date

2026-05-03

## Context

VAC's product direction is local-first, workflow-native, and TUI-first.

The default user experience should be:

```text
vac
> fix this bug
```

The internal execution model should be:

```text
semantic coding loop
  -> capability registry
  -> policy and approval
  -> bounded execution
  -> validation
  -> evidence
  -> session continuity
```

Recent cleanup successfully reduced the public CLI surface. However, source and dependency audits show the local runtime path is still built on an app-server protocol/client boundary.

Current evidence:

```text
vac-cli
  -> vac-exec
     -> vac-app-server-client
        -> vac-app-server
           -> vac-app-server-transport
              -> vac-api

vac-cli
  -> vac-tui
     -> vac-app-server-client
     -> vac-app-server-protocol
     -> vac-cloud-requirements
```

`vac-exec` currently starts and talks to an in-process app-server client for local headless execution. It uses request/response concepts such as thread start, thread resume, thread read, turn start, review start, and app-server event streams.

`vac-tui` currently starts or connects to app-server clients, imports app-server protocol DTOs, and uses protocol-shaped thread/session/status/realtime/approval models.

This means the product CLI can look clean while the runtime graph remains server/protocol-first.

## Problem

The app-server protocol is the wrong primary abstraction for the initial local VAC product path.

It creates several product and architecture problems:

1. Local task execution is modeled as server request/response traffic.
2. TUI activity and approval state are coupled to protocol DTOs rather than product events.
3. `vac exec` and the root `vac` TUI path depend on server/client crates even when running locally.
4. The dependency graph keeps API, backend, cloud, plugin, and transport stacks reachable.
5. Build failures can surface from remote/server-oriented crates unrelated to local coding UX.
6. The `.vac` workflow-control-plane would be built on an unstable and misleading runtime seam.

## Decision

VAC will introduce and migrate to a Local Runtime Contract for the initial local product path.

The Local Runtime Contract is the product event/data model used by:

```text
vac exec
root `vac` TUI path
semantic coding loop
approval UI
workflow runner
session engine
tool dispatcher
evidence/session surfaces
```

It must not be a rebranded app-server protocol.

It must model local product work in terms of:

```text
session
task
command
event
approval
validation
evidence
```

The app-server/API stack becomes deferred infrastructure for future enterprise/remote capability. It is not the default runtime dependency for local `vac` or `vac exec`.

## Local Runtime Contract shape

### RuntimeSession

```text
RuntimeSession
  id
  created_at
  cwd
  entrypoint: tui | exec | workflow
  autonomy_mode: suggest | assist | autopilot
  status: active | waiting_approval | completed | failed | cancelled
```

### RuntimeTask

```text
RuntimeTask
  id
  session_id
  prompt
  kind: semantic_coding | workflow | review | apply
  status: pending | running | waiting_approval | completed | failed | cancelled
```

### RuntimeCommand

```text
RuntimeCommand
  StartTask { prompt, autonomy_mode, entrypoint }
  CancelTask { task_id }
  Approve { approval_id }
  Reject { approval_id }
  ResumeSession { session_id }
  RunWorkflow { workflow_id, input }
```

### RuntimeEvent

Initial event set:

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

Expected near-term extensions:

```text
PlanReady
PatchPreviewReady
PatchApplied
EvidenceReady
WorkflowStepStarted
WorkflowStepFinished
```

### ApprovalRequest

```text
ApprovalRequest
  id
  task_id
  action
  risk
  reason
  affected resources
  preview kind
  validation after approval
```

## Implementation location

Start as a focused local module or crate, whichever is least disruptive.

Preferred initial low-risk path:

```text
vac-rs/control-plane/src/local_runtime/
  mod.rs
  command.rs
  event.rs
  session.rs
  approval.rs
```

Extraction into a dedicated crate is allowed after both `vac-exec` and `vac-tui` use the contract:

```text
vac-local-runtime
```

Do not create a large abstraction framework before wiring it into the real TUI and exec paths.

## Migration order

### Phase 1 — Build unblock

Ensure `cargo check -p vac-surface-cli` does not fail on unrelated vendored build inputs.

The local product build should not require vendored native source by default when a runtime readiness warning is sufficient.

### Phase 2 — Define Local Runtime Contract

Add minimal command/event/session/approval DTOs.

The contract must be local product terminology, not server-thread terminology.

### Phase 3 — Rewire `vac exec`

Move headless execution from app-server client requests to Local Runtime Contract commands/events.

Acceptance:

```text
vac-exec no longer depends on vac-app-server-client
vac exec can submit a prompt and stream local runtime events
```

### Phase 4 — Rewire the root `vac` TUI path

Move TUI startup, prompt submission, progress, approval, and completion surfaces to Local Runtime Contract events.

Acceptance:

```text
vac-tui no longer depends on vac-app-server-client for local startup
vac-tui no longer requires app-server protocol DTOs for core prompt/activity/approval lifecycle
```

### Phase 5 — Reachability audit and deletion

After `vac-exec` and `vac-tui` no longer use app-server client/protocol directly, rerun dependency reachability.

Only then delete crates that become unreachable.

Candidate crates after successful decoupling:

```text
vac-app-server-client
vac-app-server
vac-app-server-transport
vac-backend-client
vac-cloud-requirements
vac-exec-server
```

`vac-api` may remain temporarily if model/provider paths still need it. It should not remain solely because local TUI/exec pretend to be an app-server client.

## Non-goals

This ADR does not implement:

```text
remote sessions
cloud tasks
teleport/session handoff
ACP-compatible external protocol
plugin marketplace
scheduler/cron/file watcher
multi-agent swarm
```

Those are deferred capabilities and must be represented by `.vac` capability manifests before product exposure.

## Alternatives considered

### Alternative A — Keep app-server protocol as local runtime API

Rejected.

This keeps local VAC coupled to server/client concepts, preserves a heavy dependency graph, and makes `.vac` workflow-control-plane depend on the wrong seam.

### Alternative B — Patch app-server/API stack until build passes

Rejected as the final architecture.

Temporary patches can unblock compilation, but they do not make VAC local-first. This alternative treats symptoms while preserving the server-first runtime shape.

### Alternative C — Create a large new runtime crate before wiring TUI/exec

Rejected.

A standalone backend crate without immediate TUI/exec wiring risks repeating previous backend-only drift. The contract should start minimal and be wired into the real operator path immediately.

### Alternative D — Fake no-op app-server adapters

Rejected.

No-op adapters may make compile easier but can break actual user flows and hide runtime gaps. Decoupling must preserve real prompt submission, activity, approval, validation, and completion behavior.

## Consequences

### Positive

- VAC becomes genuinely local-first.
- `vac exec` and the root `vac` TUI path share one product runtime model.
- TUI activity/progress/approval can be driven by product events.
- App-server/API crates can be deleted or quarantined after reachability drops.
- `.vac` capability/workflow registry will sit on a clean local runtime seam.
- Future remote/server capability can be added explicitly instead of leaking into default local UX.

### Negative

- `vac-exec` and `vac-tui` require non-trivial rewiring.
- Some protocol DTOs must be mapped or replaced by local product DTOs.
- Session/resume flows need careful migration to avoid behavior loss.
- Tests using app-server protocol fixtures need replacement.

### Neutral

- `vac-api` may remain for model/provider transport in the short term.
- App-server stack may return later as a deferred enterprise/remote capability.
- The Local Runtime Contract can later become a separate crate if stability warrants it.

## UX impact

The user-facing result should be:

```text
vac opens TUI
user types prompt
task starts immediately
activity shows progress
approval appears when needed
validation status is visible
final evidence summary is shown
vac exec works headless with the same lifecycle
```

The user should not see or care about app-server, protocol threads, backend clients, or cloud requirements when running local VAC.

## Acceptance criteria

Architecture acceptance:

```text
cargo check -p vac-surface-cli passes
vac-exec no longer depends on vac-app-server-client
vac-tui no longer depends on vac-app-server-client for local path
cargo tree -p vac-surface-cli -i vac-app-server-client is empty or only reachable behind non-default deferred capability
cargo tree -p vac-surface-cli -i vac-app-server is empty or only reachable behind non-default deferred capability
```

UX acceptance:

```text
vac launches TUI
prompt submission starts a task
activity/progress events are visible
approval request and resolution work
validation events are visible
task completion/failure is visible
vac exec works headless
terminal exits cleanly
```

Product acceptance:

```text
local runtime events can support product.semantic-coding-loop
local runtime events can support capability dashboard readiness
app-server/API stack is no longer required for initial local coding UX
remote/server behavior is deferred behind explicit future capability
```

## Follow-up documents

This ADR supports and constrains:

```text
docs/architecture/semantic-coding-loop.md
docs/product/CAPABILITY_MAP.md
docs/product/domain-prds/autonomous-semantic-coding.md
docs/donor-migration/DONOR_BACKEND_DOC_COVERAGE_AUDIT.md
```
