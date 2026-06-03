# PRD — Runtime, scheduler, and sessions

## Overview

VAC runtime manages workflow execution, jobs, lifecycle events, sessions, checkpoints, and recovery.

## Runtime jobs

A job represents active or queued work.

Job lifecycle:

```text
pending -> running -> completed
                  -> failed
                  -> cancelled
```

## Workflow execution state

Every workflow run should track:

- run id,
- workflow id,
- input snapshot,
- step lifecycle,
- approval state,
- validation status,
- output summary,
- failure recovery hint.

## Scheduler requirements

The scheduler should:

- execute ready workflow steps,
- pause steps waiting for approval,
- cancel running work when requested,
- retry failed steps only when safe,
- emit structured lifecycle events.

## Session requirements

A session should contain:

- session id,
- task/workflow metadata,
- model/provider metadata when relevant,
- token/cost summary when available,
- changed files,
- approval records,
- lifecycle events,
- validation results,
- created/updated timestamps.

## Checkpoint and restore

VAC should support scoped recovery:

- checkpoint before risky mutation,
- resume interrupted session,
- inspect changed files,
- restore specific files when supported.

## TUI requirements

The TUI should show:

- active jobs,
- session state,
- workflow progress,
- cancel/retry where safe,
- checkpoint/recovery indicators.

## Acceptance criteria

- Runtime state is visible in TUI.
- Cancellation is real, not cosmetic.
- Session resume is tied to actual persisted state.
- Failed workflow explains retry safety.
