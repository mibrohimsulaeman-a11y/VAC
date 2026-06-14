# VAC TUI Lifecycle E2E Coverage Audit

Status: **TV-Pending for full user-agent-all-tools TUI E2E**.

This audit answers a narrow question: do existing unit/integration/smoke tests cover the lifecycle of a real user using the TUI with an agent and all available tools?

## Verdict

No. Current tests cover important slices, but they do not yet cover the full real lifecycle:

```text
terminal user input -> TUI composer -> vac-cli interactive agent loop -> model/tool-call stream -> tool approval UI -> every tool class -> tool result rendering -> completion/closeout -> terminal shutdown
```

The repository has enough tests to say **TUI terminal lifecycle smoke is covered** and **bounded agent runtime gates are covered separately**. It does **not** have a single E2E test that drives a real TUI session with the agent runtime and every tool path.

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
| Full all-tools E2E | No test starts real `vac-cli` interactive session with deterministic fake model/agent and exhaustively exercises every tool. | absence of fixture/harness | NotCovered |

## Why current PTY smoke is not full TUI+agent E2E

`vac-rs/crates/surfaces/vac-tui/examples/tui_smoke.rs` calls `vac_tui::run_tui` directly. Its backend shim only loops back:

```text
OutputEvent::PlanModeActivated -> InputEvent::PlanModeChanged
OutputEvent::UserMessage -> InputEvent::AddUserMessage
```

All other output events are ignored in that harness. Therefore it proves terminal lifecycle and selected UI routes, but not agent orchestration or tool execution.

## Tool lifecycle gaps

The following user-visible bridge files are high-risk because they handle live tool approval/result state but currently lack local Rust test modules:

```text
vac-rs/crates/surfaces/vac-tui/src/event_loop.rs
vac-rs/crates/surfaces/vac-tui/src/services/handlers/dialog.rs
vac-rs/crates/surfaces/vac-tui/src/services/handlers/tool.rs
vac-rs/crates/surfaces/vac-tui/src/services/handlers/shell.rs
```

These paths should be covered before claiming mature production E2E for “user uses TUI with agent and all tools”.

## Required next test design

A proper full E2E needs a deterministic test harness with:

1. A fake provider/model stream that emits assistant deltas, tool-call starts, streaming tool progress, tool results, errors, cancellations, and retries.
2. A tool matrix covering at least:
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
3. Real TUI event loop, not only isolated handler functions.
4. PTY or terminal backend capture proving visible states:
   - pending tool block
   - approval bar/modal
   - accept/reject path
   - stream progress
   - success result rendering
   - error/cancel rendering
   - retry path
   - shell focus/background lifecycle
   - Ask User response returns an `OutputEvent::AskUserResponse`
5. Closeout/quit proving raw-mode/alt-screen cleanup after tool lifecycle activity.

## Recommended next closure slice

```text
TUI Agent Tool Lifecycle Harness
```

Target artifacts:

```text
vac-rs/crates/surfaces/vac-tui/examples/tui_agent_tool_smoke.rs
scripts/pty-tui-agent-tool-lifecycle-smoke.py
tests/fixtures/tui-agent-tool-lifecycle/tool-matrix.json
```

Acceptance:

```text
full_user_agent_all_tools_e2e remains TV-Pending until the harness drives real TUI event loop plus deterministic agent/tool stream.
No release or audit output may claim full TUI+agent+all-tools coverage before that harness passes in CI.
```
