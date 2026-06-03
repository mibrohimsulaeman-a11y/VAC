# L-THREADITEM relocation audit

Date: 2026-05-27
Lane: L-THREADITEM
Mode: STOP decision / evidence-only documentation

## Decision

Do **not** relocate `ThreadItem` or `build_turns_from_rollout_items` into `vac_protocol` in this slice.

The move is not clean: `ThreadItem` is an app-server projection type, not the same boundary as the existing owner-native `vac_protocol::items::TurnItem`. Moving it as-is would also require moving a broad set of app-server v2 presentation DTOs and conversion helpers, or redesigning the type boundary first.

## Pre-flight evidence

- Checkout: `/home/emp/Documents/VAC/vastar-agentic-cli`.
- Current HEAD at audit time: `0f734c7 docs(plan23): update operator evidence status`.
- L-AUTH structural prerequisite is present in source/docs: `AuthMode` canonical ownership is now in `vac_protocol::auth::AuthMode`, and app-server-protocol imports it through its common protocol layer.
- Disk pre-flight passed: `df -h .` reported 54G available.
- Cargo/rustc pre-flight passed: no active `cargo` or `rustc` processes were reported.
- ThreadItem source count: `grep -rln "ThreadItem" vac-rs --include="*.rs" | grep -v target | wc -l` returned `62`.
- `build_turns_from_rollout_items` source locations:
  - `vac-rs/app-server/src/vac_message_processor.rs`
  - `vac-rs/app-server/src/bespoke_event_handling.rs`
  - `vac-rs/app-server-protocol/src/protocol/thread_history.rs`
  - `vac-rs/external-agent-sessions/src/export.rs`
  - `vac-rs/tui/src/session_protocol.rs`
  - `vac-rs/tui/src/app_server_session.rs`

## `TurnItem` vs `ThreadItem`

`vac_protocol::items::TurnItem` is the owner-native core item stream shape. It includes durable/core items such as user messages, hook prompts, agent messages, plans, reasoning, web search, image items, file changes, and context compaction.

`vac_app_server_protocol::ThreadItem` is wider and presentation-specific. It flattens and adapts core events into app-server v2 client DTOs, including fields and variants that are not owner-native `TurnItem` concepts:

- `CommandExecution` with app-server command action parsing, status, aggregated output, process id, duration, and shell-source projection.
- `McpToolCall` with JSON-shaped MCP result/error payloads.
- `DynamicToolCall` with app-server v2 output content items and status projection.
- `CollabAgentToolCall` with app-server collaboration tool/status/state projection.
- `EnteredReviewMode` / `ExitedReviewMode` review presentation items.
- App-server wire-shape conversions for `UserInput`, `MemoryCitation`, `HookPromptFragment`, `WebSearchAction`, file-change diffs, and statuses.

So `TurnItem` is the canonical core/event item model; `ThreadItem` is a v2 app-server UI/history projection over core rollout events.

## Direct dependency blockers

The `ThreadItem` enum in `vac-rs/app-server-protocol/src/protocol/v2.rs:5832-6911` directly depends on app-server-protocol-only types or app-server presentation DTOs:

| Dependency | Current owner / blocker |
| --- | --- |
| `UserInput` | v2 app-server DTO in `v2.rs`; differs from `vac_protocol::user_input::UserInput` via API wire shape/conversions. |
| `HookPromptFragment` | Duplicated v2 DTO with conversion from `vac_protocol::items::HookPromptFragment`. |
| `MemoryCitation` | v2 DTO in `v2.rs`, converting from `vac_protocol::memory_citation::MemoryCitation`. |
| `CommandExecutionSource` | v2 enum generated from `vac_protocol::protocol::ExecCommandSource`, but defined in app-server protocol. |
| `CommandExecutionStatus` | v2 status enum in app-server protocol, converting from core exec status. |
| `CommandAction` | v2 app-server command action type with `into_core` / `from_core_with_cwd` and `AbsolutePathBuf` path projection. |
| `FileUpdateChange`, `PatchChangeKind`, `PatchApplyStatus` | v2 file-change DTOs plus conversion from core patch status and `convert_patch_changes`. |
| `McpToolCallStatus`, `McpToolCallResult`, `McpToolCallError` | v2 app-server MCP presentation payloads; result intentionally uses `serde_json::Value` for schema/TS-friendly wire shape. |
| `DynamicToolCallStatus`, `DynamicToolCallOutputContentItem` | v2 app-server DTOs, with separate core dynamic-tools type. |
| `CollabAgentTool`, `CollabAgentToolCallStatus`, `CollabAgentState`, `CollabAgentStatus` | v2 collaboration presentation types, converting from `vac_protocol::protocol::AgentStatus`. |
| `WebSearchAction` | v2 DTO around `vac_protocol::models::WebSearchAction`. |
| `Turn` / `TurnStatus` / `TurnError` | `build_turns_from_rollout_items` returns app-server v2 `Turn`, not a core turn-history DTO. |

Moving `ThreadItem` to `vac_protocol` without moving all of the above would either introduce an illegal reverse dependency on `vac_app_server_protocol`, duplicate a large portion of `v2.rs`, or incorrectly make app-server v2 presentation shapes canonical core protocol.

## `build_turns_from_rollout_items` coupling

`vac-rs/app-server-protocol/src/protocol/thread_history.rs` is also app-server-protocol-coupled. Its reducer imports and constructs app-server v2 DTOs throughout:

- `ThreadItem`, `Turn`, `TurnError`, `TurnStatus`, `UserInput`, `WebSearchAction`.
- `CommandExecutionStatus`, `CommandAction`, `FileUpdateChange`, `PatchApplyStatus`, `PatchChangeKind`.
- `McpToolCallStatus`, `McpToolCallResult`, `McpToolCallError`.
- `DynamicToolCallStatus`, `DynamicToolCallOutputContentItem`.
- `CollabAgentTool`, `CollabAgentToolCallStatus`, `CollabAgentState`.
- `item_builders` helpers (`build_command_execution_*`, `build_file_change_*`, `build_item_from_guardian_event`) which explicitly document that this projection is presentation-specific and app-server-owned.

The history reducer handles many persisted `EventMsg` variants and synthesizes app-server v2 `ThreadItem`s from core rollout events. This is exactly the dependency graph that the stop condition warned about: the helper is not just a neutral rollout-to-core conversion; it is a client-facing app-server history projection.

## Alternative recommendation

Keep `ThreadItem` and `build_turns_from_rollout_items` in `vac_app_server_protocol` for now. Add a slim `vac_protocol::items::ThreadHistoryItem` boundary before retiring `external-agent-sessions` from app-server-protocol.

Recommended shape:

1. Introduce a core-owned boundary in `vac_protocol::items`, for example:

   ```rust
   pub trait ThreadHistoryItem {
       fn id(&self) -> &str;
       fn kind(&self) -> ThreadHistoryItemKind;
   }

   pub enum ThreadHistoryItemKind {
       UserMessage,
       HookPrompt,
       AgentMessage,
       Plan,
       Reasoning,
       WebSearch,
       ImageView,
       ImageGeneration,
       FileChange,
       ContextCompaction,
       ToolCall,
       ReviewMode,
       Other,
   }
   ```

2. Implement the trait for core `TurnItem` immediately.
3. Implement the trait/adapter for app-server `ThreadItem` inside `vac_app_server_protocol` (or a narrow adapter module there), preserving app-server ownership of v2 DTOs.
4. Change `external-agent-sessions` tests/import checks to assert against the trait/adapter boundary or a small core-owned rebuilt-history summary, not raw app-server `ThreadItem`.
5. Only after consumers stop needing raw app-server v2 history DTOs, revisit whether a new core `ThreadHistory` projection is necessary.

This keeps `vac_protocol` owner-native and avoids dragging app-server v2 wire DTOs into the core protocol crate.

## External-agent-sessions status

`external-agent-sessions` retirement is **not unblocked** by a clean `ThreadItem` relocation. The current direct test dependency on:

- `vac_app_server_protocol::ThreadItem`
- `vac_app_server_protocol::build_turns_from_rollout_items`

should be replaced by the trait/adapter recipe above or by a core-owned summary helper designed specifically for imported external session validation.

## Verification

No Rust source was changed in this lane. Validation is docs-only and intentionally proportional:

- `git diff --check`
- scoped docs diff review

No cargo build/check/nextest/clippy was run because the stop condition prevented code relocation.
