# .vac/registry

Declarative product metadata for the VAC control plane. The registry is the entry point that `vac doctor registry .` loads to discover product identity, domain coverage, and readiness status.

- **Schema source**: registry loaders under `vac-rs/control-plane/src/control_plane/` (consumed by `vac doctor registry`).
- **Schema doc**: [`docs/workflow-control-plane/schema/registry.schema.md`](../../docs/workflow-control-plane/schema/registry.schema.md) (index: [`docs/workflow-control-plane/schema/INDEX.md`](../../docs/workflow-control-plane/schema/INDEX.md))
- **Doctor gate**: `vac doctor registry .`

## Files

| File | `kind` | Purpose |
|---|---|---|
| `product.yaml` | `product` | Product identity and readiness: `id`, `title`, `command`, `status`, `control_plane`, `runtime`, `launcher`. |
| `domains.yaml` | `domains` | Flat list of capability domains exposed by the product, each with a lifecycle `status`. Must cover every [`.vac/capabilities/*.yaml`](../capabilities/) id (with the `vac.` prefix stripped). |
| `status.yaml` | `status` | Migration status note: current phase, gates, and outstanding constraints (e.g. donor-backed capability prerequisites). |

## Invariants

- All three files pin `schema_version: 1`.
- `product.id` is the product namespace (`vac`); `status.id` is a namespaced status identifier (`vac.migration`).
- `domains.domains[].id` is lowercase and matches the suffix of an existing capability id (`vac.chat` → `chat`, `vac.tui.pty_gate` → `tui.pty_gate`). Doctor fails when domains drift from `.vac/capabilities/*.yaml`.
- `domains.domains[].status` and `status.status` are lifecycle labels. The current root baseline is `ready`; older labels such as `planned`, `partial`, `in_progress`, and `migration` are schema vocabulary for non-ready/future entries and must not be used for the current root seed set without a reasoned downgrade.

## Canonical examples

`product.yaml`, verbatim:

```yaml
schema_version: 1
kind: product
id: vac
title: VAC
command: vac
status: ready
control_plane: .vac
runtime: vac-rs
launcher: vac-cli/bin/vac.js
```

`domains.yaml` excerpt:

```yaml
schema_version: 1
kind: domains
status: ready
domains:
  - id: chat
    status: ready
  - id: approvals
    status: ready
```

## Keeping domains in sync

After adding or renaming a capability, regenerate the domain id set:

```bash
grep -h '^id: vac\.' .vac/capabilities/*.yaml | sed 's/^id: vac\.//' | sort -u
```

The output must match the set of `domains.domains[].id` values. `vac doctor registry .` enforces this link.
