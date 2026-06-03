# Plan quality rubric — production-grade execution docs

## Purpose

This rubric defines the minimum quality bar for every workflow-control-plane plan.

A plan is production-grade only when an agent can execute it safely without guessing:

- what outcome is required,
- which files or surfaces are in scope,
- what must not be touched,
- how to validate success,
- when to stop,
- what evidence must be recorded.

This rubric does not replace historical plan notes. Historical audit/completion logs are allowed, but the active execution contract must be easy to find.

## Required top-level sections

Every active plan should include these sections, or explicitly say why a section is not applicable.

```text
## Goal
## Current status
## Target outcome
## Outputs
## Scope
## Non-goals
## Requires / Blocks
## Execution slices
## Validation matrix
## Product / UX validation
## Stop conditions
## Done criteria
## Historical notes
```

## Section standards

### Goal

One concise paragraph explaining why the plan exists.

### Current status

State whether the plan is:

- planned,
- in progress,
- implemented,
- deferred,
- superseded,
- blocked.

If implemented/deferred/superseded, include the current source of truth and do not make agents infer it from old audit notes.

### Target outcome

Describe the final production behavior, not just the implementation activity.

Good:

```text
The TUI shows workflow progress from registry-backed workflow runs and persists terminal states through reload.
```

Weak:

```text
Wire workflow progress.
```

### Outputs

List concrete artifacts expected after completion:

- files created/modified,
- Rust modules/types/APIs,
- `.vac` manifests,
- UI surfaces,
- tests,
- docs/index updates,
- closeout evidence.

### Scope

Define exact surfaces and files that may be touched.

### Non-goals

Define exclusions so agents do not broaden the slice.

### Requires / Blocks

List prerequisite plans/gates and downstream plans unblocked by this one.

### Execution slices

Break implementation into small, independently committable slices. Each slice should have:

- intended files,
- behavior change,
- validation,
- stop conditions if risky.

### Validation matrix

Split validation into automated commands and product behavior.

Automated validation examples:

```bash
cargo nextest run --manifest-path vac-rs/Cargo.toml -p vac-core --lib control_plane --no-tests=pass
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

### Product / UX validation

Required when a plan touches TUI, CLI, workflow execution, approval flows, or operator gates.

Examples:

- dashboard state visible,
- slash command appears,
- workflow progress updates,
- approval round trip succeeds,
- PTY gate passed by real TTY or recorded as `BLOCKED-OPERATOR`.

### Stop conditions

Stop rather than improvising when:

- unrelated dirty files would be touched,
- validation requires unavailable TTY/operator access,
- inverse Cargo tree still reaches forbidden dependencies,
- schema drift is discovered,
- a required owner is unclear,
- a build is already waiting on Cargo/rustc file locks.

### Done criteria

Must be objective and checkable. Avoid generic phrases like “works correctly.”

Good:

```text
Done only when registry parser rejects unknown workflow step kinds, TUI shows the failed workflow state, and the validation commands listed above pass.
```

### Historical notes

Audit findings, completion logs, and commit trails belong here unless they are the current execution contract.

## Agent execution policy

Every implementation slice should follow:

1. Inspect current files and git state.
2. Edit only intended files.
3. Validate the smallest relevant matrix.
4. Stage only intended files/hunks.
5. Commit.
6. Push to `origin main`.
7. Record unavailable operator gates as `BLOCKED-OPERATOR`, never as passed.

## Normalization priority

Normalize plans in this order:

1. high false-green risk plans,
2. plans that gate runtime/PTY/approval/release behavior,
3. new runtime-owner roadmap plans,
4. old control-plane plans with mixed audit/completion history,
5. historical Phase 00 plans.
