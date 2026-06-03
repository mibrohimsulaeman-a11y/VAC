# Evidence Surface contract

## Route

```text
/evidence
```

## Data sources

- session engine,
- changeset/evidence model,
- validation results,
- approval records,
- trace/signal summaries.

## Required sections

```text
task summary
changed files
approvals
validations
why
remaining risks
restore availability
redaction summary
```

## States

```text
empty
building
ready
failed
redacted
```

## Actions

```text
Enter: inspect item
e: export when policy allows
r: restore when available and approved
Esc: close detail
```

## Acceptance

- User can see what changed and why.
- Validation result is visible.
- Redaction is indicated without exposing hidden data.
