# Plan 04 — Policy manifest schema


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented with audit history; active contract is policy parse/enforcement shape**.
> ## Status
> 5. Expose policy metadata to TUI status/capability rows.

Code evidence:
- `vac-rs/core/src/control_plane/policy_manifest.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/04-evidence/2026-05-28-sandbox-unknown-policy-default-deny.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented with audit history; active contract is policy parse/enforcement shape**.

Target outcome: policy manifests define typed policy domains and can be consumed by workflow/capability validation without ambiguity.

Outputs: policy schema, validation tests, docs for policy domains, and integration points for approval/release/runtime-owner gates.

Requires / Blocks: requires Plan 01 skeleton; blocks approval policy integration, PTY gate policy, and runtime-owner false-green prevention.

Stop conditions: stop if policy semantics are only documented but not represented in typed validation.

Done criteria: valid policies load, invalid domains/fields fail, and downstream plans can reference policy ids safely.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Make safety rules declarative and inspectable by runtime and TUI.

## Status

Implemented in `vac-core` as `vac_core::control_plane::policy_manifest`.
The initial validator enforces schema version `1`, typed policy decisions, typed action/path/network/tool/data-class matchers, and rejects broad allow rules for mutating or sensitive actions.

## Initial policy domains

```text
approval
sandbox
tools
filesystem
network
```

## Implementation

1. Define policy manifest schema.
2. Add policy ids and strict enums.
3. Support read/write/delete/network/tool mutability policy.
4. Require approval declarations for mutating operations.
5. Expose policy metadata to TUI status/capability rows.

## Validation

- Invalid policy enum fails with field path.
- Missing approval requirement on mutating workflow fails once runner supports mutation.
- Policy can be rendered in capability dashboard.
- Broad allow rules without a narrow scope are rejected by the manifest validator.

## Done

Policy is visible and machine-readable before workflows can mutate anything.

## Completion (Phase 4)

All 5 implementation steps and 4 validation criteria are complete.

Implementation:

- [x] **Step 1** — Schema defined in `policy_manifest.rs` (`PolicyManifest`, `parse_policy_manifest`, schema_version=1).
- [x] **Step 2** — Strict enums: `PolicyAction`, `PolicyDecision`, `PolicyPathScope`, `PolicyDataClass`, `PolicyManifestKind`.
- [x] **Step 3** — Read/write/delete/network/tool mutability via `PolicyAction` variants (`FilesystemRead`/`Write`/`Delete`, `NetworkAccess`, `ToolCall`, `ProcessExecute`, `CredentialRead`, `SessionWrite`, `CheckpointWrite`).
- [x] **Step 4** — Approval declarations enforced by `evaluate_step_policy` and `workflow_runner` runtime gates (Phase 3 hardening). Broad allow rules on mutating or sensitive actions rejected by validator.
- [x] **Step 5** — Policy metadata exposed to the TUI capability dashboard. `PolicyDoctorReport::render_tui_lines()` is wired into `vac-rs/tui/src/capability_dashboard.rs` under the `Policy manifests:` section. The `vac doctor policy` CLI surface returns the same report and exits non-zero on load failure via `PolicyDoctorReport::is_failure()`.

Validation:

- [x] Invalid policy enum fails with field path (`rejects_invalid_decision`, `rejects_unknown_kind`).
- [x] Missing approval requirement on mutating workflow fails (Phase 3 runner enforcement).
- [x] Policy renders in capability dashboard (`capability_dashboard_lists_policy_manifests` test).
- [x] Broad allow rules without narrow scope rejected (`rejects_broad_allow_without_scope`).
- [x] Custom/unknown workflow `uses` values default to `UnknownPolicy` instead of bypassing policy (`custom_step_policy_decision_defaults_to_unknown_policy`).

## 2026-05-28 sandbox hardening — unknown step policy default-deny

Evidence: `04-evidence/2026-05-28-sandbox-unknown-policy-default-deny.md`.

The P1 audit finding "Custom step uses bypass policy (return `None`)" is closed.
`evaluate_workflow_step_policy_decisions` now returns a per-step policy report for
every workflow step. Built-in `uses` values still resolve through the typed
`WORKFLOW_STEP_POLICY_INTENTS` vocabulary; custom or unknown `uses` values now
produce `PolicyDecisionReport::UnknownPolicy`, which is treated as
approval-required / non-green by the workflow execution surface.

This preserves extension safety: custom workflow steps are possible only when
explicitly reviewed; they cannot fall through as ungated work merely because the
compile-time vocabulary has no built-in handler.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P0 | Architect | Runtime | ✅ resolved — `rule_matches_intent` evaluates network scope and protocol/host coverage | Keep network deny tests active |
| 2 | P1 | SE | Runtime | ✅ resolved — execution intents include canonical capability ids from the workflow vocabulary | Keep capability-scoped policy matches covered |
| 3 | P1 | SE | Runtime | ✅ resolved — custom/unknown steps now return `UnknownPolicy` instead of bypassing policy | Keep extension steps non-green by default |
| 4 | P1 | Architect | Semantics | ✅ resolved — policy precedence is documented as deny > approval_required/unavailable > allow | Keep multi-manifest semantics stable |
| 5 | P2 | Reviewer | Schema | ✅ resolved — top-level `default_decision: allow` is rejected | Preserve fail-closed fallback posture |
| 6 | P2 | Reviewer | Test | ✅ resolved — precedence tests cover deny/default interactions | Add tests with every new decision kind |
| 7 | P2 | Architect | Schema | ✅ resolved — workflow vocabulary and policy intents are set-equal | Keep intent/vocabulary drift impossible |
| 8 | P2 | SE | Runtime | ✅ resolved — workflow execution re-checks policy blocks before and after approval resolution | Preserve block-over-approval precedence |
| 9 | P3 | Planner | Docs | ✅ resolved — policy precedence is documented in code docs and plan evidence | Keep docs aligned with evaluator |
