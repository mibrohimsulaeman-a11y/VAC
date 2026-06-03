# PRD — Changeset, Diff, and Evidence

## Overview

VAC must show what changed, why it changed, what approved it, and what validated it.

This makes autonomous coding reviewable instead of opaque.

## User value

Users can trust VAC because they can answer:

```text
What files changed?
Why did they change?
Was the change approved?
What validation ran?
Can I restore it?
```

## Requirements

- create changeset preview before mutation when possible,
- classify risk per changed file/action,
- show raw and semantic diff,
- link edits to approval decisions,
- link edits to validation results,
- produce final evidence summary,
- support restore/checkpoint metadata when available,
- redact sensitive diff content before unsafe display/export.

## TUI surfaces

```text
/activity
/evidence
/changeset
approval detail
workflow progress
```

## Acceptance criteria

- User sees changed file list.
- User sees preview before approving mutation.
- User sees validation result after edit.
- User sees final evidence summary.
- User can tell whether restore/checkpoint exists.
