# Plan 23 Evidence — L19 real-operator PTY runbook

Date: 2026-05-26
Lane: L19 — Plan 23 PTY operator evidence drive
Status: `BLOCKED-OPERATOR` until a human runs one real PTY round and drops evidence here
Scope: `docs/workflow-control-plane/plans/23-evidence/`, `docs/workflow-control-plane/plans/23-pty-operator-gate.md` status only, `.vac/workflows/maintenance.tui-pty-gate.yaml` read-only verification

## Decision

Do **not** mark Plan 23 complete from this lane. The implementation is already hardened, and the workflow manifest is present, but this session cannot produce trustworthy real-operator evidence from a human-owned interactive terminal. This evidence file records the exact setup needed for a human to run one round and attach pass/fail evidence.

## Read-only manifest verification

Verified `.vac/workflows/maintenance.tui-pty-gate.yaml` in read-only mode:

- workflow id: `maintenance.tui-pty-gate`
- title: `TUI PTY Operator Gate`
- status: `ready`
- mutation policy: `mutates_files: false`
- network policy: `network: false`
- redaction policy: `redaction: true`
- process approval policy: `approval_required_for: execute_process`
- gate step: `validate` uses `capability.tui.pty_gate`
- validation gate list includes `validate`
- validation commands are still recorded as `cargo +1.93.0 check -p vac-core` and `cargo +1.93.0 check -p vac-surface-tui --tests`

No `.vac/` manifest or workflow file was edited in this lane.

## Why no pass evidence was captured here

A valid Plan 23 pass needs an operator-controlled real terminal session, not just a background automation transcript. The required evidence must show that:

1. root `vac` launches in a terminal users actually operate,
2. the alternate screen enters and restores cleanly,
3. `/` opens the slash-command list without a layout crash,
4. normal input reaches the composer,
5. `Ctrl-C` exits cleanly,
6. terminal state is restored after exit,
7. captured output is private/redacted.

This docs-only lane therefore records `BLOCKED-OPERATOR` until a human runs the checklist below.

## Required real-operator setup

Human operator needs:

- local checkout: `Documents/VAC/vastar-agentic-cli`
- branch: `main` or the intended release validation branch
- a real interactive terminal emulator with a visible PTY, for example Warp, GNOME Terminal, iTerm2, Alacritty, or equivalent
- workspace with no unrelated edits staged for this evidence commit
- enough free disk for validation if the operator chooses to run Cargo checks
- a safe/redacted environment: no secrets visible in prompt, env dump, scrollback, screenshots, or copied transcript
- root `vac` binary available through the repo workflow or previously built artifact
- permission to execute the `maintenance.tui-pty-gate` workflow or the equivalent root `vac` dogfood steps

Do not use a headless CI log as pass evidence. If the terminal cannot provide a real PTY, record `BLOCKED-OPERATOR` again rather than pass.

## One-round operator checklist

Run from the repository root:

```bash
cd Documents/VAC/vastar-agentic-cli
git status --porcelain -b
sed -n '1,220p' .vac/workflows/maintenance.tui-pty-gate.yaml
```

Then in a real terminal/PTY:

1. Start the PTY gate workflow if the workflow runner is available.
2. If the workflow runner is not available, start the root `vac` TUI directly using the current documented local command for this checkout.
3. Confirm the TUI opens on the alternate screen without panic.
4. Press `/`.
5. Confirm the slash-command list becomes visible and layout remains stable.
6. Type a short safe input such as `pty gate smoke test` and confirm it reaches the composer. Do not submit to a live external service unless test credentials/config are intentionally active.
7. Press `Ctrl-C`.
8. Confirm the process exits cleanly and the terminal prompt/screen is restored.
9. Save redacted evidence in `docs/workflow-control-plane/plans/23-evidence/`.

## Evidence file format for the human run

Create a new Markdown file named like:

```text
docs/workflow-control-plane/plans/23-evidence/YYYY-MM-DD-operator-pty-run.md
```

Use this template:

````markdown
# Plan 23 Evidence — operator PTY run

Date: YYYY-MM-DD
Operator: <name or handle>
Environment: <OS, terminal emulator, shell, local/remote>
Repo branch/ref: <branch + commit hash>
Workflow: maintenance.tui-pty-gate
Result: PASS | FAIL | BLOCKED-OPERATOR

## Commands

```bash
<commands actually run>
```

## Observations

- Root `vac` launched in a real PTY: yes/no
- Alternate screen entered: yes/no
- Slash-command list visible after `/`: yes/no
- Composer accepted safe input: yes/no
- Ctrl-C exited cleanly: yes/no
- Terminal restored after exit: yes/no
- Evidence redacted/private: yes/no

## Evidence

Attach or paste redacted transcript/screenshot summary here. Do not include tokens, keys, private prompts, or full environment dumps.

## Follow-up

- If PASS: update Plan 23 status from evidence pending to operator evidence captured.
- If FAIL: link the failing behavior and keep Plan 23 not done.
- If BLOCKED-OPERATOR: state the missing terminal/runtime requirement precisely.
````

## Status impact

Plan 23 remains not release-complete from this lane. The status should stay equivalent to:

> implementation hardened; real operator evidence pending; L19 added the human evidence runbook and read-only workflow verification.

