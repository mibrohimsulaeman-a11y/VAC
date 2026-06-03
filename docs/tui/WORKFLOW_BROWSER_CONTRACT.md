# Workflow Browser contract

## Route

```text
/workflow
```

## Data sources

- workflow registry,
- capability registry,
- policy registry,
- workflow runtime progress.

## States

```text
empty
loading
ready
running
failed
```

## Required fields

```text
workflow id
title
status
inputs
steps
policy gates
validation gates
current run state
```

## Actions

```text
Enter: inspect workflow
r: refresh
d: dry-run when supported
x: run when supported
Esc: close detail
```

## Acceptance

- Missing step capability is visible.
- Workflow current step is visible during run.
- Policy/approval gates are visible before mutation.
