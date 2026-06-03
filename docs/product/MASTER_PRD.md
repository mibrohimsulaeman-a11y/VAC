# VAC Master PRD

## Product summary

VAC is a workflow-native agentic CLI for controlled, repeatable, policy-bound software work.

VAC combines:

- a terminal operator cockpit,
- a declarative workflow control plane,
- typed capability manifests,
- safe Rust execution,
- visible policy and approval gates,
- audit-friendly sessions,
- domain-native software automation.

## Product positioning

VAC is not a prompt-only coding helper. VAC is a workflow-native operating layer for agentic software work.

The primary product promise:

```text
Every capability is declared, policy-bound, visible in the TUI, and executable through typed workflows.
```

## Target users

### Primary

- engineers who want local agentic automation with clear visibility,
- technical leads who need repeatable review, validation, and refactor workflows,
- platform engineers who want policy-bound automation across codebases,
- teams that need auditable AI-assisted software operations.

### Secondary

- solo developers who want a safer, more transparent terminal agent,
- maintainers who need repeatable release and migration workflows,
- internal tool teams that want to encode company engineering practice as workflows.

## Core product goals

1. Make agent execution visible.
2. Make capabilities explicit.
3. Make policy enforceable.
4. Make approvals structured.
5. Make workflows repeatable.
6. Make sessions resumable and auditable.
7. Make donor-backed features enter only through declared surfaces.
8. Prevent backend-only feature drift.

## Non-goals

VAC should not become:

- a loose collection of hidden subcommands,
- an untyped YAML shell runner,
- a second frontend stack,
- a backend library dump with minimal UX,
- a cloud/server product before the local workflow product is coherent,
- a plugin marketplace before the capability registry exists.

## Product pillars

## Autonomous semantic coding

VAC must support a simple coding-agent experience for users who do not know or care about workflow architecture. A user can type a normal coding request, while VAC internally routes the work through a semantic loop with policy, approval, bounded execution, validation, and evidence. See `docs/product/domain-prds/autonomous-semantic-coding.md`.

### 1. Workflow-native operation

User work is represented as typed workflows with inputs, steps, lifecycle states, policy gates, validation, and operator-facing progress.

### 2. Capability-first product model

Every feature is declared as a capability with owner, status, policy, surfaces, validation, and visible states.

### 3. TUI operator cockpit

The TUI is not decorative. It is the primary surface for observing capability status, workflow progress, approvals, diagnostics, and session state.

### 4. Policy and approval by design

Every risky action is policy-evaluated. Mutating actions route through one approval model.

### 5. Audit and recoverability

Sessions, approvals, validation, tool calls, and workflow lifecycle events should be persisted or summarized for inspection and recovery.

### 6. Domain-native automation

VAC should understand VIL/VWFD projects, workflow definitions, semantic validation, and project-specific governance without relying on ad-hoc shell scripts.

## Primary user journey

```text
1. User runs `vac`.
2. TUI opens with status, activity, capabilities, and workflow access.
3. User enters a task or selects a workflow.
4. VAC resolves the task to capability/workflow metadata.
5. VAC validates inputs and prepares a plan or dry-run.
6. Policy determines what requires approval.
7. User approves, rejects, or modifies the plan.
8. VAC executes typed steps.
9. TUI shows progress, outputs, validation, and failures.
10. VAC ends with a summary, changed files, evidence, and next actions.
```

## Required initial product surfaces

```text
vac                      launch TUI
vac exec                 headless/non-interactive task execution
vac review               review current changes when available
vac apply                apply approved patch when available
vac resume               resume session when available
vac completion           shell completion when available
/capabilities            TUI capability dashboard
/workflow                TUI workflow browser and later runner
```

All other legacy service, proxy, server, cloud, debug, or marketplace surfaces must be removed, hidden, or reintroduced only as declared capabilities.

## System requirement summary

| Area | Requirement |
|---|---|
| CLI | Small product-focused command surface. |
| TUI | Operator cockpit with capability/workflow/approval visibility. |
| Control plane | `.vac` declares capabilities, workflows, policies, surfaces, registry metadata. |
| Runtime | Rust executes typed steps and enforces policy. |
| Approval | One structured approval model. |
| Tools | Tool calls classified by trust/risk and policy-gated. |
| Sessions | Work is resumable, inspectable, exportable when supported. |
| Security | Secret redaction, sandbox policy, network policy, audit events. |
| VIL/VWFD | Native validation, explain, parity, and generation workflows. |
| Observability | Activity, trace, evidence, lifecycle, and validation status visible. |
| Release | Build, identity, TUI, policy, and workflow checks are release gates. |

## Definition of done for any feature

A feature is product-ready only when it has:

- capability manifest,
- root implementation owner,
- TUI or CLI surface,
- visible empty/loading/success/failure states,
- policy classification,
- approval integration when mutating,
- validation command,
- release gate or test coverage,
- cleanup status if sourced from donor material.

Backend-only work does not count.
