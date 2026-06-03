# Docs plan ↔ codebase reconciliation

Generated: 2026-05-30T00:00:00Z

## Scope

This file reconciles primary planning documents against the current source tree, `.vac` manifests, script gates, and evidence docs. It intentionally does not rewrite historical scheduled audit logs.

## Summary

- Workflow control-plane plan docs reconciled: **43**
- Donor domain plan docs reconciled: **11**
- Machine-readable state: `.vac/registry/plan-state.yaml`

### Workflow plan status counts

| Status | Count |
|---|---:|
| `COMPLETE` | 16 |
| `COMPLETE_DEFAULT_PATH` | 5 |
| `COMPLETE_WITH_DEFERRED_PHYSICAL_DELETE` | 2 |
| `HISTORICAL_SUPERSEDED_COMPLETE` | 1 |
| `IMPLEMENTED` | 19 |

### Donor domain plan status counts

| Status | Count |
|---|---:|
| `CONTRACT_IMPLEMENTED_DEFERRED_GUARDRAIL` | 1 |
| `CONTRACT_IMPLEMENTED_DOC_ONLY_OVERLAY` | 1 |
| `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | 9 |

## Workflow control-plane plans

| Plan | Status | Code evidence | Evidence docs | Caveat |
|---|---|---:|---:|---|
| [00](docs/workflow-control-plane/plans/00-operating-contract.md) | `IMPLEMENTED` | 4 | 0 | full workspace build is not asserted by this reconciliation |
| [00A](docs/workflow-control-plane/plans/00A-build-unblock.md) | `IMPLEMENTED` | 1 | 0 | full workspace build is not asserted by this reconciliation |
| [00B](docs/workflow-control-plane/plans/00B-local-runtime-contract.md) | `COMPLETE` | 3 | 0 | full workspace build is not asserted by this reconciliation |
| [00C](docs/workflow-control-plane/plans/00C-rewire-vac-exec.md) | `IMPLEMENTED` | 2 | 0 | full workspace build is not asserted by this reconciliation |
| [00D](docs/workflow-control-plane/plans/00D-rewire-vac-tui.md) | `COMPLETE` | 3 | 0 | full workspace build is not asserted by this reconciliation |
| [00E](docs/workflow-control-plane/plans/00E-runtime-reachability-delete-gate.md) | `COMPLETE_WITH_DEFERRED_PHYSICAL_DELETE` | 1 | 1 | full workspace build is not asserted by this reconciliation |
| [00F](docs/workflow-control-plane/plans/00F-tui-legacy-transport-retirement.md) | `COMPLETE` | 10 | 0 | full workspace build is not asserted by this reconciliation |
| [01](docs/workflow-control-plane/plans/01-repo-layout-skeleton.md) | `IMPLEMENTED` | 3 | 0 | full workspace build is not asserted by this reconciliation |
| [02](docs/workflow-control-plane/plans/02-capability-schema.md) | `IMPLEMENTED` | 1 | 1 | full workspace build is not asserted by this reconciliation |
| [03](docs/workflow-control-plane/plans/03-workflow-schema.md) | `IMPLEMENTED` | 2 | 1 | full workspace build is not asserted by this reconciliation |
| [04](docs/workflow-control-plane/plans/04-policy-schema.md) | `IMPLEMENTED` | 2 | 1 | full workspace build is not asserted by this reconciliation |
| [05](docs/workflow-control-plane/plans/05-surface-schema.md) | `IMPLEMENTED` | 1 | 1 | full workspace build is not asserted by this reconciliation |
| [06](docs/workflow-control-plane/plans/06-registry-loader.md) | `IMPLEMENTED` | 1 | 1 | full workspace build is not asserted by this reconciliation |
| [07](docs/workflow-control-plane/plans/07-registry-diagnostics.md) | `IMPLEMENTED` | 1 | 1 | full workspace build is not asserted by this reconciliation |
| [08](docs/workflow-control-plane/plans/08-initial-root-manifests.md) | `COMPLETE` | 4 | 1 | full workspace build is not asserted by this reconciliation |
| [09](docs/workflow-control-plane/plans/09-capability-dashboard.md) | `IMPLEMENTED` | 1 | 0 | full workspace build is not asserted by this reconciliation |
| [10](docs/workflow-control-plane/plans/10-workflow-browser.md) | `IMPLEMENTED` | 1 | 0 | full workspace build is not asserted by this reconciliation |
| [11](docs/workflow-control-plane/plans/11-slash-palette-convergence.md) | `COMPLETE` | 3 | 2 | full workspace build is not asserted by this reconciliation |
| [12](docs/workflow-control-plane/plans/12-safe-workflow-runner.md) | `IMPLEMENTED` | 1 | 0 | full workspace build is not asserted by this reconciliation |
| [13](docs/workflow-control-plane/plans/13-workflow-progress-tui.md) | `IMPLEMENTED` | 1 | 0 | full workspace build is not asserted by this reconciliation |
| [14](docs/workflow-control-plane/plans/14-approval-policy-integration.md) | `COMPLETE` | 4 | 1 | full workspace build is not asserted by this reconciliation |
| [15](docs/workflow-control-plane/plans/15-maintenance-identity-check.md) | `COMPLETE` | 1 | 0 | full workspace build is not asserted by this reconciliation |
| [16](docs/workflow-control-plane/plans/16-maintenance-build-check.md) | `COMPLETE` | 3 | 1 | full workspace build is not asserted by this reconciliation |
| [17](docs/workflow-control-plane/plans/17-maintenance-no-duplicate-tui.md) | `IMPLEMENTED` | 6 | 1 | full workspace build is not asserted by this reconciliation |
| [18](docs/workflow-control-plane/plans/18-release-gate.md) | `IMPLEMENTED` | 1 | 1 | full workspace build is not asserted by this reconciliation |
| [19](docs/workflow-control-plane/plans/19-root-feature-conversion.md) | `COMPLETE` | 3 | 2 | full workspace build is not asserted by this reconciliation |
| [20](docs/workflow-control-plane/plans/20-donor-gate.md) | `COMPLETE` | 2 | 1 | full workspace build is not asserted by this reconciliation |
| [21](docs/workflow-control-plane/plans/21-dead-code-and-ownership.md) | `COMPLETE` | 5 | 1 | full workspace build is not asserted by this reconciliation |
| [22](docs/workflow-control-plane/plans/22-enterprise-hygiene.md) | `IMPLEMENTED` | 3 | 3 | full workspace build is not asserted by this reconciliation |
| [23](docs/workflow-control-plane/plans/23-pty-operator-gate.md) | `COMPLETE` | 2 | 3 | full workspace build is not asserted by this reconciliation |
| [24](docs/workflow-control-plane/plans/24-local-runtime-owner-replacement.md) | `COMPLETE` | 2 | 1 | full workspace build is not asserted by this reconciliation |
| [25](docs/workflow-control-plane/plans/25-local-runtime-semantic-contract-hardening.md) | `IMPLEMENTED` | 3 | 1 | full workspace build is not asserted by this reconciliation |
| [26](docs/workflow-control-plane/plans/26-local-runtime-owner-skeleton.md) | `IMPLEMENTED` | 1 | 1 | full workspace build is not asserted by this reconciliation |
| [27](docs/workflow-control-plane/plans/27-retained-resources-startup-replacement.md) | `COMPLETE` | 3 | 1 | full workspace build is not asserted by this reconciliation |
| [28](docs/workflow-control-plane/plans/28-server-request-registry-replacement.md) | `COMPLETE_DEFAULT_PATH` | 3 | 1 | full workspace build is not asserted by this reconciliation |
| [29](docs/workflow-control-plane/plans/29-event-stream-replacement.md) | `COMPLETE_DEFAULT_PATH` | 4 | 1 | full workspace build is not asserted by this reconciliation |
| [30](docs/workflow-control-plane/plans/30-prompt-and-active-controls-cutover.md) | `COMPLETE_DEFAULT_PATH` | 1 | 10 | full workspace build is not asserted by this reconciliation |
| [31](docs/workflow-control-plane/plans/31-inventory.md) | `COMPLETE_DEFAULT_PATH` | 2 | 10 | full workspace build is not asserted by this reconciliation |
| [31](docs/workflow-control-plane/plans/31-mapping.md) | `HISTORICAL_SUPERSEDED_COMPLETE` | 2 | 10 | full workspace build is not asserted by this reconciliation |
| [31](docs/workflow-control-plane/plans/31-protocol-compatibility-retirement.md) | `COMPLETE_DEFAULT_PATH` | 4 | 10 | full workspace build is not asserted by this reconciliation |
| [32](docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md) | `COMPLETE` | 2 | 5 | full workspace build is not asserted by this reconciliation |
| [33](docs/workflow-control-plane/plans/33-app-server-cargo-retirement-delete-defer-proof.md) | `COMPLETE_WITH_DEFERRED_PHYSICAL_DELETE` | 10 | 10 | full workspace build is not asserted by this reconciliation |
| [34](docs/workflow-control-plane/plans/34-zero-config-project-workspace.md) | `COMPLETE` | 2 | 6 | full workspace build is not asserted by this reconciliation |

## Donor domain plans

| Plan | Status | Stance | Code evidence | Caveat |
|---|---|---|---:|---|
| [01](docs/donor-migration/domain-plans/01-session-engine.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [02](docs/donor-migration/domain-plans/02-tool-contract.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [03](docs/donor-migration/domain-plans/03-managed-connectors.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [04](docs/donor-migration/domain-plans/04-changeset-evidence.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [05](docs/donor-migration/domain-plans/05-trust-redaction.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [06](docs/donor-migration/domain-plans/06-context-rag-memory.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [07](docs/donor-migration/domain-plans/07-vil-native.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [08](docs/donor-migration/domain-plans/08-trace-signal-trajectory.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [09](docs/donor-migration/domain-plans/09-agent-orchestration.md) | `CONTRACT_IMPLEMENTED_DEFERRED_GUARDRAIL` | `deferred_guardrail` | 3 | contract implemented does not mean donor source row is product-migrated |
| [10](docs/donor-migration/domain-plans/10-tui-concept-extraction.md) | `CONTRACT_IMPLEMENTED_READY_FOR_GATED_SLICES` | `ready_for_gated_slices` | 3 | contract implemented does not mean donor source row is product-migrated |
| [11](docs/donor-migration/domain-plans/11-external-agent-concept-adoption.md) | `CONTRACT_IMPLEMENTED_DOC_ONLY_OVERLAY` | `doc_only_overlay` | 4 | contract implemented does not mean donor source row is product-migrated |

## Reconciliation rules

- `IMPLEMENTED` / `COMPLETE` means the current codebase contains code or evidence docs supporting the plan status.
- `CONTRACT_IMPLEMENTED_*` for donor plans means the domain plan is code-backed by donor-domain contracts; it does not mean donor source rows are product-migrated.
- Full workspace build remains outside this reconciliation unless explicitly run and recorded separately.
