# Release Gate

## Goal

Prevent release claims without local runtime, TUI, policy, and workflow validation.

## Required gates

```text
build gate
local runtime gate
TUI PTY dogfood gate
capability dashboard gate
workflow browser gate
approval/policy gate
identity/docs hygiene gate
packaging smoke when available
```

## Minimum commands

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
# Root TUI PTY/operator gate; there is no `vac tui` subcommand.
# Follow docs/validation/TUI_PTY_DOGFOOD_GATE.md for the interactive flow.
./target/debug/vac
```

## Cannot claim if failed

```text
release ready
installed command ready
production ready
```
