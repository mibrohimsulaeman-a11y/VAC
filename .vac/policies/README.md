# .vac/policies

Declarative policy manifests. The control plane evaluates each workflow step against the union of these manifests; the first matching rule wins, otherwise the manifest's `default_decision` applies.

- **Schema source**: [`vac-rs/control-plane/src/control_plane/policy_manifest.rs`](../../vac-rs/control-plane/src/control_plane/policy_manifest.rs)
- **Schema doc**: [`docs/workflow-control-plane/schema/policy-manifest.schema.md`](../../docs/workflow-control-plane/schema/policy-manifest.schema.md) (index: [`docs/workflow-control-plane/schema/INDEX.md`](../../docs/workflow-control-plane/schema/INDEX.md))
- **Doctor gate**: `vac doctor policy .`

## Invariants

The loader (`policy_manifest.rs`) enforces:

- `schema_version: 1`, `kind: policy`.
- `id` non-empty, no whitespace, must start with `vac.`, `product.`, or `maintenance.`.
- `default_decision` ∈ `allow | deny | approval_required | unavailable`.
- Each rule has a unique whitespace-free `id`, a non-empty `reason`, a `decision`, and a `match` that declares **at least one** of `action`, `path`, `tool`, `network`, `data_class`.
- `action` ∈ `filesystem_read | filesystem_write | filesystem_delete | process_execute | network_access | tool_call | credential_read | session_write | checkpoint_write`.
- `path` scope ∈ `project | workspace | any`.
- `data_class` ∈ `source_code | project_docs | config | logs | diff | secret_like | credential | personal_data | connector_knowledge | model_output | command_output`.
- `network` requires a non-empty `host` plus a typed `protocol` ∈ `http | https | socks5Tcp | socks5Udp`.
- `filesystem_*` matchers require `path`; `network_access` requires `network`; `tool_call` requires `tool`; `credential_read` requires `data_class`.
- `allow` is only valid for non-mutating, non-sensitive actions and must declare a narrow scope (`path: project`, a specific `tool`, or a `network` matcher). Mutating or sensitive actions must use `approval_required`, `deny`, or `unavailable`.

## Decision precedence

`deny` > `approval_required` > `allow`. When no rule matches and no manifest covers the intent, the evaluator returns `UnknownPolicy`, which the release gate treats as a hard block for sensitive intents.

## Canonical example

Lifted verbatim from [`default-local.yaml`](default-local.yaml):

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

## Seeded manifests

| File | Default | Purpose |
|---|---|---|
| `default-local.yaml` | `approval_required` | Baseline local execution policy (filesystem, process, secrets). |
| `approval.yaml` | `approval_required` | Operator approval for session and checkpoint writes; credential reads denied. |
| `sandbox.yaml` | `approval_required` | Process execution boundaries; sandbox-layer credential denial. |
| `tools.yaml` | `approval_required` | Tool invocation; deny credential-reading and secret-exporting tools. |
| `filesystem.yaml` | `approval_required` | Read / write / delete decisions per path scope. |
| `network.yaml` | `approval_required` | HTTPS requires approval; HTTP and SOCKS denied. |
