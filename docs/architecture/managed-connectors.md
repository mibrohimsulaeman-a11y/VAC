# Managed Connectors architecture

## Purpose

Managed Connectors let VAC expose optional knowledge and external context through product-grade one-click add-ons.

Users should not need to manually operate connector servers for first-class VAC add-ons.

The first planned example is VIL Knowledge Add-on, but this contract is generic.

## Product rule

```text
Connector UX is simple.
Connector internals are policy-scoped.
Connector results are attributable.
Connector mutation is denied by default.
```

## Connector types

```text
knowledge
rules
examples
docs
project_context
team_context
custom_tooling
```

## Connector lifecycle

```text
disconnected
connecting
connected
degraded
auth_expired
unavailable
disabled
```

TUI must show connector state clearly.

## Connector descriptor

```yaml
connector:
  id: connector.vil_knowledge
  title: VIL Knowledge
  kind: knowledge
  required: false
  connection: one_click
  trust_zone: external_managed
  default_permissions:
    - connector_read
  mutation_allowed: false
```

Required fields:

| Field | Meaning |
|---|---|
| `id` | Stable connector id. |
| `title` | Operator-facing name. |
| `kind` | Knowledge/rules/examples/etc. |
| `required` | Whether product can work without it. |
| `connection` | `one_click`, `local_config`, or `managed`. |
| `trust_zone` | Trust boundary. |
| `default_permissions` | Initial allowed actions. |
| `mutation_allowed` | Default must be false for knowledge connectors. |

## Config scopes

Connector config can be scoped.

```text
user
project
team
environment
```

Resolution order should be explicit and visible in diagnostics.

Connector secrets must not be stored in plain project manifests.

## Connector ACL

Connector ACL defines which runtime areas can use the connector.

```yaml
acl:
  allowed_capabilities:
    - vac.vil_knowledge
    - vac.semantic_loop
  allowed_tools:
    - connector.search
    - connector.get_rule
  denied_actions:
    - write_files
    - run_shell
    - install_dependency
```

## Tool categories

Connector tools should be classified as:

```text
resource_read
search
rule_lookup
example_lookup
rubric_lookup
prompt_template
side_effect_tool
```

`side_effect_tool` is denied by default and requires explicit product decision.

## Elicitation

Elicitation is a connector or runtime request for operator input.

Examples:

- choose rule pack,
- approve connector scope,
- select team knowledge source,
- resolve ambiguous project identity.

Elicitation must produce a visible TUI request and cannot silently block the agent loop.

## Attribution

Connector output used in plans/evidence must be attributable.

Evidence should include:

- connector id,
- resource/rule id when available,
- retrieval time,
- confidence or freshness when available,
- whether the content was team/project/user scoped.

## Failure behavior

Connector failure must degrade gracefully.

Failure classes:

```text
not_connected
auth_expired
network_unavailable
scope_denied
resource_not_found
rate_limited
invalid_response
```

For optional connectors, VAC should continue with native/local capabilities when safe.

## TUI requirements

TUI must show:

- connector status,
- active scopes,
- trust zone,
- enabled tools/resources,
- last error,
- connect/disconnect action,
- rule packs or knowledge packs when applicable.

## Safety requirements

- Knowledge connectors are read-only by default.
- Connector results must go through redaction before display/export.
- Connector tools cannot mutate repo files directly.
- Connector credentials must be scoped and stored securely.
- Connector network behavior must be policy-visible.
- Connector output must not be treated as trusted code.

## Acceptance criteria

MVP acceptance:

```text
connector descriptor exists
connector lifecycle can be represented
TUI can show connected/disconnected/degraded
tool calls are scoped through connector ACL
optional connector failure does not break local core features
```

Safety acceptance:

```text
knowledge connector cannot write files
network use is policy-classified
credentials are not exposed in diagnostics
evidence attributes connector-derived knowledge
```
