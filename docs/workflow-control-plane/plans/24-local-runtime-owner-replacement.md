# Plan 24 — Local runtime owner replacement


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> ## Current status
> Status: **complete as the umbrella closeout for Plans 25–33 default local runtime owner replacement**.

Code evidence:
- `vac-rs/core/src/local_runtime/mod.rs`
- `/`

Evidence docs:
- `docs/workflow-control-plane/plans/24-evidence/2026-05-28-sandbox-umbrella-closeout.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Goal

Replace the app-server-backed TUI runtime adapter with a first-class local
runtime owner while preserving one product command, one product TUI, and the
visible operator UX.

This is the post-00F / post-00E continuation:

- 00F finished the TUI consumer migration to `LocalRuntimeSession`.
- 00E closed the old delete gate as deferred because `vac-app-server*` crates
  are still reachable from `vac-cli` through `vac-tui`.
- Plan 24 turns that defer into an executable replacement plan with codebase
  evidence, hard stop conditions, and deletion/defer proof.

The purpose of this plan is not to rename app-server concepts. The purpose is
to move live local runtime ownership out of app-server-shaped transport/RPC
contracts and into a local product owner that can be validated and eventually
used to remove the app-server Cargo edges from the default local path.

## Current status

Status: **complete as the umbrella closeout for Plans 25–33 default local runtime owner replacement**.

This plan remains the architecture and safety contract for the replacement line. The child plans now provide the default-path implementation and evidence: semantic contract hardening, owner skeleton, retained-resource startup, server-request registry, event stream, prompt/control cutover, protocol compatibility retirement, runtime-owner gates, and default Cargo retirement/delete-defer proof.


## Current implementation slice

- Plans 25–33 are now closed for the default local product path.
- `vac-local-runtime-owner` owns retained-resource startup, server-request registry, owner event stream, prompt/control command surfaces, external-agent/plugin/skills owner surfaces, and shutdown handles.
- `vac-tui` default Cargo features no longer pull direct app-server protocol/server crates.
- Runtime-owner gates now fail hard for default-path app-server dependency/import regressions and false-green PTY/runtime defer states.
- Remaining app-server material is explicit non-default compatibility/delete-defer evidence governed by Plan 33, not active default product runtime ownership.

Evidence: `24-evidence/2026-05-28-sandbox-umbrella-closeout.md`.

Historical inventory tables below are retained as migration evidence. Any row describing a temporary app-server-backed adapter, missing owner crate, or Plan 30 partial state is superseded by the 2026-05-28 Plan 24/30/31/33 closeout unless the row is explicitly marked as current.


## 2026-05-28 sandbox umbrella closeout

Plan 24 is now closed for the default local product path. The executable replacement work is distributed across Plans 25–34 and recorded in [24-evidence/2026-05-28-sandbox-umbrella-closeout.md](24-evidence/2026-05-28-sandbox-umbrella-closeout.md).

The only remaining boundary is explicit Plan 33 defer material: physical deletion of all historical `vac-app-server*` workspace crates is operator-gated and outside the default product path. This is not a Plan 24 blocker.

## Target outcome

The default local product path uses a non-app-server local runtime owner. TUI no longer depends on app-server transport/protocol ownership on the default path, while one product command, one TUI, approvals, event streaming, operator UX, and shutdown behavior remain stable.

## Outputs

Expected final outputs across child plans:

- hardened `vac_core::local_runtime` semantic contract evidence,
- local runtime owner crate/module,
- retained-resource startup path,
- server-request registry,
- local event stream,
- prompt/control cutover,
- protocol compatibility retirement,
- `.vac` runtime-owner gates,
- app-server Cargo delete/defer evidence.

## Requires / Blocks

Requires:

- completed 00F TUI consumer migration,
- 00E defer evidence showing app-server crates remain reachable,
- Plan 25 semantic contract audit before lifecycle implementation,
- Plan 23 PTY gate contract for any child plan that claims root TUI operator
  evidence or release-grade TUI behavior.

Blocks:

- Plan 33 app-server Cargo retirement,
- any claim that the local product path is app-server-free,
- any Plan 30 / Plan 33 false-green that reports TUI startup, slash-command,
  approval, or Ctrl-C behavior as passed without Plan 23-compatible PTY
  evidence or an explicit `BLOCKED-OPERATOR` record.

## Deep-dive baseline — 2026-05-23

This revision is mapped against the current codebase, not just the desired
architecture.

### Current local product path

```text
vac-cli
  -> vac-tui
      -> LocalRuntimeSession trait
          -> owner-native local runtime/session contracts
              -> vac-local-runtime-owner
                  -> vac-core::local_runtime semantic contract
```

Historical 00E reachability evidence is retained only as migration context. The current default-path evidence is Plan 31/33 closeout: app-server workspace crates may remain as non-default compatibility/delete-defer material, but they are not the default local product runtime owner.

### Exact active seams

| File/crate | Current role | Replacement implication |
| --- | --- | --- |
| `vac-rs/tui/src/local_runtime_session.rs` | TUI-local product boundary trait. Historical versions imported compatibility DTOs; the default closeout now treats this as an owner-native consumer seam. | Keep this as the TUI consumer seam; any remaining compatibility adapter must stay non-default and Plan 33-classified. |
| `vac-rs/tui/src/app_server_session.rs` | Historical adapter surface. The default product path now uses owner-native runtime/session contracts; legacy compatibility material is non-default quarantine. | Preserve only explicitly classified compatibility behavior; never reintroduce default app-server runtime ownership. |
| `vac-rs/tui/src/session_protocol.rs` | Owner-native session protocol facade with legacy compatibility classified outside the default path. | Treat compatibility as quarantine. Do not add new app-server DTO dependencies here. Retire only with exact type/conversion tests. |
| `vac-rs/app-server-client/src/lib.rs` | Higher-level in-process app-server client facade for CLI/TUI surfaces. | Use as behavior reference for queueing, typed requests, event delivery, dropped request handling, and shutdown; not as final dependency. |
| `vac-rs/app-server/src/in_process.rs` | In-process app-server runtime host around `MessageProcessor`; transport-local but explicitly not protocol-free. | Do not copy `MessageProcessor`. Extract equivalent local-owner behavior through core/runtime APIs. |
| `vac-rs/core/src/local_runtime/*` | `vac_core::local_runtime` semantic contract: product-level `RuntimeCommand`, `RuntimeEvent`, sessions, tasks, approvals, projection bridge. | Use as the semantic owner model; lifecycle ownership is provided by `vac-local-runtime-owner` and TUI owner-native seams. |
| `vac-rs/tui/Cargo.toml` | Default feature path is app-server-retired; optional legacy compatibility must remain explicit and non-default. | Keep Plan 33 delete/defer proof current before any physical workspace crate deletion. |
| `vac-rs/Cargo.toml` | Workspace lists app-server crates; no `vac-local-runtime-owner` crate yet. | Creating a new crate requires an explicit workspace edit and validation. |

### Important correction from deep dive

`vac_core::local_runtime` already exists and provides semantic contract types,
including:

- `RuntimeCommand`, `StartTask`, `StartReview`, `WorkflowInput`
- `RuntimeEvent` and task/session lifecycle events
- approval DTOs and validation DTOs
- `LocalRuntimeBridge` / `BridgeOutput` projection from `vac_protocol::Event`

Historical 2026-05-24 state: the semantic contract was not yet a replacement for `AppServerSession`. Current closeout state: Plans 26–33 added the lifecycle owner, retained resources, request registry, event stream, prompt/control parity, DTO ownership, and default Cargo retirement needed for the default product path.

So Plan 24 must distinguish three layers:

```text
Semantic contract       = vac_core::local_runtime
Lifecycle owner         = new owner crate/module to be built
TUI compatibility seam  = LocalRuntimeSession + non-default legacy compatibility quarantine
```

## Target architecture

```text
vac-cli
  -> vac-tui
      -> LocalRuntimeSession trait
          -> LocalRuntimeOwnerSession
              -> local runtime owner lifecycle crate/module
                  -> vac_core::local_runtime semantic contract
                  -> vac-core ThreadManager / VACThread
                  -> vac-config / vac-thread-store / vac-login / vac-state
                  -> vac-model-provider / vac-core-skills / vac-core-plugins
                  -> vac-exec-server while structurally required
```

The target removes `vac-app-server*` from the default local product path only
after the replacement owner covers the full runtime lifecycle and the inverse
Cargo trees are clean.

## `.vac` control-plane pattern

Keep the `.vac` pattern, but keep the responsibility split strict:

- `.vac` declares ownership, capability metadata, migration gates, policy
  guardrails, validation commands, and UX invariants.
- Rust owns live runtime behavior: session startup, command dispatch, event
  streaming, pending request resolution, retained resources, and shutdown.

Do not put runtime business logic into `.vac/workflows/**`. Workflows can prove
and gate the runtime owner; they must not become the runtime owner.

Expected `.vac` additions are later implementation slices only, after schema and
registry tests are ready:

```text
.vac/capabilities/local_runtime_owner.yaml
.vac/capabilities/tui_session_runtime.yaml
.vac/capabilities/runtime_approval_bridge.yaml
.vac/workflows/maintenance.no-app-server-local-path.yaml
.vac/workflows/maintenance.runtime-owner-gate.yaml
.vac/policies/runtime-owner-replacement.yaml
```

Schema validation reference: runtime-owner manifests must stay compatible with
the existing control-plane schema contracts from Plans 02–07, especially
capability, workflow, policy, registry-loader, and registry-diagnostics
validation. Plan 32 is the first slice that may introduce these `.vac` runtime
owner gates; it must run manifest/schema validation before any gate can be
claimed ready.

## Non-goals and hard stops

- No second TUI.
- No second product command.
- No direct copy of app-server-private `MessageProcessor` internals into TUI or
  into a new owner crate.
- No direct TUI dependency on `vac-app-server-protocol` as a final fix.
- No name-only DTO migration.
- No Cargo edge removal before inverse-tree evidence is clean.
- No PTY pass claim without a real TTY; record `BLOCKED-OPERATOR` when the tool
  environment cannot drive the operator gate.
- No broad reset, broad clean, prune, or unrelated dirty-work cleanup.
- No broad `.vac/**`, `vac-rs/core/src/control_plane/**`, or `vac-rs/cli/src/**`
  edits unless the active slice explicitly calls for them.
- No heavy Cargo/build/test spawn if another Cargo/rustc job is holding locks;
  wait/tail the existing job instead.

## Behavior that must be preserved

### From `vac-rs/app-server-client/src/lib.rs`

The replacement owner must preserve these semantics before the app-server-client
edge can be removed:

- typed request/response dispatch with explicit typed errors,
- bounded worker queue behavior,
- lossless delivery for transcript-critical notifications,
- lag/backpressure markers for best-effort notifications,
- dropped `ServerRequest` must be rejected/fail back into runtime instead of
  hanging approval flows,
- `ChatgptAuthTokensRefresh` remains unsupported for in-process/local clients
  unless a deliberate new owner decision changes that with tests,
- bounded shutdown with a 5-second timeout and abort fallback,
- tests equivalent to typed roundtrip/error, startup source, visible threads,
  tiny-capacity behavior, lossless transcript under backpressure, lag markers,
  event delivery classification, retained environment manager, and shutdown
  promptness.

### From `vac-rs/app-server/src/in_process.rs`

The current in-process app-server host has critical behavior but is explicitly
transport-local, not protocol-free. Preserve the behavior, not the ownership:

- startup builds `AuthManager`, `ThreadManager`, `ThreadStore`, and
  `EnvironmentManager`-backed runtime managers,
- `initialize` / `initialized` handshake is internal to that host today,
- duplicate request IDs return `INVALID_REQUEST`,
- command queue saturation returns overload errors,
- server-request queue saturation fails the request back to the runtime,
- shutdown cancels outstanding requests and drains/aborts bounded tasks.

Do not copy `MessageProcessor`. The new owner should call lower-level core APIs
or extract new lower-level APIs where required.

### From `vac-rs/tui/src/app_server_session.rs`

`AppServerSession` already contains direct local paths that are useful stepping
stones:

- retained `ThreadManager`, `AuthManager`, `ThreadStore`, `EnvironmentManager`,
- direct thread start/resume/branch/read/list/loaded-list paths where possible,
- direct `turn_start`, `turn_interrupt`, `turn_steer`, compact, shell command,
  guardian approval, background-terminal cleanup, review, realtime, skills list,
  memory reset, logout, config reload paths where available,
- direct `VACThread` event listener converting `vac_protocol::Event` to
  TUI-visible `TuiSessionEvent`,
- `DirectPendingServerRequest` registry for exec approval, patch approval,
  user-input answer, permissions response, and MCP elicitation.

Plan 24 should harvest and harden these local paths into a real owner instead
of keeping them buried inside an app-server-named adapter.

## Runtime owner capability map

| Capability | Current source | Replacement requirement | UX invariant |
| --- | --- | --- | --- |
| Startup/bootstrap | `AppServerBootstrap`, `InProcessClientStartArgs`, app-server initialize path | Local owner starts retained managers and returns TUI bootstrap data without app-server protocol ownership | `vac` opens same TUI |
| Retained resources | `AppServerSession::new`, app-server in-process retained managers | Owner explicitly owns/provides thread manager, auth manager, thread store, environment manager | thread list/read/auth/filesystem behavior stable |
| Command dispatch | `ClientRequest` JSON-RPC fallback plus direct methods | Typed local command/request API, using `vac_core::local_runtime` where suitable | chat actions and panels keep working |
| Event stream | `TuiSessionEvent`, app-server notifications, direct VACThread event listener | Lossless local event stream with TUI-compatible adapter during migration | streaming/reasoning/tools/progress visible |
| Pending requests | `DirectPendingServerRequest`, app-server server request forwarding | First-class registry with resolve/reject/cancel/shutdown semantics | approval overlays and bottom-pane prompts never hang |
| Skills/plugins | Owner-native operation parity registry covers the default path; historical arbitrary-cwd fallback notes are superseded by Plan 30 closeout | Local owner/default parity remains the source of truth; legacy compatibility is non-default quarantine | `/skills` and UI skill metadata stable |
| Config/account | `AuthManager`, `ConfigBatchWrite`, app-server config manager fallback | Extract local config/account APIs or explicitly gate unsupported fallback | login/status/reload stay stable |
| Realtime | direct `Op::RealtimeConversation*` plus fallback | Preserve feature gate and audio lifecycle | realtime UI cannot regress silently |
| External-agent config | app-server external-agent API/direct API | Isolate lower-level provider before app-server edge removal | onboarding/attach stable |
| Shutdown | app-server-client/in-process bounded shutdown plus TUI listener aborts | bounded local-owner shutdown with pending cancellation | clean Ctrl-C/exit |
| Protocol compatibility | `session_protocol.rs` app-server DTO quarantine | Exact local owner DTOs or explicit conversion tests | no hidden dependency comeback |

## Proposed owner placement

Preferred: add a dedicated lifecycle crate when the slice is ready.

```text
vac-rs/local-runtime-owner/
  src/lib.rs
  src/session.rs
  src/startup.rs
  src/command_bus.rs
  src/event_stream.rs
  src/server_requests.rs
  src/retained_resources.rs
  src/shutdown.rs
```

Package name:

```text
vac-local-runtime-owner
```

Allowed likely dependencies:

- `vac-core`
- `vac-config`
- `vac-thread-store`
- `vac-login`
- `vac-state`
- `vac-protocol`
- `vac-model-provider`
- `vac-core-skills`
- `vac-core-plugins`
- `vac-exec-server` while structurally required

Rejected dependencies for the final owner:

- `vac-app-server`
- `vac-app-server-client`
- `vac-app-server-protocol`
- `vac-app-server-transport`

Historical note: at Plan 24 start this crate did not exist yet. Current closeout state: `vac-rs/local-runtime-owner/` exists and is the owner crate for the default path.
to `vac-rs/Cargo.toml`, which is intentionally treated as a guarded slice. Do
not add this crate casually while other migration lanes are dirty.

Allowed alternative: put lifecycle owner code under an existing crate only if it
keeps dependencies controlled and does not pull TUI/app-server concerns into
`vac-core::local_runtime`. The semantic contract already in `vac-core` should
not become a dumping ground for UI lifecycle plumbing.

## Core API direction

The TUI-facing boundary remains `LocalRuntimeSession` during migration. The new
owner implements it through `LocalRuntimeOwnerSession`, while gradually reducing
app-server-shaped DTOs.

```rust
pub struct LocalRuntimeOwnerSession {
    retained: RuntimeRetainedResources,
    command_tx: RuntimeCommandSender,
    event_rx: RuntimeEventReceiver,
    pending_requests: ServerRequestRegistry,
    shutdown: RuntimeShutdownHandle,
}
```

Use the existing `vac_core::local_runtime::RuntimeCommand` and `RuntimeEvent`
where semantics match. Add lifecycle-specific owner commands only when the core
semantic contract is too high-level or exec-focused.

The exact API must be introduced incrementally with tests; this sketch is a
shape contract, not permission for a bulk rewrite.

## Agent execution protocol

Every implementation slice must follow this order:

1. Inspect exact files and current git state.
2. Identify intended files only.
3. Edit only those files.
4. Validate the smallest relevant matrix.
5. Stage only intended files/hunks.
6. Commit.
7. Push to `origin main`.
8. Record any blocked operator gate as `BLOCKED-OPERATOR` instead of pass.

Before heavy Rust validation, check disk and active Cargo/rustc state. If a
build is already waiting on a file lock, do not start another build; wait/tail
that job.

## Slice → child plan mapping (2026-05-24)

Plan 24 slices 24A–24N are not executed as a single bulk implementation. Execution lives in child Plans 25–33. This table is the authoritative mapping. Future agents must update both this table and the listed child plan when a slice is reshaped.

Child plan sequencing is mostly serial. Plans 25–31 must run in order because
each plan changes the runtime owner seam used by the next plan. Plan 32 may run
in parallel only after the relevant gate target exists and schema validation is
green; it must not invent runtime behavior in `.vac`. Plan 33 is strictly last:
do not start Cargo edge removal until Plans 25–32 have either green evidence or
explicit blockers/defer records.

| Umbrella slice | Owned by | Notes |
| --- | --- | --- |
| 24A — Evidence hardening (docs) | This file | Deep-dive baseline stamped 2026-05-23 |
| 24B — Semantic-contract audit | Plan 25 | Direct map |
| 24C — Owner skeleton gate | Plan 26 | Direct map |
| 24D — App-server behavior characterization | Absorbed into Plan 27 / Plan 28 / Plan 29 as test-prelude | See gap notes below |
| 24E — Runtime owner starter seam | Absorbed into Plan 26 + Plan 27 and superseded by Plan 31 default owner-native closeout | Closed for default path |
| 24F — Retained-resource startup | Plan 27 | Direct map |
| 24G — Read-only command bus de-risking | Absorbed into Plan 30 (30A read-only-first sub-step) | Implemented in 30A; see Plan 30 evidence record |
| 24H — Server-request registry hard gate | Plan 28 | Direct map |
| 24I — Event stream replacement | Plan 29 | Direct map |
| 24J — Prompt submit cutover | Plan 30 | Complete for the default owner-native TUI product path after Plan 30 parity closeout |
| 24K — Active control paths | Plan 30 (30B–30G) | 30B–30E implemented/user-confirmed; 30F–30G pending; 30G added for skills/plugins + external-agent config |
| 24L — Protocol compatibility retirement | Plan 31 | Direct map |
| 24M — `.vac` runtime-owner gates | Plan 32 | Direct map |
| 24N — Cargo edge removal / delete-defer proof | Plan 33 | Direct map |

### Slice size estimates (planning guardrail)

These are rough implementation-size estimates, not approval to batch multiple
child plans together. Split further if a slice crosses its estimate or touches
unexpected dependency surfaces.

| Umbrella slice | Expected size | Sequencing note |
| --- | --- | --- |
| 24A — Evidence hardening (docs) | XS / docs-only | Complete in this umbrella before child-plan execution |
| 24B — Semantic-contract audit | S | Plan 25, first executable child |
| 24C — Owner skeleton gate | S–M | Plan 26, after 24B |
| 24D — Behavior characterization | M | Test-prelude work before affected cutovers |
| 24E — Starter seam | S | Plan 26/27 bridge; temporary only |
| 24F — Retained-resource startup | M–L | Plan 27, after skeleton seam |
| 24G — Read-only command bus | S–M | Plan 30 prelude before prompt submit |
| 24H — Server-request registry | L | Plan 28 hard gate before approval cutover |
| 24I — Event stream replacement | L | Plan 29 after registry invariants are known |
| 24J — Prompt submit cutover | M | Plan 30 after read-only bus + event stream |
| 24K — Active control paths | L | Plan 30, cannot be skipped as chat-only green |
| 24L — Protocol compatibility retirement | M | Plan 31 after controls stabilize |
| 24M — `.vac` runtime-owner gates | S–M | Plan 32 after schema validation and gate targets exist |
| 24N — Cargo edge removal / delete-defer proof | M | Plan 33, strictly last |

### Gap notes (2026-05-24)

These three umbrella slices do not have a dedicated child plan and live as absorbed sub-steps. They must not be silently skipped.

- **24D — behavior characterization.** Typed request/response, duplicate request ID rejection, bounded worker queue overload, lossless transcript delivery classification, lag/backpressure markers, retained manager visibility, bounded 5s shutdown, and the `app_server_session.rs` direct-paths harvest list must be locked by characterization tests before the corresponding cutover slice runs:
  - typed dispatch + bounded queue + 5s shutdown promptness → tests added in Plan 27 prelude or Plan 28 prelude before extraction.
  - duplicate ID + queue saturation + fail-back semantics → tests added in Plan 28 (28A registry invariants).
  - lossless/backpressure/lag markers → tests added in Plan 29 (29D backpressure behavior).
  - retained-manager visibility → tests added in Plan 27 (27B retained-resource builder).
  - If a behavior cannot be locked in any child plan, raise it as an explicit blocker in this umbrella before the cutover slice (Plan 30) starts.
- **24E — starter seam.** Historical compile-only/delegating seam is retired for the default path by Plan 31. Remaining app-server compatibility is non-default Plan 33 delete/defer material.
- **24G — read-only command bus de-risk.** Before prompt submit moves to the owner path, one low-risk read-only command (thread list/read or account read) must route through the owner command bus. This lives in Plan 30 (30A read-only-first sub-step). Skipping it makes Plan 30 a chat-only false-green.

### Capability map gaps (2026-05-24)

Three capabilities from the "Runtime owner capability map" table do not have a dedicated child plan slice. Required ownership:

- **Skills/plugins surface.** Owned by Plan 30. The default path is covered by the owner-native operation parity registry; any remaining remote/legacy compatibility must be explicitly non-default.
- **External-agent config.** Owned by Plan 30 (new slice 30G). Lower-level provider must be isolated before the app-server edge can be removed (Plan 33 prerequisite).
- **Shutdown semantics.** Distributed by design across Plan 26 (skeleton `RuntimeShutdownHandle`), Plan 28 (28E backpressure/shutdown registry tests), Plan 30 (30B–30F operator UX shutdown), and Plan 33 (Ctrl-C verification under inverse-tree-clean state). No single child plan owns it; the umbrella does. Any child plan that touches shutdown must update this list.

## Implementation slices

> Execution note (2026-05-24): These slices are not executed in this file. They are mapped to Plan 25–33 in the "Slice → child plan mapping" section above. The slice text below remains the authoritative architecture spec for the corresponding child plan.

### 24A — Evidence hardening and owner contract docs

- Keep this docs-only.
- Record the current code map, exact seams, behavior preservation list, stop
  conditions, and validation gates.
- Do not change source behavior.

Validation:

```bash
git diff --check -- docs/workflow-control-plane/plans/24-local-runtime-owner-replacement.md
```

### 24B — Local runtime semantic-contract audit

- Audit `vac-rs/core/src/local_runtime/*` against TUI needs.
- Add tests/docs only if needed to clarify what `RuntimeCommand`/`RuntimeEvent`
  already cover and what lifecycle gaps remain.
- Do not move TUI runtime lifecycle code into `vac-core` unless dependency review
  proves it is safe.

Validation:

```bash
cd vac-rs
df -h . /tmp
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
```

### 24C — Owner crate/module skeleton gate

- Decide final placement: `vac-local-runtime-owner` crate or a constrained
  existing-crate module.
- If using a new crate, add it to `vac-rs/Cargo.toml` in this slice only.
- Define empty/non-invasive modules for startup, command bus, events,
  server-request registry, retained resources, and shutdown.
- No TUI behavior change.
- No app-server Cargo edge removal.

Validation:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-local-runtime-owner
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

If no new crate is created, replace the first command with the package that owns
the selected module.

### 24D — App-server behavior characterization tests

Before moving code, add/confirm tests that lock the behavior to preserve:

- typed request roundtrip and typed error,
- duplicate request ID rejection,
- overload on request queue saturation,
- server-request saturation fails back instead of hanging,
- lossless terminal/transcript delivery classification,
- lag marker/backpressure behavior,
- retained manager visibility,
- bounded shutdown and pending request cancellation.

This slice may add tests around `app-server-client` / `app-server` as reference
coverage. It must not broaden TUI dependencies.

Validation:

```bash
cd vac-rs
df -h . /tmp
cargo nextest run --manifest-path Cargo.toml -p vac-app-server-client --lib --no-tests=pass
cargo nextest run --manifest-path Cargo.toml -p vac-app-server --lib in_process --no-tests=pass
```

### 24E — Runtime owner starter seam

- Introduce `RuntimeOwner` / `RuntimeStarter` names outside app-server modules.
- Historical starter implementation allowed delegation; default-path delegation is retired by Plan 31/33 closeout.
- Compile-check the ownership seam before moving behavior.
- No Cargo edge removal.

Validation:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
```

### 24F — Retained-resource startup extraction

- Move construction/ownership of retained resources toward the local owner:
  - `ThreadManager`,
  - `AuthManager`,
  - `ThreadStore`,
  - `EnvironmentManager`,
  - request sender abstraction.
- Preserve current bootstrap fields and model/account behavior.
- Keep only explicitly classified non-default compatibility; default local APIs are the active path.
- Do not copy `MessageProcessor`.

Validation:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
```

### 24G — Read-only command bus cutover

- Add typed local owner request/response path.
- Route one low-risk read-only command first, such as thread list/read or account
  read.
- Keep prompt submit and approvals on the old path until server-request registry
  is ready.

Validation:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
```

### 24H — Server-request registry hard gate

- Implement first-class pending request registry.
- Cover exec approval, patch approval, tool user-input, permissions request, and
  MCP elicitation.
- Prove resolve, reject, duplicate/unknown IDs, cancel, and shutdown behavior.
- Approval and bottom-pane input must be first-class, not best-effort.

Required tests:

- approval request emits TUI-visible event,
- resolve continues the pending operation,
- reject returns a safe runtime decision/error,
- shutdown cancels pending requests,
- duplicate/unknown request IDs are safe,
- event queue saturation cannot leave a pending approval hanging.

Validation:

```bash
cd vac-rs
df -h . /tmp
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
```

### 24I — Event stream replacement

- Add lossless local event stream mapping.
- Reuse or extend `vac_core::local_runtime::LocalRuntimeBridge` where semantics
  match.
- Convert `vac_protocol::Event` to TUI-visible events without app-server DTO
  ownership.
- Preserve reasoning deltas, tool events, progress/validation, turn completion,
  error handling, lag/backpressure, and stale-turn filtering.

Validation must cover:

- assistant deltas,
- reasoning deltas,
- tool start/finish,
- approval events,
- validation progress,
- turn completion/failure,
- lag markers/backpressure,
- event ordering for terminal notifications.

### 24J — Prompt submit / `turn_start` cutover

- Prompt submit has moved to the owner-native default path; keep legacy compatibility non-default.
- Preserve bottom-input UX.
- Keep approval/input bridge active.
- Keep validation/progress events visible.

Validation:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
```

PTY gate:

```text
launch ./target/debug/vac
type /
verify slash-command list visible
submit a normal prompt
verify streaming/activity/progress visible
exercise approval/input if available
press Ctrl-C
verify clean exit
```

If no real TTY is available, record `BLOCKED-OPERATOR`.

### 24K — Active control paths

Migrate the remaining runtime controls so this does not become chat-only green:

- turn steer,
- turn interrupt,
- startup interrupt,
- realtime start/audio/stop,
- shell command,
- background terminal cleanup,
- compact,
- rollback or explicit defer if rollback still lacks a local equivalent,
- review start,
- thread goal set/get/clear,
- memory mode set,
- config reload,
- logout/memory reset.

Every unsupported path must be documented as an explicit remaining blocker, not
silently routed back through app-server in the final state.

### 24L — Protocol compatibility retirement

- Replace remaining `session_protocol.rs` app-server DTOs with exact owner types
  or deliberate local conversion DTOs.
- No name-only migration.
- No direct TUI dependency on `vac-app-server-protocol` as the final state.

Validation:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
git grep -n "vac_app_server_protocol\|vac_app_server_client\|vac_app_server" -- tui/src || true
```

Any remaining matches must be historical comments, deleted files, or explicitly
non-default/deferred paths documented before Cargo edge retirement.

### 24M — `.vac` runtime-owner gates

Introduce declarative gates only after code exists to validate:

- `vac.local_runtime_owner` capability manifest,
- `maintenance.runtime-owner-gate` workflow,
- `maintenance.no-app-server-local-path` workflow,
- `runtime-owner-replacement` policy.

Schema-validation references:

- capability manifest schema from Plan 02,
- workflow manifest schema from Plan 03,
- policy manifest schema from Plan 04,
- registry loader/diagnostics from Plans 06–07,
- PTY gate result semantics from Plan 23 when a runtime-owner gate consumes
  operator evidence.

Expected gate semantics:

- inverse tree must be clean before deletion is green,
- TUI imports must not reference `vac_app_server*`,
- PTY gate cannot pass without real TTY,
- app-server-private internals cannot be copied into TUI,
- unsupported active controls must be explicit blockers or explicit non-default
  defers.

### 24N — Cargo edge removal and delete/defer proof

Only after 24B-24M are green:

- remove `vac-app-server`, `vac-app-server-client`, and
  `vac-app-server-protocol` from `vac-rs/tui/Cargo.toml`,
- rerun inverse trees,
- update 00E/Phase closeout docs with new evidence,
- delete app-server crates only if workspace-wide consumers are gone,
- otherwise classify them as explicit non-default/deferred capability crates.

Validation:

```bash
cd vac-rs
df -h . /tmp
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass
cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-transport --edges normal,build
```

Expected inverse-tree result: no `vac-cli` path, or only an explicitly
non-default future capability path documented in `.vac` and closeout docs.

## UX invariants

Every slice must preserve:

- one `vac` product command,
- one product TUI,
- bottom input behavior,
- slash-command affordance,
- chat streaming,
- reasoning/tool/progress visibility,
- approval overlay round-trip,
- bottom-pane user-input prompt round-trip,
- validation progress/result visibility,
- resume/thread lifecycle,
- thread list/read/rename/goal surfaces,
- external-agent onboarding/attach flow,
- realtime controls where enabled,
- clean Ctrl-C / shutdown behavior.

## Risk register

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Chat-only false-green | Prompt works but approvals/input/shutdown/control paths break | Server-request registry and active-control slices are hard gates |
| DTO name-only swap | Same names can hide different wire/semantic types | Require exact-owner proof or explicit conversion tests |
| App-server private copy | Recreates app-server under a new name | Extract lower-level owner APIs; never copy `MessageProcessor` |
| `vac_core::local_runtime` overreach | Semantic contract becomes lifecycle/UI dumping ground | Keep lifecycle owner separate unless dependency review proves safe |
| `.vac` overreach | Workflows become runtime logic | `.vac` declares/gates; Rust executes lifecycle |
| Cargo edge removal too early | Compile/runtime break hidden as cleanup | Inverse-tree and TUI tests before removal |
| Backpressure approval hang | Saturated event queue can stall operator approval forever | Dropped server requests must reject/fail back into runtime |
| Shutdown leaks | Background tasks survive exit | Bounded shutdown, pending cancellation, listener abort tests |
| PTY unavailable | Operator path cannot be proven in CI/tool env | Record `BLOCKED-OPERATOR`, never pass by assumption |

## Done

Plan 24 is complete when:

- TUI uses `LocalRuntimeSession` backed by a non-app-server local runtime owner,
- `vac-rs/tui/src/**` has no active `vac_app_server*` imports,
- `vac-rs/tui/Cargo.toml` no longer depends on `vac-app-server`,
  `vac-app-server-client`, or `vac-app-server-protocol`,
- inverse trees from `vac-cli` to `vac-app-server-client`, `vac-app-server`,
  and `vac-app-server-transport` are clean or explicitly non-default/deferred,
- `.vac` manifests declare runtime-owner capability, policy, and maintenance
  gates,
- validation matrix is green,
- PTY gate is passed by a real operator or recorded as `BLOCKED-OPERATOR`,
- docs record deletion or defer evidence without false-green claims.
