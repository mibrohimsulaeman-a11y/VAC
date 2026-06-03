# VAC policy and approval model

## Purpose

Policy defines what VAC may do. Approval lets the operator explicitly allow risky or mutating actions.

## Policy ownership

Policy declarations live in:

```text
.vac/policies/
```

Runtime enforcement lives in Rust.

TUI explanation lives in the operator surface.

## Policy domains

Initial domains:

```text
filesystem
network
tools
workflow
approval
sandbox
secrets
```

## Decision types

```text
allow
deny
approval_required
unavailable
```

## Mutating actions

These require policy evaluation:

- writing files,
- deleting files,
- applying patches,
- running shell commands,
- network access,
- installing dependencies,
- starting services,
- changing configuration,
- invoking donor-backed mutating code.

## Approval flow

```text
workflow step requests action
  -> policy evaluator classifies risk
  -> approval request is emitted when required
  -> TUI shows request and reason
  -> operator approves or rejects
  -> runtime continues or stops
  -> lifecycle event records decision
```

## Approval request fields

- id,
- workflow run id,
- step id,
- capability id,
- action type,
- affected resources,
- risk level,
- policy reason,
- preview/diff,
- created time,
- decision.

## Design constraints

- Approval cannot be hidden in logs.
- Approval cannot be feature-specific UI.
- Approval decisions must update workflow lifecycle.
- Rejected approvals must stop the mutating step.
- Approval requests must never expose secrets.

## Policy anti-patterns

- workflow step directly mutates files,
- tool code asks user separately outside root approval flow,
- policy reason is absent,
- approval is requested but no preview exists when a preview is possible,
- network access happens without declared policy.
