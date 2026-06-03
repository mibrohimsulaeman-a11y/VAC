# O6.3 Docs Pruning Classification

Status: Classification-only / NotEvaluated

No docs are physically deleted in this slice.

Machine-readable state:

```text
.vac/registry/o6-docs-pruning-state.yaml
```

## Summary

```yaml
schema_version: 1
kind: registry_status
id: vac.o6.docs_pruning_state
title: O6.3 docs pruning classification
status: ready
updated_at: '2026-05-30T00:00:00Z'
total_docs: 425
classes:
  archive_candidate: 108
  current: 60
  review_candidate: 145
  uncategorized: 112
samples:
  current:
  - docs/DOCS_AUDIT.md
  - docs/DOCS_INDEX_CURRENT.md
  - docs/PLAN_CODEBASE_RECONCILIATION.md
  - docs/PROJECT_STATE_CURRENT.md
  - docs/donor-migration/DOMAIN_CONTRACT_IMPLEMENTATION.md
  - docs/donor-migration/DONOR_BACKEND_DOC_COVERAGE_AUDIT.md
  - docs/donor-migration/DONOR_COMMIT_POLICY.md
  - docs/donor-migration/DONOR_DELETE_QUARANTINE_GATE.md
  - docs/donor-migration/DONOR_E2E_UX_PRODUCTION_RULE.md
  - docs/donor-migration/DONOR_EXTRACTION_PLAYBOOK.md
  - docs/donor-migration/DONOR_GUARDRAILS.md
  - docs/donor-migration/DONOR_INVENTORY_MATRIX.md
  - docs/donor-migration/DONOR_MAPPING_TEMPLATE.md
  - docs/donor-migration/DONOR_SHELL_STACK_QUARANTINE.md
  - docs/donor-migration/DONOR_STATUS_BOARD.md
  - docs/donor-migration/INDEX.md
  - docs/donor-migration/MISSED_CAPABILITY_REVIEW.md
  - docs/legal/LEGAL_RELEASE_BLOCKERS.md
  - docs/legal/NOTICES.md
  - docs/legal/SAFE_PROVENANCE_SCRUB_O4.md
  uncategorized:
  - docs/architecture/INDEX.md
  - docs/architecture/capability-model.md
  - docs/architecture/changeset-evidence.md
  - docs/architecture/execution-lifecycle.md
  - docs/architecture/local-runtime-contract.md
  - docs/architecture/managed-connectors.md
  - docs/architecture/migration-architecture.md
  - docs/architecture/policy-and-approval.md
  - docs/architecture/product-architecture.md
  - docs/architecture/repository-pattern.md
  - docs/architecture/runtime.md
  - docs/architecture/semantic-coding-loop.md
  - docs/architecture/session-engine.md
  - docs/architecture/tool-contract.md
  - docs/architecture/trust-zones-and-redaction.md
  - docs/architecture/tui-operator-surface.md
  - docs/architecture/vil-native-and-knowledge-add-on.md
  - docs/architecture/workflow-model.md
  - docs/dogfood/2026-05-30-assistant-model-vac-dogfood.md
  - docs/executor-prompts/00A-build-unblock.md
  review_candidate:
  - docs/architecture/control-plane.md
  - docs/executor-prompts/00B-local-runtime-contract-implementation-plan.md
  - docs/migration/00D_RESIDUAL_LEGACY_AUDIT.md
  - docs/migration/00E_REACHABILITY_AUDIT.md
  - docs/scheduled-plans/2026-05-23-plan.md
  - docs/scheduled-plans/2026-05-24-plan.md
  - docs/scheduled-plans/2026-05-25-plan.md
  - docs/scheduled-plans/2026-05-26-plan.md
  - docs/tui/OPERATOR_UI_HARDENING_PLAN_1_10.md
  - docs/validation/VAC_INIT_BASELINE_AUDIT.md
  - docs/validation/VAC_INIT_SEMANTIC_PLAN_GATE.md
  - docs/workflow-control-plane/IMPLEMENTATION_PLAN.md
  - docs/workflow-control-plane/INDEX.md
  - docs/workflow-control-plane/INITIAL_MANIFEST_SET.md
  - docs/workflow-control-plane/INTERFERENCE_AUDIT.md
  - docs/workflow-control-plane/plans/00-operating-contract.md
  - docs/workflow-control-plane/plans/00A-build-unblock.md
  - docs/workflow-control-plane/plans/00B-local-runtime-contract.md
  - docs/workflow-control-plane/plans/00C-rewire-vac-exec.md
  - docs/workflow-control-plane/plans/00D-rewire-vac-tui.md
  archive_candidate:
  - docs/scheduled-audits/INDEX.md
  - docs/scheduled-audits/README.md
  - docs/validation/VAC_INIT_BATCH_02_05_GATE.md
  - docs/validation/VAC_INIT_BATCH_06_12_GATE.md
  - docs/validation/VAC_INIT_BATCH_13_15_GATE.md
  - docs/workflow-control-plane/plans/32-evidence/2026-05-27-sandbox-runtime-owner-threshold-batch.md
  - docs/scheduled-plans/commit-batches/2026-05-24-uncommitted-batches.md
  - docs/scheduled-audits/2026-05-23/1702-repo-sentinel.md
  - docs/scheduled-audits/2026-05-23/1712-repo-sentinel.md
  - docs/scheduled-audits/2026-05-23/1802-repo-sentinel.md
  - docs/scheduled-audits/2026-05-23/1902-repo-sentinel.md
  - docs/scheduled-audits/2026-05-23/2002-repo-sentinel.md
  - docs/scheduled-audits/2026-05-23/2102-repo-sentinel.md
  - docs/scheduled-audits/2026-05-23/2202-repo-sentinel.md
  - docs/scheduled-audits/2026-05-23/2302-repo-sentinel.md
  - docs/scheduled-audits/2026-05-24/0002-repo-sentinel.md
  - docs/scheduled-audits/2026-05-24/0102-repo-sentinel.md
  - docs/scheduled-audits/2026-05-24/0202-repo-sentinel.md
  - docs/scheduled-audits/2026-05-24/0302-repo-sentinel.md
  - docs/scheduled-audits/2026-05-24/0402-repo-sentinel.md
action: classification_only_no_physical_delete
reason: Docs pruning requires link checking and gate coordination; this slice avoids
  silent deletion.

```

Pruning requires link checking and coordination with docs-state and plan-reconciliation gates.
