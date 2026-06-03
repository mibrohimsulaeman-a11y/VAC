# 00E reachability audit (snapshot)

**Date:** 2026-05-11
**Author:** Notion AI (working through MCP Cloudflare Tunnel; long-running memory page maintained in operator's Notion workspace)
**Repo HEAD:** `0545527` on `main` (`0545527 test(tui): include approval-aware activity trace in browser`)
**Audit scope:** dependency graph for `vac-cli` workspace member at HEAD, post 00C + 00D-B30.

This is a *read-only* audit. It does not delete or move any code. It records what each candidate crate is reachable from so that 00E can proceed deterministically. This snapshot supersedes the 2026-05-06 entry (HEAD `35aaa5d`) preserved at the bottom of this file for historical context.

## 2026-05-23 post-00F closeout addendum

After Plan 00F slices `7ec5d7c` through `2220a42`, the delete/defer gate was
re-run as a documentation and reachability closeout slice. Current evidence:

```text
cargo +1.93.0 metadata --no-deps --format-version 1
packages 96
workspace_members 96

cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
vac-app-server-client
└── vac-tui
    └── vac-cli

cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
vac-app-server
├── vac-app-server-client
│   └── vac-tui
│       └── vac-cli
└── vac-tui (*)

cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-transport --edges normal,build
vac-app-server-transport
└── vac-app-server
    ├── vac-app-server-client
    │   └── vac-tui
    │       └── vac-cli
    └── vac-tui (*)
```

Classification: 00E is definitively closed for Phase 00 as deferred. The
remaining graph is reachable by design through the app-server-backed TUI
adapter and protocol compatibility facade. Physical deletion or Cargo edge
removal before a non-app-server runtime owner replaces startup, request
dispatch, event streaming, server-request resolution, retained managers/stores,
external-agent detection, and bounded close would be a false-green.

## Current worktree delta (2026-05-15)

The historical B30 snapshot below is now only a point-in-time reference. In the current worktree:

- `vac-rs/tui/src/app_server_session.rs` uses the shared `vac-app-server-client` facade instead of the local copy of the in-process client.
- `vac-rs/tui/src/session_protocol.rs` now re-exports DTOs from `vac-app-server-protocol`.
- `vac-rs/tui/src/in_process_app_server_client.rs` was deleted, so `vac-rs/tui/src/**` no longer contains direct `vac_app_server::*` imports.
- `vac-rs/tui/Cargo.toml` still lists `vac-app-server`, `vac-app-server-client`, and `vac-app-server-protocol`, so `vac-app-server` remains reachable both directly from `vac-tui` and transitively through `vac-app-server-client`.
- `vac-rs/tui/src/app/background_requests.rs` now takes a `LocalRuntimeSession` provider for background request surfaces, `vac-rs/tui/src/lib.rs` uses that session boundary for cleanup shutdown, and `vac-rs/tui/src/onboarding/auth.rs` uses a `LocalRuntimeRequestHandle` boundary for request-handle-driven helpers instead of hard-coding the concrete handle type in every helper signature.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `RequestId` from `vac-protocol`, and `vac-rs/app-server-protocol/src/jsonrpc_lite.rs` aliases its JSON-RPC request id to the same owner type, so the TUI request-id seam now uses the shared protocol owner rather than an app-server-specific copy.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `ThreadGoalStatus` from `vac-protocol`, and `vac-rs/app-server-protocol/src/protocol/v2.rs` aliases its thread-goal status enum to the same owner type, so goal status handling is now exact-owner instead of a mirrored app-server copy.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `SessionSource` from `vac-protocol`, and the TUI adapter converts the app-server thread-source wrapper back to the core owner only at the thread-loading and request-construction boundaries, so the session-source seam now uses the shared protocol owner.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `ThreadGoal` from `vac-protocol`, and the TUI adapter converts the app-server thread-goal wrapper back to the core owner only at the goal notification/response boundary, so the goal model now uses the shared protocol owner too.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `ThreadSortKey` from `vac-thread-store`, while the TUI adapter converts back to the app-server request shape at the request-construction boundary so resume-picker ordering keeps the same visible behavior without keeping the app-server copy in the facade.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `SortDirection` from `vac-thread-store`, while the TUI adapter converts back to the app-server request shape only at the request-construction boundary so thread-list ordering semantics stay stable without keeping the app-server copy in the facade.
- `vac-rs/tui/src/session_protocol.rs` also re-exports `ReviewDelivery`, `SkillScope`, `SandboxMode`, `ThreadMemoryMode`, `McpAuthStatus`, and `PermissionGrantScope` from their core/product owners. The adapter converts them back to the app-server wire shape only at the request boundary, while the turn-plan update path now converts the app-server wrapper to core `StepStatus` at the event boundary.
- `vac-rs/tui/src/session_protocol.rs` now also re-exports `HookEventName`, `HookExecutionMode`, `HookHandlerType`, `HookScope`, `HookSource`, `ModelVerification`, and `NetworkAccess` from `vac-protocol`. The TUI converts the hook and verification values back to the app-server wrapper shape only where the legacy request or fixture boundary still needs it; one sandbox-policy test still constructs the app-server `NetworkAccess` enum explicitly because that reverse conversion is not provided.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `ApprovalsReviewer` from `vac-protocol::config_types`. The TUI adapter converts that value back to the app-server request shape only at the request-construction boundary, so the approvals reviewer seam is now exact-owner rather than app-server-owned.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `AskForApproval` from `vac-protocol::protocol`. The TUI adapter still converts back to the app-server request shape only at the request boundary where the legacy wrapper is required, while the core-facing config paths now stay on the shared protocol owner.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `GuardianAssessmentDecisionSource` from `vac-protocol::approvals`. The TUI adapter converts the app-server notification wrapper back to the core owner only at the guardian-review event boundary.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `GuardianCommandSource`, `GuardianRiskLevel`, and `GuardianUserAuthorization` from `vac-protocol::approvals`. The TUI adapter keeps the app-server wrapper only at the guardian-review event boundary, while tests that still need the wrapper use the app-server protocol type directly.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `ReviewTarget` from `vac-protocol::protocol`. The TUI adapter keeps the app-server `ReviewStartParams` wrapper only at the request-construction boundary, so review selection logic now uses the shared protocol owner in the facade.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `RequestPermissionProfile` from `vac-protocol::request_permissions`. The TUI adapter keeps the app-server granted-permission wrapper only at the approval conversion boundary, so request-permission fixtures now use the shared protocol owner in the facade.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `AdditionalPermissionProfile` from `vac-protocol::models`. The TUI adapter keeps the app-server granted-permission wrapper only at the approval conversion boundary, so additional-permission rendering and fixtures now use the shared model owner in the facade.
- `vac-rs/tui/src/session_protocol.rs` now re-exports `NetworkApprovalContext` and `NetworkApprovalProtocol` from `vac-protocol::approvals`. The TUI adapter keeps the app-server network-approval wrapper only at the approval conversion boundary, so network approval rendering and fixtures now use the shared approval owner in the facade.
- `vac-rs/cli/src/doctor_cli.rs` and `vac-rs/tui/src/workflow_browser.rs` now both carry approval-aware workflow regression coverage, seeding the minimal capability registry and asserting the `waiting_approval` progress path through the shared workflow report, including the browser activity log trace.
- `vac-rs/tui/src/resume_picker.rs`, `vac-rs/tui/src/onboarding/onboarding_screen.rs`, `vac-rs/tui/src/app/config_persistence.rs`, `vac-rs/tui/src/app/input.rs`, `vac-rs/tui/src/app/side.rs`, `vac-rs/tui/src/app/thread_goal_actions.rs`, `vac-rs/tui/src/app/thread_routing.rs`, `vac-rs/tui/src/app/event_dispatch.rs`, and `vac-rs/tui/src/app.rs` now route their visible product flows through `LocalRuntimeSession` instead of pinning the helper signatures to `AppServerSession`.
- `vac-rs/tui/src/local_runtime_session.rs` now provides the product-level facade names for the session boundary, request handle, started-thread bundle, and startup helpers. Consumer code no longer imports the adapter file directly for the session boundary, but the adapter implementation still remains app-server-backed.
- `vac-rs/tui/src/lib.rs` now constructs the session through an opaque `LocalRuntimeSession` helper, so the bootstrap path no longer names `AppServerSession` directly.
- A later recon pass exhausted the remaining obvious exact-owner DTO candidates in the session/protocol seam. `SessionSource`, `ThreadGoal`, and `ThreadStatus` now come from `vac-protocol`; `ThreadSourceKind` diverges from the thread-store source-kind enum. The remaining seam therefore needs a conversion boundary, not another rename-only pass.
- The remaining `AppServerSession` textual matches are confined to `vac-rs/tui/src/app_server_session.rs`, which still owns the concrete app-server-backed adapter during migration.

## Current phase 00 closeout state (2026-05-15)

- `b7ffda0` moved `RequestId` and `ThreadGoalStatus` to shared exact-owner protocol paths.
- `6a44518` moved `SessionSource` to the shared exact-owner protocol path.
- `2af6e75` moved `ThreadGoal` to the shared exact-owner protocol path.
- `acaed57` moved `ThreadStatus` to the shared exact-owner protocol path.
- `3c7d249` converted the turn-plan status path to the event boundary.
- `8b0b0c8` landed the product-level TUI local-session facade and adapter boundary.
- `dbb3cf0` tightened the core local runtime projection contract.
- `5aba4f3` stabilized subagent session initialization after the stack-overflow debug pass.
- `2e82716` moved `ThreadSortKey` to the store owner while keeping the app-server request shape at the adapter boundary.
- `a25db31` moved `SortDirection` to the store owner while keeping the app-server request shape at the adapter boundary.
- `7f1372c` classified the remaining 00F DTO seams as conversion-only or deferred debt instead of low-risk owner moves.
- `14b2c56` codified the session-source conversion seam in the TUI adapter while leaving the app-server wrapper at the notification/request boundary.
- `ReviewTarget` now also comes from `vac-protocol::protocol`, with the adapter keeping the app-server request wrapper only at the request-construction boundary.
- `RequestPermissionProfile` now also comes from `vac_protocol::request_permissions`, with the adapter keeping the app-server granted-permission wrapper only at the approval conversion boundary.
- `AdditionalPermissionProfile` now also comes from `vac_protocol::models`, with the adapter keeping the app-server granted-permission wrapper only at the approval conversion boundary.
- `NetworkApprovalContext` and `NetworkApprovalProtocol` now also come from `vac_protocol::approvals`, with the adapter keeping the app-server network-approval wrapper only at the approval conversion boundary.
- `acb1c91` expanded the hook and verification DTO boundary with exact-owner moves for the hook metadata and model-verification family.
- `ApprovalsReviewer` now also comes from `vac-protocol::config_types`, with the adapter converting back to the app-server request shape only at the request-construction boundary.
- `AskForApproval` now also comes from `vac-protocol::protocol`, with the adapter converting back to the app-server request shape only where the legacy wrapper is still required.
- `GuardianAssessmentDecisionSource` now also comes from `vac-protocol::approvals`, with the adapter converting the app-server notification wrapper back to the core owner only at the guardian-review event boundary.
- `748c035` removed the PTY crash by tolerating early reasoning deltas without an active item instead of panicking in debug builds.
- The PTY operator gate passes in a real terminal on the current binary.
- `vac-app-server`, `vac-app-server-client`, and `vac-app-server-protocol` remain reachable from `vac-tui`, so 00E is closed as deferred and the delete/defer decision is documented rather than open.

Current inverse-tree results from this worktree:

```bash
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
vac-app-server v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server)
├── vac-app-server-client v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-client)
│   └── vac-tui v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/tui)
│       └── vac-cli v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/cli)
└── vac-tui v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/tui) (*)

cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
vac-app-server-client v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/app-server-client)
└── vac-tui v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/tui)
    └── vac-cli v0.0.0 (/home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/cli)
```

This is still a deferred state for 00E: the direct `vac-app-server` edge is present by design until a non-app-server runtime owner replaces the legacy transport, and physical deletion remains blocked by the operator guardrail.

## What landed since the 2026-05-06 snapshot

- **Plan 00C — `vac-exec` rewire** landed (cloud purge FCDG slice, commit `868cc5a`). `vac-rs/exec/Cargo.toml` no longer lists `vac-app-server-client`, `vac-app-server-protocol`, or `vac-cloud-requirements`. `vac-rs/exec/src/**` and `vac-rs/exec/tests/**` contain zero `vac_app_server_client` / `vac_app_server_protocol` imports.
- **Plan 00D-B11 through 00D-B30** landed in slices `9cddd29` (Sesi A) through `61f4bfc` (Sesi E, B30). TUI direct deps on `vac-app-server-client` and `vac-app-server-protocol` are removed: `vac-rs/tui/Cargo.toml` no longer lists them, and the only residual textual matches across `vac-rs/tui/src/**` are historical comment lines in `vac-rs/tui/src/legacy_core.rs:3-4` describing the former import path. The module itself only re-exports from `vac_core`.
- **TUI cargo gates green at B30:** `cargo test -p vac-surface-tui --lib` → 2159 passed; `cargo check -p vac-surface-tui --tests` → passed; `cargo check --workspace --tests` → passed; `cargo test -p vac-core local_runtime --lib` → passed; `cargo check -p vac-app-server-client --tests` → passed.

## Method

From `vac-rs/`:

```bash
cargo +1.93.0 tree -p vac-surface-cli -i <crate> --edges normal,build
```

for each candidate family in `docs/migration/OLD_RUNTIME_DELETE_GATE.md`. Inverse trees give the *direct* callers (1 level up from the candidate); indirect callers are reachable through those.

Supplemented by direct grep on `vac-rs/*/Cargo.toml` and `vac-rs/*/src/**` for `vac_app_server_*` references.

## Historical per-crate result (post 00C + 00D-B30)

### `vac-app-server-client`

**Direct callers from `vac-cli` graph:**
- ~~`vac-exec`~~ — **cleared** by 00C. `vac-rs/exec/Cargo.toml` and `vac-rs/exec/src/**` no longer reference the crate.
- ~~`vac-tui`~~ — **cleared** by 00D-B30. `vac-rs/tui/Cargo.toml` does not list it; `vac-rs/tui/src/**` has zero non-comment references.

**Indirect from `vac-cli`:** none. The crate is no longer reachable from the local product path.

**Non-`vac-cli` callers (workspace-wide, NOT in the local product path):** still present in `vac-rs/{models-manager,core-plugins,otel,login,external-agent-sessions,config,core}` Cargo.toml + sources. These are non-product crates and do not affect the 00E vac-cli reachability gate, but they block any physical `cargo workspace member` removal until they migrate to Local Runtime Contract DTOs.

**Classification:** **STILL REACHABLE from `vac-cli` via `vac-tui`**. The TUI consumer surface now routes through `local_runtime_session.rs`, but the Cargo graph still keeps this crate reachable through `vac-tui`, so the crate is not yet deleteable from the local product path. Physical crate deletion remains BLOCKED by (a) the operator guardrail "Jangan hapus `vac-rs/app-server*/`" issued for this session, and (b) the non-product consumers listed above. Resolution path is Plan 19 (root-feature-conversion) and Plan 21 (dead-code-and-ownership), not a 00E one-shot.

### `vac-app-server`

Direct caller from `vac-cli` graph at HEAD `61f4bfc`: **`vac-tui`** still lists `vac-app-server = { workspace = true }` at `vac-rs/tui/Cargo.toml:29`. The B30 scope explicitly only retired `-client` and `-protocol` direct deps; the parent server crate (likely used for shared types via re-export) is still on the TUI dependency surface.

**Indirect (via app-server-client):** now empty in vac-cli (see above).

**Classification:** **STILL REACHABLE from `vac-cli` via `vac-tui`**. The direct consumer code is now behind `local_runtime_session.rs`, but the Cargo edge is still present. Not eligible for delete in this audit. Next concrete decoupling step is to enumerate what `vac-tui` still pulls from `vac_app_server` (likely DTO re-exports) and either migrate those to `vac-core` or to a fresh shared DTO crate — that is the natural 00D-B31+ work or a Plan 11 (slash-palette-convergence) sub-task once the surface is mapped.

### `vac-app-server-transport`

Direct callers (vac-cli graph): `vac-app-server`, `vac-otel`, `vac-login`, `vac-model-provider`. Still reachable through `vac-app-server` via `vac-tui`.

**Classification:** **STILL REACHABLE** via the `vac-tui → vac-app-server` edge. Same gate as `vac-app-server`.

### `vac-cloud-requirements`

This package is not present in the current workspace tree, so it is not part of
the active 00E reachability gate. The historical 2026-05-06 snapshot below
still records the earlier direct-edge analysis for context.

### `vac-app-server-protocol`

Reach unchanged from 2026-05-06: ~30 workspace crates depending on DTOs (`ConfigLayerSource`, `AppInfo`, `AskForApproval`, `AuthMode`, status enums, elicitation schemas, `AdditionalPermissionProfile`, `CommandExecutionApprovalDecision`, `ExecPolicyAmendment`, `NetworkPolicyAmendment`, etc.).

TUI and exec product-path imports are now zero, but the crate remains a workspace-wide shared DTO library. Removing it would require Local Runtime Contract equivalents to be promoted into `vac-core` (or a new DTO crate) and ~30 consumers to migrate.

**Classification:** **DEFERRED** (unchanged). Keep as shared DTO library. Reclassify when a Local Runtime Contract DTO crate exists and consumers migrate. Candidate plan: Plan 19 (root-feature-conversion).

### `vac-api`

Unchanged from 2026-05-06. Model/provider transport (responses API, websocket SSE, realtime, compaction, memories), not app-server. Per ADR-0007 § *Migration order* and § *Non-goals*, `vac-api` may remain temporarily for model/provider paths.

**Classification:** **DEFERRED structural**. Out of scope for 00E.

### `vac-exec-server`

Unchanged from 2026-05-06. Still reached from `vac-cli` via `vac-exec`, `vac-tui`, `vac-app-server`, `vac-apply-patch`, `vac-arg0`, `vac-rmcp-client`, `vac-core-plugins`, `vac-core-skills`, `vac-mcp`, `vac-utils-plugins`, and `vac-core`. Heavy producer of FS/executor/runtime path types.

**Classification:** **DEFERRED structural**. Out of scope for 00E. Removing it requires a new abstraction inside Local Runtime Contract or migrating consumers to `vac-core` types directly.

### `vac-backend-client`

Unchanged from 2026-05-06. Direct callers: `vac-cloud-requirements`, `vac-memories-write`, `vac-model-provider`, `vac-otel`. Most usage is backend memories sync / cloud auth telemetry.

**Classification:** **DEFERRED**. Re-evaluate after the memories pipeline decision lands.

## Summary table (post 00C + 00D-B30)

| Crate | Direct callers from `vac-cli` at HEAD `61f4bfc` | 00E action |
|---|---|---|
| `vac-app-server-client` | `vac-tui` | Still reachable; **physical delete BLOCKED** by guardrail + non-product consumers |
| `vac-app-server` | `vac-tui` (line 29 of tui/Cargo.toml) | **STILL REACHABLE**; decouple from TUI before delete-gate |
| `vac-app-server-transport` | via `vac-app-server` via `vac-tui` | **STILL REACHABLE**; same gate as parent |
| `vac-cloud-requirements` | not in current workspace tree | Not part of the active 00E gate |
| `vac-app-server-protocol` | ~30 DTO consumers workspace-wide | Keep, **DEFERRED** (shared DTO) |
| `vac-api` | model/provider transport | Keep, **DEFERRED structural** |
| `vac-exec-server` | many file-system / executor consumers (including `vac-tui` directly) | Keep, **DEFERRED structural** |
| `vac-backend-client` | `vac-cloud-requirements`, `vac-memories-write`, `vac-model-provider`, `vac-otel` | Keep, **DEFERRED** |

## Pre-condition checklist (status post 00D-B30)

- [x] `vac-rs/exec/src/lib.rs` no longer imports `vac_app_server_client::*` or `vac_app_server_protocol::*`. Verified by grep on `vac-rs/exec/src` and `vac-rs/exec/tests` → zero matches.
- [x] `vac-rs/exec/Cargo.toml` no longer lists `vac-app-server-client`, `vac-app-server-protocol`, `vac-cloud-requirements`. Verified at HEAD `61f4bfc`.
- [x] `vac-rs/tui/src/**` no longer imports `vac_app_server_client::*` or `vac_app_server_protocol::*`. Only historical comments in `vac-rs/tui/src/legacy_core.rs:3-4` remain.
- [x] `vac-rs/tui/Cargo.toml` no longer lists `vac-app-server-client`, `vac-app-server-protocol`, `vac-cloud-requirements`. Verified at HEAD `61f4bfc`.
- [ ] `vac-rs/tui/Cargo.toml` no longer lists `vac-app-server`. **Open.** Still listed at line 29. This is the next direct-dep to retire before `vac-app-server` becomes unreachable from `vac-cli`.
- [ ] `vac-rs/tui/Cargo.toml` no longer lists `vac-exec-server`. **Open**, but **DEFERRED structural** — `vac-exec-server` provides FS/executor types and is not on the 00E delete path.
- [ ] PTY operator gate (`docs/validation/TUI_PTY_DOGFOOD_GATE.md`) passes after 00C and 00D rewires. **Open** — run gate before any delete attempt.
- [ ] `cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build` returns no callers. **Expected to pass at HEAD `61f4bfc`** (not yet re-executed in this session; disk 88% used, deferred to next cargo gate run).
- [ ] Operator guardrail "Jangan hapus `vac-rs/app-server*/`" lifted, OR migration of non-product consumers (Plan 19 + Plan 21) lands so physical delete becomes safe.

## 00D-B31 precise map — `vac-tui → vac-app-server` edge (2026-05-11)

B31 recon shows that the remaining TUI dependency on `vac-app-server` is **not** a safe one-line `Cargo.toml` removal. The edge is narrow at the direct import layer, but broad at the DTO/facade layer.

### Direct TUI refs to `vac_app_server`

```text
vac-rs/tui/src/in_process_app_server_client.rs:45  pub use vac_app_server::in_process::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY
vac-rs/tui/src/in_process_app_server_client.rs:46  pub use vac_app_server::in_process::InProcessServerEvent
vac-rs/tui/src/in_process_app_server_client.rs:47  use vac_app_server::in_process::InProcessStartArgs
vac-rs/tui/src/in_process_app_server_client.rs:48  use vac_app_server::in_process::LogDbLayer
vac-rs/tui/src/in_process_app_server_client.rs:429 vac_app_server::in_process::start(...)
vac-rs/tui/src/session_protocol.rs:7              pub(crate) use vac_app_server::protocol::*
```

`cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build` confirms the only `vac-cli` path is:

```text
vac-app-server
└── vac-tui
    └── vac-cli
```

### Why B32 cannot be a naive dependency drop

A temporary dry-run that removed only `vac-app-server = { workspace = true }` from `vac-rs/tui/Cargo.toml` failed `cargo +1.93.0 check -p vac-surface-tui --tests` with ~640 compile errors. The first class of errors came from `session_protocol.rs` no longer exporting app-server protocol DTOs, causing unresolved imports across the UI surface. Highest fan-out consumers:

| File | `crate::session_protocol::*` import count |
|---|---:|
| `vac-rs/tui/src/app_server_session.rs` | 168 |
| `vac-rs/tui/src/chatwidget/tests/status_and_layout.rs` | 87 |
| `vac-rs/tui/src/chatwidget/tests.rs` | 87 |
| `vac-rs/tui/src/chatwidget.rs` | 68 |
| `vac-rs/tui/src/app/tests.rs` | 60 |
| `vac-rs/tui/src/app.rs` | 49 |
| `vac-rs/tui/src/in_process_app_server_client.rs` | 41 |
| `vac-rs/tui/src/app/app_server_requests.rs` | 36 |
| `vac-rs/tui/src/app_server_approval_conversions.rs` | 30 |

Representative DTOs that still flow through `session_protocol`: `ThreadStartParams`, `ThreadStartResponse`, `ServerNotification`, `ServerRequest`, `ThreadGoal`, `ThreadGoalStatus`, `ToolRequestUserInputParams`, `McpServerElicitationRequest`, `RateLimitSnapshot`, `SkillsListResponse`, plugin marketplace responses, approval decision DTOs, realtime audio DTOs, and JSON-RPC request IDs/results.

### B31 classification

- `vac-app-server-client` remains **reachable from `vac-cli` via `vac-tui`** until the Cargo edge is removed.
- `vac-app-server` remains **reachable** only because TUI still uses:
  1. the embedded `in_process` runtime start/handle/event API, and
  2. app-server protocol DTOs through `session_protocol.rs`.
- Replacing `session_protocol.rs` with `vac-app-server-protocol` directly would reintroduce the direct TUI protocol dependency that 00D-B30 intentionally removed, so that is **not** an acceptable final fix.

### Safe next implementation plan

1. **B32-A:** split `session_protocol.rs` into explicit TUI-owned groups instead of wildcard re-export, preserving import names while documenting owner crate per DTO. This is a no-behavior-change refactor and makes the remaining DTO debt visible.
2. **B32-B:** move DTOs that already exist in `vac-protocol`, `vac-core`, `vac-core-skills`, `vac-config`, or `vac-state` to those owner crates behind the same `crate::session_protocol::*` facade. Do this in small compile-tested slices.
3. **B32-C:** isolate DTOs that exist only in `vac-app-server-protocol` and decide whether to promote them to Local Runtime Contract / `vac-protocol` or keep them as deferred app-server compatibility.
4. **B33:** replace `in_process_app_server_client.rs` startup dependency with a Local Runtime Contract-backed adapter, then remove `vac-app-server` from TUI.

**UX impact:** no visual TUI change in B31. The impact is operator/developer clarity: future slices can retire the app-server edge without breaking chat, approvals, thread status, realtime, skills, plugins, or bottom-pane request UX.

## 00D-B32 facade migration status — `session_protocol` owner split (2026-05-11)

B32-A/B landed after the B31 map and converts the TUI `session_protocol` module from a wildcard facade into an explicit, auditable compatibility boundary.

### Landed slices

- **B32-A:** `vac-rs/tui/src/session_protocol.rs` no longer uses `pub(crate) use vac_app_server::protocol::*`. It now lists the session/protocol DTOs explicitly. This keeps call sites unchanged while making each remaining DTO dependency visible.
- **B32-A1:** the explicit facade has a scoped `#![allow(unused_imports)]` because it intentionally serves multiple lib/test/config surfaces. This avoids new validation noise.
- **B32-B1:** `ConfigLayerSource` moved from the app-server protocol block to `vac_config::ConfigLayerSource`. This is an exact re-export path and does not change type identity.
- **B32-B2:** `AppInfo` moved from the app-server protocol block to `vac_core::connectors::AppInfo`. This is also an exact re-export path and does not change type identity.
- **B32-B3:** facade comments now separate owner-migrated DTOs from the remaining app-server protocol compatibility block.
- **B32-C1:** the remaining app-server protocol DTOs are now routed through a private `app_server_protocol_compat` module. The top-level `session_protocol` facade still exports the same symbol names to TUI callers, but the exact app-server protocol seam is constrained to one active private module.
- **B32-C1-fmt:** `session_protocol.rs` was formatted after the seam split, with no DTO identity or call-site behavior changes.

### Validation

Each B32 slice was validated with:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

The gate passed after B32-A, B32-A1, B32-B1, B32-B2, B32-B3, B32-C1, and B32-C1-fmt.

### Current classification after B32-C1-fmt

- The wildcard app-server protocol facade is gone.
- The remaining protocol DTO debt is isolated behind the private `app_server_protocol_compat` seam in `vac-rs/tui/src/session_protocol.rs`. The public/local TUI facade remains explicit and call sites still import `crate::session_protocol::*` with unchanged DTO identities.
- TUI still depends on `vac-app-server` because `session_protocol.rs` still sources remaining compatibility DTOs from the app-server protocol owner, and `in_process_app_server_client.rs` still delegates startup to the private `runtime_bridge` over `vac_app_server::in_process`.
- Exact owner re-export candidates found so far are exhausted. Do **not** bulk swap app-server protocol DTOs to `vac_protocol` just because names match: many app-server protocol DTOs are wrapper/conversion DTOs around `vac_protocol` core types, not identical type aliases.
- Latest B32 pushed HEAD: `ebfd1f764a255de2e349d27c4d2854ccc3111eac` (`ebfd1f7 tui: format session protocol compat seam (00D-B32C1-fmt)`).

### Safe next implementation options

1. **B32-D:** only migrate a DTO family out of `app_server_protocol_compat` when compile/test evidence proves exact type compatibility or a deliberate conversion layer exists. Start with low-risk display-only DTOs; avoid approval/request/response payloads until equivalence is covered.
2. **B33-C:** replace the embedded app-server startup path only after a runtime-first owner seam can supply equivalent request dispatch, event stream, server-request resolution, retained managers/stores, external-agent detection, and graceful shutdown.
3. **Cargo drop gate:** remove `vac-app-server` from `vac-rs/tui/Cargo.toml` only after both remaining TUI seams are gone: the `session_protocol` compatibility module and the `runtime_bridge` startup delegation.

**UX impact:** B32 has no visual TUI changes. Its user-facing value is risk reduction: future runtime-first slices can retire app-server DTOs without accidentally breaking chat rendering, approval overlays, thread lifecycle, realtime audio, plugins, skills, or bottom-pane prompts.

## 00D-B33-A startup boundary status — `in_process_app_server_client` narrowing (2026-05-11)

B33-A continued from the B32 facade work by narrowing the TUI in-process startup boundary without changing runtime behavior or TUI call sites.

### Landed slices

- **B33-A1:** `LogDbLayer` moved from `vac_app_server::in_process::LogDbLayer` to the actual owner path `vac_state::log_db::LogDbLayer`.
- **B33-A2:** raw `InProcessServerEvent` stopped being publicly re-exported from the TUI in-process facade. It remains an internal worker/backpressure type.
- **B33-A3:** `DEFAULT_IN_PROCESS_CHANNEL_CAPACITY` is now owned by the TUI `app_server_session` boundary as a local constant with the same value as the app-server default (`4_096`). This avoided touching unrelated dirty `tui/src/lib.rs` while preserving existing call sites.
- **B33-A3-fix:** the parent capacity constant import in `in_process_app_server_client.rs` is test-scoped to avoid a new normal-build unused-import warning.
- **B33-A4:** `InProcessAppServerClient::next_event()` now returns the local `AppServerEvent` facade instead of raw `vac_app_server::in_process::InProcessServerEvent`; raw events remain internal to the worker/backpressure layer.
- **B33-A5:** historical raw `vac_app_server::in_process` paths were removed from facade comments, so grep output now points only at active dependency surface.

### Validation

Each B33-A code slice was validated with:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

The gate passed after B33-A1, B33-A2, B33-A3, B33-A3-fix, B33-A4, and B33-A5.

### Current classification after B33-A5

The remaining direct TUI `vac_app_server::in_process` references are active startup dependencies, not comments or re-export noise:

```text
vac-rs/tui/src/in_process_app_server_client.rs:
use vac_app_server::in_process::InProcessServerEvent;
use vac_app_server::in_process::InProcessStartArgs;
let mut handle = vac_app_server::in_process::start(args.into_runtime_start_args()).await?;
```

These symbols are defined only in `vac-rs/app-server/src/in_process.rs` and have no exact owner re-export outside the app-server stack. Do **not** attempt another mechanical owner swap here. The next safe implementation must replace or wrap the startup function itself with a runtime-first equivalent.

### Safe next implementation options

1. **B33-B1:** introduce a TUI-owned startup seam that constructs/manages `ThreadManager`, `AuthManager`, `EnvironmentManager`, and `ThreadStore` directly, then translates its event stream into `AppServerEvent`/`TuiSessionEvent` without exposing raw app-server event types.
2. **B33-B2:** move `InProcessStartArgs` shape into a TUI/local-runtime owned argument struct only after the actual startup function no longer requires `vac_app_server::in_process::start`.
3. **B33-C:** remove `vac-app-server` from `vac-rs/tui/Cargo.toml` only after the remaining three active startup references above are gone and `session_protocol` compatibility debt is resolved or isolated behind non-app-server owners.

**UX impact:** B33-A has no visual TUI changes. Its user-facing value is risk reduction: future startup rewrite work has a precise boundary and preserves chat streaming, approval overlays, thread lifecycle, realtime events, skills/plugins, and bottom-pane request UX.

## 00D-B33-B startup rewrite recon — direct copy is blocked (2026-05-11)

B33-B recon inspected the remaining active TUI startup dependency on `vac_app_server::in_process` after B33-A narrowed the facade.

### Remaining active TUI references

```text
vac-rs/tui/src/in_process_app_server_client.rs:
use vac_app_server::in_process::InProcessServerEvent;
use vac_app_server::in_process::InProcessStartArgs;
let mut handle = vac_app_server::in_process::start(args.into_runtime_start_args()).await?;
```

These are the only remaining direct `vac_app_server::in_process` references in TUI. They are active runtime startup dependencies, not re-export/comment noise.

### Why direct startup copy is not safe

`vac_app_server::in_process::start` is not a thin owner re-export. It constructs app-server internals directly:

- `AuthManager::shared_from_config`
- `analytics_events_client_from_config`
- `thread_store_from_config`
- `ThreadManager::new`
- `OutgoingMessageSender`
- `ConfigManager`
- `MessageProcessor::new(MessageProcessorArgs { ... })`
- `ConnectionSessionState`, outbound routing, request serialization, thread listener attachment, background drain/shutdown

The key blocker is visibility and dependency shape:

- `MessageProcessor` and `MessageProcessorArgs` are `pub(crate)` in `vac-rs/app-server/src/message_processor.rs`.
- `MessageProcessor` construction depends on app-server-only modules such as `OutgoingMessageSender`, `ConfigManager`, `ConnectionSessionState`, outbound routing, request serialization queues, plugin startup tasks, and app-server transport/RPC enums.
- Copying this loop into TUI would pull app-server internals across the boundary and likely require reintroducing direct protocol/server dependencies such as `vac-app-server-protocol` and `vac-analytics`.

That would undo the B30/B32 direction. Do **not** port `start_uninitialized` by copy/paste into TUI.

### Correct B33-B direction

The next safe code path is a new owner seam, not a mechanical move:

1. Add or identify a runtime-first session startup API outside `vac-app-server` that owns the product-level primitives TUI needs: `ThreadManager`, `AuthManager`, `EnvironmentManager`, `ThreadStore`, request/response dispatch, event stream, server-request resolution, and graceful shutdown.
2. Translate events into the local TUI facade (`AppServerEvent`/`TuiSessionEvent`) at the boundary. Raw app-server event types must not leak out again.
3. Keep chat streaming, approval overlays, thread lifecycle, realtime events, skills/plugins, external-agent config, and bottom-pane requests covered before removing `vac-app-server` from TUI Cargo.

### Safe immediate next slice

- **B33-B2:** design a minimal runtime-first session handle shape in TUI/core docs or code comments, with explicit equivalence against the current `InProcessAppServerClient` methods:
  - `request_typed`
  - `request_handle`
  - `next_event`
  - `resolve_server_request` / `reject_server_request`
  - `thread_manager` / `auth_manager` / `thread_store` / `environment_manager`
  - `external_agent_config_detect`
  - `shutdown`
- Only after that shape is explicit should implementation start moving one behavior cluster at a time.

**UX impact:** no visual change from this recon. The user-facing value is risk reduction: the next startup rewrite must preserve visible chat, approvals, thread status, realtime, plugins/skills, and bottom-pane request UX instead of silently replacing app-server internals with incomplete runtime plumbing.

## 00D-B33-B runtime bridge status — raw startup details isolated behind private bridge (2026-05-11)

B33-B continued the startup boundary narrowing after recon showed that directly copying `vac_app_server::in_process::start_uninitialized` into TUI is not safe.

### Landed B33-B slices

- **B33-B1:** recorded why direct startup copy is blocked: `MessageProcessor`/`MessageProcessorArgs` are app-server-private and the startup loop depends on app-server transport/RPC/config machinery.
- **B33-B2:** documented the runtime-first session-handle replacement contract directly in `vac-rs/tui/src/in_process_app_server_client.rs`.
- **B33-B2-fix:** kept exact `vac_app_server::in_process` grep results active-only by removing exact raw paths from explanatory comments.
- **B33-B3:** concentrated the app-server in-process dependency behind private aliases in the TUI facade.
- **B33-B4:** introduced a private startup helper so startup had one narrow bridge callsite.
- **B33-B5:** moved the worker event queue from raw runtime events to local `AppServerEvent`.
- **B33-B5-fix / fix4:** repaired raw-vs-facade event matching and test assertions after the queue move; the final gate was green.
- **B33-B6:** renamed the private raw event alias so public/local TUI surfaces kept using `AppServerEvent` while raw app-server event names stayed constrained to the bridge.
- **B33-B7:** recorded the initial B33-B bridge status in this audit.
- **B33-B8:** moved the exact `vac_app_server::in_process` import into the private `runtime_bridge` module.
- **B33-B9:** wrapped raw app-server handle/sender types in `runtime_bridge::Handle` and `runtime_bridge::Sender`, exposing only the methods TUI currently uses.
- **B33-B10:** moved raw app-server `InProcessStartArgs` construction into `runtime_bridge`; the outer TUI facade starts from `InProcessClientStartArgs` only.
- **B33-B11:** made `runtime_bridge::Handle::next_event()` return local `AppServerEvent`, so the TUI worker no longer receives raw runtime events.
- **B33-B12:** moved raw app-server event conversion fully inside `runtime_bridge`; the outer facade no longer has a raw-event `From` impl.
- **B33-B13:** kept the app-server `StartArgs` alias private to the bridge and removed the outer runtime-handle alias.
- **B33-B14:** removed the outer startup helper and made `runtime_bridge::start(InProcessClientStartArgs)` the single private bridge entrypoint; raw `StartArgs` construction and app-server `start` call remain inside `start_raw`.
- **B33-B15:** kept the private bridge entrypoint explicit while preserving the outer TUI facade shape; no caller-facing request/event API changed.
- **B33-B16:** added `runtime_bridge::RetainedResources` and replaced individual `Handle` resource accessors with one retained-resource package containing `ThreadManager`, `AuthManager`, `ThreadStore`, `EnvironmentManager`, and the request sender.
- **B33-B17:** added `runtime_bridge::StartedRuntime { handle, retained_resources }` and made `runtime_bridge::start(args)` return the started handle together with retained resources. The outer `InProcessAppServerClient::start` now destructures one bridge-owned startup bundle instead of asking the handle for resources after startup.
- **B33-B19:** builds `StartedRuntime` directly from the raw in-process handle inside `start_raw`, then removes the now-unused `Handle::retained_resources()` accessor. Resource extraction is now fully part of the raw-start bridge conversion instead of a later handle method.
- **B33-B21:** moved `configured_thread_config_loader` and its `NoopThreadConfigLoader` / `ThreadConfigLoader` imports into the private `runtime_bridge` module, so outer TUI startup no longer carries thread-config-loader startup wiring.
- **B33-B23:** documented the runtime bridge replacement capability map that any runtime-first owner seam must cover before `vac-rs/tui` can drop `vac-app-server`.
- **B33-B24:** renamed the in-process test client label from `vac-app-server-client-test` to `vac-tui-in-process-client-test`, so exact app-server grep output now reflects only active dependency seams.
- **B33-B26:** added a source-level capability and UX gate to the private `runtime_bridge` module so future runtime-first replacement work preserves the full startup/request/event/shutdown contract.
- **B33-B27:** added a source-level compatibility and UX gate to the private `session_protocol::app_server_protocol_compat` DTO seam so future DTO owner migrations do not rely on name-only swaps.
- **B33-B29:** made the runtime bridge source gate grep-neutral while preserving the same capability/UX guardrail, so exact app-server grep output remains active-only.
- **B33-B31:** mapped runtime-first owner candidates. No existing non-app-server crate currently covers the full bridge capability map; the likely path is a new extracted runtime owner boundary, with `vac-app-server-client` as a facade reference and `vac-core::local_runtime` as the long-term semantic contract.
- **B33-B32:** introduced a local `runtime_owner_contract` with `RuntimeSessionHandle` and `RuntimeSessionSender` traits, then implemented those traits with the current private server-backed bridge. This gives future owner extraction a compile-checked method boundary while keeping runtime behavior unchanged.
- **B33-B34:** extracted the in-process worker loop into `spawn_runtime_worker<H, S>` generic over `RuntimeSessionHandle` and `RuntimeSessionSender`, so the worker now depends on the owner contract instead of concrete bridge methods. B33-B34-fix removed follow-up warning noise without changing behavior.
- **B33-B36:** moved the started-runtime and retained-resource bundle shapes into `runtime_owner_contract` as `StartedRuntime<H, S>` and `RuntimeRetainedResources<S>`. The server-backed bridge remains the only constructor, but the outer TUI start path now destructures contract-owned bundle types.
- **B33-B38:** added a `RuntimeStarter` startup trait to `runtime_owner_contract` and routed the private server-backed owner through it. `runtime_bridge::start(args)` remains the only outer startup path, but startup ownership now has a compile-checked contract seam.
- **B33-B40:** removed the bridge-local startup wrapper and made `InProcessAppServerClient::start()` delegate through `start_with_runtime_owner<O>()`, with `runtime_bridge::ServerBackedRuntimeOwner` still the only concrete owner. This keeps the public start path unchanged while making the outer client startup generic over the runtime-owner contract.
- **B33-B42:** scoped the server-backed startup helper methods under `runtime_bridge::ServerBackedRuntimeOwner`, so raw start-args conversion, raw app-server startup, and thread-config-loader selection are now owned by the concrete runtime owner instead of bridge-free functions.
- **B33-B44:** encapsulated the contract-owned startup bundle fields behind constructors and `into_parts()` accessors. The server-backed owner now constructs `StartedRuntime` / `RuntimeRetainedResources` through contract methods, and the outer client consumes them through contract methods instead of destructuring public fields.
- **B33-B46:** bundled the outer `InProcessAppServerClient` retained manager/store fields into `InProcessClientRetainedResources`, so the client now keeps one retained-resource package after startup instead of four separate optional fields.
- **B33-B48:** added accessor methods on `InProcessClientRetainedResources` and routed the public client resource accessors through them, so retained manager/store reads no longer reach directly into bundle fields.
- **B33-B50:** extracted `InProcessAppServerClient::from_started_runtime(...)`, so the generic startup path now only starts the owner and delegates client construction from the contract-owned started-runtime bundle to one helper.
- **B33-B52:** extracted `InProcessAppServerClient::normalized_channel_capacity(...)`, isolating the startup queue-capacity clamp before the runtime-owner start handoff.
- **B33-B54:** extracted `InProcessAppServerClient::worker_channels(...)`, isolating command/event channel creation from started-runtime client construction.
- **B33-B56:** added `InProcessClientRetainedResources::from_runtime_retained_resources(...)`, so conversion from the contract-owned runtime retained-resource bundle into the client-owned retained-resource bundle now happens in one helper.
- **B33-B58:** extracted `InProcessAppServerClient::spawn_worker(...)`, so started-runtime client construction now delegates runtime-worker spawning through a client-local helper instead of calling the free worker helper inline.
- **B33-B60:** extracted `InProcessAppServerClient::from_worker_parts(...)`, so final client assembly from worker channels, worker handle, and retained resources is now a named construction seam.
- **B33-B62:** extracted `InProcessAppServerClient::start_worker_parts(...)`, grouping channel creation and runtime-worker spawning into one helper that returns the command sender, event receiver, and worker handle.
- **B33-B64:** introduced `InProcessWorkerParts`, bundling the command sender, event receiver, and worker handle returned by worker startup before final facade assembly.
- **B33-B66:** encapsulated `InProcessWorkerParts` behind `new(...)` and `into_parts(...)`, so worker-startup output is constructed and consumed through explicit bundle APIs.
- **B33-B68:** extracted `InProcessAppServerClient::split_started_runtime(...)`, so started-runtime unpacking and retained-resource conversion now happen behind one named helper before worker startup.
- **B33-B70:** introduced `InProcessStartedRuntimeParts<H, S>`, bundling the runtime handle, client retained resources, and request sender returned from started-runtime splitting before worker startup.
- **B33-B72:** moved the `Option` wrapping for retained resources into `InProcessAppServerClient::from_worker_parts(...)`, so final facade assembly owns how retained resources are stored.
- **B33-B74:** extracted `InProcessAppServerClient::from_started_runtime_parts(...)`, so building the facade from the started-runtime client-parts bundle is now separated from splitting the contract-owned runtime bundle.
- **B33-B76:** encapsulated `InProcessAppServerRequestHandle` construction behind `new(...)`, so request-handle creation no longer directly writes facade fields at the call site.
- **B33-B78:** encapsulated `AppServerRequestHandle::InProcess(...)` construction behind `AppServerRequestHandle::in_process(...)`, so outer request-handle wrapping is an explicit seam.
- **B33-B80:** added `AppServerClient::in_process(...)`, establishing an explicit outer client-construction seam for wrapping the in-process facade.
- **B33-B82:** introduced `InProcessWorkerChannels`, bundling command/event channel endpoints before worker-task spawning.
- **B33-B84:** extracted `InProcessAppServerClient::start_worker_from_channels(...)`, so worker-task startup from a prepared channel bundle is now a named seam.
- **B33-B86:** introduced `InProcessWorkerRuntimeParts<H, S>`, bundling the runtime handle and request sender before channel-backed worker startup.
- **B33-B88:** introduced `InProcessWorkerStartupParts<H, S>`, bundling worker runtime inputs plus prepared channels before worker startup.
- **B33-B90:** introduced `InProcessClientAssemblyParts`, bundling worker parts with retained resources before final in-process client construction.
- **B33-B92:** added a private `InProcessAppServerClient::new(...)` constructor, centralizing final field assignment for the in-process client facade.
- **B33-B94:** added private `command_tx()`, `event_rx_mut()`, and `retained_resources()` accessors so in-process client field reads have named seams.
- **B33-B96:** added `InProcessAppServerRequestHandle::command_tx()`, routing request-handle dispatch through a named command-sender accessor.
- **B33-B98:** added `AppServerRequestHandle::in_process_handle()`, routing outer request-handle enum dispatch through a named variant accessor.
- **B33-B100:** added `AppServerClient` in-process variant accessors, routing outer client request/event/resource dispatch through named seams.
- **B33-B102:** added `InProcessClientShutdownParts`, routing consuming in-process client close through a named parts bundle.
- **B33-B104:** added runtime bridge handle/sender constructors and inner accessors, routing app-server-backed bridge calls through named seams.
- **B33-B106:** extracted runtime bridge start-args conversion into `ServerBackedRuntimeOwner::start_args(...)`.
- **B33-B108:** extracted runtime bridge retained-resources construction into `ServerBackedRuntimeOwner::retained_resources(...)`.
- **B33-B110:** extracted runtime bridge raw start call into `ServerBackedRuntimeOwner::start_inner(...)`.
- **B33-B112:** scoped runtime bridge event conversion onto `Handle::event_into_facade(...)`.
- **B33-B114:** moved runtime bridge retained resources onto `Handle::retained_resources(&self)`.
- **B33-B116:** routed runtime bridge teardown through `Handle::teardown_inner(self)`.
- **B33-B118:** routed runtime bridge sender dispatch through `Sender::inner_ref()` and `Sender::dispatch_*` helpers.
- **B33-B120:** routed runtime bridge external-agent detection through `Handle::external_agent_config_detect_inner(&self, ...)`.
- **B33-B122:** routed runtime bridge event polling through `Handle::next_event_inner(&mut self)`.

### Current TUI bridge shape after B33-B14

`vac-rs/tui/src/in_process_app_server_client.rs` now has a single exact active `vac_app_server::in_process` import, scoped inside `runtime_bridge`:

```rust
mod runtime_bridge {
    use vac_app_server::in_process as app_server_in_process;
    // ...
}
```

The raw app-server runtime types are constrained to the bridge internals:

```rust
type StartArgs = app_server_in_process::InProcessStartArgs;

pub(super) struct Handle {
    inner: app_server_in_process::InProcessClientHandle,
}

#[derive(Clone)]
pub(super) struct Sender {
    inner: app_server_in_process::InProcessClientSender,
}

fn event_into_facade(
    event: app_server_in_process::InProcessServerEvent,
) -> AppServerEvent { /* ... */ }
```

The only outer startup callsite is now bridge-owned and client-arg based, and returns the handle plus retained resources as one bridge-owned startup result:

```rust
let runtime_bridge::StartedRuntime {
    mut handle,
    retained_resources,
} = runtime_bridge::start(args).await?;
```

The retained resources cross the bridge as one explicit package instead of individual handle accessors:

```rust
let runtime_bridge::RetainedResources {
    thread_manager,
    auth_manager,
    thread_store,
    environment_manager,
    sender: request_sender,
} = retained_resources;
```

The TUI-facing event queue stores local facade events only:

```rust
event_rx: mpsc::Receiver<AppServerEvent>
let (event_tx, event_rx) = mpsc::channel::<AppServerEvent>(channel_capacity);
pub async fn next_event(&mut self) -> Option<AppServerEvent>
```

### Validation

The latest B33-B bridge hardening slices were each validated with:

```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

Latest pushed bridge-hardening HEAD: `1a729edbf27833acc4be70d4e55855d6dbd6de5f` (`1a729ed tui: move runtime start bundle into owner contract (00D-B33B36)`).

B33-B23, B33-B25, B33-B28, B33-B30, B33-B31, B33-B33, B33-B35, B33-B37, B33-B39, B33-B41, B33-B43, B33-B45, B33-B47, B33-B49, B33-B51, B33-B53, B33-B55, B33-B57, B33-B59, B33-B61, B33-B63, B33-B65, B33-B67, B33-B69, B33-B71, B33-B73, B33-B75, B33-B77, B33-B79, B33-B81, B33-B83, B33-B85, B33-B87, B33-B89, B33-B91, B33-B93, B33-B95, B33-B97, B33-B99, B33-B101, B33-B103, B33-B105, B33-B107, B33-B109, B33-B111, B33-B113, B33-B115, B33-B117, B33-B119, and B33-B121 are documentation/recon-only. B33-B26/B33-B27/B33-B29 are source-comment gates only; B33-B32/B33-B34/B33-B36/B33-B38/B33-B40/B33-B42/B33-B44/B33-B46/B33-B48/B33-B50/B33-B52/B33-B54/B33-B56/B33-B58/B33-B60/B33-B62/B33-B64/B33-B66/B33-B68/B33-B70/B33-B72/B33-B74/B33-B76/B33-B78/B33-B80/B33-B82/B33-B84/B33-B86/B33-B88/B33-B90/B33-B92/B33-B94/B33-B96/B33-B98/B33-B100/B33-B102/B33-B104/B33-B106/B33-B108/B33-B110/B33-B112/B33-B114/B33-B116/B33-B118/B33-B120/B33-B122 are source contract/worker-boundary slices only. None changes runtime behavior.

### Validation

B33-B16, B33-B17, B33-B19, B33-B21, B33-B24, B33-B26, B33-B27, B33-B29, B33-B32, B33-B34, B33-B34-fix, B33-B36, B33-B38, B33-B40, B33-B42, B33-B44, B33-B46, B33-B48, B33-B50, B33-B52, B33-B54, B33-B56, B33-B58, B33-B60, B33-B62, B33-B64, B33-B66, B33-B68, B33-B70, B33-B72, B33-B74, B33-B76, B33-B78, B33-B80, B33-B82, B33-B84, B33-B86, B33-B88, B33-B90, B33-B92, B33-B94, B33-B96, B33-B98, B33-B100, B33-B102, B33-B104, B33-B106, B33-B108, B33-B110, B33-B112, B33-B114, B33-B116, B33-B118, B33-B120, and B33-B122 were each validated with:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

B33-B17, B33-B19, B33-B21, B33-B24, B33-B26, B33-B29, B33-B32, B33-B34, B33-B34-fix, B33-B36, B33-B38, B33-B40, B33-B42, B33-B44, B33-B46, B33-B48, B33-B50, B33-B52, B33-B54, B33-B56, B33-B58, B33-B60, B33-B62, B33-B64, B33-B66, B33-B68, B33-B70, B33-B72, B33-B74, B33-B76, B33-B78, B33-B80, B33-B82, B33-B84, B33-B86, B33-B88, B33-B90, B33-B92, B33-B94, B33-B96, B33-B98, B33-B100, B33-B102, B33-B104, B33-B106, B33-B108, B33-B110, B33-B112, B33-B114, B33-B116, B33-B118, B33-B120, and B33-B122 each staged and committed only `vac-rs/tui/src/in_process_app_server_client.rs`; B33-B27 staged and committed only `vac-rs/tui/src/session_protocol.rs`. After B33-B122 push, local `main`, `vac/main`, and `origin/main` all pointed at `530d3e39c73b5996383fe44dda4cd054a5b631b8`. The remaining dirty files are pre-existing unrelated workspace changes and were preserved.

### Remaining blocker

TUI still depends on `vac-app-server` because startup still delegates to the app-server-owned in-process runtime inside `runtime_bridge`, and `session_protocol.rs` still keeps an explicit app-server protocol compatibility block for DTOs that do not yet have proven owner-crate equivalents.

After B33-B24, exact app-server grep output is active-only:

```text
vac-rs/tui/src/in_process_app_server_client.rs: runtime_bridge imports vac_app_server::in_process as app_server_in_process
vac-rs/tui/src/session_protocol.rs: app_server_protocol_compat imports vac_app_server::protocol::{...}
vac-rs/tui/Cargo.toml: vac-app-server = { workspace = true }
```

This is the current 00D hard blocker. The `vac-app-server` Cargo edge cannot be safely removed until both active seams are replaced by non-app-server owner APIs. A direct dependency swap to `vac-app-server-protocol` is not an acceptable final fix because 00D-B30 intentionally removed direct TUI protocol dependency and the current session guardrail says not to introduce it. A direct copy of `MessageProcessor` / in-process startup internals is also blocked because those types are app-server-private and would pull app-server transport/RPC machinery into TUI.

B33-B26 and B33-B27 mirror this blocker in source comments at the two active seams, so future implementation work sees the same capability/UX gates directly where edits would happen. B33-B29 keeps those source gates grep-neutral, preserving active-only exact app-server grep output.

### Runtime bridge replacement capability map

Before `vac-rs/tui` can remove the `vac-app-server` dependency, a runtime-first owner seam must replace every capability currently hidden behind the private `runtime_bridge`. The replacement must provide these capabilities without copying app-server-private `MessageProcessor` internals into TUI:

| Current bridge capability | Current bridge surface | Runtime-first replacement requirement | UX invariant |
|---|---|---|---|
| Raw startup argument conversion | `runtime_bridge::start(InProcessClientStartArgs)` builds private `StartArgs` | Accept the existing TUI-owned start args or an equivalent owner-owned shape without leaking app-server DTOs | Session startup, auth/onboarding, and config warnings remain unchanged |
| Thread config loader selection | private `configured_thread_config_loader(&Config)` | Keep loader choice owner-local; today this is `NoopThreadConfigLoader` | No visible startup/config behavior change |
| Started runtime bundle | `StartedRuntime { handle, retained_resources }` | Return handle/lifecycle plus retained resources as one startup result | TUI receives managers/stores/sender before event loop starts |
| Retained resources | `RetainedResources { thread_manager, auth_manager, thread_store, environment_manager, sender }` | Preserve the same manager/store identities and request sender semantics | Thread list/read, auth state, filesystem/env behavior, and request dispatch remain stable |
| Request dispatch | `Sender::request(ClientRequest)` | Continue JSON-RPC-compatible request/result behavior for typed requests | Chat actions, config reads/writes, thread ops, skills/plugins calls keep working |
| Notification dispatch | `Sender::notify(ClientNotification)` | Preserve fire-and-forget notification behavior | No regressions for client notification paths |
| Server request resolve/reject | `respond_to_server_request` / `fail_server_request` | Resolve or reject approval and bottom-pane server requests by request ID | Approval modals and user-input prompts never hang |
| Event stream | `Handle::next_event() -> AppServerEvent` via `event_into_facade` | Emit local `AppServerEvent` only; no raw app-server event leakage | Chat streaming, lag markers, status, realtime, and terminal events render identically |
| External agent detection | `Handle::external_agent_config_detect` | Preserve external-agent config detection semantics or provide an equivalent owner API | Onboarding/agent attach flows remain unchanged |
| Shutdown | `Handle::shutdown` plus TUI worker timeout/abort fallback | Preserve graceful shutdown and bounded fallback behavior | No leaked background runtime tasks or stuck exit |

This map is the B33-B23 gate for future implementation: replacing the bridge is allowed only when the new owner seam covers the full table above. A partial replacement that drops request resolution, lossless event delivery, retained resources, or bounded shutdown would regress visible UX.

### B33-B31 runtime-first owner candidate map

B33-B31 recon mapped candidate owners for the runtime-first seam that would eventually replace the private `runtime_bridge` without copying app-server-private startup internals into TUI.

| Candidate owner | Evidence from recon | Classification | Why it can/cannot own the final seam |
|---|---|---|---|
| `vac-app-server::in_process` | Public module provides `InProcessStartArgs`, `InProcessClientHandle`, `InProcessClientSender`, `InProcessServerEvent`, `start`, request/notification dispatch, server-request response/error, retained managers/stores, external-agent detect, and shutdown. Internally it constructs `MessageProcessor::new(MessageProcessorArgs { ... })`, `OutgoingMessageSender`, `ConfigManager`, `ThreadManager`, `AuthManager`, `ThreadStore`, and app-server routing state. | Current implementation only | It covers the full capability map today, but it is app-server-owned and is the edge being removed. Keeping it cannot close the `vac-tui -> vac-app-server` reachability gate. |
| `vac-app-server-client` | Workspace crate already wraps `vac_app_server::in_process` behind an async facade with startup args, typed/raw request helpers, event forwarding/backpressure, server-request resolve/reject, retained resources, external-agent detect, and bounded shutdown. It depends on `vac-app-server` and `vac-app-server-protocol`, and TUI intentionally no longer depends on it after B30. | Template / extraction source, not final owner | Its facade shape is close to what TUI needs, but using it directly would reintroduce the dependency B30 removed and still keeps the app-server edge. It is useful as a reference for a future extracted runtime owner API. |
| `vac-core::local_runtime` | Existing Local Runtime Contract exports product-level `RuntimeCommand`, `RuntimeEvent`, approval/task/session types, and `LocalRuntimeBridge` projections. It does not expose live startup, request dispatch, server-request resolution, retained managers/stores, external-agent detection, or shutdown handle semantics. | Long-term semantic owner candidate only | Good destination for product-level event/command semantics, but insufficient as the immediate lifecycle owner. It would need a new live session/handle layer before it can replace `runtime_bridge`. |
| `vac-core` outside `local_runtime` | Owns `ThreadManager`, `thread_store_from_config`, config/session primitives, plugins/skills integration, and several DTO owner re-exports already used by previous B32 slices. | Partial primitive owner | Owns important retained resources but not the JSON-RPC-compatible app-server request/event/server-request loop or external-agent integration. Cannot replace the bridge alone. |
| `vac-protocol` | Owns lower-level protocol/session/MCP data (`SessionSource`, `ThreadGoal`, MCP `RequestId`, `ThreadGoalStatus`, core protocol types). Recon also showed app-server-protocol has its own JSON-RPC `RequestId`/result/error shape; that seam now aliases the shared owner type. | DTO subset owner only | Useful for proven exact owner migrations, but not a runtime owner and not safe for name-only swaps. |
| `vac-app-server-protocol` | Owns most `ClientRequest`, `ClientNotification`, `ServerRequest`, `ServerNotification`, JSON-RPC result/error/id, and many TUI session DTOs. | Protocol compatibility owner, not runtime owner | It is protocol-only and has no lifecycle/runtime implementation. Direct TUI dependency is still not an acceptable final fix because B30 intentionally removed it. |
| `vac-app-server-transport` | Owns socket/websocket/transport machinery around app-server protocol. | Reject for local runtime seam | Transport concerns are the opposite direction from the in-process runtime-first seam; pulling it into TUI would widen the dependency surface. |
| New extracted runtime owner crate (for example a future local session runtime/facade crate) | No existing crate currently covers the full B33-B23 capability table outside app-server. `vac-app-server-client` shows a facade shape but still sits on app-server; `vac-core::local_runtime` shows product semantics but lacks live lifecycle. | Best candidate for implementation planning | The safest final owner is an extracted non-app-server runtime facade that depends on the real primitive owners and provides the existing capability table without app-server-private `MessageProcessor` leakage into TUI. |

**B33-B31 decision:** no existing non-app-server crate can currently replace `runtime_bridge` one-for-one. The next implementation should not be a dependency swap. It should introduce a new runtime-owner boundary in small slices, using `vac-app-server-client` as a facade reference and `vac-core::local_runtime` as the long-term semantic contract, while keeping app-server-private `MessageProcessor` and transport/RPC machinery out of TUI.

**Safe next code slice after B33-B31:** add a non-invasive owner-facing interface/contract first (trait or small facade types) that mirrors the current bridge methods (`start`, request/typed request, request handle, event stream, resolve/reject, retained resources, external-agent detect, shutdown). Keep the current app-server bridge as the only implementation until the contract compiles and is validated; then migrate one capability cluster at a time.

**UX impact:** B33-B31 is documentation/recon only. The user-facing value is risk reduction: it prevents the next slices from choosing an incomplete owner that would break chat streaming, approvals, thread lifecycle/status, realtime, skills/plugins, onboarding/auth, external-agent attach, bottom-pane prompts, or bounded shutdown.

### B33-B32 runtime-owner contract status

B33-B32 implemented the B33-B31 next step in `vac-rs/tui/src/in_process_app_server_client.rs` without moving runtime behavior. The new local `runtime_owner_contract` module defines two compile-checked traits:

- `RuntimeSessionHandle` for lifecycle/event/external-agent detection (`external_agent_config_detect`, `next_event`, `shutdown`).
- `RuntimeSessionSender` for request/notification/server-request response paths (`request`, `notify`, `respond_to_server_request`, `fail_server_request`).

The current private server-backed `runtime_bridge::Handle` and `runtime_bridge::Sender` implement those traits. Outer TUI call sites still use the same facade methods and the current bridge remains the only implementation. This is deliberately a boundary slice: it gives the future extracted runtime owner an explicit method contract before any capability cluster is migrated.

**B33-B32 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed. The commit staged only `vac-rs/tui/src/in_process_app_server_client.rs` and pushed `0a1694bab2dac5e15763c28125ad39cd7e9d853d`.

**UX impact:** no runtime or visual behavior change. The current bridge continues to drive chat streaming, approvals, realtime, skills/plugins, onboarding/auth, external-agent attach, bottom-pane prompts, and bounded shutdown while the owner boundary becomes explicit.

### B33-B34 runtime-worker contract usage

B33-B34 made the worker loop consume the contract introduced by B33-B32. The inline worker inside `InProcessAppServerClient::start` moved into a private `spawn_runtime_worker<H, S>` helper constrained by `RuntimeSessionHandle` and `RuntimeSessionSender`. The current `runtime_bridge::Handle` and `runtime_bridge::Sender` are still the values passed into the helper, so runtime behavior remains unchanged, but the worker no longer needs to be written against concrete bridge method impls.

The traits now return explicit `impl Future + Send` values instead of `async fn` trait methods, which keeps the worker-boundary contract suitable for `tokio::spawn` without adding public async-trait warning noise. B33-B34-fix then removed the unused trait imports and unnecessary mutable binding introduced during extraction.

**B33-B34 validation:** both B33-B34 and B33-B34-fix passed `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests`. Both staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after B33-B34-fix, local `main`, `vac/main`, and `origin/main` pointed at `9bf58eb59a8b44eca77b75272c5d7126ff8821e9`.

**UX impact:** no runtime or visual behavior change. The current bridge still drives the same chat streaming, approval, realtime, skills/plugins, onboarding/auth, external-agent attach, bottom-pane prompt, and bounded-shutdown paths; only the worker's dependency boundary changed.

### B33-B36 started-runtime contract bundle

B33-B36 moved the startup result shape into the local owner contract. `runtime_owner_contract` now defines `StartedRuntime<H, S>` and `RuntimeRetainedResources<S>`, covering the lifecycle handle plus retained thread manager, auth manager, thread store, environment manager, and request sender. The private server-backed `runtime_bridge` still constructs the bundle from the current in-process runtime, but `InProcessAppServerClient::start` now destructures the contract-owned types instead of bridge-local bundle structs.

This narrows the remaining extraction seam: a future non-server runtime owner must still provide the same live startup capabilities, but the outer TUI start path no longer names bridge-owned bundle types for the result shape.

**B33-B36 validation:** B33-B36 passed `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests`. It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `1a729edbf27833acc4be70d4e55855d6dbd6de5f`.

**UX impact:** no runtime or visual behavior change. Startup, retained managers/stores, event streaming, request forwarding, approval prompts, and lifecycle cleanup still route through the same server-backed bridge; only the internal bundle ownership boundary changed.

### B33-B38 runtime-owner startup contract

B33-B38 added `RuntimeStarter` to the local `runtime_owner_contract` module. The trait owns the startup handoff shape:

```rust
fn start(
    args: InProcessClientStartArgs,
) -> impl Future<Output = IoResult<StartedRuntime<Self::Handle, Self::Sender>>> + Send;
```

The private `runtime_bridge` now defines `ServerBackedRuntimeOwner` as the only implementation. `runtime_bridge::start(args)` remains a thin outer entrypoint and still calls the current server-backed startup internally, so runtime behavior is unchanged. The value of this slice is boundary clarity: future non-server runtime owners now have an explicit startup trait to satisfy before they can replace the current bridge.

**B33-B38 validation:** B33-B38 passed `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests`. It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `d8f21c684a96ea3658be5adb9a6517d1ea845c69`.

**UX impact:** no runtime or visual behavior change. Startup still flows through the same server-backed bridge, preserving chat streaming, approval prompts, bottom-pane requests, retained manager/store access, external-agent detection, and bounded shutdown; the change only makes the startup-owner seam explicit for future extraction.

### B33-B40 parameterized runtime-owner startup

B33-B40 made the outer TUI client startup path consume the startup contract directly. `InProcessAppServerClient::start(args)` now delegates to:

```rust
Self::start_with_runtime_owner::<runtime_bridge::ServerBackedRuntimeOwner>(args).await
```

and the new private helper is generic over `runtime_owner_contract::RuntimeStarter`:

```rust
async fn start_with_runtime_owner<O>(args: InProcessClientStartArgs) -> IoResult<Self>
where
    O: runtime_owner_contract::RuntimeStarter,
```

The redundant `runtime_bridge::start(args)` wrapper was removed, so the bridge no longer owns a second startup entrypoint. `runtime_bridge::ServerBackedRuntimeOwner` remains the only implementation and still constructs the same server-backed `StartedRuntime` bundle internally. No TUI caller, request/event DTO, retained manager/store, external-agent detection, or shutdown behavior changed.

**B33-B40 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `5dad2341a6016f0a2f57ffdae86e72a33cdfd6ec`.

**UX impact:** no visual/runtime change. Startup still uses the same server-backed owner, preserving chat streaming, approvals/server requests, bottom-pane prompts, retained managers/stores, external-agent detection, and bounded shutdown. The value is extraction safety: a future non-server owner can plug into the same private helper only after satisfying the compile-checked startup contract.

### B33-B42 server-backed startup helper scoping

B33-B42 moved the remaining server-backed startup helper functions into the concrete owner implementation block:

- `ServerBackedRuntimeOwner::start_server_backed(args)` owns TUI start-arg normalization and raw start-arg construction.
- `ServerBackedRuntimeOwner::start_raw(args)` owns the call into the current in-process app-server runtime and builds the contract-owned `StartedRuntime` bundle.
- `ServerBackedRuntimeOwner::configured_thread_config_loader(config)` owns the current no-op thread-config-loader selection.

`RuntimeStarter for ServerBackedRuntimeOwner` now delegates with `Self::start_server_backed(args)`. This is still the same server-backed runtime path, but the startup internals now sit behind the concrete owner type instead of bridge-level free functions.

**B33-B42 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `24G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `7e77380bb41ab3b922adf0bc1a015a4e7bc295e9`.

**UX impact:** no visual/runtime change. Chat streaming, approvals/server requests, bottom-pane prompts, retained managers/stores, external-agent detection, and bounded shutdown still route through the same server-backed owner. The value is boundary clarity: future runtime owners get a tighter owner-shaped startup template without inheriting bridge-free helper functions.

### B33-B44 runtime startup bundle encapsulation

B33-B44 made the contract-owned startup bundle opaque at the field level:

- `RuntimeRetainedResources<S>` now exposes `new(...)` and `into_parts()` instead of public fields.
- `StartedRuntime<H, S>` now exposes `new(handle, retained_resources)` and `into_parts()` instead of public fields.
- `ServerBackedRuntimeOwner::start_raw(args)` constructs the bundle through the contract constructors.
- `InProcessAppServerClient::start_with_runtime_owner<O>()` consumes the bundle through `into_parts()`.

This keeps the same data moving across the startup boundary, but future owners must now use the contract API rather than depending on the internal field layout.

**B33-B44 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `d797924abb7a01950c93f139c715130c1989f94e`.

**UX impact:** no visual/runtime change. Startup still returns the same handle, retained managers/stores, request sender, event stream, external-agent detection, and bounded shutdown behavior. The value is extraction safety: future runtime-owner implementations get a stable constructor/accessor API without relying on bridge-owned public fields.

### B33-B46 in-process retained-resource bundle

B33-B46 made the outer in-process client mirror the startup contract's resource packaging. `InProcessAppServerClient` now stores retained managers/stores as one `Option<InProcessClientRetainedResources>` instead of four separate optional fields. The public accessors still return the same `Option<Arc<...>>` values:

- `thread_manager()`
- `auth_manager()`
- `thread_store()`
- `environment_manager()`

Startup still receives the same contract-owned `RuntimeRetainedResources` bundle, converts it once into the client-owned bundle, and passes the request sender into the worker unchanged. The test-only manual client construction used by the lag-marker test now sets `retained_resources: None`, preserving the existing no-manager test shape.

**B33-B46 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `b27514049dca1746efe62a033909b3b81a611138`.

**UX impact:** no visual/runtime change. The same managers/stores remain available to TUI session code, while requests, events, approvals, bottom-pane prompts, external-agent detection, and bounded shutdown still use the same server-backed runtime path. The value is narrowing: retained resources now travel through named bundles instead of scattered fields, which reduces risk for future runtime-owner extraction.

### B33-B48 retained-resource bundle accessors

B33-B48 completed the client-side retained-resource bundle narrowing by adding accessor methods on `InProcessClientRetainedResources`:

- `thread_manager()`
- `auth_manager()`
- `thread_store()`
- `environment_manager()`

The public `InProcessAppServerClient` accessors now delegate to those bundle methods instead of cloning bundle fields directly. This keeps the same external return types and behavior, while ensuring the retained-resource bundle owns its internal field layout.

**B33-B48 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `e6dfdba80ca6d480413934cf5221d62ef75c3ba9`.

**UX impact:** no visual/runtime change. TUI callers still receive the same retained managers/stores, and the same request/event/approval/bottom-pane/external-agent/shutdown path remains server-backed. The value is extraction safety: retained-resource access is now mediated by a client-owned bundle API rather than scattered field reads.

### B33-B50 started-runtime client builder

B33-B50 extracted `InProcessAppServerClient::from_started_runtime(...)` as the single helper that turns a contract-owned `StartedRuntime<H, S>` plus channel capacity into an `InProcessAppServerClient`. The generic startup path now does only two things:

1. clamp the requested channel capacity;
2. start the selected runtime owner and hand the resulting bundle to `from_started_runtime(...)`.

The new helper remains generic over `RuntimeSessionHandle` and `RuntimeSessionSender`, unwraps the `StartedRuntime` / `RuntimeRetainedResources` bundles through their contract accessors, builds the client retained-resource bundle, creates the command/event channels, and spawns the existing worker.

**B33-B50 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `7a18a8fca3e437d9d0d2839a1631bf7183e50525`.

**UX impact:** no visual/runtime change. Startup still uses the same server-backed owner, request/event worker, retained managers/stores, approvals, bottom-pane prompts, external-agent detection, and bounded shutdown. The value is extraction safety: client construction from a started runtime is now isolated and can be reused by a future non-server owner that satisfies the same contract.

### B33-B52 channel-capacity normalization helper

B33-B52 extracted the startup queue-capacity clamp into `InProcessAppServerClient::normalized_channel_capacity(requested_capacity)`. The behavior remains the same (`max(1)`), but the clamp is now a named local helper before the runtime-owner start handoff.

**B33-B52 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `a8647edb6985367e006eb05f40c39bc28659139f`.

**UX impact:** no visual/runtime change. Queue capacity still clamps to at least one, so request/event channels keep the same overload behavior while startup remains server-backed. The value is isolation: channel sizing is now separated from owner startup and client construction for future runtime-owner extraction.

### B33-B54 worker channel setup helper

B33-B54 extracted `InProcessAppServerClient::worker_channels(channel_capacity)` as the single helper that creates the facade command channel and event channel pair used by the worker. `from_started_runtime(...)` now asks this helper for `(command_tx, command_rx, event_tx, event_rx)` before spawning the existing generic runtime worker.

**B33-B54 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `e77009f246012fe554937233968a143f1269d5b1`.

**UX impact:** no visual/runtime change. Command and event queues keep the same capacity and overload/backpressure behavior, while startup still uses the same server-backed runtime owner. The value is extraction safety: worker channel construction is now isolated from started-runtime unpacking and worker spawning.

### B33-B56 runtime-retained-resource conversion helper

B33-B56 added `InProcessClientRetainedResources::from_runtime_retained_resources(...)` as the single conversion point from the contract-owned `RuntimeRetainedResources<S>` bundle into the client-owned retained-resource bundle plus request sender. `from_started_runtime(...)` now delegates this conversion instead of unpacking all retained managers/stores inline.

**B33-B56 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `63453f8fb3eacd64332e0a8935554c121b17fe1c`.

**UX impact:** no visual/runtime change. The same retained managers/stores and request sender flow into the client and worker, while chat streaming, approvals, bottom-pane prompts, external-agent detection, and shutdown stay server-backed. The value is extraction safety: runtime-to-client retained-resource conversion now has one explicit seam.

### B33-B58 runtime-worker spawn helper

B33-B58 extracted `InProcessAppServerClient::spawn_worker(...)` as the client-local helper that delegates to the generic `spawn_runtime_worker(...)`. `from_started_runtime(...)` now asks this helper to start the worker after resource conversion and worker-channel setup.

**B33-B58 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `93e9de94aa3ab0f27f1f2dd4d0c2ecc822a17a30`.

**UX impact:** no visual/runtime change. The same worker loop still drives request dispatch, event forwarding, approvals, bottom-pane prompts, external-agent detection, and bounded shutdown. The value is extraction safety: worker spawning is now a named client-construction step rather than inline glue.

### B33-B60 in-process client worker-parts assembly helper

B33-B60 extracted `InProcessAppServerClient::from_worker_parts(...)` as the final construction helper that assembles the facade from the command sender, event receiver, worker task handle, and retained resources. `from_started_runtime(...)` now performs runtime bundle conversion, channel setup, and worker spawn, then delegates the final struct assembly to this helper.

**B33-B60 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `addecf791a97dc466692e3d81c863e5aea5cf09d`.

**UX impact:** no visual/runtime change. The same command sender, event receiver, worker handle, and retained resources are stored on the client, preserving request dispatch, event streaming, approvals, prompts, external-agent detection, and bounded shutdown. The value is extraction safety: final facade assembly is now isolated from runtime unpacking and worker startup.

### B33-B62 in-process worker-parts startup helper

B33-B62 extracted `InProcessAppServerClient::start_worker_parts(...)` as the helper that owns the paired channel creation and runtime-worker spawn step. `from_started_runtime(...)` now converts retained runtime resources, asks `start_worker_parts(...)` for the command sender, event receiver, and worker handle, then delegates final assembly to `from_worker_parts(...)`.

**B33-B62 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `a2e3a0bca4c8eda021cbb4227a5470464273ef23`.

**UX impact:** no visual/runtime change. The same command/event channels and worker task are created with the same capacity and worker loop, preserving request dispatch, event streaming, approvals, prompts, external-agent detection, and bounded shutdown. The value is extraction safety: worker startup has one named handoff between runtime resource conversion and final client assembly.

### B33-B64 in-process worker-parts bundle

B33-B64 introduced `InProcessWorkerParts` as the client-owned bundle for the command sender, event receiver, and worker task handle produced by `start_worker_parts(...)`. Final client assembly now accepts this named bundle instead of three separate positional values.

**B33-B64 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `24G` available / `89%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `2b9cd6fe9c7bd3cb375b6e08a96fd0a2316ad9ad`.

**UX impact:** no visual/runtime change. The same worker command sender, event receiver, and join handle are stored on the facade, preserving request dispatch, event streaming, approvals, prompts, external-agent detection, and bounded shutdown. The value is extraction safety: worker-startup output is now a named bundle, reducing positional coupling before future runtime-owner extraction.

### B33-B66 in-process worker-parts encapsulation

B33-B66 encapsulated `InProcessWorkerParts` by adding `new(...)` and `into_parts(...)`. Worker startup now constructs the bundle through `InProcessWorkerParts::new(...)`, and final facade assembly consumes it through `into_parts()` instead of reaching into fields directly.

**B33-B66 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `189G` used / `24G` available / `89%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `32b50de611b5ae0120bc9ca90af33835cc67e044`.

**UX impact:** no visual/runtime change. The same command sender, event receiver, and worker join handle are passed into the facade, preserving request dispatch, event streaming, approvals, prompts, external-agent detection, and bounded shutdown. The value is extraction safety: worker-startup bundle internals are now accessed through explicit construction/consumption APIs.

### B33-B68 started-runtime split helper

B33-B68 extracted `InProcessAppServerClient::split_started_runtime(...)` as the helper that consumes the contract-owned `StartedRuntime`, splits out the runtime handle, converts retained runtime resources into the client-owned retained-resource bundle, and returns the runtime request sender. `from_started_runtime(...)` now calls this helper before starting worker parts.

**B33-B68 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `24G` available / `89%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `dbdee00bd94e3429d553446027de07536cee1cf0`.

**UX impact:** no visual/runtime change. The same runtime handle, retained managers/stores, and request sender flow into worker startup and the facade, preserving chat streaming, approvals, prompts, external-agent detection, and bounded shutdown. The value is extraction safety: started-runtime decomposition is now isolated from worker startup and final facade assembly.

### B33-B70 started-runtime client-parts bundle

B33-B70 introduced `InProcessStartedRuntimeParts<H, S>` as the client-owned bundle for the runtime handle, converted retained-resource bundle, and request sender produced by `split_started_runtime(...)`. The split helper now returns this named bundle, and `from_started_runtime(...)` consumes it through `into_parts()` before worker startup.

**B33-B70 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `4eaee2bf2a4a2495b8a64edc1fd19f1c6a1fa15b`.

**UX impact:** no visual/runtime change. The same runtime handle, retained managers/stores, and request sender are still used for worker startup and facade assembly, preserving chat streaming, approvals, prompts, external-agent detection, and bounded shutdown. The value is extraction safety: started-runtime split output is now a named bundle instead of positional glue.

### B33-B72 retained-resource storage during facade assembly

B33-B72 moved retained-resource `Option` wrapping into `InProcessAppServerClient::from_worker_parts(...)`. The started-runtime path now passes the client retained-resource bundle directly to the final assembly helper, and that helper owns storing it on the facade as `Some(...)`.

**B33-B72 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `e4565eb618530ff916f308ab5b9267128f8996ec`.

**UX impact:** no visual/runtime change. The same retained managers/stores remain available through the same accessors, while request dispatch, event streaming, approvals, prompts, external-agent detection, and bounded shutdown are unchanged. The value is extraction safety: final facade assembly now owns the optional storage detail instead of leaking it into started-runtime decomposition.

### B33-B74 started-runtime-parts facade builder

B33-B74 extracted `InProcessAppServerClient::from_started_runtime_parts(...)` as the helper that builds the facade from `InProcessStartedRuntimeParts<H, S>` plus the normalized channel capacity. `from_started_runtime(...)` now only splits the contract-owned runtime bundle, then delegates worker startup and final facade assembly to the new helper.

**B33-B74 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `ff316003d5afc78e4d6a1c9d2e0bc247d0bca44e`.

**UX impact:** no visual/runtime change. The same runtime handle, request sender, worker channels, and retained resources are still used, preserving chat streaming, approvals, prompts, external-agent detection, and bounded shutdown. The value is extraction safety: facade construction from started-runtime parts is now a named seam that can later be backed by a runtime-first owner.

### B33-B76 in-process request-handle construction

B33-B76 added `InProcessAppServerRequestHandle::new(...)` and routed `InProcessAppServerClient::request_handle(...)` through it. This keeps request-handle field construction local to the request-handle type instead of open-coding the field assignment at the client accessor.

**B33-B76 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `24G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `248cd183c57c99effdc7f4723df4125a7a465bc7`.

**UX impact:** no visual/runtime change. Background and onboarding callers still receive a clone-backed request handle with the same request, approval-response, and shutdown behavior. The value is extraction safety: request-handle construction is now an explicit seam rather than direct facade-field wiring.

### B33-B78 outer request-handle construction seam

B33-B78 added `AppServerRequestHandle::in_process(...)` and routed `AppServerClient::request_handle(...)` through it. The outer request-handle enum now owns its in-process wrapping instead of open-coding `AppServerRequestHandle::InProcess(...)` at the client facade accessor.

**B33-B78 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `d79919eab6792c9cde472d644df362b206d9e12e`.

**UX impact:** no visual/runtime change. Existing request-handle callers still get the same in-process handle and the same request/request_typed behavior. The value is extraction safety: the outer handle conversion is now a named seam that can remain stable while the runtime owner behind it changes.

### B33-B80 outer client construction seam

B33-B80 added `AppServerClient::in_process(...)` as the named constructor for wrapping an `InProcessAppServerClient` in the outer facade enum. This is behavior-preserving and does not replace existing call sites yet; it creates the stable seam first so follow-up slices can route outer construction through it safely.

**B33-B80 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `bb3facd0a4e254c57c0f057e5d52af5c227a232d`.

**UX impact:** no visual/runtime change. Existing app-server client wrapping still works as before, and the new constructor is only a named path for later replacement safety. The value is extraction safety: outer client construction can be redirected without exposing app-server-private startup internals.

### B33-B82 worker channel bundle

B33-B82 introduced `InProcessWorkerChannels` with `new(...)` and `into_parts(...)`. `worker_channels(...)` now returns this named bundle, and `start_worker_parts(...)` consumes it before spawning the worker task.

**B33-B82 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `e8f7874b2bfc59fb226e23fc347c2a603507fd3c`.

**UX impact:** no visual/runtime change. Command/event queue capacities and worker-task behavior remain identical, preserving request dispatch, event streaming, approvals, prompts, and shutdown. The value is extraction safety: channel setup now has a named bundle boundary before worker startup.

### B33-B84 worker startup from channel bundle

B33-B84 extracted `InProcessAppServerClient::start_worker_from_channels(...)` as the named helper that consumes `InProcessWorkerChannels`, spawns the worker task, and returns `InProcessWorkerParts`. `start_worker_parts(...)` now owns capacity-to-channel creation and delegates channel-to-worker assembly.

**B33-B84 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `8399957382a1e665259f9b13efb84e764813be1f`.

**UX impact:** no visual/runtime change. Worker channels, request/event routing, approval prompt responses, and bounded shutdown remain identical. The value is extraction safety: worker startup from already-created channels is now isolated from channel allocation.

### B33-B86 worker runtime-parts bundle

B33-B86 introduced `InProcessWorkerRuntimeParts<H, S>` with `new(...)` and `into_parts(...)`. `start_worker_parts(...)` now bundles the runtime handle plus request sender before allocating worker channels, and `start_worker_from_channels(...)` consumes that bundle when spawning the worker task.

**B33-B86 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `11faee05c18345ed82e8990039f4ff6d05d5125b`.

**UX impact:** no visual/runtime change. The same runtime handle and request sender are used to serve request dispatch, event streaming, approval responses, external-agent detection, and shutdown. The value is extraction safety: runtime-owned worker inputs are now named separately from channel endpoints.

### B33-B88 worker startup-parts bundle

B33-B88 introduced `InProcessWorkerStartupParts<H, S>` with `new(...)` and `into_parts(...)`, bundling worker runtime inputs together with prepared worker channels. `start_worker_parts(...)` now creates the combined startup bundle, and `start_worker_from_startup_parts(...)` delegates to channel-backed worker startup.

**B33-B88 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `711269a7ea6d496b8197a719dd3ee2bdd183dd47`.

**UX impact:** no visual/runtime change. Runtime inputs, worker channels, request/event routing, approval responses, and bounded shutdown remain identical. The value is extraction safety: worker startup now has a single named bundle containing both runtime-owned inputs and channel endpoints.

### B33-B90 in-process client assembly-parts bundle

B33-B90 introduced `InProcessClientAssemblyParts` with `new(...)` and `into_parts(...)`, bundling final worker parts together with retained resources before constructing `InProcessAppServerClient`. `from_started_runtime_parts(...)` now builds this bundle and delegates through `from_client_assembly_parts(...)` before the final facade assembly.

**B33-B90 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `593b5aab1b51cdb1d8e6b30a6d4c069512c5ebab`.

**UX impact:** no visual/runtime change. The same worker command sender, event receiver, worker task, and retained managers are used by the facade. The value is extraction safety: final client assembly now has a named handoff between worker startup and facade construction.

### B33-B92 in-process client construction helper

B33-B92 added a private `InProcessAppServerClient::new(...)` constructor and routed final facade assembly through it. The helper centralizes assignment of the command sender, event receiver, worker task handle, and optional retained resources while keeping the externally visible start/request/event behavior unchanged.

**B33-B92 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `817a82e2c3fe07cd99988f75cd49e7c0a6f0a6cd`.

**UX impact:** no visual/runtime change. The same channels, worker task, retained resources, request dispatch, event stream, and shutdown path are preserved. The value is extraction safety: future facade replacement can target one constructor seam instead of direct field initialization.

### B33-B94 in-process client field accessors

B33-B94 added private `command_tx()`, `event_rx_mut()`, and `retained_resources()` helpers and routed in-process request/event/retained-resource reads through them. This keeps the same fields and ownership model while reducing direct facade field access in methods that dispatch requests, consume events, or expose retained managers.

**B33-B94 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `7bb888583aff9eeb319b1601b983931c8f4da71a`.

**UX impact:** no visual/runtime change. Request dispatch, notifications, server-request resolution/rejection, event polling, retained-manager exposure, and shutdown semantics remain unchanged. The value is extraction safety: facade internals now have named read seams before any deeper runtime-owner replacement work.

### B33-B96 request-handle command access seam

B33-B96 added `InProcessAppServerRequestHandle::command_tx()` and routed request-handle request dispatch through it. The request handle still owns the same cloned command sender, but direct field reads are now isolated behind a named seam.

**B33-B96 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `5bef4b75e60b3db3431228681ddb36ebeeef53d1`.

**UX impact:** no visual/runtime change. Detached request handles still send the same request command to the same in-process worker channel and receive the same response/error handling. The value is extraction safety: standalone request-handle dispatch now has the same command-sender access seam as the full client facade.

### B33-B98 outer request-handle variant access seam

B33-B98 added `AppServerRequestHandle::in_process_handle()` and routed outer request-handle `request(...)` and `request_typed(...)` calls through it. The enum still has the same in-process variant, but variant matching for borrowed request-handle dispatch now lives behind one named helper.

**B33-B98 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `2060e76441841a96a7123ef0004ec55f3c144931`.

**UX impact:** no visual/runtime change. The outer request handle still forwards to the same in-process request handle and preserves typed deserialization and transport/server error behavior. The value is extraction safety: the outer request-handle facade now has a named in-process variant seam.

### B33-B100 outer app-server client variant access seam

B33-B100 added `AppServerClient::in_process_client()`, `in_process_client_mut()`, and `into_in_process_client()`, then routed outer client request, notification, server-request response, event, shutdown, retained-resource, and external-agent detection dispatch through those helpers. The enum still has the same in-process variant, but borrowed, mutable, and consuming variant access now have named seams.

**B33-B100 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `9167c5ea932f02f1f4e373e37e76603c988eeed8`.

**UX impact:** no visual/runtime change. The outer app-server client still forwards to the same in-process client for requests, typed requests, notifications, server-request responses, event polling, retained managers, external-agent detection, and shutdown. The value is extraction safety: the outer client facade now has named variant seams for future runtime-owner replacement.

### B33-B102 in-process client close-parts bundle

B33-B102 introduced `InProcessClientShutdownParts` with `new(...)` and `into_parts(...)`, plus `InProcessAppServerClient::into_shutdown_parts()`, then routed consuming client close through that bundle before sending the worker close command. The same command sender, event receiver, and worker task handle are moved, but consuming field destructuring now has a named handoff seam.

**B33-B102 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `190G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `3a3a686112b7a19b8eace689ff61368c8bc8376f`.

**UX impact:** no visual/runtime change. Client close still drops the event receiver before asking the worker to close, still waits with the same timeout, and still aborts only on timeout. The value is extraction safety: consuming in-process client teardown now has a bundle seam instead of inline field destructuring.

### B33-B104 runtime bridge inner accessors

B33-B104 added `runtime_bridge::Handle::{new, inner, inner_mut, into_inner}` and `runtime_bridge::Sender::{new, inner}`, then routed bridge construction, external-agent detection, event polling, close, request, notification, and server-request response/failure through those helpers. The bridge still wraps the same app-server-owned handle and sender, but direct inner access now has named seams.

**B33-B104 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `192G` used / `22G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `051b5e5eeddd073c0cddb1350d875105064a4786`.

**UX impact:** no visual/runtime change. The runtime bridge still starts the same app-server-backed runtime, forwards the same request/notification/server-request operations, converts the same events, and closes with the same underlying handle. The value is extraction safety: the last app-server-owned handle/sender dereferences in the private bridge now pass through explicit access seams.

### B33-B106 runtime bridge start-args conversion helper

B33-B106 extracted `ServerBackedRuntimeOwner::start_args(...)` so conversion from TUI-owned `InProcessClientStartArgs` into the current app-server raw start shape is isolated from the async raw-start call. The same initialize params, thread-config loader, managers, warnings, session source, API-key flag, and channel capacity are passed through unchanged.

**B33-B106 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `194G` used / `20G` available / `91%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `e5c0b14f7de019efe8ad2b756e7eff153511fa2b`.

**UX impact:** no visual/runtime change. Startup still builds identical raw app-server start arguments before launching the same private bridge. The value is extraction safety: startup DTO conversion is now a named seam that can be replaced independently of runtime launch.

### B33-B108 runtime bridge retained-resources helper

B33-B108 extracted `ServerBackedRuntimeOwner::retained_resources(&inner)` so the construction of `RuntimeRetainedResources<Sender>` from the just-started app-server-backed `InProcessClientHandle` is isolated from the raw start sequence. The same thread manager, auth manager, thread store, environment manager, and request sender are passed through unchanged.

**B33-B108 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `194G` used / `20G` available / `91%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `5db647217d531229cc59ab2d6311d2acb62414bb`.

**UX impact:** no visual/runtime change. The runtime bridge still starts the same app-server-backed runtime and exposes the same retained handles (thread manager, auth manager, thread store, environment manager, request sender) to the runtime owner. The value is extraction safety: retained-resource assembly is now a named seam that can later route to a non-app-server backing without altering `start_raw`.

### B33-B110 runtime bridge raw start helper

B33-B110 extracted `ServerBackedRuntimeOwner::start_inner(args)` so the only raw `app_server_in_process::start(...)` call inside the runtime bridge is isolated behind a named helper. `start_raw` now calls `Self::start_inner(args).await?` before assembling retained resources and the `StartedRuntime` facade; the surrounding behavior, error propagation, and types are unchanged.

**B33-B110 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `191G` used / `23G` available / `90%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `d99b4924ee5d4dce4674c6b602dd259ee9b720ca`.

**UX impact:** no visual/runtime change. The runtime bridge still starts the same app-server-backed runtime via the same `app_server_in_process::start(...)` call; the value is extraction safety: the raw start call is now a single named seam that a future non-app-server backing can replace without touching `start_raw`'s assembly logic.

### B33-B112 runtime bridge event conversion seam

B33-B112 moved the free `event_into_facade(...)` helper inside `runtime_bridge` onto `Handle` as `Handle::event_into_facade(event)`. The `RuntimeSessionHandle::next_event` impl now maps `self.inner_mut().next_event().await` through `Handle::event_into_facade` instead of a sibling free function. Match arms, error propagation, and the resulting `AppServerEvent` are unchanged.

**B33-B112 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` passed after checking disk (`225G` total / `193G` used / `21G` available / `91%`). It staged only `vac-rs/tui/src/in_process_app_server_client.rs`; after push, local `main`, `vac/main`, and `origin/main` pointed at `908aac4e655ff081b4858a31a29fe4e103ea7bb5`.

### B33-B114 runtime bridge retained resources seam

B33-B114 memindahkan `retained_resources` dari `ServerBackedRuntimeOwner::retained_resources(&inner)` menjadi `Handle::retained_resources(&self)`. `start_raw` sekarang membungkus hasil `start_inner` ke `Handle::new(...)` lebih dulu, lalu memanggil `handle.retained_resources()` untuk merakit `RuntimeRetainedResources<Sender>`. Thread manager, auth manager, thread store, environment manager, dan request sender tetap dialirkan tanpa perubahan.

**B33-B114 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` lulus (47.62s, exit 0) setelah cek disk `225G total / 193G used / 21G available / 91%`. Staged dan committed hanya `vac-rs/tui/src/in_process_app_server_client.rs`; setelah push, `main`, `vac/main`, dan `origin/main` semua menunjuk `1626a509c10a05827579985df89cdd4cf1579549`.

**UX impact:** tidak ada perubahan runtime/visual. Bridge tetap memulai runtime app-server yang sama dan mengembalikan resource yang sama; perubahan ini hanya memindahkan titik perakitan ke `Handle` agar future non-app-server backing bisa override tanpa menyentuh `start_raw`.

### B33-B116 runtime bridge teardown handle seam

B33-B116 memindahkan body `RuntimeSessionHandle::shutdown(self)` untuk handle server-backed ke method `Handle::teardown_inner(self) -> IoResult<()>`. Implementasi trait sekarang hanya memanggil `self.teardown_inner().await`, sementara helper baru tetap meneruskan ke inner app-server handle yang sama melalui `self.into_inner().shutdown().await`. Tidak ada perubahan timeout, worker loop, atau fallback abort di facade luar.

**B33-B116 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` lulus (49.61s, exit 0) setelah cek disk `225G total / 193G used / 21G available / 91%`. Staged dan committed hanya `vac-rs/tui/src/in_process_app_server_client.rs`; setelah push, `main`, `vac/main`, dan `origin/main` semua menunjuk `1fd0c8c6e3106eb34d34aa05bd368bc5f8720125`.

**UX impact:** tidak ada perubahan runtime/visual. Close/teardown tetap memakai inner app-server handle, response channel, timeout, dan abort fallback yang sama; perubahan ini hanya memberi seam bernama di `Handle` agar future non-app-server backing bisa mengganti cara teardown tanpa menyentuh body trait worker.

### B33-B118 runtime bridge sender dispatch seam

B33-B118 mengganti akses langsung `Sender::inner()` di implementasi `RuntimeSessionSender` dengan helper `Sender::inner_ref()` plus dispatch methods: `dispatch_request(...)`, `dispatch_notification(...)`, `dispatch_server_request_response(...)`, dan `dispatch_server_request_failure(...)`. Request, notification, resolve, dan reject tetap diteruskan ke inner app-server sender yang sama; perubahan hanya memindahkan titik dispatch ke method bernama di wrapper `Sender`.

**B33-B118 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` lulus (53.53s, exit 0) setelah cek disk `225G total / 193G used / 21G available / 91%`. Staged dan committed hanya `vac-rs/tui/src/in_process_app_server_client.rs`; setelah push, `main`, `vac/main`, dan `origin/main` semua menunjuk `41693b68ecb7e5b13a1e8c17f4c34e2e3c628870`.

**UX impact:** tidak ada perubahan runtime/visual. Chat actions, notification fire-and-forget, approval resolve/reject, dan bottom-pane server request path tetap memakai sender yang sama; seam ini hanya membuat replacement future sender bisa override dispatch tanpa menyentuh body trait.

### B33-B120 runtime bridge external-agent detection seam

B33-B120 memindahkan body `RuntimeSessionHandle::external_agent_config_detect(&self, ...)` untuk handle server-backed ke method `Handle::external_agent_config_detect_inner(&self, ...)`. Implementasi trait sekarang hanya memanggil helper tersebut, sementara helper tetap meneruskan params ke inner app-server handle yang sama melalui `self.inner().external_agent_config_detect(params).await`. Tidak ada perubahan response shape, error propagation, atau caller-facing API.

**B33-B120 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` lulus (44.65s, exit 0) setelah cek disk `225G total / 193G used / 21G available / 91%`. Staged dan committed hanya `vac-rs/tui/src/in_process_app_server_client.rs`; setelah push, `main`, `vac/main`, dan `origin/main` semua menunjuk `eced2314dc1d0cfd837222365ff825434361a5d3`.

**UX impact:** tidak ada perubahan runtime/visual. Onboarding/agent attach flow tetap memakai external-agent detection app-server-backed yang sama; perubahan ini hanya memberi seam bernama di `Handle` agar future non-app-server backing bisa mengganti implementasi detection tanpa mengubah trait worker.

### B33-B122 runtime bridge event polling seam

B33-B122 memindahkan body `RuntimeSessionHandle::next_event(&mut self)` untuk handle server-backed ke method `Handle::next_event_inner(&mut self)`. Implementasi trait sekarang hanya memanggil helper tersebut, sementara helper tetap melakukan polling inner app-server handle yang sama melalui `self.inner_mut().next_event().await.map(Handle::event_into_facade)`. Event conversion tetap memakai `Handle::event_into_facade(...)`; tidak ada perubahan event shape, lag handling, atau caller-facing stream API.

**B33-B122 validation:** `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests` lulus (54.56s, exit 0) setelah cek disk `225G total / 193G used / 21G available / 91%`. Staged dan committed hanya `vac-rs/tui/src/in_process_app_server_client.rs`; setelah push, `main`, `vac/main`, dan `origin/main` semua menunjuk `530d3e39c73b5996383fe44dda4cd054a5b631b8`.

**UX impact:** tidak ada perubahan runtime/visual. Chat streaming, lag marker, approval prompt, server request, realtime/status event, dan terminal completion event tetap lewat stream dan conversion yang sama; perubahan ini hanya memberi seam bernama di `Handle` agar future non-app-server backing bisa mengganti cara polling event tanpa menyentuh body trait worker.

**UX impact:** no visual/runtime change. The TUI still receives the same `AppServerEvent` stream from the same in-process app server. The value is scoping: event-shape conversion is now owned by `Handle`, so a future non-app-server backing can supply its own `Handle::event_into_facade` (or a different mapper) without exposing a sibling free function.

### B33-B124 runtime bridge handle access recon

B33-B124 recon reviewed the remaining `runtime_bridge::Handle` and `Sender` helper surface after B33-B122. The only direct inner access left in `vac-rs/tui/src/in_process_app_server_client.rs` is already confined to private bridge-local wrapper methods, and any further split would only introduce another pass-through layer without shrinking app-server coupling or exposing a new owner seam.

**Decision:** stop the B33 micro-seam series here. The next safe implementation step is the runtime-first owner seam described below, not another accessor extraction.

**B33-B124 validation:** recon-only. No cargo gate was rerun because no source behavior changed.

The next safe code path is not to copy app-server internals. Continue by introducing or identifying a runtime-first owner seam that can supply the current bridge capabilities without exposing app-server-private `MessageProcessor` machinery:

- request / typed request dispatch
- request handles that can outlive a mutable session borrow
- local `AppServerEvent` stream
- server-request resolve/reject paths for approvals and bottom-pane requests
- retained managers/stores (`ThreadManager`, `AuthManager`, `ThreadStore`, `EnvironmentManager`)
- external-agent config detection
- graceful shutdown

**UX impact:** no visual/runtime change. The user-facing value is risk reduction: TUI callers now see only local facade events, exact grep output now identifies only active app-server seams, both active seams now carry source-level UX gates, raw runtime startup DTOs, handle/sender/event types, and thread-config-loader startup wiring are isolated inside the private bridge; retained managers/stores move as one startup bundle built at raw-start conversion time; and the existing chat streaming, approvals/server requests, thread lifecycle, realtime events, skills/plugins, onboarding, and bottom-pane request overload behavior remain preserved.

## What is the next concrete step after 00D-B30?

The natural sequence at HEAD `61f4bfc`:

1. **00D-B31 (proposed): map `vac-tui` consumption of `vac-app-server`.** Read `vac-rs/tui/src/**` for `vac_app_server::*` imports. Most likely DTO re-exports (config layer types, status enums, etc.). Decide whether to migrate those imports to `vac-core` or to a new shared DTO crate. This is the only edge still attaching `vac-tui` to the app-server tree.
2. **00D-B32 (proposed): drop `vac-app-server` from `vac-rs/tui/Cargo.toml`.** Once B31 lands, removing the dep entry should be a 1-line patch with no source edits. Cargo gates: `cargo check -p vac-surface-tui --tests`, `cargo test -p vac-surface-tui --lib`, `cargo check --workspace --tests`.
3. **00E-validation: run validation gates.** Execute `docs/validation/LOCAL_RUNTIME_GATE.md` and `docs/validation/TUI_PTY_DOGFOOD_GATE.md` command sequences and capture results.
4. **Operator decision on physical delete.** With the boundary facade split landed, the consumer code is now isolated, but `vac-app-server`, `vac-app-server-client`, `vac-app-server-transport`, and `vac-cloud-requirements` only become unreachable from `vac-cli` after the remaining Cargo edges are removed or explicitly deferred. Physical removal still requires (a) the operator guardrail lifted, and (b) non-product consumers migrated. Plan 19 + Plan 21 cover (b).

**Gate reminder:** Plan 01 (`.vac/` skeleton) may now proceed if the Phase 00 owner accepts the deferred 00E classification; the above sequence is the evidence trail that closes 00E as deferred rather than removing the crates.

## Legacy comment cleanup (informational)

`vac-rs/tui/src/legacy_core.rs` still has ~80 callsites across `vac-rs/tui/src/{lib.rs, main.rs, app/**, status/**, chatwidget/tests/**, debug_config.rs, resize_reflow_cap.rs}`. The module itself is now a pure `vac_core::*` re-export shim. Inlining its callsites is a low-risk cleanup but touches `vac-rs/tui/src/{lib.rs, main.rs}`, which are flagged as unrelated-dirty in the current session checkpoint, so the cleanup is **deferred to a future session** when those files are clean.

## Validation commands (re-run before each delete)

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 metadata --no-deps --format-version 1
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-transport --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-cloud-requirements --edges normal,build
```

## 00E current status (2026-05-15)

The TUI consumer surface now imports the local session boundary through
`vac-rs/tui/src/local_runtime_session.rs`, but the product still carries the
app-server-backed adapter underneath it. That means 00E is closed as a
deferred reachability/delete gate: the old runtime graph remains reachable,
but the deletion decision is explicitly deferred by policy rather than left
unresolved.

### Current evidence

- `cargo +1.93.0 metadata --no-deps --format-version 1` completed
  successfully against `vac-rs/`.
- `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli`
  is green.
- The PTY operator gate passed in a real terminal with `TERM=xterm-256color`.
- The current root TUI help affordance is `/`, which opens the slash-command
  list in this build. `/help` is not a valid command here, so the operator
  gate text has been updated to match the live binary.
- The PTY session showed visible task/activity output for a normal prompt and
  exited cleanly on `Ctrl-C`.

### Current classification

- `vac-app-server-client`: still reachable from `vac-cli` via `vac-tui`
  because the app-server-backed adapter remains in the product build.
- `vac-app-server`: still reachable from `vac-cli` via `vac-tui`.
- `vac-app-server-transport`: reachable via `vac-app-server`.
- `vac-cloud-requirements`: not present in the current workspace tree, so it
  is not part of the active 00E reachability gate.
- `vac-app-server-protocol`: deferred shared DTO crate.
- `vac-api`: deferred structural.
- `vac-exec-server`: deferred structural.
- `vac-backend-client`: deferred.

00E is closed for Phase 00 as a defer classification. The graph remains
reachable, but the remaining runtime debt is now explicitly deferred rather
than left as an open gate.

---

## Historical snapshot — 2026-05-06 (HEAD `35aaa5d`)

Preserved for diff context. The pre-condition checkboxes below were the open questions when 00D had not yet started rewiring TUI.

**Date:** 2026-05-06
**Repo HEAD:** `35aaa5d` on `main`
**Audit scope:** dependency graph for `vac-cli` workspace member at HEAD.

### Per-crate result (2026-05-06)

- `vac-app-server-client` — direct callers from `vac-cli`: `vac-exec`, `vac-tui`. Classification after 00C+00D: unreachable from `vac-cli` → **DELETE**.
- `vac-app-server` — indirect-only from `vac-cli` via `vac-app-server-client`. Classification: unreachable after 00C+00D → **DELETE**.
- `vac-app-server-transport` — direct callers: `vac-app-server`, `vac-otel`, `vac-login`, `vac-model-provider`. Classification: **DELETE** with the same gate as `vac-app-server`.
- `vac-cloud-requirements` — direct callers from `vac-cli`: `vac-app-server`, `vac-exec`, `vac-tui`. Classification: **DELETE-CANDIDATE**.
- `vac-app-server-protocol` — ~30 DTO consumers workspace-wide. Classification: **DEFERRED** (shared DTO).
- `vac-api` — model/provider transport. Classification: **DEFERRED structural**.
- `vac-exec-server` — FS/executor types. Classification: **DEFERRED structural**.
- `vac-backend-client` — backend memories / cloud auth. Classification: **DEFERRED**.

### Pre-condition checklist (2026-05-06)

- [ ] `vac-rs/exec/src/lib.rs` no longer imports `vac_app_server_client::*` or `vac_app_server_protocol::*` (currently imports both at lines 71–109).
- [ ] `vac-rs/exec/Cargo.toml` no longer lists `vac-app-server-client`, `vac-app-server-protocol`, `vac-cloud-requirements` (currently listed at lines 27–29).
- [ ] `vac-rs/tui/src/lib.rs` no longer imports `vac_app_server_client::*` (currently imports at lines 38–43, 76).
- [ ] `vac-rs/tui/Cargo.toml` no longer lists `vac-app-server-client`, `vac-app-server-protocol`, `vac-cloud-requirements`, `vac-exec-server` (currently listed at lines 29–40).
- [ ] PTY operator gate (`docs/validation/TUI_PTY_DOGFOOD_GATE.md`) passes after 00C and 00D rewires.
- [ ] `cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build` returns no callers.

All items above except the PTY gate, `vac-exec-server` dep on TUI, and the live `cargo tree` re-run have been cleared as of the 2026-05-11 snapshot above.
