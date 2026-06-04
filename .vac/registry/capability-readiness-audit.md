# Capability Production Readiness Audit

Generated from root `.vac/capabilities` and `.vac/surfaces` after the pruning/fix pass on 2026-06-04.

## Validation Snapshot

- `vac doctor registry .`: diagnostics=0
- `vac doctor surfaces .`: duplicates=0, owner_conflicts=0, palette_drift=0, route_drift=0, ready=true
- `vac doctor docs .`: issues=0
- `vac doctor architecture .`: pass=10, warn=0, fail=0
- `vac doctor workflow .`: supported=66, blocked=0, exit=0, release_status=Pass
- `vac doctor policy .`, `vac doctor evidence .`, `vac doctor init .`: pass

## Rubric

- `ready`: score 90-100, ready manifest, ownership, validation, and current route/registry checks are clean.
- `ready_with_warnings`: score 75-89, reachable but still has non-blocking production hardening work.
- `needs_hardening`: score 60-74, partial or materially incomplete capability.
- `legacy_or_planned`: score 40-59, planned/deprecated/historical capability.
- `disabled_or_quarantined`: score below 40, disabled or quarantined from active execution.

## Summary

- ready: 73
- ready_with_warnings: 0
- needs_hardening: 0
- legacy_or_planned: 0
- disabled_or_quarantined: 0
- total capabilities: 73

## Capability Map

| capability | score | band | status | routes | visible | notes |
|---|---:|---|---|---:|---:|---|
| vac.approvals | 100 | ready | ready | 9 | 9 | root-seed, owned, validation, routes=9 visible=9 |
| vac.architecture | 100 | ready | ready | 2 | 2 | root-seed, owned, validation, routes=2 visible=2 |
| vac.autopilot.scheduler-monitor-only | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.autopilot.scheduler | 100 | ready | ready | 4 | 4 | owned, validation, routes=4 visible=4 |
| vac.build | 100 | ready | ready | 2 | 2 | root-seed, owned, validation, routes=2 visible=2 |
| vac.chat | 100 | ready | ready | 12 | 12 | root-seed, owned, validation, routes=12 visible=12 |
| vac.docs.plan-reconciliation | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.docs.state-refresh | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.docs | 100 | ready | ready | 1 | 1 | root-seed, owned, validation, routes=1 visible=1 |
| vac.init.dogfood-session | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.donor_migration | 100 | ready | ready | 2 | 2 | owned, validation, routes=2 visible=2 |
| vac.init.durable-stores | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.evidence-why-live | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.fixtures-security-regression | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.release.legal-build-blockers | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.identity.check | 100 | ready | ready | 2 | 1 | root-seed, owned, validation, routes=2 visible=1 |
| vac.identity | 100 | ready | ready | 9 | 9 | root-seed, owned, validation, routes=9 visible=9 |
| vac.local_runtime_owner | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.migration-runtime | 100 | ready | ready | 2 | 2 | owned, validation, routes=2 visible=2 |
| vac.monolith.godfile-staging | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.o5-o6.completion-state | 100 | ready | ready | 3 | 3 | owned, validation, routes=3 visible=3 |
| vac.o5-o6.monolith-quality | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.o5o6.audit_closeout_remediation | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.quality.unsafe-safety-coverage | 100 | ready | ready | 2 | 2 | owned, validation, routes=2 visible=2 |
| vac.quality.depanic-unsafe-triage | 100 | ready | ready | 2 | 2 | owned, validation, routes=2 visible=2 |
| vac.ownership | 100 | ready | ready | 3 | 2 | root-seed, owned, validation, routes=3 visible=2 |
| vac.init.production-rc | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.release | 100 | ready | ready | 2 | 2 | root-seed, owned, validation, routes=2 visible=2 |
| vac.runtime_approval_bridge | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.rebrand.safe-provenance-scrub | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.sandbox | 100 | ready | ready | 3 | 2 | root-seed, owned, validation, routes=3 visible=2 |
| vac.init.scanner-hardening-spec-flow | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.scanner-policy-hardening | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.sessions | 100 | ready | ready | 20 | 20 | root-seed, owned, validation, routes=20 visible=20 |
| vac.source_artifact.packaging_gate | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.tools | 100 | ready | ready | 15 | 14 | root-seed, owned, validation, routes=15 visible=14 |
| vac.tui.agent-streaming-runtime | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.ansi-style | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.approval-popup-safety | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.capability-dashboard-runtime | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.operator-live-adapter | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.operator-visual-fidelity-matrix | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.operator-visual-gate | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.pty_gate | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.tui-real-data | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.tui.renderer-semantic-contract | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.tui.source-artifact-hygiene | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.spec-driven-consolidation | 99 | ready | ready | 1 | 0 | owned, validation, routes=1 visible=0 |
| vac.tui.status-output | 100 | ready | ready | 2 | 2 | owned, validation, routes=2 visible=2 |
| vac.tui | 100 | ready | ready | 3 | 3 | root-seed, owned, validation, routes=3 visible=3 |
| vac.tui_session_runtime | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.approval-binding | 100 | ready | ready | 3 | 1 | owned, validation, routes=3 visible=1 |
| vac.init.cli-runtime-foundation | 100 | ready | ready | 9 | 9 | owned, validation, routes=9 visible=9 |
| vac.init.command-gate | 100 | ready | ready | 3 | 1 | owned, validation, routes=3 visible=1 |
| vac.init.control-plane | 100 | ready | ready | 5 | 2 | owned, validation, routes=5 visible=2 |
| vac.init.deterministic-hardening-p0-p4 | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.doctor-release | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.evidence-chain | 100 | ready | ready | 3 | 1 | owned, validation, routes=3 visible=1 |
| vac.init.gap-bc-depth | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.lifecycle | 100 | ready | ready | 4 | 2 | owned, validation, routes=4 visible=2 |
| vac.init.manifest-contracts | 100 | ready | ready | 3 | 2 | owned, validation, routes=3 visible=2 |
| vac.init.memory-governance | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.ownership-scanner | 100 | ready | ready | 3 | 2 | owned, validation, routes=3 visible=2 |
| vac.init.patch-guard | 100 | ready | ready | 3 | 1 | owned, validation, routes=3 visible=1 |
| vac.init.policy-evaluator | 100 | ready | ready | 3 | 1 | owned, validation, routes=3 visible=1 |
| vac.init.registry-strictness | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.registry-validator | 100 | ready | ready | 3 | 2 | owned, validation, routes=3 visible=2 |
| vac.init.risk-policy | 100 | ready | ready | 3 | 1 | owned, validation, routes=3 visible=1 |
| vac.init.runtime-gate-enforcement | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.safe-rationale | 100 | ready | ready | 1 | 1 | owned, validation, routes=1 visible=1 |
| vac.init.schema-envelope | 100 | ready | ready | 3 | 2 | owned, validation, routes=3 visible=2 |
| vac.init.semantic-plan | 100 | ready | ready | 3 | 1 | owned, validation, routes=3 visible=1 |
| vac.workflow | 100 | ready | ready | 15 | 15 | root-seed, owned, validation, routes=15 visible=15 |

## Residual Production Readiness Notes

- All 73 loaded capabilities are scored as `ready`; no capability remains `partial`, `needs_hardening`, planned, disabled, or quarantined.
- Workflow doctor exits successfully and its aggregate now reports `release_status: Pass`.
- Full TUI/CLI builds pass. The full `vac-surface-tui` graph still emits pre-existing optional-path dead-code/unused warnings; functional and visual readiness gates are green, and warning cleanup remains a separate lint-hygiene slice.
