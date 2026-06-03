# Changeset and Evidence contract

## Purpose

Changeset and Evidence contracts explain what changed, why it changed, what approved it, and what validated it.

They are central to safe autonomous coding.

## Changeset model

A changeset is a structured representation of file mutations.

```yaml
changeset:
  id: changeset_123
  task_id: task_123
  status: preview
  entries:
    - path: src/auth.rs
      action: modify
      reason: Fix expired token error mapping.
      risk: safe_edit
      preview_available: true
```

Statuses:

```text
preview
approved
applied
failed
reverted
```

Entry actions:

```text
create
modify
delete
rename
chmod
```

## Diff levels

VAC should support multiple diff levels.

```text
raw_diff
semantic_diff
domain_diff
evidence_diff
```

### raw_diff

Line-level diff.

### semantic_diff

Diff with reason, owner capability, risk, and validation link.

### domain_diff

Domain-specific diff such as workflow step changes, expression changes, or handler parity changes.

### evidence_diff

Final review summary linking changes to approvals and validation.

## Patch preview

Before mutation, TUI should show patch preview when available.

Preview must include:

- files affected,
- action type,
- risk,
- reason,
- line/file summary,
- validation plan,
- approval requirement.

## Evidence model

Evidence summarizes outcome.

```yaml
evidence:
  id: evidence_123
  task_id: task_123
  result: completed
  changed_files:
    - src/auth.rs
  approvals:
    - approval_123
  validations:
    - command: cargo test -p auth
      status: passed
  why:
    - Fixed expired token error mapping.
  remaining_risks: []
```

## Evidence sources

Evidence can cite:

- user prompt,
- semantic plan,
- tool result,
- approval decision,
- diff entry,
- validation result,
- connector-derived rule,
- policy decision.

Connector-derived evidence must include attribution.

## Revert and restore

Changeset should support restore metadata when available.

```yaml
restore:
  supported: true
  checkpoint_id: checkpoint_123
  files:
    - src/auth.rs
```

Restore must be approval-gated when it mutates files.

## TUI requirements

TUI must show:

- changed file list,
- preview before mutation,
- risk per file or operation,
- approval state,
- validation status,
- evidence summary,
- restore availability.

## Acceptance criteria

MVP acceptance:

```text
planned edits produce changeset preview
approved edits link to approval id
applied edits list changed files
validation results link to task/evidence
final evidence summary is visible
```

Safety acceptance:

```text
delete/rename operations require approval
broad edits are classified
restore action is policy-gated
secret-like diff content is redacted before unsafe display/export
```
