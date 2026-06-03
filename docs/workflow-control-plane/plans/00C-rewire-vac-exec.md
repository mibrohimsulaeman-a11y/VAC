# Plan 00C — Rewire `vac exec` to Local Runtime Contract


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> Status: **implemented / exec path baseline**.
> - exit status mapping.

Code evidence:
- `vac-rs/exec/src/runtime_adapter.rs`
- `vac-rs/core/src/local_runtime/command.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented / exec path baseline**.

Target outcome: `vac exec` emits and consumes Local Runtime Contract semantics while preserving CLI behavior and validation visibility.

Outputs: exec rewiring, contract projection, tests, and validation evidence for local runtime events.

Requires / Blocks: requires 00B contract; blocks control-plane workflow execution confidence for non-TUI local runtime semantics.

Stop conditions: stop if exec rewiring changes root TUI behavior, hides validation state, or introduces app-server ownership into the contract.

Done criteria: exec validation passes and event/progress semantics are represented through Local Runtime Contract types.


## Goal

Move `vac exec` away from app-server client request/response semantics for local prompt submission.

## Scope

- headless prompt submission,
- event stream consumption,
- approval-unavailable behavior,
- validation/evidence output,
- exit status mapping.

## Implementation

1. Replace local prompt submission through app-server client with `RuntimeCommand::StartTask`.
2. Consume `RuntimeEvent` stream for assistant text, tool calls, approval, validation, completion, and failure.
3. Preserve useful existing UX where possible.
4. Make non-interactive approval behavior explicit.
5. Remove direct `vac-app-server-client` dependency from `vac-exec` when possible.

## Validation

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
```

## Done

- `vac exec` no longer uses app-server client for local prompt submission.
- Headless output is driven by RuntimeEvent.
- Completion/failure maps to correct process exit behavior.
