# Plan 25 — Local runtime semantic contract hardening


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **IMPLEMENTED**

Source status lines:
> ## Current status
> Status: **implemented — semantic contract hardened (2026-05-24)**.
> | `session.rs` | `RuntimeSession`, `RuntimeSessionStatus`, `RuntimeEntrypoint`, `AutonomyMode`, session FSM helpers. | reusable semantic state | Good status vocabulary. It is not a startup handle and does not retain managers/resources. |
> | `task.rs` | `RuntimeTask`, task kind/status, task FSM helpers. | reusable semantic state | Good per-task status vocabulary. It is not an execution scheduler or cancellation token. |

Code evidence:
- `vac-rs/core/src/local_runtime/validation.rs`
- `vac-rs/core/src/local_runtime/task.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/25-evidence/2026-05-28-sandbox-semantic-contract-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Harden `vac_core::local_runtime` as the product-level semantic contract that Plan 24's lifecycle owner can safely build on.

Plan 25 does not create the lifecycle owner and does not rewire TUI behavior. It proves what the existing semantic contract already owns, what is missing, and which gaps must be solved outside `vac-core`.

## Current status

Status: **implemented — semantic contract hardened (2026-05-24)**.

This is the first executable child plan after Plan 24. It should run before any owner skeleton or TUI cutover work.

Execution note (2026-05-24): this slice intentionally stayed inside `vac_core::local_runtime` plus this plan document. It did not create the lifecycle owner, did not touch TUI runtime plumbing, and did not remove any app-server Cargo edge.

## Target outcome

`vac_core::local_runtime` has a clear, tested semantic-contract boundary. Agents can tell which types are reusable for the TUI owner and which lifecycle concerns must remain outside `vac-core`.

## Outputs

- semantic surface inventory for `vac-rs/core/src/local_runtime/*`,
- gap list for lifecycle owner work,
- focused tests/docs for ambiguous command/event/projection semantics,
- no behavior change outside semantic contract tests.

## 25A semantic surface inventory — 2026-05-24

| File | Contract surface | Classification | Notes for owner work |
| --- | --- | --- | --- |
| `mod.rs` | Re-export boundary for commands, events, ids, sessions, tasks, approvals, validation, projection, and errors. | reusable for TUI owner | This is the import surface the future owner should prefer over submodule-private paths. |
| `command.rs` | `RuntimeCommand`, `StartTask`, `StartReview`, `WorkflowInput`, review target serde. | semantic-only, partially reusable | Covers task/review/workflow intent, cancellation, approval decisions, and resume by ID. It does **not** cover owner startup/bootstrap, retained resources, active TUI controls, typed request/response dispatch, or shutdown. |
| `event.rs` | `RuntimeEvent` plus session/task/tool/approval/validation/review lifecycle DTOs. | reusable for TUI-visible semantic stream | Covers assistant deltas, tools, approvals, validation, completion/failure/cancel, and review mode. It has no sequence envelope, no queue/backpressure marker, and no dedicated terminal event variant. Reasoning is projected through `AssistantDelta` in this slice to preserve visibility without widening the enum. |
| `projection.rs` | `LocalRuntimeBridge`, `BridgeOutput`, `vac_protocol::Event` → `RuntimeEvent` mapping, preview/evidence helpers. | reusable semantic projection, not lifecycle owner | Owns protocol-to-contract projection and stale-turn filtering. This slice adds reasoning-delta mapping plus tests for reasoning, validation/tool projection, stale-turn filtering, completion semantics, and buffer/evidence caps. It still does not own live event queues, lag policy, request handles, or pending request continuation. |
| `approval.rs` | `ApprovalRequest`, resources, preview, risk/action/decision enums. | reusable DTO, registry missing | Models request/decision payloads but not a long-lived pending request registry. Resolve/reject/cancel/shutdown semantics belong in Plan 28 owner work. |
| `ids.rs` | UUID-backed `SessionId`, `TaskId`, `ApprovalId`, `WorkflowId`, `ToolCallId`. | reusable for registries | Stable serde/display/parse UUID newtypes are sufficient for a TUI pending registry. Registry ownership policy remains outside `vac-core`. |
| `session.rs` | `RuntimeSession`, `RuntimeSessionStatus`, `RuntimeEntrypoint`, `AutonomyMode`, session FSM helpers. | reusable semantic state | Good status vocabulary. It is not a startup handle and does not retain managers/resources. |
| `task.rs` | `RuntimeTask`, task kind/status, task FSM helpers. | reusable semantic state | Good per-task status vocabulary. It is not an execution scheduler or cancellation token. |
| `validation.rs` | `ValidationStarted`, `ValidationFinished`, `ValidationStatus`. | reusable semantic stream | Validation detection is projection-level heuristic today; future owner may need a typed validation source. |
| `error.rs` | `RuntimeError`, code/owner classification, `TaskFailure`. | reusable semantic error DTO | Good product error shape; lifecycle owner still needs typed request errors and shutdown/cancellation errors. |
| `transition.rs` | `InvalidTransition`. | semantic-only | Guards local FSM invariants only. |
| `macros.rs` | private display macro. | internal only | No owner dependency. |
| `tests.rs`, `tests_serde.rs` | FSM, DTO, serde, and snapshot coverage. | contract guard | Existing schema guards remain the wire-format tripwire for future enum/field changes. |

## Required audit answers — 2026-05-24

| Question | Answer |
| --- | --- |
| Does `RuntimeCommand` cover TUI session lifecycle? | **No.** It covers semantic task/review/workflow commands and approval decisions. Missing lifecycle concerns: startup/bootstrap, retained resources, local request handle, pending request registry, typed read-only command bus, active controls, TUI bootstrap data, and bounded shutdown. |
| Does `RuntimeEvent` cover TUI-visible stream needs? | **Partially.** Assistant, plan, reasoning-as-assistant-delta, tool start/finish, approval-required, validation, completion, failure, cancel, and review mode can be represented. Missing or deferred: sequence envelope, queue lag/backpressure markers, terminal output delta/interactions, dedicated reasoning event kind, and owner-level shutdown/pending-cancel stream events. |
| Can approvals model resolve/reject long-lived pending requests? | **DTO only.** `ApprovalRequest`, `ApprovalDecision`, `Approve`, `Reject`, and `ApprovalResolved` are enough vocabulary, but there is no pending request registry, duplicate/unknown ID handling, cancel-on-shutdown, or continuation/fail-back owner. Plan 28 must own this. |
| Does projection preserve reasoning/tools/validation/terminal events? | **Tools and validation are covered; reasoning is now kept visible through `AssistantDelta`; terminal output remains a blocker/deferred mapping.** `ExecCommandBegin/End`, MCP/web/image/patch begin/end map to tool events/evidence. Validation maps from common command heuristics. `ExecCommandOutputDelta` and `TerminalInteraction` are not represented by the current semantic enum. |
| Are ids stable enough for TUI pending request registry? | **Yes for identity shape.** UUID-backed serde/display/parse newtypes are stable enough for registry keys. Registry ownership policy remains outside `vac-core`. |

## Event/projection coverage matrix — 2026-05-24

| Source event class | Runtime projection | Coverage status |
| --- | --- | --- |
| `TurnStarted` | `TaskStarted` | Covered by bridge tests. |
| `AgentMessageContentDelta` | `AssistantDelta` | Covered by bridge tests. |
| `PlanDelta` | `AssistantDelta` | Covered by bridge tests. |
| `ReasoningContentDelta` / `ReasoningRawContentDelta` | `AssistantDelta` | Covered by new bridge tests; this is a visibility-preserving semantic compromise, not a dedicated reasoning model. |
| `ExecCommandBegin/End` | `ToolCallStarted/Finished`; validation start/finish when command matches validation heuristic | Covered by new bridge tests. |
| MCP / web / image / patch tool events | `ToolCallStarted/Finished` plus evidence | Existing projection; evidence cap path hardened for web/image completion. |
| Approval-required protocol events | `ApprovalRequested` then `TaskFailed(ApprovalRequired)` in exec-mode bridge | DTO covered by serde tests; long-lived continuation remains outside this bridge. |
| `TurnComplete` | pending validation cancellation, `TaskCompleted`, summary/evidence | Covered by buffer/evidence cap test. |
| `TurnAborted` / stream errors | `TaskCancelled` or `TaskFailed`; evidence/error flags | Existing projection; no owner shutdown semantics implied. |
| Stale turn IDs | drop event with debug trace | Covered by new bridge test. |
| `ExecCommandOutputDelta` / `TerminalInteraction` | none | Blocker for Plan 29 event stream replacement unless a terminal-specific semantic event/envelope is added or a separate TUI adapter channel is retained. |

## Lifecycle owner gap list

- Startup/bootstrap: no `vac-core::local_runtime` type starts retained managers or returns TUI bootstrap payloads.
- Retained resources: no owner for thread manager, auth manager, thread store, environment manager, or runtime managers.
- Request handle / command bus: `RuntimeCommand` is semantic vocabulary, not a typed dispatch/response channel.
- Pending request registry: approvals have DTOs but no long-lived registry, duplicate/unknown handling, timeout/cancel policy, or shutdown fail-back.
- Event stream transport: `BridgeOutput { Vec<RuntimeEvent> }` is synchronous projection output, not a lossless stream with ordering, lag markers, backpressure, or subscriber lifecycle.
- Terminal stream fidelity: current semantic enum has no terminal-output/interactions variant.
- TUI bootstrap and active controls: slash/bottom-input bootstrap, turn steer/interrupt, realtime, shell command, compact, rollback, config/account controls, skills/plugins, and external-agent config remain outside this contract.
- Shutdown: no shutdown handle, pending cancellation, listener abort, or 5-second bounded shutdown semantics.

## Requires / Blocks

Requires:

- Plan 24 architecture baseline.

Blocks:

- Plan 26 owner skeleton placement,
- Plan 29 event stream reuse decisions.

## Starting point

Current contract lives under:

```text
vac-rs/core/src/local_runtime/
```

Known exports include:

- `RuntimeCommand`, `StartTask`, `StartReview`, `WorkflowInput`
- `RuntimeEvent`, task/session lifecycle events
- approval and validation DTOs
- `LocalRuntimeBridge` / `BridgeOutput`
- ids, sessions, tasks, transitions, runtime errors

This is a semantic contract, not a TUI lifecycle owner.

## Scope

- Audit all `vac-rs/core/src/local_runtime/*` files.
- Map each type to one of:
  - reusable for TUI owner,
  - exec-only / semantic-only,
  - missing lifecycle concern,
  - should remain outside `vac-core`.
- Add focused docs/tests for ambiguity.
- Preserve current public semantics unless tests prove a bug.

## Non-goals

- No TUI cutover.
- No new runtime owner crate.
- No app-server dependency removal.
- No `MessageProcessor` movement.
- No UI lifecycle plumbing dumped into `vac-core`.

## Required audit questions

| Question | Required output |
| --- | --- |
| Does `RuntimeCommand` cover TUI session lifecycle? | explicit yes/no gap list |
| Does `RuntimeEvent` cover TUI-visible stream needs? | event mapping matrix |
| Can approvals model resolve/reject long-lived pending requests? | registry gap list |
| Does projection preserve reasoning/tools/validation/terminal events? | tests or blocker list |
| Are ids stable enough for TUI pending request registry? | compatibility note |

## Implementation slices

### 25A — Semantic surface inventory

- Read every file under `vac-rs/core/src/local_runtime/*`.
- Produce a short inventory in this plan or a companion migration note.
- Do not edit code unless a test-only clarification is needed.

### 25B — Event/projection coverage tests

- Add or extend tests for `LocalRuntimeBridge` where coverage is missing.
- Include assistant deltas, reasoning/tool events, validation, approval-required events, turn completion/failure, stale-turn filtering, and buffer/evidence caps.

### 25C — Command/session gap report

- Record gaps that must be handled by the lifecycle owner, especially startup, retained resources, request handle, pending request registry, TUI bootstrap, and shutdown.

## Validation

```bash
cd vac-rs
df -h . /tmp
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-core
```

## Stop conditions

Stop if:

- the audit requires moving TUI lifecycle ownership into `vac-core`,
- event semantics differ from TUI needs without a conversion strategy,
- tests require app-server dependencies in `vac-core`,
- unrelated control-plane or TUI files would need to be touched.

## Done criteria

- `vac_core::local_runtime` reusable surface is documented.
- Missing lifecycle concerns are explicitly listed.
- New/updated tests pass.
- No app-server Cargo edge is changed.
