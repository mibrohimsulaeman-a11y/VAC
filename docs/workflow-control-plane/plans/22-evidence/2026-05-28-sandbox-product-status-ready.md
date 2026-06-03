# Product registry status ready sync — sandbox 2026-05-28

Status: ready.

Changes:

- `.vac/registry/product.yaml` now reports `status: ready`, matching `.vac/registry/status.yaml` and the ready domain/capability/workflow manifests.
- `architecture_invariants` now treats `ready` as the expected status-manifest value instead of warning when the root registry is no longer `in_progress`.
- Architecture invariant test fixtures now use `ready` for both product and status manifests.

Rationale:

Plan 19/22 promoted the root manifest/status/domain baseline to ready. Keeping architecture invariants hard-coded to `in_progress` would create stale warning evidence even when the default local product path is closed.

Validation:

- rustfmt on `architecture_invariants.rs`.
- Static scan confirms no `instead of in_progress` invariant warning remains.
- YAML parse confirms registry manifests remain valid.
