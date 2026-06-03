# VAC TUI operator surface

## Purpose

The TUI is the operator cockpit. It must show what VAC can do, what it is doing, what needs approval, and what failed.

## Required primary surfaces

```text
chat/input composer
activity log
status bar
approval detail
capability dashboard
workflow browser
workflow progress
policy/diagnostic surface
```

## Capability dashboard

The capability dashboard answers:

```text
What features exist?
Which are ready, partial, planned, broken, retired, or CLI-only?
Where is each feature exposed?
What validates each feature?
What policy applies?
Who owns it?
```

Minimum fields:

- id,
- title,
- status,
- owner,
- TUI surface,
- slash command,
- palette visibility,
- CLI visibility,
- validation command,
- policy classification.

## Workflow browser

The workflow browser answers:

```text
What workflows exist?
What inputs do they need?
What steps will run?
What policy gates apply?
Can this workflow dry-run?
What validates it?
```

## Workflow progress panel

Every workflow run should expose:

```text
pending
running
waiting_approval
success
failed
cancelled
```

Each step row should show:

- step id,
- capability used,
- status,
- short output,
- failure reason,
- approval state if relevant.

## Approval UI

Approval UI must be singular. Feature-specific approval widgets are not allowed.

The approval surface must show:

- requested action,
- risk summary,
- affected files/tools/network,
- policy reason,
- diff/preview when available,
- approve/reject controls,
- resulting lifecycle update.

## TUI completeness rule

A backend capability is not product-ready until the TUI can show:

```text
empty state
loading/running state
success state
failure state
policy/approval state when relevant
```

## Operator anti-patterns

- command succeeds but activity shows nothing,
- backend runs but no progress is visible,
- manifest fails but TUI is blank,
- workflow waits for approval but no approval UI appears,
- capability exists but not listed anywhere.
