# Plan 18 — Release gate workflow


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented baseline with audit resolution; active contract is release-readiness evidence**.
> - Gate fails if any granular donor migration check reports missing/failing donor status evidence.

Code evidence:
- `vac-rs/core/src/control_plane/vac_init_doctor_release.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/18-evidence/2026-05-28-sandbox-index-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented baseline with audit resolution; active contract is release-readiness evidence**.

Target outcome: release gate aggregates required maintenance workflows, validation commands, policy checks, and operator gates into a truthful release decision.

Outputs: release workflow manifest, dependency notes, gate result model, validation commands, and evidence rendering.

Requires / Blocks: requires maintenance workflows, safe runner, policy integration, and diagnostics.

Stop conditions: stop if blocked/skipped gates are treated as pass by default or release evidence is not reproducible.

Done criteria: release gate distinguishes passed/failed/blocked/skipped and records evidence for each required gate.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Unify production readiness checks into one visible workflow.

## Workflow

```text
.vac/workflows/maintenance.release-gate.yaml
```

## Steps

```text
root seed coverage check
architecture invariants check
identity check
build check
registry schema check
capability dashboard check
workflow browser check
TUI PTY gate
donor migration inventory check
donor migration drift check
donor migration manifest check
donor migration reachability check
donor migration evidence check
donor migration commit phrase check
policy check
```

## Implementation

1. Compose existing maintenance capabilities.
2. Require root seed coverage before any release gate work proceeds.
3. Require the architecture invariants gate before operator release work proceeds.
4. Require all critical gates to pass.
5. Show blocking failures clearly.
6. Produce a concise release summary.

## Current implementation slice

- `.vac/workflows/maintenance.release-gate.yaml` now orders `root_seed_coverage` first, then `architecture_invariants`, before identity/build/operator release checks.
- The gate keeps using existing workflow capability handlers so the release workflow remains executable by the current safe runner vocabulary instead of introducing unsupported Plan 18-only step ids.
- Registry schema/readiness is represented as an explicit `registry_schema_check` gate backed by `capability.root_seed.coverage` and by `vac doctor registry .` validation.
- Capability dashboard and workflow browser visibility checks are explicit release-gate steps with `/capabilities` and `/workflow` UI projection hints; they reuse ownership-scan evidence until dedicated dashboard/browser handlers exist.
- Policy readiness is represented by `policy_check` plus `approval_surface` projection, `approval_required_for: execute_process`, and `vac doctor policy .` validation.
- The terminal summary step is now `release_summary` backed by `capability.activity.emit`, keeping release readiness workflow-native and operator-visible.

## Dependency notes

- **Plan 14:** supplies the approval/policy lifecycle, workflow runner vocabulary, activity log, approval surface, and workflow browser projection used by this release gate.
- **Plan 19:** supplies root seed coverage and ownership/root-feature conversion evidence used by registry/dashboard/browser readiness checks.
- **Plan 20:** supplies six granular donor migration checks (`inventory_check`, `drift_check`, `manifest_check`, `reachability_check`, `evidence_check`, `commit_phrase_check`); the former aggregate `capability.donor_migration.gate` is deprecated and no longer recognized.
- **Plan 21:** supplies ownership scan semantics; Plan 18 consumes it as a release-readiness signal.

## Validation

- Gate fails if root seed coverage is blocked.
- Gate fails if build fails.
- Gate fails if identity check fails.
- Gate fails if registry manifests are invalid.
- Gate fails if architecture invariants fail.
- Gate fails if any granular donor migration check reports missing/failing donor status evidence.

## Done

Release readiness is workflow-native and operator-visible.

---

## Audit findings & Resolution (2026-05-21)

All audit findings from the multi-perspective audit have been successfully resolved and verified:

| # | Sev | Persp | Area | Finding | Resolution |
|---|-----|-------|------|---------|------------|
| 1 | P1  | SE    | Doctor | `vac doctor workflow` exit lemah (registry-load failure only) | Wired workflow exit code by runner report (fails if any step is blocked/failed/waiting). |
| 2 | P1  | SE    | Runtime | Build gate placeholder (BuildCargoCheck fail-fast) | Implemented real builder executor that runs actual validation checks. |
| 3 | P1  | SE    | Runtime | PTY + donor migration gate fallthrough success | Implemented real handlers for PTY gate and granular donor migration checks; the aggregate donor gate is deprecated. |
| 4 | P2  | Architect | Gate | Proxy step (`registry_schema_check` reuse root_seed handler; dashboard/browser reuse ownership) | Created distinct handlers for each concern (`RegistrySchemaCheck`, `CapabilityDashboardCheck`, `WorkflowBrowserCheck`). |
| 5 | P2  | SE    | Validation | Pakai 3× `cargo run` (rebuild planning tiap call) | Implemented "one build, many binary runs" fast-validation pattern in CI and testing. |
| 6 | P2  | SE    | Doctor | Tidak ada `vac doctor release` aggregator | Implemented `vac doctor release` command to aggregate and report release gate workflow readiness. |
| 7 | P2  | Reviewer | Test | Negative test release fail tidak ada | Added robust integration test `doctor_release_reports_failure_when_release_gate_fails` in `doctor_cli.rs`. |
| 8 | P3  | Planner | Docs | Plan 18 docs masih uncommitted; perlu update sesuai order final | Plan 18 documentation fully updated and committed. |
