# Capability Dashboard contract

## Route

```text
/capabilities
```

## Data sources

- capability registry,
- manifest diagnostics,
- validation results,
- runtime readiness,
- donor migration status when relevant.

## States

```text
empty
loading
ready
partial
failed
```

## Required columns

```text
id
title
status
owner
surfaces
policy
validation
diagnostics
```

## Empty state

Show:

```text
No capabilities loaded. Run doctor or check .vac registry.
```

## Failure state

Show invalid manifests and recovery hints.

## Keyboard/actions

```text
Enter: inspect capability
/: filter
r: refresh
Esc: close detail
```

## Acceptance

- Ready capability without surface is flagged.
- Invalid capability manifest is visible.
- Planned/partial/ready states are distinct.
