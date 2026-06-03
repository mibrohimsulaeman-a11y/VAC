# Plan 14 — Approval and policy integration


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **complete for default approval readiness; active contract is policy-backed approval safety**.

Code evidence:
- `vac-rs/core/src/control_plane/approval_lifecycle.rs`
- `vac-rs/core/src/control_plane/approval_readiness.rs`
- `vac-rs/core/src/control_plane/approval_store.rs`
- `vac-rs/core/src/control_plane/workflow_runner.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/14-evidence/2026-05-28-sandbox-approval-readiness-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **complete for default approval readiness; active contract is policy-backed approval safety**.

Target outcome: workflow and capability actions that require approval are gated by typed policy and visible approval state.

Outputs: policy/approval integration, canonical step vocabulary mapping, durable approval store, tests for approval-required actions, and TUI evidence.

Requires / Blocks: requires policy schema and safe workflow runner; blocks PTY execute-process gate and release gates.

Stop conditions: stop if approvals can be bypassed by alias naming, hidden defaults, or untyped runner paths.

Done criteria: approval-required steps fail or prompt safely according to policy and tests cover allow/deny paths.


> 📋 **Audit findings (2026-05-20)** — lihat [section di akhir file](#audit-findings-2026-05-20). 21 sub-agent paralel + 1 SE cross-cut. Read-only.

## Goal

Ensure workflow actions obey product safety policies.

## Implementation

1. Connect workflow runner to policy registry.
2. Check mutating file/tool/network steps before execution.
3. Route required approvals to existing root approval UI.
4. Block unauthorized actions with visible error.
5. Record approval decisions in workflow lifecycle.

## Validation

- Mutating workflow step requests approval.
- Rejected approval stops workflow.
- Read-only workflow runs without approval.

## Done

The workflow control plane is safe by default.

## Current implementation slice

- The workflow runner now exposes an approval policy report for each workflow manifest, combining workflow policy metadata with the loaded control-plane capability registry.
- The approval policy evaluator resolves the built-in workflow step vocabulary to canonical capability ids so seeded `capability.*` workflows no longer report false `missing capability manifest` blocks when the matching `vac.*` capability manifests are loaded.
- The workflow execution report now respects that approval policy: supported steps that require operator approval pause in `waiting_approval` instead of reporting a false success.
- The workflow browser renders the approval policy summary and per-step decision trace before the dry-run and execution sections.
- The workflow browser also renders the typed activity log trace, so approval-aware browser output shows the started-workflow line and the waiting-approval progress path together.
- `vac doctor workflow` already prints the same approval policy report text, so policy decisions are now visible in both operator diagnostics and the TUI workflow browser.
- Regression coverage now locks the approval-aware report path in three places: `workflow_runner.rs` asserts `load_workflow_run_report(...)` reaches `waiting_approval`, `doctor_cli.rs` covers `vac doctor workflow`, and `workflow_browser.rs` covers the TUI browser progress and activity trace rendering.
- Plan 14A starts the approval lifecycle contract with `ApprovalRequest`, auditable `ApprovalDecision`, `ApprovalStore`, and an in-memory store plus regression coverage for pending, rejected, and double-resolution behavior.
- Workflow execution now creates concrete approval requests when a step enters `waiting_approval`, records the generated `approval_id` in execution events/reports, and exposes that id in the TUI workflow progress model.
- Approval resolution now has a core runner contract: approved requests resume and complete the waiting step, while rejected/expired requests fail the workflow with auditable resolution events and TUI progress lifecycle support.
- Workflow approval requests now export into the local runtime approval contract, producing deterministic local `ApprovalId`/`TaskId` values and `RuntimeEvent::ApprovalRequested` payloads for the existing approval UI/runtime path.

## Built-in step vocabulary mapping

- The mapping from the built-in `capability.*` step namespace to canonical `vac.*` capability ids lives in a single table, `WORKFLOW_STEP_VOCABULARY`, in `vac-rs/core/src/control_plane/workflow_runner.rs`.
- This is a canonical built-in vocabulary mapping for seeded maintenance workflows. It keeps the `capability.*` workflow step namespace and the `vac.*` capability manifest namespace synchronized through one closed table.
- `capability.tui.pty_gate` remains policy-gated through the sandbox/PTY gate capability contract and is validated by Plan 23 release-policy evidence.
- Custom, non-built-in `uses` strings still resolve via exact match against the capability id, so external workflow vocabularies are not blocked by the bridge.
- The bridge is intentionally a closed table: adding a new built-in step means adding one row to `WORKFLOW_STEP_VOCABULARY`, which simultaneously updates the safe-step allowlist, the handler resolver, and the approval-policy alias resolver.

## 2026-05-28 sandbox closeout

- `vac.approvals` is now `ready`.
- `approval_readiness.rs` verifies the ready manifest, policy manifest, release workflow approval step, workflow vocabulary mapping, and durable `FileApprovalStore` reachability.
- Evidence: `14-evidence/2026-05-28-sandbox-approval-readiness-closeout.md`.

---

## Audit findings (2026-05-20)

Hasil audit lintas-perspektif (Software Engineer, Software Architect, Code Reviewer, Planner) oleh 21 sub-agent paralel + 1 agent cross-cut. Read-only, tanpa mutasi file. Severity: **P0** (critical/bypass) · **P1** (functional gap) · **P2** (hygiene) · **P3** (polish/docs).

| # | Sev | Persp | Area | Finding | Recommended fix |
|---|-----|-------|------|---------|-----------------|
| 1 | P0 | Architect | Security | `WorkflowApprovalDecision::ApprovalRequired` shadow `PolicyDecisionReport::Block`; `resolve_approval(Approved)` resume tanpa cek policy → operator approval dapat bypass deny | Revalidate Block setelah approve di `resolve_approval` |
| 2 | P1 | Architect | Runtime | Step-level `WorkflowStep.policy` tidak dievaluasi (`evaluate_workflow_approval_policy` hanya manifest + capability policy) | Add `evaluate_workflow_approval_policy_for_step` |
| 3 | P1 | SE | Runtime | Workflow-level approval policy blanket ke semua step (termasuk step read-only awal) | Filter by step policy intent |
| 4 | P1 | SE | Persistence | Approval persistence/notification belum durable | Implement durable store |
| 5 | P1 | SE | Lifecycle | Resume contract belum live (TUI tidak refresh state) | Wire resume flow |
| 6 | P2 | SE | Lifecycle | Expiry/TTL sweeper mati | Implement sweeper |
| 7 | P2 | SE | Doctor | `vac doctor approval` exit lemah | Exit by run report |
| 8 | P3 | Reviewer | Test | Test minimal untuk precedence | Tambah test |
