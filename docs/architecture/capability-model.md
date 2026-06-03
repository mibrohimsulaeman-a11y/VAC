# VAC capability model

## Definition

A capability is a product feature with a declared owner, status, policy, surfaces, states, and validation.

Capabilities are the bridge between product UX and runtime implementation.

## Capability manifest responsibilities

A capability manifest declares:

- id,
- title,
- status,
- owner crate/module,
- exposed surfaces,
- operator states,
- policy class,
- validation commands,
- donor source metadata if applicable.

## Capability lifecycle

```text
planned
partial
ready
broken
retired
```

Optional classifications:

```text
cli_only
test_only
donor_backed
```

## Capability owner

Each capability must have exactly one product owner:

```yaml
owner:
  crate: vac-rs/core
  module: workflow
```

If ownership is unclear, the capability is not ready.

## Capability surfaces

A capability may appear in:

- TUI panel,
- slash command,
- command palette,
- CLI command,
- workflow step,
- status/capability dashboard.

Visible surfaces must match implementation state.

## Capability validation

Every capability must list validation commands or explain why it is documentation-only/planned.

Examples:

```yaml
validation:
  commands:
    - cargo check -p vac-surface-cli
    - cargo test -p vac-core workflow_registry
```

## Donor-backed capabilities

Donor-backed capabilities require extra metadata:

```yaml
donor:
  source: donor/vac/crates/example
  root_target: vac-rs/example
  cleanup_status: donor_only
```

Donor frontend code cannot be a product surface.

## Capability anti-patterns

- implementation exists but no manifest,
- manifest says ready but TUI cannot show it,
- feature has multiple owners,
- command exists but no policy,
- donor code copied without cleanup status.
