# Surface manifest schema

## Purpose

Surface manifests declare how capabilities and workflows appear in TUI, slash commands, command palette, and CLI.

## File location

```text
.vac/surfaces/<surface-id>.yaml
```

## Required fields

| Field | Type | Meaning |
|---|---|---|
| `schema_version` | integer | Manifest schema version. |
| `kind` | string | Must be `surface`. |
| `id` | string | Stable surface id. |
| `title` | string | Operator-facing title. |
| `routes` | list | TUI/slash/palette/CLI routes. |
| `capabilities` | list | Capability ids shown by this surface. |

## Route fields

| Field | Type | Meaning |
|---|---|---|
| `kind` | string | One of `tui`, `slash`, `palette`, `cli`, or `statusline`. |
| `path` | string | Required for `tui` routes. |
| `command` | string | Required for `slash` and `cli` routes. |
| `action` | string | Required for `palette` routes. |
| `label` | string | Required for `statusline` routes. |
| `capability` | string | Capability id owned by the route. |
| `owner` | string | Required when `visible: true`. |
| `visible` | bool | Whether the route is operator-visible. |
| `status` | string | `ready`, `partial`, `planned`, `unavailable`, or `cli_only`; `cli-only` is accepted as a display alias. |
| `reason` | string | Required for `unavailable`, `cli_only`, and `cli-only` routes. |

## Route kinds

```text
tui
slash
palette
cli
statusline
```

## Example

```yaml
schema_version: 1
kind: surface
id: surface.capabilities
title: Capability dashboard
routes:
  - kind: tui
    path: /capabilities
    capability: vac.capabilities
    owner: vac-rs/tui
    visible: true
    status: ready
  - kind: slash
    command: /capabilities
    capability: vac.capabilities
    owner: vac-rs/tui
    visible: true
    status: ready
  - kind: palette
    action: open_capabilities
    capability: vac.capabilities
    owner: vac-rs/tui
    visible: true
    status: partial
capabilities:
  - vac.capabilities
  - vac.semantic_loop
  - vac.approval
  - vac.tools
```

## Validation errors

| Code | Meaning |
|---|---|
| `surface.no_routes` | Surface has no route. |
| `surface.unknown_capability` | References missing capability. |
| `surface.duplicate_route` | Route conflicts with another surface. |
| `surface.hidden_ready_capability` | Ready capability has no surface. |
| `surface.visible_route_missing_owner` | Visible route has no owner. |
| `surface.visible_route_missing_capability` | Visible route capability is not listed on the surface. |
| `surface.route_requires_reason` | `cli_only`, `cli-only`, or `unavailable` route lacks a reason. |

## TUI projection

Surface diagnostics must appear in `/doctor` and `/capabilities`.

## Ready gate

A surface cannot be ready unless:

```text
route is unique
capabilities resolve
keyboard/palette/slash behavior documented
empty/loading/success/failure states exist
```
