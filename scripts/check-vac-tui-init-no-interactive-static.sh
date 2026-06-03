#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "tui init no-interactive gate: $*" >&2; exit 1; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() { ! grep -R -qE "$1" "$2" || fail "forbidden pattern in $2: $1"; }

# TUI /init is already an interactive/operator surface. It must not require
# the CLI-only `vac init --interactive` spelling.
require_grep 'SlashCommand::Init => "start VAC-Init guided setup in the TUI \(no --interactive flag\)"' vac-rs/crates/surfaces/tui/src/slash_command.rs
require_grep 'TUI_VAC_INIT_OPERATOR_CHOICES' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'surface: tui' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'source: tui_slash_init' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'requires_cli_interactive_flag: false' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'run `vac init` without `--interactive`' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs
require_grep 'slash_init_prepares_tui_vac_init_without_cli_interactive_flag' vac-rs/crates/surfaces/tui/src/chatwidget/tests/slash_commands.rs
require_grep 'requires_cli_interactive_flag: false' vac-rs/crates/surfaces/tui/src/chatwidget/tests/slash_commands.rs
reject_grep 'vac init --interactive' vac-rs/crates/surfaces/tui/src
reject_grep 'Skipping /init' vac-rs/crates/surfaces/tui/src/chatwidget/slash_dispatch.rs

printf 'tui init no-interactive gate: PASS\n'
