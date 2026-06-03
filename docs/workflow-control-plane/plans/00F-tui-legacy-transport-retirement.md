# Plan 00F - TUI legacy transport retirement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **completed for Phase 00 scope; runtime-owner replacement continues in Plans 24–33**.
> The goal-status seam has also moved: `ThreadGoalStatus` now comes from
> `ThreadMemoryMode`, `McpAuthStatus`, and `PermissionGrantScope`. The adapter
> `StepStatus` at the event boundary instead of re-exporting the app-server

Code evidence:
- `vac-rs/tui/src/legacy_app_server_compat.rs`
- `vac-rs/tui`
- `vac-rs/tui/src/resume_picker.rs`
- `vac-rs/tui/src/onboarding/onboarding_screen.rs`
- `vac-rs/tui/src/app/config_persistence.rs`
- `vac-rs/tui/src/app/side.rs`
- `vac-rs/tui/src/app/input.rs`
- `vac-rs/tui/src/app/thread_goal_actions.rs`
- `vac-rs/tui/src/app/thread_routing.rs`
- `vac-rs/tui/src/app/event_dispatch.rs`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **completed for Phase 00 scope; runtime-owner replacement continues in Plans 24–33**.

Target outcome: TUI consumers route through `LocalRuntimeSession` and intentional app-server seams are isolated so future owner replacement can proceed safely.

Outputs: `LocalRuntimeSession` boundary, app-server-backed adapter quarantine, protocol compatibility seam, validation evidence, and handoff to 00E/Plan 24.

Requires / Blocks: requires 00D TUI contract baseline; blocks Plan 24 runtime-owner replacement architecture and 00E delete/defer classification.

Stop conditions: stop if direct TUI consumers bypass `LocalRuntimeSession`, if app-server seams are hidden instead of isolated, or if Cargo edge removal is attempted without 00E evidence.

Done criteria: TUI consumers use the session boundary, intentional seams are documented, validation passes, and app-server Cargo retirement is handed off to Plan 24–33 / 00E.


## Goal

Retire the remaining app-server-backed legacy transport inside `vac-rs/tui`
as the product-local source of truth by introducing a runtime-session boundary
that can be backed by the current app-server implementation during migration.

This plan exists so the root product can keep one TUI, one visible operator
surface, and one migration path while the transport underneath is replaced in
small, verifiable slices.

Current baseline: the TUI session facade already routes through the shared
`vac-app-server-client` and `vac-app-server-protocol` crates, while the direct
`vac-app-server` Cargo edge is still intentionally retained until a
non-app-server runtime owner can replace the legacy transport.

One small DTO seam has already moved out of the app-server-owned compat list:
`RequestId` now comes from `vac-protocol`, and the app-server JSON-RPC layer
aliases the same owner type so request-id handling stays shape-compatible while
the seam shrinks.

The goal-status seam has also moved: `ThreadGoalStatus` now comes from
`vac-protocol`, and the app-server protocol layer aliases that owner type so
goal display and mutation stay wire-compatible.

The session-source seam has also moved: `SessionSource` now comes from
`vac-protocol`, and the TUI adapter converts the app-server thread source
wrapper back to the core owner only at the thread-loading and request-
construction boundaries.

The goal model has also moved: `ThreadGoal` now comes from
`vac_protocol::protocol`, and the TUI adapter converts the app-server wrapper
back to the core owner only at the goal notification/response boundary.

The thread-sort-key seam has moved as well: `ThreadSortKey` now comes from
`vac-thread-store`, while the adapter converts back to the app-server request
shape at the request-construction boundary so resume-picker ordering stays
stable.

The sort-direction seam now follows the same store-owned path: `SortDirection`
also comes from `vac-thread-store`, with the adapter mapping back to the
app-server request shape only when constructing list-thread requests.

Several other low-risk session DTOs now follow exact owners directly from the
core/product crates: `ReviewDelivery`, `SkillScope`, `SandboxMode`,
`ThreadMemoryMode`, `McpAuthStatus`, and `PermissionGrantScope`. The adapter
keeps the app-server wire shape only at the request-construction boundary.
The chat plan-update event now converts the app-server wrapper to core
`StepStatus` at the event boundary instead of re-exporting the app-server
enum into the facade.

`AskForApproval` now follows the same exact-owner pattern from
`vac_protocol::protocol`. The adapter keeps the app-server wire shape only
where the legacy wrapper is still required.

`GuardianAssessmentDecisionSource` now follows the same exact-owner pattern
from `vac_protocol::approvals`. The adapter keeps the app-server notification
wrapper only at the guardian-review event boundary.

`GuardianCommandSource`, `GuardianRiskLevel`, and `GuardianUserAuthorization`
now also follow the same exact-owner pattern from `vac_protocol::approvals`.
The adapter keeps the app-server wrapper only at the guardian-review event
boundary, while tests that need the wrapper use the app-server protocol type
directly.

`ReviewTarget` now follows the same exact-owner pattern from
`vac_protocol::protocol`. The adapter keeps the app-server `ReviewStartParams`
wrapper only at the request-construction boundary, so review selection logic
in the facade now stays on the shared protocol owner.

`RequestPermissionProfile` now follows the same exact-owner pattern from
`vac_protocol::request_permissions`. The adapter keeps the app-server
granted-permission wrapper only at the approval conversion boundary, so the
request-permission fixtures and facade stay on the shared protocol owner.

`AdditionalPermissionProfile` now follows the same exact-owner pattern from
`vac_protocol::models`. The adapter keeps the app-server granted-permission
wrapper only at the approval conversion boundary, so additional-permission
rendering and fixtures now stay on the shared model owner.

`NetworkApprovalContext` and `NetworkApprovalProtocol` now follow the same
exact-owner pattern from `vac_protocol::approvals`. The adapter keeps the
app-server network-approval wrapper only at the approval conversion boundary,
so network approval rendering and fixtures now stay on the shared approval
owner.

Another exact-owner batch now follows the same pattern: `HookEventName`,
`HookExecutionMode`, `HookHandlerType`, `HookScope`, `HookSource`,
`ModelVerification`, and `NetworkAccess` come from their core/product owners.
The TUI still converts those values back to the app-server shape only at the
visible request or fixture boundary where the legacy wrapper is still required,
while the plan-update notification path now converts to core `StepStatus` at
the event boundary.

`ApprovalsReviewer` now follows the same exact-owner pattern from
`vac_protocol::config_types`. The adapter converts it back to the app-server
request shape only at the request-construction boundary, so the approvals
reviewer seam is now exact-owner rather than app-server-owned.

The remaining session/protocol seam has been rechecked after those exact-owner
moves. `SessionSource` has already moved to the shared protocol owner.
`ThreadSourceKind` is still not a safe exact-owner swap because it diverges
from the thread-store source-kind enum. `ThreadStatus` has moved to the shared
protocol owner, so only the source-kind seam remains deferred here.
Those are conversion seams, not low-risk owner moves, so 00F stops at the
current boundary and hands the rest to a future conversion layer.

`ReviewTarget` remains compat/deferred: its exact mapping is still a
conversion seam rather than a no-op owner move.

The first implementation seam is now present in TUI: background request
helpers and onboarding auth use a `LocalRuntimeRequestHandle` boundary rather
than hard-coding the concrete request-handle type everywhere. The background
request surface now also accepts a `LocalRuntimeSession` provider so the call
sites are no longer pinned directly to `AppServerSession` for these request
paths.

The session boundary also carries lifecycle shutdown so cleanup behavior can
move with the session owner instead of staying hard-wired to the concrete TUI
session type.

Current progress extends beyond the first seam:

- `vac-rs/tui/src/resume_picker.rs` now consumes `LocalRuntimeSession` for session listing and cleanup.
- `vac-rs/tui/src/onboarding/onboarding_screen.rs` now reads the event stream through `LocalRuntimeSession`.
- `vac-rs/tui/src/app/config_persistence.rs`, `vac-rs/tui/src/app/side.rs`, `vac-rs/tui/src/app/input.rs`, and `vac-rs/tui/src/app/thread_goal_actions.rs` are now boundary-driven instead of pinning helper signatures to `AppServerSession`.
- `vac-rs/tui/src/app/thread_routing.rs` now routes submit/steer/start/interrupt and session refresh helpers through `LocalRuntimeSession`.
- `vac-rs/tui/src/app/event_dispatch.rs` and `vac-rs/tui/src/app.rs` are now generic over the session boundary for event dispatch and the main TUI run loop.
- `vac-rs/tui/src/local_runtime_session.rs` now provides the product-level facade names for the session boundary, request handle, started-thread bundle, and startup helpers; the remaining app-server-backed implementation stays hidden in `app_server_session.rs`.
- `vac-rs/tui/src/lib.rs` now constructs the session through an opaque `LocalRuntimeSession` helper, so bootstrap code no longer names `AppServerSession` directly.
- The remaining `AppServerSession` textual matches are confined to `vac-rs/tui/src/app_server_session.rs`, which still owns the concrete app-server-backed adapter during migration.

## Scope

- Prompt submit from the bottom input.
- Activity, progress, approval, validation, and completion surfaces.
- Session, thread, branch, and resume lifecycle reads.
- Auxiliary TUI surfaces that still depend on the legacy session boundary:
  model catalog, skills/plugins, external-agent attach, and realtime controls.
- Tests and PTY validation for the visible root path.
- Dependency cleanup when the boundary is complete.

## Non-goals

- A second TUI.
- Changing the visible layout or interaction model of the root TUI.
- Copying app-server-private `MessageProcessor` internals into TUI.
- Bulk swapping DTO owners without a proven replacement.
- Removing guarded `vac-app-server*` crates before the product path is stable.
- Remote/cloud session redesign.

## Capability Map

| Capability | Current product owner | 00F target | Notes |
| --- | --- | --- | --- |
| Prompt submit | Legacy app-server transport | `LocalRuntimeSession::start_task` | Keep bottom input UX identical. |
| Approval request / resolve | Legacy app-server transport | `LocalRuntimeSession::approve` / `reject` | Keep the same overlay and decision flow. |
| Event stream | Legacy app-server transport | `LocalRuntimeSession::next_event` | Preserve lag handling and completion ordering. |
| Thread lifecycle read | Legacy app-server transport | `LocalRuntimeSession::read_session_state` | Keep resume, branch, and list visible. |
| External-agent detection | Legacy app-server transport | Session boundary helper | Keep onboarding and attach unchanged. |
| Retained managers / stores | Legacy app-server startup | Session boundary bundle | Preserve thread manager, auth, store, and environment access. |
| Request handle | Legacy app-server request channel | Session boundary request handle | Permit background paths that outlive a mutable borrow. |

## Implementation

### 00F-1 - Audit remaining `vac_app_server` consumption

Audit remaining `vac_app_server` consumption in `vac-rs/tui/src/**` and
classify what is still visible product behavior versus transport-only
coupling.

### 00F-2 - Introduce a `LocalRuntimeSession` boundary

Introduce a `LocalRuntimeSession` boundary in `vac_core::local_runtime` or
the current TUI adapter layer with the minimum methods needed by the visible
surfaces.

Initial progress: TUI now has a `LocalRuntimeRequestHandle` boundary for the
background request and onboarding auth helpers. This is the smallest
request-path seam needed before a fuller session boundary can take over more
of `AppServerSession`.

The background request helpers now take `LocalRuntimeSession` instead of the
concrete `AppServerSession`, which makes the request-path seam usable from any
future session owner that can vend the same request-handle contract.

The bootstrap path in `vac-rs/tui/src/lib.rs` now uses the opaque session
helper rather than constructing `AppServerSession` inline.

### 00F-3 - Add an app-server-backed adapter

Add a TUI adapter that wraps the current app-server-backed session so
behavior stays stable while the call sites migrate.

### 00F-4 - Route prompt submit through the session boundary

Route prompt submit through the session boundary first, then migrate
approvals, validation, completion, thread lifecycle, and auxiliary surfaces
in small slices.

### 00F-5 - Migrate tests to product events

Migrate tests to product events and session-boundary fixtures instead of
app-server DTOs where the behavior is already equivalent.

### 00F-6 - Drop TUI app-server dependencies only when safe

Drop TUI app-server dependencies only when the root product path no longer
needs them for visible behavior or validation.

### 00F-7 - Hand off to 00E

Hand the resulting reachability state to 00E for the final inverse-tree and
delete/defer gate.

Current handoff state: the consumer code is on the local session facade, but
the Cargo dependency graph still keeps `vac-app-server-client` and
`vac-app-server` reachable through the adapter. The delete/defer decision
therefore remains with 00E.

2026-05-22 execution update: 00F-A through 00F-D3 have landed on `origin/main`
(`7ec5d7c`, `5e18fa4`, `1680645`, `faaeaf4`, `8505093`, `5e454af`).
`local_runtime_session.rs` now declares the runtime/session/request/started-thread
traits, `ReviewTarget` is owned by `vac_protocol::protocol`, and the divergent
`ThreadSourceKind` conversion seam lives in `session_protocol.rs`. Cargo edge
removal remains explicitly handed off to 00E.

The current closeout snapshot is recorded in
`docs/migration/PHASE00_CLOSEOUT_STATUS.md`. It reflects the committed
milestones through `2220a42` and the current validation/result split between
landed boundary work and deferred Cargo retirement. The 2026-05-23 00E
inverse-tree pass accepts the handoff as an explicit defer decision: the old
runtime graph remains reachable, so Cargo edge removal and physical retirement
must not be marked green in Phase 00.

2026-05-25 execution update: `LocalRuntimeStartedThread` no longer carries the
unused `TODO(local-runtime-owner)` future/test accessors. The TUI session
lifecycle still consumes started threads through `into_parts`, and direct
runtime-owner command-bus validation passed with
`CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo check --manifest-path vac-rs/Cargo.toml -p vac-local-runtime-owner --profile dev-small`. The approval persistence slice now uses a SQLite-backed `FileApprovalStore`, matching the scheduled plan while preserving the existing workflow runner API.

## Worktree Slice Map

- Landed code slices:
  - protocol DTO owner moves in `b7ffda0`,
  - TUI local-session facade in `8b0b0c8`,
  - core local-runtime projection tightening in `dbb3cf0`,
  - subagent session initialization fix in `5aba4f3`,
  - thread sort key owner move in `2e82716`,
  - sort direction owner move in `a25db31`,
  - early reasoning-delta crash fix in `748c035`,
  - 00F-A real `LocalRuntimeSession`/request/started-thread trait extraction in `7ec5d7c`,
  - 00F-B `ReviewTarget` owner unification in `5e18fa4`,
  - 00F-C `ThreadSourceKind` seam isolation in `1680645`,
  - 00F-D1 chatwidget facade migration in `faaeaf4`,
  - 00F-D2 app-helper facade migration in `8505093`,
  - 00F-D3 remaining TUI consumer facade migration in `5e454af`.
- Still deferred beyond Phase 00:
  - app-server Cargo-edge removal from TUI,
  - physical retirement of `vac-app-server`, `vac-app-server-client`, and
    `vac-app-server-transport` after a future non-app-server runtime owner
    satisfies the 00E replacement gate.
- Closed by 00E as deferred for Phase 00:
  - inverse-tree delete/defer gate for old runtime crates; 2026-05-23 evidence
    still shows reachability through `vac-tui`, so deletion is intentionally
    deferred rather than attempted.
- Remaining intentional seams after 00F-D:
  - `vac-rs/tui/src/app_server_session.rs` owns the concrete app-server-backed adapter,
  - `vac-rs/tui/src/session_protocol.rs` owns the app-server-protocol compatibility facade,
  - `vac-rs/tui/src/local_runtime_session.rs` keeps product-facing aliases for bootstrap/startup helpers,
  - `ThreadSourceKind::AppServer` and sub-agent source variants stay transport-side/deferred.
- Intentionally preserved dirty work:
  - app-server-sensitive config/message-processor/tests files,
  - core config and hook tests,
  - TUI snapshot `.snap.new` artifacts.

## Validation

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-core local_runtime --lib
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-surface-tui --lib local_runtime
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
```

## PTY Operator Gate

Run the product in a real terminal and verify the visible root workflow:

```text
launch ./target/debug/vac
type /
press Enter
verify slash-command list visible
type normal prompt
press Enter
verify task/activity visible
verify validation visible when a validation tool is used
verify approval overlay round-trips approve/reject
press Ctrl-C
verify clean exit and clean working tree
```

Current builds open the slash-command list on `/`. Do not assume `/help`
exists unless a later change reintroduces it.

If the shell cannot drive a real TTY, report the gate as
`BLOCKED-OPERATOR` and hand the steps to a human operator.

## UX Invariants

- One product command: `vac`.
- One product TUI.
- Bottom input stays runtime-first.
- Activity, progress, approval, validation, and completion remain visible.
- No duplicate TUI or second operator surface.
- No user-facing behavior change beyond transport migration.

## Stop Conditions

- If a slice requires copying `MessageProcessor` or other app-server-private
  startup internals into TUI, stop and classify the remainder as deferred.
- If a slice regresses the visible product path, narrow the boundary before
  continuing.
- Remove dependencies only after the root path is stable and validation passes.
