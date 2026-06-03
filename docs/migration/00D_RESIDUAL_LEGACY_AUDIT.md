# 00D residual legacy audit (post-sidebar landing)

**Date:** 2026-05-06
**Author:** Notion AI (MCP session)
**Repo HEAD:** `35aaa5d` on `main`
**Trigger:** 00D-8 Activity Sidebar landed; baseline `cargo check -p vac-surface-cli` passes (28.79s, only dead_code warnings).

This audit complements `00E_REACHABILITY_AUDIT.md`. It maps how deeply `vac-rs/tui/` still depends on the legacy app-server stack so that scope of "00D deps cleanup" is honest before any code is touched.

## TL;DR

00D deps cleanup is **not** the 1-2 hour job that the same task was for `vac-exec`. The TUI still routes essentially every product surface (config loading, account/auth, model list, thread lifecycle, turn lifecycle, approvals, skills, marketplace, realtime audio, external agent migration, etc.) through `AppServerSession` over `vac-app-server-client` + `vac-app-server-protocol`. Removing those deps means rewriting `AppServerSession` (~1500 LoC, 50+ typed RPC methods) and migrating the entire `crate::legacy_core::*` re-export bridge.

The Local Runtime Contract path landed by 00D-5 (`RuntimeCommand::StartTask` as canonical input) and 00D-8 (Activity Sidebar projection) is a **projection on top of the legacy transport**, not a replacement. Plan 00D explicitly accepts this:

> The legacy `AppCommand::UserTurn` op that the chatwidget submit path eventually constructs is **a transport** that carries the runtime-first submission across the in-process app-server boundary so the existing execution machinery keeps working without a wholesale backend rewrite.

So "00D deps cleanup" is **not** part of Plan 00D done criteria. It belongs in a follow-up plan (suggested name: **Plan 00F — TUI legacy transport retirement**) that should be drafted before 00E delete is attempted.

## Scope numbers

| Item | Count |
|---|---|
| Files in `vac-rs/tui/src/` referencing the legacy stack | 41+ |
| Lines of `vac-rs/tui/src/app_server_session.rs` (the central facade) | 1,500+ (74,868 bytes) |
| Typed RPC methods on `AppServerSession` | ~50 |
| `vac_app_server_protocol::*` types imported across `vac-rs/tui/src/` | ~120 distinct |
| Test files relying on `vac_app_server_protocol::*` | `app/tests.rs` (68 refs), `chatwidget/tests/*`, `status/tests.rs` (36), `app/pending_interactive_replay.rs` (23), `app/thread_session_state.rs` (12), `app/thread_events.rs` (20), `app/app_server_requests.rs` (36) |

## What `legacy_core` actually is

`vac-rs/tui/src/lib.rs:76` re-exports:

```rust
pub(crate) use vac_app_server_client::legacy_core;
```

Everything written as `crate::legacy_core::config::Config`, `ConfigBuilder`, `ConfigOverrides`, `find_vac_home`, `format_exec_policy_error_with_source`, `windows_sandbox::WindowsSandboxLevelExt`, `personality_migration::maybe_migrate_personality`, `otel_init::build_provider`, `test_support::*`, `DEFAULT_AGENTS_MD_FILENAME`, etc. is **physically defined inside `vac-app-server-client`**. Removing the dep on `vac-app-server-client` therefore requires either:

1. Moving those modules into `vac-core` (or another crate the TUI already owns), then dropping the re-export; **or**
2. Creating a new shared `vac-runtime-shared` crate with the migrated modules.

Grep shows ~25 files in `vac-rs/tui/src/` use `crate::legacy_core::*` directly — not just `lib.rs`.

## Dedicated app-server adapter files (must be rewritten or replaced)

- `vac-rs/tui/src/app_server_session.rs` (~1500 LoC) — **central facade**. Owns: `bootstrap`, `read_account`, `external_agent_config_detect/import`, `start_thread{,_with_session_start_source}`, `resume_thread`, `branch_thread`, `thread_list`, `thread_loaded_list`, `thread_read`, `thread_inject_items`, `turn_start`, `turn_interrupt`, `startup_interrupt`, `turn_steer`, `thread_set_name`, `thread_memory_mode_set`, `memory_reset`, `thread_goal_get/set/clear`, `logout_account`, `thread_unsubscribe`, `thread_compact_start`, `thread_shell_command`, `thread_approve_guardian_denied_action`, `thread_background_terminals_clean`, `thread_rollback`, `review_start`, `skills_list`, `reload_user_config`, `thread_realtime_start/audio/stop`, `reject_server_request`, `resolve_server_request`, `shutdown`, plus all `*_params_from_config` mappers and `thread_session_state_from_thread_*_response` helpers. Embedded vs Remote `ThreadParamsMode` baked in.
- `vac-rs/tui/src/app_server_approval_conversions.rs` — maps `vac_app_server_protocol` approval DTOs into local types and back.
- `vac-rs/tui/src/app/app_server_event_targets.rs` — routes `ServerNotification` / `ServerRequest` to UI targets.
- `vac-rs/tui/src/app/app_server_events.rs` — event dispatch surface.
- `vac-rs/tui/src/app/app_server_requests.rs` — server-request response builders.

These files are not "plumbing leaks". They are **the TUI's session engine**. Until a Local Runtime Contract equivalent exists for everything they expose, the TUI cannot operate without them.

## Other pervasive consumers

- `vac-rs/tui/src/lib.rs` — startup wiring (`bootstrap_in_process_app_server_client`, `RemoteAppServerClient::connect`, `cloud_requirements_loader_for_storage`, `EnvironmentManager`).
- `vac-rs/tui/src/main.rs:3,26` — `legacy_core::util::resume_command(thread_id)`.
- `vac-rs/tui/src/debug_config.rs`, `resize_reflow_cap.rs`, `status/{card,helpers,rate_limits,tests}.rs`, `app/{startup_prompts,thread_routing,thread_events,thread_session_state,thread_goal_actions,session_lifecycle,loaded_threads,replay_filter,pending_interactive_replay,event_dispatch,test_support,config_persistence,background_requests,model_catalog,tests}.rs`, `external_agent_config_migration*.rs`, `bottom_pane/{approval_overlay,hooks_browser_view,mcp_server_elicitation}.rs`, `chatwidget*.rs` and its tests, `history_cell*.rs`, `resume_picker.rs`, `permission_compat.rs`, `onboarding/*`, `app_event*.rs`, `app_command.rs`, `app.rs`, `additional_dirs.rs`.
- `vac-rs/tui/Cargo.toml` lists `vac-app-server-client`, `vac-app-server-protocol`, `vac-cloud-requirements`, `vac-exec-server` as workspace deps. None can be dropped while any of the above consumers exists.

## Why "can't we just drop the deps now?" is wrong

A naïve `Cargo.toml` deletion would break:

1. The entire `crate::legacy_core::*` re-export path → `Config`, `ConfigBuilder`, `find_vac_home`, `otel_init::build_provider`, `windows_sandbox::*` all become unresolved.
2. `AppServerSession` cannot compile (no `AppServerClient`, no `AppServerEvent`, no `ClientRequest`, no `ThreadStartParams`/`ThreadStartResponse`/etc.).
3. Every test under `vac-rs/tui/src/app/tests.rs` and friends (~68 + 36 + 23 + ... refs) constructs `vac_app_server_protocol` payloads directly to drive the App through synthetic event streams. Those tests double-duty as the contract for `App::handle_event`.
4. `bootstrap`, model picker, account/auth status, marketplace plugin install, external agent config migration, realtime audio, skills list — all currently routed through typed app-server RPCs.

The TUI's product surface **is** the projection of those RPCs. There is no alternative source for that data in `vac-core` today.

## Proposed scope of follow-up Plan 00F (suggested)

This is intentionally laid out in *small* steps so each can land independently behind `cargo check -p vac-surface-cli` green and behind the PTY operator gate:

1. **00F-1 — Move `legacy_core` modules into `vac-core` (or a new `vac-runtime-shared` crate).** Drop the re-export from `vac-app-server-client`. After this step, `vac-rs/tui/` can use `vac-core::config::*` etc. without going through the app-server crate. Build still depends on the app-server crate transitively because `AppServerSession` lives in TUI. **No deps removed yet.**
2. **00F-2 — Introduce `LocalRuntimeSession` trait** in `vac-core::local_runtime` with the subset of methods the TUI bootstrap actually needs (start/resume/branch thread, turn start/interrupt/steer, thread read, account read, model list, etc.). Provide a `LocalAppServerSession` impl in `vac-rs/tui/src/local_runtime/session_adapter.rs` that wraps the existing `AppServerSession`. Migrate call sites in `vac-rs/tui/src/app/*.rs` to the trait one surface at a time (loaded_threads, thread_goal, memory mode, realtime, etc.). **Still no deps removed.**
3. **00F-3 — Replace `AppServerSession::turn_start` with a Local Runtime Contract path** that goes directly into `vac-core` runtime (already 80% in place via `vac-rs/tui/src/local_runtime.rs` projection). Remove the `Op::UserTurn` compat transport and the matching `InProcessAppServerClient::start` boot path in `vac-rs/tui/src/lib.rs`.
4. **00F-4 — Migrate test infrastructure.** Replace `vac_app_server_protocol::*` payloads in `vac-rs/tui/src/app/tests.rs` and friends with synthetic `RuntimeEvent` streams. This is the largest mechanical change because tests directly construct ~120 distinct DTO types.
5. **00F-5 — Migrate auxiliary surfaces** (account, marketplace, external agent migration, skills, realtime audio) to either Local Runtime Contract or to lower-level provider crates (`vac-login`, `vac-skills`, etc.) so the TUI no longer needs the typed RPC for them.
6. **00F-6 — Drop deps.** Once all consumers are gone, remove `vac-app-server-client`, `vac-app-server-protocol`, `vac-cloud-requirements`, `vac-exec-server` from `vac-rs/tui/Cargo.toml`. Re-run inverse trees and `cargo check -p vac-surface-cli`.
7. **00F-7 — Hand off to 00E.** With `vac-tui` clean and `vac-exec` 00C-resume/review still pending in parallel, the unreachability conditions in `00E_REACHABILITY_AUDIT.md` can finally be satisfied.

## What 00D-8 actually delivered (so the record is accurate)

- `vac-rs/tui/src/activity_sidebar.rs` — floating right-side panel reading `RuntimeActivityState` from `chatwidget::ChatWidget`.
- `vac-rs/tui/src/local_runtime.rs::project_runtime_events` (per the 00D-8 file header) — pushes typed `RuntimeActivityItem` entries from the projection of `RuntimeEvent` produced inside the TUI process.
- Layout helper `split_for_activity_sidebar(area) → (main, Option<sidebar>)` with `ACTIVITY_SIDEBAR_MIN_TOTAL_WIDTH = 120`, `ACTIVITY_SIDEBAR_WIDTH = 36`, `ACTIVITY_SIDEBAR_HEIGHT = 12`.

Dead-code warnings from `cargo check` are expected: `runtime_activity_mut`, `prompt`/`cwd` on `RuntimeSubmitPlan`, `session_id`/`task_id`/`record_*_decision`/`label_for`/`activity_label_for` on `RuntimeBridge`, `validation_status_from_exec_status`, `turn_abort_reason_label`, `set_expanded` on `RuntimeActivityState`. These are scaffolding for upcoming sub-tasks (`/activity` slash command body, validation projection through PTY-visible labels, expanded state). They do **not** indicate that 00D-8 is incomplete.

## Recommendation

Do not attempt a partial 00D deps drop now. The next concrete sessions should pick up from `00E_REACHABILITY_AUDIT.md`'s pre-condition checklist:

- Finish **00C-resume** and **00C-review** rewires in `vac-rs/exec/src/lib.rs` (smaller scope, ~1886 LoC file but a single binary surface).
- Draft **Plan 00F** based on the 7-step scope above before touching `vac-rs/tui/Cargo.toml`.
- Keep `vac-app-server-protocol`, `vac-api`, `vac-exec-server`, `vac-backend-client` classified as **DEFERRED** in 00E (already documented).

This audit replaces the optimistic active-task entry “00D deps cleanup mirror for `vac-rs/tui/Cargo.toml`” with a realistic multi-step plan owner.
