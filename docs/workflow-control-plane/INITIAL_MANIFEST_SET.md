# Initial `.vac` manifest set

> **Historical seed proposal, not current inventory.**
>
> This file records the initial control-plane seed design. It intentionally does not represent the current live `.vac` manifest inventory. For current state, inspect `.vac/`, `vac-rs/core/src/control_plane/root_feature_catalog.rs`, and `./vac-rs/target/debug/vac doctor registry .`.
>
> Current docs should distinguish:
>
> ```text
> root seed catalog = canonical required root features
> current .vac files = live workspace inventory
> product capability map = broader target taxonomy
> ```


## Purpose

This document defines the first manifests that should exist once the Local Runtime Contract gate is complete.

Do not create these manifests as product-ready before Plans 00A through 00E are complete.

## Initial directories

```text
.vac/
  capabilities/
  workflows/
  policies/
  surfaces/
  registry/
```

## P0 capability manifests

| Manifest | Capability id | Initial status | Reason |
|---|---|---|---|
| `.vac/capabilities/chat.yaml` | `vac.chat` | partial | User prompt entry. |
| `.vac/capabilities/local-runtime.yaml` | `vac.local_runtime` | partial | Local product runtime gate. |
| `.vac/capabilities/semantic-loop.yaml` | `vac.semantic_loop` | planned/partial | Default coding loop. |
| `.vac/capabilities/approval.yaml` | `vac.approval` | partial | Single approval model. |
| `.vac/capabilities/policy.yaml` | `vac.policy` | partial | Policy classification. |
| `.vac/capabilities/tools.yaml` | `vac.tools` | partial | Tool dispatch. |
| `.vac/capabilities/tool-contract.yaml` | `vac.tool_contract` | planned/partial | Tool metadata/result envelope. |
| `.vac/capabilities/session-engine.yaml` | `vac.session_engine` | planned/partial | Durable session state. |
| `.vac/capabilities/changeset.yaml` | `vac.changeset` | planned | Edit preview/review. |
| `.vac/capabilities/evidence.yaml` | `vac.evidence` | planned/partial | Final summary. |
| `.vac/capabilities/doctor.yaml` | `vac.doctor` | planned/partial | Readiness surface. |
| `.vac/capabilities/architecture.yaml` | `vac.architecture` | partial | Operating contract and architecture invariants. |
| `.vac/capabilities/donor_migration.yaml` | `vac.donor_migration` | partial | Donor cleanup and reachability gate. |
| `.vac/capabilities/tui.yaml` | `vac.tui` | partial | Root TUI. |
| `.vac/capabilities/exec.yaml` | `vac.exec` | partial | Headless execution. |

## Initial workflow manifests

| Manifest | Workflow id | Initial status | Reason |
|---|---|---|---|
| `.vac/workflows/semantic-coding-loop.yaml` | `product.semantic-coding-loop` | planned | Default prompt-to-work loop. |
| `.vac/workflows/build-check.yaml` | `maintenance.build-check` | planned | Build validation. |
| `.vac/workflows/local-runtime-gate.yaml` | `maintenance.local-runtime-gate` | planned | Validate local runtime decouple. |
| `.vac/workflows/tui-pty-gate.yaml` | `maintenance.tui-pty-gate` | planned | Real operator gate. |
| `.vac/workflows/maintenance.donor-migration-gate.yaml` | `maintenance.donor-migration-gate` | planned | Donor cleanup and reachability gate. |

## Initial policy manifests

| Manifest | Policy id | Initial status | Reason |
|---|---|---|---|
| `.vac/policies/default-local.yaml` | `vac.default-local` | partial | Baseline local policy. |
| `.vac/policies/redaction.yaml` | `vac.redaction-default` | planned/partial | Secret protection. |
| `.vac/policies/connectors.yaml` | `vac.connector-default` | planned | Connector defaults. |

## Initial surface manifests

| Manifest | Surface id | Routes |
|---|---|---|
| `.vac/surfaces/activity.yaml` | `surface.activity` | `/activity`, default TUI activity |
| `.vac/surfaces/capabilities.yaml` | `surface.capabilities` | `/capabilities` |
| `.vac/surfaces/workflow.yaml` | `surface.workflow` | `/workflow` |
| `.vac/surfaces/approvals.yaml` | `surface.approvals` | `/approvals` |
| `.vac/surfaces/evidence.yaml` | `surface.evidence` | `/evidence` |
| `.vac/surfaces/doctor.yaml` | `surface.doctor` | `/doctor`, `/status` |

## Initial registry manifests

```text
.vac/registry/product.yaml
.vac/registry/load-order.yaml
```

## Status rule

Initial manifests should start as `planned` or `partial` unless the feature is proven through the real local product path.

Do not mark `ready` until:

```text
TUI/CLI surface works
policy declared
validation declared
runtime event visible when relevant
dogfood gate passes
```
