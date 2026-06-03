# Phase 00 closeout status

## Snapshot

- **Current HEAD:** `2220a42` (`docs(migration): record 00F slice A–D landing, hand off Cargo retirement to 00E`)
- **Phase 00 committed milestones:**
  - `b7ffda0` - `refactor(protocol): align session DTO owners for 00F`
  - `8b0b0c8` - `refactor(tui): route runtime consumers through local session facade`
  - `dbb3cf0` - `refactor(core): tighten local runtime projection contract`
  - `5aba4f3` - `fix(core): stabilize subagent session initialization`
  - `2e82716` - `refactor(tui): move thread sort key to store owner`
  - `a25db31` - `refactor(tui): move sort direction to store owner`
  - `7f1372c` - `docs(migration): classify remaining 00F DTO seams`
  - `14b2c56` - `refactor(tui): codify session source conversion seam`
  - `acb1c91` - `refactor(tui): expand hook and verification dto boundary`
  - `40d4001` - `refactor(tui): move additional permission profile to core owner`
  - `bf55097` - `docs(migration): record additional permission owner batch`
  - `0b1e643` - `refactor(tui): move network approval context to core owner`
  - `2af6e75` - `refactor(tui): move thread goal to core owner`
  - `acaed57` - `refactor(tui): move thread status to core owner`
  - `3c7d249` - `refactor(tui): convert turn plan status at event boundary`
  - `28d8177` - `docs(migration): refresh phase 00 turn-plan status truth`
  - `b269a96` - `docs(migration): close phase 00 as deferred`
  - `5e5a462` - `test(control-plane): cover approval-aware workflow report`
  - `34198d0` - `test(cli): cover approval-aware workflow doctor`
  - `3b53f43` - `test(tui): tighten workflow browser approval summary`
  - `3450e7d` - `test(tui): cover approval-aware workflow browser progress`
  - `97d55dc` - `docs(migration): sync phase 00 closeout and approval plan truth`
  - `0545527` - `test(tui): include approval-aware activity trace in browser`
  - `ApprovalsReviewer` now comes from `vac_protocol::config_types`, with the
    TUI adapter converting back to the app-server request shape only at the
    request-construction boundary.
  - `AskForApproval` now comes from `vac_protocol::protocol`, with the TUI
    adapter converting back to the app-server request shape only where the
    legacy wrapper is still required.
  - `GuardianAssessmentDecisionSource` now comes from
    `vac_protocol::approvals`, with the TUI adapter converting the app-server
    notification wrapper back to the core owner only at the event boundary.
  - `GuardianCommandSource`, `GuardianRiskLevel`, and
    `GuardianUserAuthorization` now also come from
    `vac_protocol::approvals`, while the TUI adapter keeps the app-server
    wrapper only at the guardian-review event boundary and the tests use the
    wrapper type directly where required.
  - `ReviewTarget` now also comes from `vac_protocol::protocol`, while the
    TUI adapter keeps the app-server `ReviewStartParams` wrapper only at the
    request-construction boundary.
  - `workflow_runner.rs`, `doctor_cli.rs`, and `workflow_browser.rs` now all
    carry approval-aware regression coverage that seeds the minimal capability
    registry and asserts the `waiting_approval` progress path through the
    shared workflow report, including the browser activity log trace.
  - `RequestPermissionProfile` now also comes from
    `vac_protocol::request_permissions`, while the TUI adapter keeps the
    app-server granted-permission wrapper only at the approval conversion
    boundary.
  - `SessionSource` now also comes from `vac_protocol::protocol`, while the
    TUI adapter keeps the app-server thread source wrapper only at the thread
    loading and request-construction boundaries.
  - `ThreadGoal` now also comes from `vac_protocol::protocol`, while the TUI
    adapter keeps the app-server wrapper only at the goal notification/
    response boundary.
  - `748c035` - `fix(core): tolerate early reasoning deltas in turn stream`
  - `7ec5d7c` - `refactor(tui): extract LocalRuntimeSession trait from app_server_session`
  - `5e18fa4` - `refactor(tui): unify ReviewTarget on vac_protocol owner`
  - `1680645` - `refactor(tui): isolate ThreadSourceKind seam (00F-C)`
  - `faaeaf4` - `refactor(tui): migrate chatwidget to LocalRuntimeSession trait (00F-D1)`
  - `8505093` - `refactor(tui): migrate app helpers to LocalRuntimeSession trait (00F-D2)`
  - `5e454af` - `refactor(tui): migrate remaining tui consumers to LocalRuntimeSession trait (00F-D3)`
  - `2220a42` - `docs(migration): record 00F slice A–D landing, hand off Cargo retirement to 00E`
- **Working tree rule:** preserve unrelated dirty files. Plan 01 is unblocked by the Phase 00 gate.

## Status By Track

### 00B

Complete enough for Phase 00 closeout:

- local runtime event/contract surface is split into explicit owner files,
- `RuntimeTask` projection/transition helpers are in place,
- serde and compile gates were validated on the contract path.

### 00C

Complete enough for Phase 00 closeout:

- `vac exec` default prompt path is runtime-first,
- legacy resume/review logging was made explicit,
- bootstrap/runtime setup is split from the legacy app-server path,
- output buffers and stale-turn tracing were bounded/documented.

### 00D

Complete enough for Phase 00 closeout:

- the TUI product surface now routes through `LocalRuntimeSession`,
- the session facade is product-level rather than consumer-facing adapter detail,
- `in_process_app_server_client.rs` is removed from the TUI consumer path,
- low-risk local-runtime bridge seams are exhausted.

### 00F

Complete enough for Phase 00 closeout:

- TUI consumer code is now boundary-driven through `local_runtime_session.rs`,
- `RequestId` and `ThreadGoalStatus` are exact-owner protocol moves,
- `SessionSource` is now also an exact-owner protocol move,
- `ThreadGoal` is now also an exact-owner protocol move,
- `ThreadStatus` is now also an exact-owner protocol move,
- `ThreadSortKey` now comes from `vac-thread-store`, with the TUI converting
  back to the app-server request shape only at the request-construction
  boundary,
- `SortDirection` also comes from `vac-thread-store`, with the adapter
  converting back to the app-server request shape only at the request list
  boundary,
- `ReviewDelivery`, `SkillScope`, `SandboxMode`, `ThreadMemoryMode`,
  `McpAuthStatus`, and `PermissionGrantScope` now come from their exact
  core/product owners, while the adapter converts back to the app-server wire
  shape only at the request boundary,
- `HookEventName`, `HookExecutionMode`, `HookHandlerType`, `HookScope`,
  `HookSource`, `ModelVerification`, and `NetworkAccess` now come from their
  exact core/product owners, while the adapter and the remaining app-server
  fixture boundary convert only where the legacy wrapper still has to exist,
- `ApprovalsReviewer` now comes from `vac_protocol::config_types`, with the
  adapter converting back to the app-server request shape only at the
  request-construction boundary,
- `AskForApproval` now comes from `vac_protocol::protocol`, with the adapter
  converting back to the app-server request shape only where the legacy
  wrapper is still required,
- `GuardianAssessmentDecisionSource` now comes from
  `vac_protocol::approvals`, with the adapter converting the app-server
  notification wrapper back to the core owner only at the event boundary,
- `GuardianCommandSource`, `GuardianRiskLevel`, and `GuardianUserAuthorization`
  now also come from `vac_protocol::approvals`, with the adapter keeping the
  app-server wrapper only at the guardian-review event boundary and tests
  using the wrapper type directly where required,
  - `ReviewTarget` now also comes from `vac_protocol::protocol`, with the
    adapter keeping the app-server `ReviewStartParams` wrapper only at the
    request-construction boundary,
  - `RequestPermissionProfile` now also comes from `vac_protocol::request_permissions`,
    with the adapter keeping the app-server granted-permission wrapper only at
    the approval conversion boundary,
  - `AdditionalPermissionProfile` now also comes from `vac_protocol::models`,
    with the adapter keeping the app-server granted-permission wrapper only at
    the approval conversion boundary,
  - `NetworkApprovalContext` and `NetworkApprovalProtocol` now also come from
    `vac_protocol::approvals`, with the adapter keeping the app-server
    network-approval wrapper only at the approval conversion boundary,
  - the turn-plan update notification path now converts the app-server wrapper
    to core `StepStatus` at the event boundary,
- the remaining `session_protocol` candidates were rechecked and are now
  classified as conversion-only seams or deferred app-server protocol debt:
  `ThreadSourceKind` is not an exact-owner swap,
- 2026-05-22 Plan 00F execution landed the deferred TUI split in ordered slices:
  - `7ec5d7c` extracted `LocalRuntimeSession`, `LocalRuntimeRequestHandle`,
    and `LocalRuntimeStartedThread` into `vac-rs/tui/src/local_runtime_session.rs`,
  - `5e18fa4` unified local-runtime `ReviewTarget` on
    `vac_protocol::protocol::ReviewTarget` while preserving the legacy serde
    wire shape via `review_target_serde`,
  - `1680645` isolated the divergent `ThreadSourceKind` conversion seam in
    `vac-rs/tui/src/session_protocol.rs`,
  - `faaeaf4`, `8505093`, and `5e454af` moved chatwidget, app-helper,
    resume/list, and approval-conversion consumers through the local runtime
    facade instead of direct app-server protocol imports,
- `LocalRuntimeSession` / `LocalRuntimeRequestHandle` /
  `LocalRuntimeStartedThread` are product-facing names declared in
  `vac-rs/tui/src/local_runtime_session.rs`,
- the app-server-backed adapter still exists as an implementation detail in
  `vac-rs/tui/src/app_server_session.rs`,
- remaining non-adapter app-server textual references are intentional seams:
  `vac-rs/tui/src/session_protocol.rs` is the app-server-protocol compatibility
  facade, `vac-rs/tui/src/local_runtime_session.rs` keeps the bootstrap alias,
  and `vac-rs/tui/src/legacy_core.rs` keeps a historical comment only,
- the Cargo dependency retirement is deferred to 00E because the old runtime graph is still reachable.

### 00E

Closed as deferred with post-00F evidence:

- `cargo +1.93.0 metadata --no-deps --format-version 1` completed on 2026-05-23 against `vac-rs/` with 96 packages / 96 workspace members,
- `vac-app-server-client` is still reachable from `vac-cli` through `vac-tui`,
- `vac-app-server` is still reachable from `vac-cli` through both `vac-tui` and `vac-app-server-client`,
- `vac-app-server-transport` is still reachable through `vac-app-server`,
- the active TUI app-server seams are the concrete adapter (`app_server_session.rs`) and compatibility facade (`session_protocol.rs`); direct consumers already route through `LocalRuntimeSession`,
- physical deletion remains blocked by the runtime-owner replacement gate and is explicitly deferred by policy,
- the delete gate is therefore definitively closed for Phase 00 as a defer decision, not as a hidden pass.

## Validation Matrix

### Passed

- `df -h . /tmp`
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 build --bin vac`
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli`
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests`
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-core tool_handlers_cascade_close_and_resume_and_keep_explicitly_closed_subtrees_closed -- --nocapture`
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-app-server-protocol`
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-app-server-protocol --lib`
- PTY operator gate:
  - `/` opens the slash-command list,
  - a normal prompt starts streaming without the earlier `ReasoningRawContentDelta without active item` panic,
  - `Ctrl-C` exits cleanly.

### Known residual noise

- The earlier `tools::handlers::multi_agents::tests::tool_handlers_cascade_close_and_resume_and_keep_explicitly_closed_subtrees_closed` stack overflow is fixed, but a full `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-core --lib` still reports unrelated existing failures in `agent::control`, `config`, `guardian`, and `unavailable_tool` tests. Those failures are outside the current 00E/00F boundary work.

## Worktree Slice Map

These dirty areas are intentionally preserved outside the closeout docs slice:

- `vac-rs/app-server/**`
  - config manager / service tests
  - message processor
  - thread branch / resume / start suite files
- `vac-rs/core/**`
  - config loader / config tests / config module
  - connectors tests
  - hook runtime
- `vac-rs/tui/src/chatwidget/**/*.snap.new`
  - snapshot artifacts still dirty and not part of the closeout commit

## Gate Result

- **00E:** closed as deferred with 2026-05-23 inverse-tree evidence
- **00F:** A-D landed; 00F-E handoff recorded; Cargo retirement defer decision accepted by 00E
- **Plan 01:** unblocked by the Phase 00 gate

## Phase 00 Result

Phase 00 is closed for the local-product gate: the remaining old-runtime
reachability is documented as an explicit 00E defer decision, not a false-green
deletion. Future work can proceed without re-opening Phase 00 unless a new ADR
or runtime-owner plan changes the deletion policy.
