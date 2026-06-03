# Capability manifest schema

## Purpose

Capability manifests declare what VAC can do and where that capability appears.

Every backend capability must have a manifest before it can be considered product-ready.

## Implementation note

The current Rust validator accepts the documented canonical shape and the
transition shorthands that appear in the implementation plans:

- `owner` can be a string path or a `{ crate, module, team? }` object.
- `surfaces.tui` and `surfaces.slash` can be a single route/command string or a structured collection.
- `surfaces.cli` can be a boolean or a structured collection with an `enabled` flag and commands.
- `states` can be a string list or a labeled `{ empty, loading, success, failure }` object.

## File location

```text
.vac/capabilities/<capability-id>.yaml
```

## Required fields

| Field | Type | Meaning |
|---|---|---|
| `schema_version` | integer | Manifest schema version. |
| `kind` | string | Must be `capability`. |
| `id` | string | Stable capability id, for example `vac.semantic_loop`. |
| `title` | string | Operator-facing name. |
| `status` | enum | `planned`, `partial`, `ready`, `deprecated`, `disabled`. |
| `owner` | string | Owning crate/module/team. |
| `surfaces` | object | TUI/slash/palette/CLI surfaces. |
| `policy` | object | Policy/risk behavior. |
| `validation` | object | Validation commands or gates. |

## Optional fields

| Field | Type | Meaning |
|---|---|---|
| `description` | string | Short product description. |
| `depends_on` | list | Required capability ids. |
| `optional_depends_on` | list | Optional capability ids. |
| `runtime_events` | list | Runtime events emitted/consumed. |
| `states` | list | TUI states capability can show. |
| `donor_source` | object | Donor mapping if adapted. |
| `docs` | list | PRD/architecture docs. |

## Status values

```text
planned
partial
ready
deprecated
disabled
```

Status semantics:

- `planned`: documented but not implemented.
- `partial`: implemented but missing acceptance criteria.
- `ready`: implemented, visible, policy-bound, and validated.
- `deprecated`: intentionally retained but not product target.
- `disabled`: unavailable by configuration or policy.

## Example

```yaml
schema_version: 1
kind: capability
id: vac.semantic_loop
title: Semantic Coding Loop
status: planned
owner: vac-rs/core/local_runtime
description: Converts normal coding prompts into bounded semantic execution.
surfaces:
  tui:
    routes:
      - /activity
      - /workflow
  slash:
    commands: []
  palette: true
  cli:
    commands:
      - vac exec
policy:
  risk: safe_edit
  mutates_files: policy_decided
  approval_required_for:
    - write_files
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
docs:
  - docs/product/domain-prds/autonomous-semantic-coding.md
  - docs/architecture/semantic-coding-loop.md
```

## Validation errors

| Code | Meaning | TUI/doctor behavior |
|---|---|---|
| `capability.missing_id` | Missing `id`. | Show manifest invalid. |
| `capability.invalid_status` | Unknown status. | Show allowed values. |
| `capability.missing_surface` | No surface declaration. | Show hidden capability warning. |
| `capability.missing_policy` | No policy. | Block ready status. |
| `capability.missing_validation` | No validation. | Block ready status. |

## TUI projection

`/capabilities` must show:

- id,
- title,
- status,
- owner,
- surfaces,
- policy class,
- validation status,
- manifest diagnostics.

## Ready gate

A capability cannot be `ready` unless:

```text
manifest valid
owner exists
surface exists
policy declared
validation declared
TUI/CLI route works
acceptance gate passed
```
