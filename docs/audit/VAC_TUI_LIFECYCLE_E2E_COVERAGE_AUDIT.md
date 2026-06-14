# VAC TUI Lifecycle E2E Coverage Audit

Status: **TV-Pass for deterministic TUI agent all-tools harness; TV-Pending for real provider/MCP IO E2E**.

This audit answers a narrow question: do existing unit/integration/smoke tests cover the lifecycle of a real user using the TUI with an agent and all available tools?

## Verdict

Partially closed. The repository now has a deterministic PTY harness that drives:

```text
terminal user input -> TUI composer -> deterministic agent backend -> model/tool-call stream -> approval UI -> every required tool class -> tool result rendering -> ask_user response -> clean terminal shutdown
```

This is **not** a real provider/MCP IO test. It does not execute the actual MCP tools against the filesystem/network/remote hosts; it injects deterministic `InputEvent`/`OutputEvent` traffic through the real `run_tui` event loop. Therefore the proper status is:

```text
deterministic_user_agent_all_tools_tui_e2e=TV-Pass
real_provider_mcp_all_tools_tui_e2e=TV-Pending
```

## Current coverage map

| Layer | Existing coverage | Evidence | Status |
| --- | --- | --- | --- |
| Key mapping / composer basics | Unit tests map BackTab/Ctrl+P into plan review; many text-area and command parsing tests exist. | `vac-rs/crates/surfaces/vac-tui/src/event.rs`; `services/textarea.rs`; `services/commands.rs` | TV-Pass for local units |
| Direct terminal lifecycle | PTY smoke enters/leaves alt screen, `/plan`, Shift+Tab, `/context`, plain text echo, mock-tabs absent. | `scripts/pty-tui-lifecycle-smoke.py`; `vac-rs/crates/surfaces/vac-tui/examples/tui_smoke.rs` | TV-Pass for direct TUI harness |
| Static TUI lifecycle guards | Static gate checks first-launch timer, right rail/context panel, canonical overlays, plan route, lifecycle dispatch helpers, no mock tabs. | `scripts/check-tui-lifecycle-e2e-static.py` | SV-Pass |
| Plan mode / plan review | Unit tests cover plan parsing, review formatting, comments, markdown styling. PTY covers basic route visibility. | `services/plan.rs`; `services/plan_review.rs`; PTY smoke | TV-Pass for components; partial for lifecycle |
| Ask User modal | Strong handler-level async tests cover questions, tab/option navigation, custom input, submit/cancel, multi-select. | `services/handlers/ask_user.rs` | TV-Pass for handler unit lifecycle |
| Approval bar / auto approve | Unit tests exist for approval bar / auto-approve services. | `services/approval_bar.rs`; `services/auto_approve.rs` | TV-Pass for service units |
| Agent runtime gates | Runtime E2E covers Semantic Plan, patch/command gates, validation, closeout, L1/L2 honesty. | `vac-agent-loop` tests; `scripts/vac-runtime-agent-e2e-sv.py`; `tests/fixtures/runtime/bound_agent_e2e_cases.json` | TV-Pass for runtime core; not TUI-coupled |
| Tool display/rendering blocks | Some render helpers are unit-tested, e.g. bash block wrapping and stream block alignment. | `services/bash_block.rs`; message renderer tests | Partial |
| TUI event loop backend bridge | No direct unit test for `event_loop.rs`; PTY harness does not inject real agent tool streams. | `event_loop.rs` | TV-Pending |
| Tool/dialog/shell handlers | `handlers/dialog.rs`, `handlers/tool.rs`, `handlers/shell.rs` have no test modules. | source scan | TV-Pending |
| Deterministic all-tools TUI E2E | PTY harness starts real `run_tui`, injects deterministic agent stream, drives approval/Ask User, and checks all required tool markers. | `scripts/pty-tui-agent-tool-lifecycle-smoke.py`; `vac-rs/crates/surfaces/vac-tui/examples/tui_agent_tool_smoke.rs`; `tests/fixtures/tui-agent-tool-lifecycle/tool-matrix.json` | TV-Pass |

## Harness distinction

`vac-rs/crates/surfaces/vac-tui/examples/tui_smoke.rs` remains a direct terminal lifecycle harness. It proves alt-screen/raw-mode cleanup and selected UI routes.

`vac-rs/crates/surfaces/vac-tui/examples/tui_agent_tool_smoke.rs` is the deterministic agent/tool lifecycle harness. It listens to `OutputEvent::UserMessage`, emits assistant/tool streams, waits for `OutputEvent::AcceptTool`, emits `InputEvent::ToolResult`, drives `ShowAskUserPopup`, waits for `OutputEvent::AskUserResponse`, and exits through the real `run_tui` event loop.

The new harness covers the TUI bridge. It still does not execute real provider/MCP IO.

## Tool lifecycle gaps

The following user-visible bridge files are high-risk because they handle live tool approval/result state but currently lack local Rust test modules:

```text
vac-rs/crates/surfaces/vac-tui/src/event_loop.rs
vac-rs/crates/surfaces/vac-tui/src/services/handlers/dialog.rs
vac-rs/crates/surfaces/vac-tui/src/services/handlers/tool.rs
vac-rs/crates/surfaces/vac-tui/src/services/handlers/shell.rs
```

These paths should be covered before claiming mature production E2E for “user uses TUI with agent and all tools”.

## Deterministic tool matrix covered by the new harness

The harness covers:

- `run_command`
- `run_command_task`
- `run_remote_command`
- `run_remote_command_task`
- `get_all_tasks`
- `wait_for_tasks`
- `get_task_details`
- `cancel_task`
- `view`
- `str_replace`
- `create`
- `remove`
- `view_web_page`
- `generate_password`
- `ask_user`

Acceptance now enforced by `scripts/pty-tui-agent-tool-lifecycle-smoke.py`:

```text
entered_alt_screen
exited_alt_screen
user_prompt_echo_visible
agent_started_visible
approval_lifecycle_visible
ask_user_visible
done_marker_visible
mock_tabs_absent
tool_visible:<all 15 tools>
marker_visible:<all tool result markers>
```

## Remaining closure slice

```text
Real Provider/MCP Tool IO E2E
```

Target: run real `vac-cli` interactive mode with a deterministic local provider and actual MCP/local tool implementations in a sandboxed workspace. This should prove that the same lifecycle works with real tool execution, not only deterministic event injection.
