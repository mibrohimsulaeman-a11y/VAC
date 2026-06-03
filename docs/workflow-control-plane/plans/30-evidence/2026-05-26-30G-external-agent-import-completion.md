# Plan 30G external-agent import completion — L1 continuation

Date: 2026-05-26
Lane: L1-continuation
Scope: residual 30G blockers #2 and #3 only

## Summary

Blocker #1 is already closed by `fd61ed8` (`feat(plan30): add owner plugin provider path`). L1 verified the commit diff before editing: local plugin list/read/install/uninstall/set-enabled are owner-backed, while remote marketplace parity and external-agent background completion remained out of that commit.

This slice narrows blocker #2 by moving external-agent **plugin** migration completion into the owner path:

- `ExternalAgentConfigService::import` now returns `ExternalAgentConfigImportOutcome` instead of a bare `Vec<PendingPluginImport>`.
- Plugin migration items no longer split remote marketplace plugins into pending work. The service calls `import_plugins(...)` synchronously for the selected plugin migration details and records each `PluginImportOutcome` in stable input order.
- `RuntimeCommandBus::external_agent_config_import` now fails with `ExternalAgentConfigBackgroundImportRequired` only when true pending session import work remains.
- Session import selections are now represented explicitly as `pending_session_imports` in the outcome instead of being silently ignored.

## Owner-native shape decision

Chosen shape: **blocking-equivalent command** for plugin import completion.

Rationale:

- `server_requests.rs` models operator-facing pending approval/input/MCP elicitation requests. That shape is not a good fit for plugin import completion because plugin import does not need a human decision handle after the user has selected migration items.
- `event_stream.rs` provides sequenced runtime event delivery, but `RuntimeCommandBus` does not currently own a completion event stream for this command path. Adding a synthetic completion event would expand the protocol surface without solving the retained/direct TUI fallback location constraint.
- The app-server background task primarily existed so remote plugin imports could continue after the immediate JSON-RPC response. The owner bus can safely await that plugin completion in the write command because it is already an async command boundary.

Cancellation: **not applicable for plugin completion** in this owner shape. There is no returned pending handle; completion is awaited before the command returns. Session import still requires a separate owner-owned thread-manager-backed completion path before cancellation semantics can be claimed.

## Tests added

Local-runtime-owner coverage added:

- `external_agent_config::tests::plugin_import_completes_synchronously_even_when_marketplace_fails`
  - proves plugin migration attempts complete synchronously and return a failure outcome instead of pending background plugin work.
- `external_agent_config::tests::session_imports_remain_explicit_pending_background_work`
  - proves selected sessions are surfaced as pending background work rather than ignored.
- `external_agent_config::tests::import_items_are_processed_in_stable_input_order`
  - proves plugin import outcome ordering follows migration item order.
- `tests::command_bus_p30g_external_agent_import_accepts_no_background_work`
  - proves the command bus accepts empty/no-background import.
- `tests::command_bus_p30g_external_agent_session_import_requires_completion_owner`
  - proves session import remains an explicit owner blocker rather than a silent no-op.

## Blocker status

- Blocker #1: **CLOSED** by `fd61ed8`.
- Blocker #2: **PARTIAL**.
  - Plugin import completion: **CLOSED** in owner path by this slice.
  - Session import completion: **ESCALATED / remaining sub-gap**. Owner service can detect sessions and now preserves selected sessions as pending work, but full session import needs the app-server-equivalent thread creation path (`InitialHistory::Branched(rollout_items)`) plus imported-session ledger recording. Wiring the retained/direct TUI import call to pass thread-manager resources lives in forbidden `vac-rs/tui/src/app_server_session.rs` for this lane.
- Blocker #3: **ESCALATED**.
  - `vac-rs/tui/src/app/background_requests.rs` was inspected and contains plugin typed request helpers but no external-agent detect/import fallback branch to edit in this lane.
  - The owner-first external-agent detect/import fallback branches are in forbidden `vac-rs/tui/src/app_server_session.rs` (`ExternalAgentConfigBackgroundImportRequired` and unexpected owner-error fallback handling). L1 did not edit that file per lane constraints.

## Remaining concrete ask

To fully close sessions and remove the retained/direct fallback, a follow-up lane that owns `vac-rs/tui/src/app_server_session.rs` must wire an owner-native session import command that has access to retained `ThreadManager` resources and can:

1. validate/prepare selected sessions with `vac_external_agent_sessions::prepare_validated_session_imports`,
2. start a new local runtime thread with `InitialHistory::Branched(rollout_items)`,
3. record imported sessions with `vac_external_agent_sessions::record_imported_session`, and
4. replace the app-server fallback arm for `ExternalAgentConfigBackgroundImportRequired` with an owner-only path or an explicit observable warning if policy intentionally retains fallback.

No `vac_protocol` or `vac-app-server-protocol` schema change was required for this slice.

## Validation

Commands attempted after implementation:

```text
cd vac-rs && cargo +1.93.0 build -p vac-local-runtime-owner -p vac-surface-tui
cargo +1.93.0 nextest run -p vac-local-runtime-owner
cargo +1.93.0 nextest run -p vac-surface-tui -- background_requests
cargo +1.93.0 clippy -p vac-local-runtime-owner -- -D warnings
```

Results:

- `cargo +1.93.0 build -p vac-local-runtime-owner -p vac-surface-tui`: **PASS**.
- `cargo +1.93.0 nextest run -p vac-local-runtime-owner`: **PASS**, 43 passed / 0 skipped.
- `cargo +1.93.0 nextest run -p vac-surface-tui -- background_requests`: **PASS**, 5 passed / 2199 skipped.
- `cargo +1.93.0 clippy -p vac-local-runtime-owner --no-deps -- -D warnings`: **PASS** after fixing local-runtime-owner lint surfaced by Rust 1.93.
- `cargo +1.93.0 clippy -p vac-local-runtime-owner -- -D warnings`: **BLOCKED by out-of-scope dependency** in forbidden `vac-rs/app-server-protocol/src/protocol/v2.rs`:

```text
error: useless conversion to the same type: `vac_protocol::protocol::ThreadGoalStatus`
    --> app-server-protocol/src/protocol/v2.rs:4009:21
     |
4009 |             status: value.status.into(),
     |                     ^^^^^^^^^^^^^^^^^^^ help: consider removing `.into()`: `value.status`
     |
     = note: `-D clippy::useless-conversion` implied by `-D warnings`

error: could not compile `vac-app-server-protocol` (lib) due to 1 previous error
```

This file is explicitly forbidden for L1 (`vac-rs/app-server-protocol/**`, Lane L13). L1 did not modify it.

- `cargo +1.93.0 clippy -p vac-surface-tui -- -D warnings`: **BLOCKED by the same out-of-scope dependency** in forbidden `vac-rs/app-server-protocol/src/protocol/v2.rs`.
- `vac doctor registry ..`: **BLOCKED by current CLI surface / out-of-scope L14 area**:

```text
error: unexpected argument 'registry' found

Usage: vac doctor [OPTIONS]

For more information, try '--help'.
```

`vac-rs/cli/**` is explicitly forbidden for L1, so this lane did not modify the doctor command surface.


Disk safety note: pre-validation free disk was below the hard stop threshold (`14G`). L1 ran `cargo +1.93.0 clean --manifest-path vac-rs/Cargo.toml` as disk recovery for this repo, freeing the local `vac-rs/target` build cache before validation. Post-clean disk had `115G` free.
