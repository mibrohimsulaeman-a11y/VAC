# Plan 28 — Server-request registry replacement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE_DEFAULT_PATH**

Source status lines:
> ## Current status
> Status: **complete for the default owner-native server-request registry path**.

Code evidence:
- `vac-rs/core/src/local_runtime/command.rs`
- `vac-rs/exec/src/runtime_adapter.rs`
- `vac-rs/tui/src/app_server_session.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/28-evidence/2026-05-28-sandbox-registry-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Replace app-server-shaped pending server-request handling with a first-class local runtime owner registry.

This is a hard gate before prompt submit cutover. A chat-only cutover without approvals/input is false-green.

## Current status

Status: **complete for the default owner-native server-request registry path**.

Plan 28 is implemented and validated for the local runtime owner registry path. Plan 30 uses this registry as the approval/input gate for prompt-submit cutover, and Plan 31/33 classify any remaining compatibility as non-default delete/defer material.

## Target outcome

Every operator-facing pending request is owned by a local registry with deterministic resolve, reject, cancel, shutdown, and backpressure behavior.

## Outputs

- `ServerRequestRegistry` or equivalent owner type,
- mapping tests for exec approval, patch approval, user input, permissions, and MCP elicitation,
- shutdown/backpressure tests,
- TUI-visible error/evidence behavior for unresolved requests.

## Requires / Blocks

Requires:

- Plan 26 owner skeleton,
- Plan 27 retained-resource startup if registry needs live thread handles.

Blocks:

- Plan 30 prompt submit cutover,
- Plan 33 delete/defer proof.

## Current source behavior

`vac-rs/tui/src/app_server_session.rs` already has a direct local registry:

- `DirectPendingServerRequest`
- `DirectPendingServerRequestKind`
- resolve/reject helpers for direct thread events

Current kinds:

- exec approval
- patch/file-change approval
- tool user-input answer
- permissions response
- MCP elicitation resolution

## Target behavior

The local runtime owner owns pending requests independently from app-server DTOs and guarantees that every pending request is resolved, rejected, cancelled, or failed back on shutdown.

## Non-goals

- No prompt submit cutover until registry tests are green.
- No silent best-effort approval handling.
- No app-server DTO as final registry type.
- No UI-only implementation; runtime must receive the decision.

## Required semantics

- request IDs are unique and stable,
- duplicate IDs are rejected safely,
- unknown IDs are safe no-ops or structured errors,
- resolve submits the correct runtime operation,
- reject maps to safe abort/cancel decisions,
- queue saturation cannot leave a request hanging,
- shutdown cancels all pending requests,
- errors are visible enough for TUI/operator diagnosis.

## Implementation slices

### 28A — Registry type and invariants

Create `ServerRequestRegistry` with insert/resolve/reject/cancel/shutdown operations and unit tests.

### 28B — Approval request mapping

Map exec and patch approvals from `vac_protocol::Event` to owner requests and back to runtime decisions.

### 28C — User-input and permissions mapping

Map tool user-input and permission requests with exact round-trip tests.

### 28D — MCP elicitation mapping

Map MCP elicitation with explicit cancel/reject semantics.

### 28E — Backpressure/shutdown tests

Prove saturation and shutdown cannot leave pending requests unresolved.

## Validation

Completed validation:

```bash
cd vac-rs
rustfmt +1.93.0 --edition 2024 --check local-runtime-owner/src/server_requests.rs local-runtime-owner/src/lib.rs tui/src/app_server_session.rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-local-runtime-owner server_requests --lib --profile dev-small
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-local-runtime-owner -p vac-surface-tui --profile dev-small
```

Result:

- `server_requests` tests: 12 passed, 0 failed.
- `vac-local-runtime-owner` + `vac-tui` focused check: passed under `dev-small`.
- Existing `vac-tui` warnings remain unrelated dead-code warnings.

## Stop conditions

Stop if:

- any pending request kind lacks a safe reject/cancel mapping,
- event queue saturation can leave an approval/input hanging,
- registry types require final app-server DTO ownership,
- unknown/duplicate request IDs are not deterministic.

## Done criteria

- Local owner registry covers every current pending request kind.
- Resolve/reject/cancel/shutdown are tested.
- No pending approval/input can hang on queue saturation.
