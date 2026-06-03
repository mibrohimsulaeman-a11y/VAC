# Plan 32 optional manifest operations — 2026-05-27

## Scope

L-32YAML reconciles the three optional Plan 32 manifest operations cited by `docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md` and the L25 threshold proposal.

## Source-of-truth reviewed

- `docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md`
- `docs/workflow-control-plane/plans/32-evidence/2026-05-26-threshold-proposal.md`
- Existing Plan 32 capability manifests:
  - `.vac/capabilities/local_runtime_owner.yaml`
  - `.vac/capabilities/tui_session_runtime.yaml`
  - `.vac/capabilities/runtime_approval_bridge.yaml`

## Operation results

| Operation | Result | Notes |
| --- | --- | --- |
| `.vac/workflows/maintenance.runtime-owner-gate.yaml` | Done | Originally added as `status: planned`; promoted to `ready` after Plan 30/31/33/32 hard-gate closeout. The workflow now points at existing `vac doctor runtime-owner-gates .` / registry commands as release-visible evidence. |
| `.vac/workflows/maintenance.no-app-server-local-path.yaml` | Done | Originally added as `status: planned`; promoted to `ready` after the default TUI path retired app-server protocol/client imports. The workflow records no-app-server local-path scans as release-visible evidence while physical workspace crate deletion remains Plan 33 deferred material. |
| `.vac/policies/runtime-owner-replacement.yaml` | Done | Added a conservative policy manifest for local runtime-owner replacement checks: local read/process/write are approval-visible and network is denied. |

## Deferrals

None in this lane after the 2026-05-28 closeout. The manifests are now `ready`; later Plan 31/33 evidence promoted the default runtime-owner thresholds from staged warnings to hard release-visible gates where safe.

## Validation

- YAML syntax parsed with Python `yaml.safe_load` for the three new manifests.
- 2026-05-28 follow-up: runtime-owner/no-app-server workflow statuses are ready and aligned with the default-path hard-gate closeout.
- `git diff --check` passed.
- Stale-marker scan on touched docs was run with `rg -n "stale|outdated|deprecated|obsolete|TBD|TODO|FIXME|NEEDS HARDENING|DEFERRED|source of truth|canonical"`.
