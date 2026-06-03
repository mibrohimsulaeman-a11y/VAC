# Plan 31 Inventory — App-server Protocol Retirement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE_DEFAULT_PATH**

Source status lines:
> **Status: historical inventory; default-path implementation is closed.**

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

**Status: historical inventory; default-path implementation is closed.**

This document is retained as the Plan 31 inventory landing page. Older raw scan
outputs from 2026-05-25/26 are superseded by the 2026-05-28 owner-native
closeout evidence and are no longer active blockers.

## Active default-path result

Plan 31 is complete for the default TUI DTO owner-native path:

- active TUI session DTO imports route through owner-native/runtime protocol
  surfaces;
- active default TUI Cargo path no longer depends directly on
  `vac-app-server` or `vac-app-server-protocol`;
- legacy app-server compatibility is explicit non-default quarantine material;
- app-server workspace crate physical deletion remains Plan 33
  operator-gated delete/defer material, not a Plan 31 implementation blocker.

## Current source-of-truth evidence

- `31-evidence/2026-05-27-sandbox-compatibility-inventory.md`
- `31-evidence/2026-05-27-sandbox-dto-owner-mapping.md`
- `31-evidence/2026-05-27-sandbox-owner-native-default-cutover.md`
- `31-evidence/2026-05-27-sandbox-owner-native-runtime-protocol-complete.md`
- `31-evidence/2026-05-27-sandbox-runtime-protocol-dto-closure.md`
- `31-evidence/2026-05-28-sandbox-mapping-superseded.md`

## Active verification

The active Plan 31 check is not the historical raw grep count. It is the default
path invariant:

```text
session_protocol default path: owner-native/runtime protocol
app_server_session default path: owner-native session handle/stub
legacy app-server transport: non-default compatibility feature/quarantine
Plan 33 physical deletion: explicit deferred/operator-gated evidence
```

Suggested operator checks:

```bash
rg -n "^use .*vac_app_server|^pub use .*vac_app_server" vac-rs/tui/src/session_protocol.rs vac-rs/tui/src/app_server_session.rs
rg -n "vac-app-server =|vac-app-server-protocol =" vac-rs/tui/Cargo.toml
rg -n "legacy-app-server-compat|app_server_protocol_compat" vac-rs/tui/src vac-rs/tui/Cargo.toml .vac/capabilities
```

## Historical evidence policy

Historical rows may mention earlier Plan 30G or 31C blockers inside dated
evidence files. Those statements are evidence of prior sequencing only. They do
not describe the current default product path after the 2026-05-28 closeout.
