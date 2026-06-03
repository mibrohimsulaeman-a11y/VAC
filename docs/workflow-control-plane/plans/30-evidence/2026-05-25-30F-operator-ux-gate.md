# Plan 30F — Operator UX Gate Evidence

Date: 2026-05-25T06:25:42+07:00
Repo: `/home/emp/Documents/VAC/vastar-agentic-cli`
Agent lane: `P-30F — Operator UX Gate`
Result: `BLOCKED-OPERATOR`
Typed state: `blocked_operator`

## Decision

`BLOCKED-OPERATOR: no interactive TTY available in this environment`

This is a truthful non-pass result for Plan 30F. The current execution environment is headless/non-interactive, so the real alternate-screen root `vac` TUI smoke was not run and is not claimed as passed.

## Runbook inspected

Required runbook inspected with:

`sed -n '1,240p' docs/validation/TUI_PTY_DOGFOOD_GATE.md`

Relevant runbook rule followed: MCP shells, automated CI scripts, and containerized/headless sandboxes must not fake PTY alternate-screen validation; they must record `BLOCKED-OPERATOR` when no real operator TTY is available.

## TTY/environment evidence

```text
=== ENV evidence ===
2026-05-25T06:25:42+07:00
/home/emp/Documents/VAC/vastar-agentic-cli
TERM=dumb
not a tty
stdin_tty=1
stdout_tty=1
SSH_AGENT_LAUNCHER=gnome-keyring
SSH_AUTH_SOCK=/run/user/1000/keyring/ssh
```

## Operator smoke result

- Real TTY available: no
- Interactive root `vac` TUI launched: no
- Fake/headless interactive smoke attempted: no
- Manual pass claimed: no
- Result recorded: `blocked_operator` / `BLOCKED-OPERATOR`

## Validation

No Rust code was touched, so no Cargo validation was required or run.

YAML validation/sanity performed after updating `.vac/workflows/maintenance.tui-pty-gate.yaml`:

- PyYAML parse if `yaml` module is available.
- Otherwise grep/sed sanity checks for `result_state: blocked_operator`, `BLOCKED-OPERATOR`, and this evidence path.

## Plan 31 impact

If `BLOCKED-OPERATOR` is accepted as the terminal Plan 30F state, Plan 30F no longer blocks Plan 31 on its own. If release policy requires a real TTY pass, Plan 31 remains blocked pending human/operator PTY verification.
