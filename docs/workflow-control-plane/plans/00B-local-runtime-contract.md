# Plan 00B — Local Runtime Contract implementation


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **implemented / semantic contract baseline**.

Code evidence:
- `vac-rs/core/src/local_runtime/mod.rs`
- `vac-rs/core/src/local_runtime/session.rs`
- `docs/architecture/local-runtime-contract.md`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented / semantic contract baseline**.

Target outcome: Local Runtime Contract types exist as product-level session/task/event semantics independent from legacy runtime transport ownership.

Outputs: contract modules/types/tests, validation evidence, and clear distinction between semantic contract and lifecycle owner.

Requires / Blocks: requires 00A build unblock; blocks 00C, 00D, and later runtime-owner plans.

Stop conditions: stop if contract types import app-server transport/protocol ownership as final semantics or require TUI lifecycle plumbing.

Done criteria: contract compiles, tests pass, and downstream exec/TUI rewiring can target it without duplicating product commands.


## Goal

Introduce the minimal Local Runtime Contract described by ADR-0007 and `docs/architecture/local-runtime-contract.md`.

## Scope

Initial contract:

```text
RuntimeSession
RuntimeTask
RuntimeCommand
RuntimeEvent
ApprovalRequest
```

Initial events:

```text
SessionStarted
TaskStarted
AssistantDelta
ToolCallStarted
ToolCallFinished
ApprovalRequested
ApprovalResolved
ValidationStarted
ValidationFinished
TaskCompleted
TaskFailed
TaskCancelled
```

## Implementation

1. Add a focused local runtime module near current core runtime code.
2. Use product terminology, not server-thread terminology.
3. Add tests for command/event serialization or construction if serialization is not yet needed.
4. Wire at least one smoke path so the contract is not backend-only.
5. Avoid creating a large standalone crate until both TUI and exec use the contract.

## Validation

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
```

## Done

- Local runtime DTO/event model exists.
- Model is referenced by product-local code.
- No app-server/protocol terminology leaks into the new contract.
- TUI/exec rewiring can start.
