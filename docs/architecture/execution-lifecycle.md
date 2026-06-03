# VAC execution lifecycle

## Purpose

The execution lifecycle defines how work moves from operator intent to safe completion.

## High-level lifecycle

```text
intent
  -> capability/workflow resolution
  -> input validation
  -> dry-run or plan
  -> policy evaluation
  -> approval when required
  -> execution
  -> validation
  -> evidence/output
  -> lifecycle completion
```

## Detailed lifecycle

### 1. Intent

Intent comes from:

- chat input,
- slash command,
- command palette,
- CLI command,
- workflow selection.

### 2. Resolution

VAC resolves intent to:

- capability id,
- workflow id,
- handler,
- policy class,
- TUI projection.

### 3. Validation

Inputs and manifests are validated before execution.

### 4. Planning/dry-run

For risky or mutating tasks, VAC prepares a dry-run summary.

### 5. Policy evaluation

Policy determines allow, deny, or approval required.

### 6. Approval

TUI presents approval requests through one approval model.

### 7. Execution

Runtime executes typed steps through registered handlers.

### 8. Progress

TUI receives lifecycle events and updates activity/progress.

### 9. Validation

Validation commands or checks run after execution when configured.

### 10. Completion

VAC emits final summary:

- result,
- changed files,
- validation status,
- unresolved issues,
- next recommended action.

## Failure recovery

Every failed execution should say:

- what failed,
- why it failed,
- whether retry is safe,
- what the operator can do next,
- which workflow/capability owns the failure.
