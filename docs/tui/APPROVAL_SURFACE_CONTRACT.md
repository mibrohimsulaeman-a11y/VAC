# Approval Surface contract

## Routes

```text
/approvals
approval detail overlay
```

## Data source

Local Runtime Contract `ApprovalRequested` and `ApprovalResolved` events.

## Required fields

```text
approval id
task id
action
risk
reason
resources
preview
validation after approval
```

## States

```text
none_pending
pending
approved
rejected
expired
failed
```

## Actions

```text
a: approve
r: reject
v: view preview/diff
Esc: return
```

## Acceptance

- Mutating action can show approval reason.
- Reject stops mutating step.
- Approval decision appears in activity/evidence.
