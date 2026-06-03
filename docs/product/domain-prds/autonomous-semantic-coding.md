# PRD — Autonomous Semantic Coding

## Overview

Autonomous Semantic Coding is the default user-facing coding experience for VAC.

A user should be able to use VAC like a normal terminal coding agent:

```bash
vac
```

Then type a plain coding request:

```text
Fix the failing tests in the auth module.
```

The user should not need to know about the workflow-control-plane architecture, capability manifests, policy manifests, or workflow files to get value from VAC.

Internally, VAC still routes the task through a structured semantic loop:

```text
intent -> orientation -> semantic plan -> risk classification -> approval checkpoint -> bounded execution -> validation -> diagnosis -> safe retry -> evidence summary
```

The product goal is simple:

```text
User feels: “I just code with VAC.”
VAC internally does: workflow, policy, approval, semantic loop, validation, and evidence.
```

## Product promise

```text
VAC gives vibe coders autonomous speed without giving up control, safety, or explainability.
```


## Detailed architecture

The canonical architecture for this capability is documented in [`docs/architecture/semantic-coding-loop.md`](../../architecture/semantic-coding-loop.md).

This PRD defines the product requirements. The architecture document defines the step model, capability handlers, policy gates, TUI states, workflow manifest shape, and acceptance gates.

## Positioning

VAC must compete with lightweight terminal coding agents on the basic prompt-to-code loop while providing a stronger autonomous execution model.

The key difference:

```text
Generic CLI coding agent:
  prompt -> model -> tool calls -> file edits -> result

VAC:
  prompt -> semantic coding loop -> implicit workflow -> policy -> approval -> bounded execution -> validation -> evidence
```

The workflow architecture is an internal engine and an optional inspectability layer. It must not become required user knowledge for basic coding.

## Target users

### Primary users

- vibe coders who want to move quickly,
- engineers who want autonomous help but still need safety,
- users who prefer chat-style coding but want stronger visibility,
- teams that want autonomous execution without losing approval/audit controls.

### Secondary users

- power users who want to inspect workflows,
- maintainers who need evidence and validation summaries,
- domain users who want VIL/VWFD-aware coding assistance.

## User modes

VAC should support progressive disclosure through three layers.

### Layer 1 — Vibe coding mode

Default mode.

User experience:

```text
vac
> fix this bug
```

No workflow setup required. No manifest knowledge required. No architecture jargon required.

VAC internally creates or selects an implicit workflow such as:

```text
product.semantic-coding-loop
```

### Layer 2 — Inspectable autonomy

For users who want more visibility.

User surfaces:

```text
/workflow
/approvals
/evidence
/why
/activity
/status
```

The user can inspect the active implicit workflow, approval decisions, evidence, and validation.

### Layer 3 — Workflow engineering

For teams and power users.

User surfaces:

```text
.vac/workflows/
.vac/capabilities/
.vac/policies/
.vac/surfaces/
```

Users can define repeatable workflows, policy, and capability surfaces.

## Core semantic coding loop

VAC's default autonomous coding loop should be represented as a typed product workflow internally.

```text
SemanticCodingLoop
  1. Intent intake
  2. Repo orientation
  3. Semantic plan
  4. Risk and policy classification
  5. Approval checkpoint
  6. Bounded patch execution
  7. Validation
  8. Failure diagnosis
  9. Safe retry
  10. Evidence summary
```

### 1. Intent intake

VAC captures the user's plain-language request and extracts:

- task goal,
- target files/modules when known,
- constraints,
- requested validation,
- autonomy preference,
- safety posture.

### 2. Repo orientation

VAC inspects the repository with read-only tools first.

It should identify:

- relevant files,
- existing tests,
- build system,
- language/framework signals,
- domain signals such as VIL/VWFD inputs,
- likely validation commands.

### 3. Semantic plan

VAC creates an operator-readable plan before mutating files when the task is non-trivial.

Plan should include:

- intended changes,
- affected files,
- validation commands,
- risk class,
- approval needs,
- retry budget.

### 4. Risk and policy classification

VAC classifies every planned action:

```text
read_only
safe_edit
destructive
network
execute
config_change
install_dependency
```

Policy decides whether the action can proceed, requires approval, or is denied.

### 5. Approval checkpoint

Approval is required for risky or mutating operations according to policy.

Approval request must show:

- action type,
- affected files/resources,
- reason,
- risk level,
- preview/diff when available,
- validation plan.

### 6. Bounded patch execution

VAC applies changes within explicit bounds.

Recommended bounds:

```yaml
limits:
  max_files_changed: configurable
  max_retries: configurable
  max_shell_commands: configurable
  require_approval_for:
    - delete
    - dependency_install
    - network
    - config_change
    - broad_rewrite
```

### 7. Validation

VAC runs targeted validation when known.

Examples:

- package test,
- build check,
- formatter,
- linter,
- VIL/VWFD validation,
- release gate subset.

### 8. Failure diagnosis

If validation fails, VAC should classify the failure:

- compile error,
- test assertion failure,
- formatting failure,
- missing dependency,
- permission/policy denial,
- unrelated pre-existing failure,
- uncertain.

### 9. Safe retry

VAC may retry only inside the configured budget and safety policy.

Retry should be semantic, not random:

```text
observe failure -> map to likely changed file -> adjust patch -> rerun targeted validation
```

### 10. Evidence summary

Final result should include:

- changed files,
- why they changed,
- validations run,
- pass/fail status,
- approvals used,
- remaining risks,
- suggested next steps.

## Autonomy modes

VAC should expose autonomy in user-friendly terms, not architecture jargon.

### Suggested modes

```text
Suggest
Assist
Autopilot
```

### Suggest

Read-only or mostly read-only mode.

Behavior:

- inspects repo,
- creates plan,
- proposes patch,
- does not write without explicit approval,
- validation commands may be suggested rather than run.

### Assist

Default balanced mode.

Behavior:

- safe edits with approval,
- shell execution with approval or policy allowance,
- bounded retry,
- clear progress and evidence.

### Autopilot

Bounded autonomous mode.

Behavior:

- executes safe steps within policy,
- mutating/risky steps still approval-gated,
- retries within budget,
- stops on uncertainty, policy denial, or exhausted budget,
- produces evidence summary.

## TUI requirements

The TUI must make autonomous coding feel simple by default and inspectable when needed.

### Default visible state

The user should see a compact task status:

```text
Task: Fix failing auth tests
Step: validation
Risk: safe edit
Changed: 2 files
Approval: none pending
```

### Expanded workflow view

When the user opens `/workflow`, VAC should show the implicit workflow:

```text
product.semantic-coding-loop
  ✓ intent
  ✓ orient
  ✓ plan
  ✓ approval checkpoint
  ✓ patch
  → validate
  · summarize
```

### Approval view

When approval is required, the TUI should show:

```text
Approval required: edit 2 files
Risk: safe_edit
Files:
  - src/auth/error.rs
  - tests/auth_error.rs
Validation after approval:
  - cargo test -p auth
```

### Evidence view

After completion, the TUI should show:

```text
Changed:
- src/auth/error.rs
- tests/auth_error.rs

Validated:
- cargo test -p auth passed

Why:
- old mapping returned 500 instead of 401
- added regression test for expired token
```

## Required product capabilities

Autonomous Semantic Coding depends on these capabilities.

| Capability | Required for MVP | Description |
|---|---:|---|
| `vac.chat` | yes | Accepts plain-language coding requests. |
| `vac.semantic_loop` | yes | Coordinates intent, plan, execute, validate, summarize. |
| `vac.repo_orientation` | yes | Finds relevant files/tests/build commands. |
| `vac.policy` | yes | Classifies action risk. |
| `vac.approval` | yes | Gates risky/mutating actions. |
| `vac.tools` | yes | Reads/searches/edits/runs commands safely. |
| `vac.validation` | yes | Runs targeted checks. |
| `vac.evidence` | yes | Summarizes changes and why. |
| `vac.session` | P1 | Resume/checkpoint/autonomous task history. |
| `vac.workflow` | P1 | Makes implicit loop inspectable as workflow. |
| `vac.vil` | P2 | Adds VIL-native validation and repair. |
| `vac.vwfd` | P2 | Adds VWFD workflow understanding. |

## Workflow-control-plane relationship

The semantic coding loop should be implemented as an implicit workflow, but the user should not need to know that.

Internal model:

```yaml
schema_version: 1
kind: workflow
id: product.semantic-coding-loop
title: Semantic coding loop
status: planned
ui:
  surface: /activity
  progress_panel: true
  activity_log: true
steps:
  - id: intent
    uses: capability.intent.parse
  - id: orient
    uses: capability.repo.orient
  - id: plan
    uses: capability.plan.semantic
  - id: classify
    uses: capability.policy.classify
  - id: approve
    uses: capability.approval.request
    when: classify.approval_required == true
  - id: execute
    uses: capability.patch.apply_bounded
  - id: validate
    uses: capability.validation.run
  - id: diagnose
    uses: capability.failure.diagnose
    when: validate.failed == true
  - id: summarize
    uses: capability.evidence.summarize
```

The TUI can expose this workflow only when the user asks to inspect it.

## Safety requirements

Autonomous coding must remain bounded.

Required safeguards:

- no destructive action without approval,
- no dependency installation without approval,
- no broad rewrite without approval,
- no hidden network access,
- no infinite retry loop,
- no silent file deletion,
- no approval bypass in nested agent/subagent work,
- no raw secret exposure in logs or TUI.

## Vibe coding safety promise

VAC should let users code quickly while preserving control.

User expectation:

```text
I can let VAC work autonomously, but it will stop before risky actions and explain what it did.
```

## Differentiation from prompt-only coding agents

| Area | Prompt-only coding agent | VAC autonomous semantic coding |
|---|---|---|
| User input | plain prompt | plain prompt |
| Internal model | agent loop | semantic workflow loop |
| User burden | low | low |
| Progress | stream/log | lifecycle + activity + optional workflow view |
| Safety | approval mode | policy + approval + sandbox + bounded autonomy |
| Retry | model-driven | semantic diagnosis + retry budget |
| Evidence | summary | changed files + why + validation + approvals |
| Team readiness | limited | workflows, policies, sessions, release gates |
| Domain awareness | generic | VIL/VWFD-capable when enabled |

## Acceptance criteria

### MVP acceptance

- User can run `vac` and type a plain coding request.
- VAC does not require workflow knowledge for basic coding.
- VAC shows visible progress in TUI.
- VAC asks approval before file mutation according to policy.
- VAC can apply bounded edits.
- VAC can run targeted validation when known.
- VAC shows final evidence summary.
- User can inspect the implicit workflow through `/workflow` once workflow browser exists.

### Safety acceptance

- Destructive changes require explicit approval.
- Retry loop has a budget.
- Policy denial is visible and stops execution.
- Approval rejection stops the mutating step.
- Secret-like values are not printed in diagnostics.

### Product acceptance

- Basic coding UX remains fast and simple.
- Workflow architecture is invisible until user asks to inspect it.
- Capability dashboard shows semantic coding dependencies.
- Evidence summary makes the agent's work reviewable.
