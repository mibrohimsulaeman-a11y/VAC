# Plan 31 Mapping — App-server DTO Owner-Native Replacement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **HISTORICAL_SUPERSEDED_COMPLETE**

Source status lines:
> **Status: superseded historical mapping blueprint. Active default DTO

Code evidence:
- `vac-rs/runtime-protocol`
- `vac-rs/core/src/local_runtime`

Evidence docs:
- `docs/workflow-control-plane/plans/31-evidence/2026-05-26-app-server-protocol-dep-graph.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-authmode-relocation.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-compat-defer-marker.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-compatibility-inventory.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-dto-owner-mapping.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-owner-native-default-cutover.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-owner-native-runtime-protocol-complete.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-sandbox-runtime-protocol-dto-closure.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-27-threaditem-relocation.md`
- `docs/workflow-control-plane/plans/31-evidence/2026-05-28-sandbox-mapping-superseded.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

**Status: superseded historical mapping blueprint. Active default DTO
owner-native closure is complete in Plan 31.**

This file previously held row-by-row DTO migration blockers. The active mapping
contract is now the closeout evidence set, not the old blocker table.

## Active mapping result

- Default TUI DTO facade resolves through owner-native/runtime protocol DTOs.
- Default TUI session path is app-server-protocol-free.
- Compatibility DTOs that still exist are classified as non-default quarantine
  or delete/defer material.
- Plan 33 owns physical workspace crate deletion and inverse-Cargo proof.

## Current evidence set

- `31-evidence/2026-05-27-sandbox-dto-owner-mapping.md`
- `31-evidence/2026-05-27-sandbox-owner-native-default-cutover.md`
- `31-evidence/2026-05-27-sandbox-runtime-protocol-dto-closure.md`
- `31-evidence/2026-05-27-sandbox-owner-native-runtime-protocol-complete.md`
- `31-evidence/2026-05-28-sandbox-mapping-superseded.md`
- Plan 33 evidence for remaining delete/defer proof.

## Operator meaning

Plan 31 should not be reopened merely because a dated evidence file mentions an
old blocker. Reopen Plan 31 only if the **default** TUI DTO/session path regains
an active `vac_app_server*` import or direct default Cargo dependency.

## Verification class

```text
fail: active default TUI source imports vac_app_server* directly
fail: vac-rs/tui/Cargo.toml default path depends on vac-app-server/protocol
pass: app-server references exist only inside legacy compatibility quarantine or dated evidence
```
