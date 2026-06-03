# VAC-Init Batch 6-12 Implementation

Status: implemented as contract-level, dependency-free Rust modules with targeted validation.

This batch extends the VAC-Init control plane from registry/lifecycle/ownership foundation into the bounded agent loop and audit evidence layer. It intentionally avoids full workspace build/link and does not add dependencies.

## Batch 6 — Risk Scanner and Policy Inference

Implemented in `vac-rs/core/src/control_plane/vac_init_risk_policy.rs`.

Contracts:

- `RiskFinding`
- `RiskAction`
- `RiskDetectionMethod`
- `ConfidenceLabel`
- `PolicyInferenceReport`
- `detect_risk_findings()`
- `infer_policy_from_findings()`

Behavior:

- Detects process spawn, network, file mutation/deletion, credential access, and migration/secret filename patterns.
- Uses confidence labels aligned with the spec: uncertain, low, moderate, high, certain.
- Ambiguous findings require operator review.
- Credential/delete findings are never inferred as allow.

## Batch 7 — Fail-Closed Policy Evaluator

Implemented in `vac-rs/core/src/control_plane/vac_init_policy_evaluator.rs`.

Contracts:

- `PolicyLayer`
- `PolicyRule`
- `PolicyDecision`
- `PolicyAction`
- `ActionContext`
- `evaluate_policies()`

Behavior:

- No policy loaded means deny.
- Hardcoded safety denials are applied before workspace/capability/workflow/plan/session layers.
- Merge rule is most-restrictive-wins: `deny > approval_required > allow`.
- Credential/secret-like access is denied by hardcoded safety until a future secret broker exists.

## Batch 8 — Structured Command Gate

Implemented in `vac-rs/core/src/control_plane/vac_init_command_gate.rs`.

Contracts:

- `StructuredCommand`
- `CommandRunnerRegistry`
- `CommandGateReport`
- `CommandGateDecision`
- `evaluate_structured_command()`

Behavior:

- Rejects arbitrary runner paths.
- Rejects `bash -c` and other inline shell command execution.
- Rejects shell metacharacters in arguments.
- Allows script runner forms such as `bash scripts/check.sh` when no inline shell is used.
- `execute_process`, high, and critical risks require approval under policy mode.

## Batch 9 — Semantic Plan Validator

Implemented in `vac-rs/core/src/control_plane/vac_init_semantic_plan.rs`.

Contracts:

- `SemanticPlan`
- `PlanAllowedFile`
- `PlanBounds`
- `PlanValidationContext`
- `validate_semantic_plan()`
- `can_transition_plan_status()`

Behavior:

- Capability must exist and be executable.
- Planned/deprecated capabilities cannot execute.
- Allowed files must be owned and bounded by line range or semantic anchor.
- New files must stay within plan budget.
- Validation commands must remain structured.
- Executing plans that revise back to draft require re-approval.

## Batch 10 — Approval Binding and Replay Protection

Implemented in `vac-rs/core/src/control_plane/vac_init_approval_binding.rs`.

Contracts:

- `ApprovalRequestRecord`
- `ApprovalBinding`
- `ApprovalReplayStore`
- `validate_approval_binding()`
- `consume_approval()`

Behavior:

- Approval is invalid if plan hash, diff hash, or policy snapshot hash changes.
- Approval expires after `expires_at`.
- Nonce replay is rejected.
- Pending/denied/timeout records cannot be consumed as approved authorization.

## Batch 11 — Patch Guard

Implemented in `vac-rs/core/src/control_plane/vac_init_patch_guard.rs`.

Contracts:

- `ApprovedPatchScope`
- `PatchBudget`
- `PatchAttempt`
- `PatchGuardContext`
- `validate_patch_attempt()`

Behavior:

- Patch target must be in `plan.allowed_files`.
- Operation must match plan.
- Line range must be within approved range.
- Semantic anchor must resolve when range is absent.
- New files must be declared.
- Patch budget and forbidden glob rules are enforced.

## Batch 12 — Evidence Canonical Hash Chain

Implemented in `vac-rs/core/src/control_plane/vac_init_evidence_chain.rs`.

Contracts:

- `EvidenceRecord`
- `EvidenceChainLink`
- `canonical_evidence_payload()`
- `compute_evidence_self_hash()`
- `verify_evidence_record()`
- `verify_evidence_chain()`
- `sha256_hex()`

Behavior:

- Canonical payload excludes `self_hash`.
- Mapping keys are sorted.
- SHA-256 is implemented dependency-free and tested against known vectors.
- Chain verification detects broken previous id/hash and tampered records.

## Deferred

- Runtime serde YAML loaders for all contracts.
- Full `vac init` interactive CLI.
- Cargo crate-level integration tests.
- Full workspace build/link.

These are intentionally deferred to avoid dependency/vendor churn and sandbox reset risk.
