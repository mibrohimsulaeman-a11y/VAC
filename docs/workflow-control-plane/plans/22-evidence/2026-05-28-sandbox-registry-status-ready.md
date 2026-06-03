# Plan 22 Evidence — Registry status/domain sync ready

Date: 2026-05-28
Environment: ChatGPT sandbox source checkpoint

## Decision

`.vac/registry/status.yaml` and `.vac/registry/domains.yaml` now reflect the current root control-plane state:

- root registry status: `ready`
- every declared root capability has a ready domain entry
- every capability manifest is `ready`
- every maintenance workflow manifest is `ready`

## Interpretation

`ready` describes the default local product path and manifest-backed control plane. It does not imply that historical/non-default app-server compatibility crates have been physically deleted; that remains explicit Plan 33 deferred material.

## Validation

- YAML parse for registry manifests.
- Static status comparison across `.vac/capabilities/*.yaml`, `.vac/workflows/*.yaml`, and `.vac/registry/domains.yaml`.
