# Plan 14 approval readiness closeout — 2026-05-28

Status: complete for the default product approval path.

## What changed

- `vac.approvals` is now `ready` instead of `partial`.
- `vac-rs/core/src/control_plane/approval_readiness.rs` records a code-backed readiness report that verifies:
  - the approvals capability manifest is ready,
  - the approval policy manifest exists and has an `approval_required` posture,
  - `maintenance.release-gate` includes `capability.approval.request`,
  - `WORKFLOW_STEP_VOCABULARY` maps approval requests to `vac.approvals`,
  - `FileApprovalStore` remains compile-time reachable as the durable approval store.
- The previous alias bridge wording is replaced by a canonical built-in step vocabulary contract. It is still a closed translation table, but no longer a temporary blocker.

## Validation evidence

- `rustfmt --edition 2024 --check vac-rs/core/src/control_plane/approval_readiness.rs`: pass.
- Targeted Rust harness for approval readiness: pass.
- YAML parse of `.vac/capabilities/approvals.yaml`: pass.

## Release policy

Operator approval evidence remains part of release gate input, but it is no longer an implementation gap in `vac.approvals`. Missing or blocked operator evidence is represented by the release/PTY non-pass policy rather than by keeping the capability partial.
