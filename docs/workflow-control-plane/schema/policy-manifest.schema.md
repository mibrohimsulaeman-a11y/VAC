# Policy manifest schema

## Purpose

Policy manifests define safety behavior for files, tools, processes, network, connectors, memory, evidence, and workflows.

## Implementation note

The initial Rust validator lives in `vac-core` under `vac_core::control_plane::policy_manifest`.
It enforces schema version `1`, typed decisions, and a strict manifest shape with explicit matchers for action, path, tool, network, and data class.

## File location

```text
.vac/policies/<policy-id>.yaml
```

## Required fields

| Field | Type | Meaning |
|---|---|---|
| `schema_version` | integer | Manifest schema version. |
| `kind` | string | Must be `policy`. |
| `id` | string | Stable policy id. |
| `title` | string | Operator-facing name. |
| `default_decision` | enum | `deny`, `approval_required`, `allow`. |
| `rules` | list | Ordered policy rules. |

## Policy decisions

```text
allow
deny
approval_required
unavailable
```

## Rule fields

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Stable rule id. |
| `match` | object | Resource/action matcher. |
| `decision` | enum | Policy decision. |
| `reason` | string | Operator-facing reason. |

## Match fields

| Field | Type | Meaning |
|---|---|---|
| `action` | enum | Permission/action class such as `filesystem_read`, `filesystem_write`, `process_execute`, `network_access`, or `tool_call`. |
| `path` | enum | Path scope such as `project`, `workspace`, or `any`. |
| `tool` | string | Tool id or tool name when matching tool invocations. |
| `network` | object | Network target matcher. |
| `data_class` | enum | Data classification matcher such as `secret_like` or `credential`. |

## Example

```yaml
schema_version: 1
kind: policy
id: vac.default-local
title: Default local policy
default_decision: approval_required
rules:
  - id: read-project
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: Read-only project inspection is allowed.
  - id: write-project
    match:
      action: filesystem_write
      path: project
    decision: approval_required
    reason: File writes require operator approval.
  - id: deny-secrets
    match:
      data_class: secret_like
    decision: deny
    reason: Secret-like values cannot be exposed.
```

## Validation errors

| Code | Meaning |
|---|---|
| `policy.missing_default` | No default decision. |
| `policy.invalid_decision` | Unknown policy decision. |
| `policy.rule_without_reason` | Rule lacks operator-facing reason. |
| `policy.unsafe_allow` | Broad allow rule requires explicit review. |

## TUI projection

Policy diagnostics must appear in:

```text
/status
/doctor
/approvals
/capabilities
```

## Ready gate

A policy cannot be active unless:

```text
manifest valid
all decisions known
unsafe broad rules reviewed
approval behavior mapped
redaction behavior declared where applicable
```

The initial validator rejects broad allow rules for mutating or sensitive actions; approval-required or deny rules must stay explicit until later runtime policy wiring is ready.
