# PRD — Approvals, policy, and governance

## Overview

VAC requires a structured approval and policy model so agentic work remains safe, inspectable, and operator-controlled.

## Approval model

Every risky or mutating action should produce an approval request unless policy explicitly allows it.

Approval lifecycle:

```text
pending -> approved
        -> rejected
        -> expired
        -> stale
```

## Approval request fields

- id,
- workflow run id,
- step id,
- capability id,
- action type,
- risk level,
- affected resources,
- policy reason,
- preview or diff when available,
- decision state.

## Policy domains

Initial policy domains:

- filesystem,
- network,
- tools,
- workflow,
- sandbox,
- secrets,
- export/import.

## Policy decisions

```text
allow
deny
approval_required
unavailable
```

## Governance requirements

- Secret-like values must be redacted before unsafe exposure.
- Mutating file operations must be policy-gated.
- Network access must be classified.
- Tool execution must include risk metadata.
- Approval decisions must be recorded in workflow/session lifecycle.
- Rejected approvals stop the mutating step.

## TUI requirements

The TUI must show:

- pending approval count,
- approval detail,
- reason and risk,
- affected files/resources,
- approve/reject controls,
- policy source.

## Acceptance criteria

- One approval UI model exists.
- Feature-specific approval UI is not introduced.
- Mutating workflow step routes through policy and approval.
- Approval result is visible in activity and workflow progress.
