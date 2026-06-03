# PRD — Agent orchestration

## Overview

VAC may support multi-agent orchestration, but it must be integrated through the workflow control plane and TUI lifecycle model.

Agent orchestration is not a standalone hidden runtime. It is a capability-backed execution strategy.

## Roles

Initial role model:

- planner: decomposes work,
- executor: performs bounded steps,
- reviewer: validates outputs,
- observer: summarizes state and evidence when needed.

## Communication lanes

Agent communication should remain typed:

- trigger: task initiation and policy signals,
- data: files, tool results, context,
- control: status, checkpoints, approvals, errors.

## State machine

Each agent step should follow an observable lifecycle:

```text
attempt -> observe -> diagnose -> plan -> retry_or_terminal
```

## Orchestration requirements

- every agent action must be tied to a workflow run,
- subagents inherit policy and sandbox constraints,
- token/context budget is visible when available,
- tool calls route through the same tool/policy/approval model,
- nested work emits lifecycle events to TUI.

## TUI requirements

The TUI should show:

- active agents,
- role,
- current step,
- status,
- tool calls,
- approval wait state,
- failure reason,
- result summary.

## Acceptance criteria

- Agent orchestration cannot bypass workflow/policy.
- Subagent work is visible in TUI.
- Approval applies consistently to all agent levels.
- Failed agent step is linked to workflow progress.
