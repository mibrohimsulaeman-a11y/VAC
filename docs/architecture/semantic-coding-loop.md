# Semantic Coding Loop architecture

## Purpose

The Semantic Coding Loop is VAC's default internal execution model for ordinary coding requests.

The user experience must stay simple:

```text
vac
> Fix the failing tests in the auth module.
```

The internal execution must stay structured:

```text
intent
  -> orient
  -> plan
  -> risk_classify
  -> approval_checkpoint
  -> execute_patch
  -> validate
  -> diagnose
  -> retry
  -> summarize_evidence
```

This loop is the bridge between vibe-coding UX and VAC's workflow-control-plane architecture.

## Product rule

```text
User feels: “I just code with VAC.”
VAC internally does: workflow, policy, approval, semantic execution, validation, retry, and evidence.
```

The workflow model is an internal safety and explainability layer. It must not force normal users to select or understand workflow manifests before coding.

## Workflow identity

The canonical internal workflow id is:

```text
product.semantic-coding-loop
```

It may be implicit in the default TUI and visible only when the user opens `/workflow`, `/activity`, `/evidence`, or debug/inspection surfaces.

## Lifecycle summary

| Step | Capability handler | Policy posture | TUI compact state |
|---|---|---|---|
| `intent` | `capability.intent.parse` | read-only | Understanding request |
| `orient` | `capability.repo.orient` | read-only | Scanning repo |
| `plan` | `capability.plan.semantic` | read-only | Plan ready |
| `risk_classify` | `capability.policy.classify` | read-only | Risk classified |
| `approval_checkpoint` | `capability.approval.request` | approval when required | Approval needed / approved / rejected |
| `execute_patch` | `capability.patch.apply_bounded` | policy-gated mutation | Editing files |
| `validate` | `capability.validation.run` | command/tool policy | Running validation |
| `diagnose` | `capability.failure.diagnose` | read-only | Diagnosing failure |
| `retry` | `capability.retry.safe` | bounded and policy-gated | Retrying safely |
| `summarize` | `capability.evidence.summarize` | read-only | Summarizing evidence |

## Step 1 — Intent intake

### Goal

Convert the user's plain-language request into a typed task object.

Example input:

```text
Fix failing tests in auth module.
```

Expected extraction:

```yaml
intent:
  raw: Fix failing tests in auth module.
  goal: fix_failing_tests
  target_hint: auth module
  mutation_expected: true
  validation_expected: true
  autonomy_mode: assist
  constraints:
    - do not modify unrelated files
```

### Capability handler

```text
capability.intent.parse
```

### Output

```yaml
task:
  id: task_123
  goal: fix_tests
  target_scope:
    modules:
      - auth
  expected_actions:
    - inspect
    - edit
    - validate
  constraints:
    max_scope: inferred
```

### TUI state

```text
Step: Understanding request
Status: done
```

### Failure state

```text
VAC could not understand the task. Please rephrase or provide target files.
```

## Step 2 — Repo orientation

### Goal

Read the repository before editing.

VAC should identify:

- relevant files,
- test files,
- build system,
- language/framework signals,
- recent errors if provided,
- project conventions,
- domain markers such as VIL/VWFD files.

### Capability handler

```text
capability.repo.orient
```

### Output

```yaml
orientation:
  repo_type: rust_workspace
  target_files:
    - crates/auth/src/lib.rs
    - crates/auth/tests/auth_tests.rs
  validation_candidates:
    - cargo test -p auth
  confidence: medium
  domain_markers:
    vil: false
    vwfd: false
```

### Policy

Repo orientation is read-only. It must not require approval unless it crosses a policy boundary such as protected paths or external network access.

### TUI state

```text
Step: Scanning repo
Files inspected: 12
Likely target: auth
```

### Failure state

```text
VAC could not find the target module. It can continue with a broader scan, but approval is required before edits.
```

## Step 3 — Semantic plan

### Goal

Create a concise operator-readable plan before mutation.

The plan must answer:

- what will change,
- which files are affected,
- why the change is needed,
- what validation will run,
- what risk class applies,
- whether approval is needed.

### Capability handler

```text
capability.plan.semantic
```

### Output

```yaml
plan:
  summary: Fix auth test failures by correcting error mapping and adding regression coverage.
  proposed_changes:
    - file: crates/auth/src/error.rs
      reason: Map expired token to unauthorized error.
      action: edit
    - file: crates/auth/tests/auth_error.rs
      reason: Add regression test for expired token.
      action: edit
  validation:
    - cargo test -p auth
  confidence: high
```

### TUI compact state

```text
Plan ready:
- edit 2 files
- run cargo test -p auth
Risk: safe edit
Approval: needed before write
```

### Expanded workflow state

```text
✓ intent
✓ orient
→ plan
  proposed files: 2
  validation: cargo test -p auth
```

## Step 4 — Risk classification

### Goal

Classify the plan with policy-aware risk metadata.

Risk classes:

```text
read_only
safe_edit
broad_edit
destructive
execute
network
dependency_install
config_change
unknown
```

### Capability handler

```text
capability.policy.classify
```

### Output

```yaml
risk:
  level: safe_edit
  mutates_files: true
  shell_required: true
  network_required: false
  destructive: false
  approval_required: true
  reason: File write planned.
```

### Autonomy mode mapping

```yaml
suggest:
  file_write: approval_required
  shell: approval_required
  retry: manual

assist:
  file_write: approval_required
  shell: approval_required
  safe_retry: allowed

autopilot:
  file_write: allowed_inside_scope
  shell: sandboxed_with_policy
  retry_budget: bounded
  destructive: approval_required
  network: approval_required
```

### TUI state

```text
Risk: safe edit
Approval needed: write 2 files
```

### Failure state

```text
Policy denied this action: broad rewrite outside declared scope.
```

## Step 5 — Approval checkpoint

### Goal

Use one approval model for risky or mutating actions.

### Capability handler

```text
capability.approval.request
```

### Approval request

```yaml
approval:
  id: approval_123
  risk: safe_edit
  action: write_files
  files:
    - crates/auth/src/error.rs
    - crates/auth/tests/auth_error.rs
  reason: VAC needs to edit files to fix failing tests.
  validation_after:
    - cargo test -p auth
```

### TUI approval card

```text
Approval required

Action:
  Edit 2 files

Risk:
  Safe edit

Files:
  crates/auth/src/error.rs
  crates/auth/tests/auth_error.rs

After edit:
  cargo test -p auth

[Approve] [Reject]
```

### Failure state

```text
Approval rejected. No files were changed.
```

## Step 6 — Bounded patch execution

### Goal

Apply changes inside explicit bounds.

### Capability handler

```text
capability.patch.apply_bounded
```

### Execution bounds

```yaml
bounds:
  max_files_changed: 6
  max_lines_changed: 400
  allowed_paths:
    - crates/auth/**
  denied_paths:
    - .env
    - secrets/**
    - package-lock.json
  destructive_actions: approval_required
```

### Output

```yaml
patch_result:
  changed_files:
    - crates/auth/src/error.rs
    - crates/auth/tests/auth_error.rs
  lines_added: 24
  lines_removed: 4
  within_bounds: true
```

### TUI state

```text
Editing files
Changed: 2
Bounds: OK
```

### Failure state

```text
Patch exceeded allowed scope. VAC stopped before applying broad changes.
```

## Step 7 — Validation

### Goal

Run relevant validation after changes.

### Capability handler

```text
capability.validation.run
```

### Validation command selection

VAC chooses validation commands from:

- task hint,
- repo orientation,
- capability metadata,
- workflow metadata,
- language conventions,
- previous failures.

### Output

```yaml
validation_result:
  status: failed
  command: cargo test -p auth
  error_class: test_assertion_failure
  relevant_output: expected 401, got 500
```

or:

```yaml
validation_result:
  status: passed
  command: cargo test -p auth
```

### TUI state

```text
Running validation:
cargo test -p auth
```

### Failure state

```text
Validation failed:
expected 401, got 500
```

## Step 8 — Failure diagnosis

### Goal

Diagnose validation failure semantically, not through random retry.

### Capability handler

```text
capability.failure.diagnose
```

### Failure classes

```text
compile_error
test_assertion_failure
format_error
lint_error
missing_dependency
policy_denied
pre_existing_failure
uncertain
```

### Output

```yaml
diagnosis:
  class: test_assertion_failure
  likely_cause: error mapping still returns internal server error
  related_files:
    - crates/auth/src/error.rs
  retry_recommended: true
  retry_scope:
    - crates/auth/src/error.rs
```

### TUI state

```text
Diagnosing failure
Likely cause: auth error mapping
Retry: safe
```

### Failure state

```text
VAC cannot safely diagnose this failure. Manual review recommended.
```

## Step 9 — Safe retry

### Goal

Retry within budget and policy limits only.

### Capability handler

```text
capability.retry.safe
```

### Retry policy

```yaml
retry:
  max_attempts: 3
  current_attempt: 1
  allowed_if:
    - failure_class_known
    - scope_not_expanded
    - no_destructive_action
    - validation_command_known
  stop_if:
    - policy_denied
    - approval_rejected
    - scope_expands
    - same_failure_repeats
    - budget_exhausted
```

### TUI state

```text
Retry 1/3
Scope: crates/auth/src/error.rs
Reason: known test failure
```

### Failure state

```text
Retry budget exhausted. VAC stopped with evidence summary.
```

## Step 10 — Evidence summary

### Goal

Make the outcome auditable.

### Capability handler

```text
capability.evidence.summarize
```

### Output

```yaml
evidence:
  task: Fix failing tests in auth module
  changed_files:
    - crates/auth/src/error.rs
    - crates/auth/tests/auth_error.rs
  approvals:
    - approval_123
  validation:
    - command: cargo test -p auth
      status: passed
  why:
    - Expired token error was mapped incorrectly.
    - Regression test added for expired token.
  remaining_risks: []
```

### TUI final summary

```text
Done

Changed:
- crates/auth/src/error.rs
- crates/auth/tests/auth_error.rs

Validated:
✓ cargo test -p auth

Why:
- fixed expired token error mapping
- added regression test
```

## Canonical workflow manifest shape

The implementation should eventually add this as a `.vac/workflows` manifest after schema support lands.

```yaml
schema_version: 1
kind: workflow
id: product.semantic-coding-loop
title: Semantic coding loop
status: planned

inputs:
  prompt:
    type: string
    required: true
  autonomy_mode:
    type: enum
    values:
      - suggest
      - assist
      - autopilot
    default: assist

ui:
  surface: /activity
  inspect_surface: /workflow
  progress_panel: true
  activity_log: true
  approval_surface: true
  evidence_surface: true

steps:
  - id: intent
    uses: capability.intent.parse
  - id: orient
    uses: capability.repo.orient
  - id: plan
    uses: capability.plan.semantic
  - id: risk_classify
    uses: capability.policy.classify
  - id: approval_checkpoint
    uses: capability.approval.request
    when: risk_classify.approval_required == true
  - id: execute_patch
    uses: capability.patch.apply_bounded
    when: approval_checkpoint.approved == true || risk_classify.approval_required == false
  - id: validate
    uses: capability.validation.run
  - id: diagnose
    uses: capability.failure.diagnose
    when: validate.status == "failed"
  - id: retry
    uses: capability.retry.safe
    when: diagnose.retry_recommended == true
  - id: summarize
    uses: capability.evidence.summarize
```

## Capability dependencies

Required capabilities:

```text
vac.chat
vac.intent
vac.repo_orientation
vac.semantic_plan
vac.policy
vac.approval
vac.patch
vac.validation
vac.failure_diagnosis
vac.retry
vac.evidence
vac.session
```

Optional domain capabilities:

```text
vac.vil_native
vac.vwfd_workbench
vac.vil_knowledge
vac.mcp_tools
vac.memory
```

## TUI progressive disclosure

### Default view

Normal users should see a compact status:

```text
Task: Fix auth tests
Step: validation
Risk: safe edit
Changed: 2 files
Approval: none pending
```

### `/workflow`

Power users can inspect the underlying workflow:

```text
product.semantic-coding-loop

✓ intent
✓ orient
✓ plan
✓ risk_classify
✓ approval_checkpoint
✓ execute_patch
→ validate
· diagnose
· retry
· summarize
```

### `/evidence`

Users can inspect:

- changed files,
- validations,
- approvals,
- why each change happened,
- remaining risk.

### `/capabilities`

Users can inspect readiness:

```text
vac.semantic_loop       ready
vac.approval            ready
vac.validation          partial
vac.evidence            planned
vac.vil_native          optional
```

## Acceptance criteria

### MVP

- User can type a normal coding request.
- VAC creates a semantic plan.
- VAC asks approval before mutation according to policy.
- VAC applies bounded edits.
- VAC runs targeted validation when known.
- VAC shows final evidence summary.

### Safety

- No destructive action without approval.
- No hidden network access.
- No infinite retry loop.
- No file mutation outside approved scope.
- No raw secret in output.
- Approval rejection stops mutation.

### UX

- No workflow knowledge required for basic coding.
- Progress is visible.
- Failure has reason and next step.
- Power user can inspect workflow.
- Final summary is reviewable.

### Architecture

- Semantic Coding Loop is represented as workflow metadata.
- Each step maps to a capability handler.
- Policy and approval are not bypassed.
- TUI receives lifecycle events.
- Evidence is structured.

## Implementation order

1. Finish decoupling old server/API paths from the initial local product path.
2. Restore `cargo check -p vac-surface-cli`.
3. Add control-plane registry loader.
4. Add initial capability manifests for the required loop handlers.
5. Add TUI compact progress state.
6. Add `/workflow` inspection once workflow browser exists.
7. Add evidence summary and retry budget.
