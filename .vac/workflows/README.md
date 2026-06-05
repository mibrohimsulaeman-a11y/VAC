# .vac/workflows

Declarative workflow manifests. A workflow composes capability handlers into a typed, reproducible sequence with operator-visible progress, approvals, and validation gates.

- **Schema source**: [`vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_manifest.rs`](../../vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_manifest.rs)
- **Schema doc**: [`docs/workflow-control-plane/schema/workflow-manifest.schema.md`](../../docs/workflow-control-plane/schema/workflow-manifest.schema.md) (index: [`docs/workflow-control-plane/schema/INDEX.md`](../../docs/workflow-control-plane/schema/INDEX.md))
- **Doctor gate**: `vac doctor workflow .`

## Invariants

The loader (`workflow_manifest.rs`) enforces:

- `schema_version: 1`, `kind: workflow`.
- `id` non-empty, no whitespace, must start with `product.`, `maintenance.`, or `vac.`.
- `status` ∈ `planned | partial | ready | disabled`. `ready` workflows must declare a non-empty `validation.commands[]` or `validation.gates[]`.
- `inputs` is a map from input name to `{ type, required, values?, default?, description? }`. `type: enum` requires non-empty `values[]`; other types must not declare `values`. `default` must match `type`; enum defaults must appear in `values`.
- `steps[]` non-empty. Each step has a unique whitespace-free `id`, a `uses` that begins with `capability.`, an optional `when` (boolean expression with `&& || ! == != ( )`, literals, and identifiers), and optional `policy` / `ui` overrides — when present each override must declare at least one decision or projection hint.
- `ui.surface` and `ui.inspect_surface` are absolute paths (`/...`).
- `policy` declares at least one of `default_risk`, `mutates_files`, `network`, `redaction`, or `approval_required_for[]`.
- `validation.gates[]` entries must reference an existing `steps[].id`.

## File naming

`<workflow-id>.yaml` — dots are preserved. Examples: `maintenance.build-check.yaml`, `maintenance.release-gate.yaml`, `maintenance.tui-pty-gate.yaml`.

## Canonical example

Lifted verbatim from [`maintenance.identity-check.yaml`](maintenance.identity-check.yaml):

```yaml
schema_version: 1
kind: workflow
id: maintenance.identity-check
title: Identity check
status: ready
inputs: {}
ui:
  surface: /workflow
  inspect_surface: /debug-config
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
steps:
  - id: orient
    uses: capability.identity.check
  - id: validate
    uses: capability.identity.check
  - id: report
    uses: capability.activity.emit
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.95.0 check -p vac-surface-cli
```

## Active workflows

`maintenance.build-check`, `maintenance.donor-migration-gate`, `maintenance.identity-check`, `maintenance.no-duplicate-tui`, `maintenance.ownership-scan`, `maintenance.release-gate`, `maintenance.tui-pty-gate`.

All maintenance workflows must stay green before donor-backed capabilities are added (see [`.vac/registry/status.yaml`](../registry/status.yaml)).
