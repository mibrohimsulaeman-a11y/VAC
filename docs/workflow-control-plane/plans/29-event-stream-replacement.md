# Plan 29 — Event stream replacement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE_DEFAULT_PATH**

Source status lines:
> ## Current status
> Status: **complete for the default owner-native event stream path** — 29A–29E are represented in `vac-rs/local-runtime-owner/src/event_stream.rs` and `vac-rs/local-runtime-owner/src/session.rs`. The local runtime owner now has an app-server-protocol-free owner event stream, protocol mapping matrix, lossless/best-effort delivery classification, backpressure/lag tests, projection reuse through `LocalRuntimeBridge`, and Plan 31/33 closeout classifies any remaining compatibility as non-default.

Code evidence:
- `vac-rs/core/src/local_runtime/event.rs`
- `vac-rs/core/src/local_runtime/projection.rs`
- `vac-rs/local-runtime-owner/src/event_stream.rs`
- `vac-rs/local-runtime-owner/src/session.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/29-evidence/2026-05-28-sandbox-event-stream-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Replace app-server event ownership with a local runtime owner event stream while preserving TUI-visible streaming, progress, approvals, and completion semantics.

## Current status

Status: **complete for the default owner-native event stream path** — 29A–29E are represented in `vac-rs/local-runtime-owner/src/event_stream.rs` and `vac-rs/local-runtime-owner/src/session.rs`. The local runtime owner now has an app-server-protocol-free owner event stream, protocol mapping matrix, lossless/best-effort delivery classification, backpressure/lag tests, projection reuse through `LocalRuntimeBridge`, and Plan 31/33 closeout classifies any remaining compatibility as non-default.

This plan should run after Plan 25 identifies projection reuse and after Plan 28 defines pending request semantics for approval/user-input events.

## Target outcome

The local runtime owner emits the event stream for active TUI runtime behavior. Any old-shape compatibility is non-default quarantine governed by Plan 31/33 evidence, not the default Plan 29 stream.

## Outputs

- event mapping matrix,
- owner event stream type,
- lossless/best-effort classification,
- backpressure tests,
- compatibility adapter with explicit retirement path.

## Requires / Blocks

Requires:

- Plan 25 event/projection audit,
- Plan 28 pending request semantics.

Blocks:

- Plan 30 prompt submit cutover,
- Plan 31 protocol compatibility retirement.

## Current source behavior

Events currently reach TUI through a mix of:

- app-server notifications and requests,
- `TuiSessionEvent`,
- direct `VACThread` event listener in `app_server_session.rs`,
- `session_protocol` conversion helpers,
- `vac_core::local_runtime::LocalRuntimeBridge` for semantic projection.

## Target behavior

The lifecycle owner streams local runtime events and exposes a compatibility adapter for TUI until the TUI no longer needs app-server-shaped DTOs.

## Non-goals

- No prompt submit cutover in this plan unless event replacement is already proven.
- No app-server protocol DTO as final event type.
- No dropping terminal events silently.
- No removal of current path before backpressure tests are green.

## Required preservation

- assistant message deltas,
- reasoning deltas,
- plan deltas,
- tool start/finish,
- exec output deltas,
- patch progress,
- validation started/finished,
- approval requested,
- user-input requested,
- thread name/goal updates,
- realtime started/closed,
- turn completed/interrupted/failed,
- lag/backpressure markers,
- stale-turn filtering.

## Implementation slices

### 29A — Event matrix

Implemented in `classify_protocol_event` with `ProtocolEventMapping`, `ProtocolProjection`, and explicit unsupported/deferred entries. Required TUI-visible event classes are marked lossless where dropping would be false-green: assistant/reasoning/plan deltas, tool lifecycle, exec output, terminal interaction, approvals, user input, metadata, realtime lifecycle, and turn terminal events.

### 29B — Owner event stream type

Implemented owner stream primitives with lossless vs best-effort delivery classification.

`RuntimeEventStream`, `RuntimeEventEnvelope`, `RuntimeOwnerEventPayload`, `RuntimeEventDelivery`, `RuntimeEventKind`, `RuntimeEventStreamItem`, and `RuntimeEventSubscriber` are bounded and app-server-protocol-free. The stream drops only best-effort events under pressure, emits lag markers to subscribers, and fails loudly when lossless events cannot be delivered. `LocalRuntimeOwnerSession` now owns a `RuntimeEventStream`.

### 29C — Projection reuse

Implemented `RuntimeOwnerEventProjector`, which reuses `vac_core::local_runtime::LocalRuntimeBridge` for semantic runtime events and keeps lifecycle-specific payloads outside the semantic contract. Owner-native payloads cover terminal output, terminal interaction, patch progress, user input, thread metadata, realtime lifecycle, failure, and shutdown without widening `RuntimeEvent`.

### 29D — Backpressure behavior

Implemented and tested. Best-effort events can be dropped only with `Lagged { dropped, next_sequence }` markers. Lossless events either enqueue, displace older best-effort events, or return `LosslessBackpressure` instead of silently dropping terminal/transcript-critical data.

### 29E — Compatibility adapter

Implemented `RuntimeTuiCompatibilityEvent` plus `to_tui_compatibility_event` on envelopes/items. Native owner events are distinguishable from legacy-adapter-required payloads so TUI surfaces can stay stable while Plan 31 retires app-server-shaped protocol DTOs.

## Validation

Focused validation for the completed 29A–29E owner-stream slice:

```bash
cd vac-rs
df -h . /tmp
cargo test --manifest-path Cargo.toml -p vac-local-runtime-owner event_stream -- --nocapture
# Result on 2026-05-24: 9 passed; 0 failed
```

Broader validation before cutover remains:

```bash
cd vac-rs
df -h . /tmp
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

## Stop conditions

Stop if:

- terminal/transcript-critical events can be dropped silently,
- reasoning/tool/progress events cannot be mapped with tests,
- stale-turn filtering regresses,
- final event types require app-server protocol ownership.

## Done criteria

- Event stream is owned by local runtime owner: **done** via `LocalRuntimeOwnerSession::events` and `RuntimeOwnerEventProjector`.
- Critical events are lossless or explicitly failed: **done** via `RuntimeEventDelivery::Lossless` and `LosslessBackpressure` tests.
- TUI-visible behavior remains stable: **done for Plan 29 scope** via owner event stream coverage; Plan 31 has closed the default DTO retirement path and any remaining compatibility is non-default.
