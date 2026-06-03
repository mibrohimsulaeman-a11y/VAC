# Executor prompt — 00D rewire root `vac` TUI path

You are working in `/home/emp/Documents/VAC/vastar-agentic-cli`.

Goal: rewire root TUI prompt/activity/approval/validation path to Local Runtime Contract.

Read first:

```text
docs/migration/VAC_TUI_REWIRE_SPEC.md
docs/tui/APPROVAL_SURFACE_CONTRACT.md
docs/tui/EVIDENCE_SURFACE_CONTRACT.md
docs/validation/TUI_PTY_DOGFOOD_GATE.md
```

Constraints:

- one product TUI only,
- do not wire donor shell stack,
- no backend-only success claim,
- validate real operator flow when build allows.

Validation:

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
# Root TUI PTY/operator gate; there is no `vac tui` subcommand.
# Follow docs/validation/TUI_PTY_DOGFOOD_GATE.md for the interactive flow.
./target/debug/vac
```

PTY gate:

```text
type / + Enter
verify slash-command list visible
type normal prompt + Enter
verify task/activity
Ctrl-C clean exit
```

Final response format:

```text
status
files changed
TUI path fixed
tests run
PTY result
remaining blocker
UX impact
```
