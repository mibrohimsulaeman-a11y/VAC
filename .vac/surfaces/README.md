# .vac/surfaces

Declarative surface manifests mapping capabilities to operator interaction points. Surfaces guarantee that every capability is reachable from at least one visible TUI route, slash command, palette action, CLI command, or status line.

- **Schema source**: [`vac-rs/crates/control-plane/control-plane/src/control_plane/surface_manifest.rs`](../../vac-rs/crates/control-plane/control-plane/src/control_plane/surface_manifest.rs)
- **Schema doc**: [`docs/workflow-control-plane/schema/surface-manifest.schema.md`](../../docs/workflow-control-plane/schema/surface-manifest.schema.md) (index: [`docs/workflow-control-plane/schema/INDEX.md`](../../docs/workflow-control-plane/schema/INDEX.md))
- **Doctor gate**: `vac doctor surfaces .`

## Invariants

The loader (`surface_manifest.rs`) enforces:

- `schema_version: 1`, `kind: surface`.
- `id` non-empty, no whitespace, must start with `surface.` (`surface.tui`, `surface.slash`, `surface.palette`, `surface.cli`).
- `capabilities[]` non-empty, unique, and every entry must start with `vac.`.
- `routes[]` non-empty. Each route declares `kind` ∈ `tui | slash | palette | cli | statusline`, `capability` (must start with `vac.`), `visible: bool`, and `status` ∈ `ready | partial | planned | unavailable | cli_only`; the loader also accepts `cli-only` as a display alias.
- Route kind constrains route fields:
  - `tui`: `path` only — must start with `/`, no whitespace.
  - `slash`: `command` only — must start with `/`.
  - `palette`: `action` only — no whitespace.
  - `cli`: `command` only.
  - `statusline`: `label` only.
- `visible: true` routes require a non-empty `owner` and `status` ∈ `ready | partial | planned`, and the route's `capability` must appear in the surface's `capabilities[]`.
- `unavailable`, `cli_only`, and `cli-only` routes require a non-empty `reason`.
- Every capability listed in `capabilities[]` must back at least one `visible: true` route.

## File naming

`<surface-id-suffix>.yaml` — surface id with the `surface.` prefix dropped (`surface.cli` → `cli.yaml`).

## Canonical example

Excerpt from [`cli.yaml`](cli.yaml) (truncated to the first three routes):

```yaml
schema_version: 1
kind: surface
id: surface.cli
title: CLI surface
capabilities:
  - vac.chat
  - vac.sandbox
  - vac.tools
routes:
  - kind: cli
    command: vac
    capability: vac.chat
    owner: "vac-rs/cli"
    visible: true
    status: ready
  - kind: cli
    command: vac exec
    capability: vac.sandbox
    owner: "vac-rs/cli"
    visible: true
    status: ready
  - kind: cli
    command: vac e
    capability: vac.tools
    owner: "vac-rs/cli"
    visible: true
    status: ready
```

## Seeded manifests

`cli.yaml`, `tui.yaml`, `slash.yaml`, `palette.yaml`. Each manifest represents one interaction vector and is composed independently by the doctor; duplicate routes across manifests are rejected.
