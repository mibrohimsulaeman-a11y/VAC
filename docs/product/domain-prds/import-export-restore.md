# PRD — Import, Export, Restore, and Migration

## Overview

VAC needs recovery and portability features so autonomous work is not a one-way operation.

This PRD covers:

```text
vac.import_export
vac.restore
vac.migrate
vac.update_catalog
vac.backups
vac.recovery
```

## Product goal

VAC should make it safe to recover from agent work, inspect session artifacts, migrate product state, and export evidence.

## User problems

- Agent changes can be hard to reverse.
- Session/evidence cannot be shared or reviewed easily.
- Config/schema changes can break old sessions.
- Users need confidence before approving bigger tasks.

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.backups` | Backup files before risky mutation. | P1 |
| `vac.restore` | Restore files/session state from checkpoint when supported. | P1/P2 |
| `vac.recovery` | Show recovery actions after failure. | P1 |
| `vac.import_export` | Import/export safe session/evidence artifacts. | P2 |
| `vac.migrate` | Migrate config/session/control-plane schemas. | P2 |
| `vac.update_catalog` | Catalog product/runtime updates when supported. | P3 |

## Backup requirements

Before risky mutations, VAC should capture restore metadata.

Backup metadata:

```yaml
backup:
  id: backup_123
  task_id: task_123
  checkpoint_id: checkpoint_123
  files:
    - src/auth.rs
  restore_supported: true
  redaction_status: clean
```

Backups must not copy forbidden files.

## Restore requirements

Restore is a mutating action and must be policy/approval-gated.

Restore UI should show:

- files affected,
- checkpoint source,
- risk,
- preview when available,
- validation suggestion after restore.

## Import/export requirements

Export packages may include:

- evidence summary,
- sanitized transcript,
- changed file list,
- validation summary,
- approval ids,
- redaction report.

Exports must not include raw secrets.

Import should validate schema and trust source before loading.

## Migration requirements

Migration applies to:

- control-plane schema,
- session schema,
- evidence/export schema,
- connector config schema,
- memory/index metadata.

Migration must support dry-run when mutating files.

## Recovery surface

TUI should show recovery actions after:

- failed patch,
- validation failure,
- interrupted task,
- denied policy,
- corrupted session metadata,
- stale connector config.

## TUI surfaces

```text
/recovery
/evidence
/sessions
/status
```

## Acceptance criteria

### MVP

- Changeset/evidence can indicate whether restore is available.
- Failed tasks show recovery hints.
- Restore action is policy-gated.

### Safety

- Backups avoid forbidden paths.
- Exports are redacted.
- Imports validate schema and trust.

### UX

- User can understand what can be restored.
- User can export safe evidence summary.
- User sees migration dry-run before config/schema mutation.
