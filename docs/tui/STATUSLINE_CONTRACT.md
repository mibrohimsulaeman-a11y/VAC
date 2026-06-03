# Statusline contract

## Purpose

Statusline gives compact operator state without crowding the TUI.

## Required signals

```text
session status
task status
approval count
sandbox status
provider/model status
connector status
validation status
policy status
```

## Example

```text
session active | task validating | approvals 0 | sandbox degraded | model ready
```

## States

```text
normal
attention
blocked
error
```

## Acceptance

- Pending approval is visible.
- Blocking readiness failure is visible.
- Statusline does not replace detailed doctor/evidence surfaces.
