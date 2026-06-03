# Plan 20 evidence — sandbox donor gate ready sync

Date: 2026-05-28

## Scope

The donor migration capability is promoted as a ready safety gate, not as a claim
that all donor code has been physically deleted.

Ready means:

- donor metadata is represented in `.vac/capabilities/donor_migration.yaml`;
- donor gate workflow steps are granular and safe-runner supported;
- root seed coverage runs before donor migration checks;
- CLI validation remains routed through `vac doctor donor` and donor status scripts.

Individual donor lifecycle closure stays tracked by donor migration status,
compatibility manifests, and delete/defer evidence.
