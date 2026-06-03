# TUI PTY Dogfood Gate Runbook

## Goal

Verify the real operator path, not only unit tests. The Local Runtime Contract wiring is only considered passing when an operator can submit a prompt through the bottom input of the root `vac` TUI and observe activity, approval, validation, and completion driven by Local Runtime Contract semantics — not just compile-time symbol presence.

This runbook acts as the human dogfooding protocol to satisfy the `vac.tui.pty_gate` capability requirements before any candidate build is promoted to production.

---

## CLI Baseline

The real binary surface is intentionally small. There is no `vac tui` subcommand and there are no `--smoke` / `--dry-run` flags. The operator entrypoints relevant to this gate are:

```text
vac                       # default → root interactive TUI
vac resume [--last|<id>]  # resume an existing local session in the TUI
vac exec [PROMPT]         # headless local runtime path
```

If a future iteration adds a non-interactive smoke entrypoint, this doc must be updated alongside the binary. Do not assume flags exist that are not in `vac --help`.

---

## Required PTY Verification Flow

Run this protocol in a real terminal environment (not in an MCP shell or restricted CI shell, as alternate-screen alternate buffers require a genuine interactive TTY):

```bash
# 1. Compile the active product binary cleanly
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 build --bin vac

# 2. Launch the root TUI in alternate-screen mode
./target/debug/vac

# 3. Perform interactive checks:
#    a. Press the "/" key.
#    b. Verify that the command palette popup overlay renders cleanly without clipping.
#    c. Press Enter to select/dismiss or ESC to close.
#    d. Type a normal read-only prompt: "summarize README.md".
#    e. Press Enter.
#    f. Verify that the task and activity sidebar registers the request and initiates analysis.
#    g. Press Ctrl-C.
#    h. Verify the process terminates cleanly, returning exit code 0, and restores the terminal screen state.
```

In current builds, `/` opens the slash-command list. Do not use `/help` unless a future release explicitly adds it back.

---

## Visual Layout Verification Criteria

During the PTY session, operators must manually audit the visual state of the TUI:
- **Responsive Sizing**: Test resizing the terminal window down to `80x24` and up to `120x40`. Verify the layout shifts smoothly and does not panic due to layout constraints.
- **Alternate Buffer Restoration**: When exiting via `Ctrl-C`, the terminal must restore the original prompt buffer immediately. No visual artifacts, broken cursor states, or styling leaks should remain in the parent terminal.
- **Contrast and Chromatic Harmony**: Color schemes must be consistent across views. Focus borders and selected list items should have clear high-contrast states.

---

## Local Runtime Contract Evidence

While the PTY session above is running, the operator must be able to verify that the Local Runtime Contract is engaged for the fresh prompt path. The minimum evidence set is:

1. `RuntimeCommand::StartTask` is created from the bottom-input Enter and appears in the `RUST_LOG=info` trace output, e.g.
   `local-runtime: session=… task=… entrypoint=tui autonomy=assist prompt_len=…`.
2. The runtime turn driven by that command is the source of truth for activity, approval, validation, and completion.
3. `RuntimeEvent` projections are visible in the existing root TUI surfaces (activity rows, approval overlay, validation status line, evidence summary). No second TUI is started.
4. Approve and Reject from the existing approval overlay round-trip back through `RuntimeCommand::Approve` / `RuntimeCommand::Reject`.
5. Ctrl-C exits cleanly and `git status` shows only intended files changed.

To capture the trace in the same terminal:

```bash
RUST_LOG=info ./target/debug/vac 2> /tmp/vac-pty-gate.log
# in another terminal: tail -F /tmp/vac-pty-gate.log
```

---

## Structured Workflow Result

The workflow-control-plane result for `capability.tui.pty_gate` must use one of these states:

```text
passed
failed
blocked_operator
skipped_not_applicable
```

Release-gate consumers may treat only `passed` as passing evidence by default. `blocked_operator` is a truthful non-pass state: it means the current environment could not provide a usable real or virtual PTY and a human operator must run this checklist. When rendered for handoff, use the explicit marker:

```text
BLOCKED-OPERATOR: no interactive TTY available in this environment
```

The automated executor may capture only private, redacted diagnostics such as binary path, working directory, terminal size, whether screen output changed after `/`, and Ctrl-C exit status. It must not dump the full process environment or terminal transcript into public logs.

---

## Evidence Checklist to Capture

Before signing off on a release gate, the following evidence must be collated:
- [ ] **PTY Transcript / Recording**: A terminal capture showing the alternate buffer rendering the `vac` TUI, opening the command palette, and exiting cleanly.
- [ ] **Log Telemetry**: Snippet from `/tmp/vac-pty-gate.log` proving successful execution of `StartTask` and event streams.
- [ ] **Exit Code**: Zero exit status verification (`echo $?` returning `0`).
- [ ] **Working Tree State**: `git status --short` after run, proving no stray file modifications.

---

## Handling Headless Sandbox Environments

MCP shells, automated CI scripts, and containerized sandboxes cannot drive a real alternate-screen TTY. If you are operating in such an environment:

1. Do **NOT** attempt to mock or fake the PTY alternate-screen validation.
2. Run the automated capability check to guarantee parser coverage:
   ```bash
   ./vac-rs/target/debug/vac doctor registry .
   ./vac-rs/target/debug/vac doctor workflow .
   ```
3. Mark the manual PTY checkpoint as `BLOCKED-OPERATOR` in the workflow run report.
4. Export the candidate binary and delegate the manual verification flow described in this runbook to a human developer operating in a fully interactive terminal environment.
